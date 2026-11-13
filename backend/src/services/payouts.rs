//! DEMO cash-out to mobile money / bank.
//!
//! No money moves. In production this step would be the merchant's own node paying
//! a cash-out provider (e.g. a mobile-money Lightning address) with a separate,
//! spend-capable credential, limits and confirmation. Soko's invoice-only macaroon
//! cannot spend, which is why this is a recorded simulation.

use sqlx::PgPool;

use crate::{
    errors::{ApiError, ApiResult},
    fx::{RateService, currencies},
    models::{Merchant, Payout},
    repositories::payouts::{self as repo, NewPayout},
    services::money,
};

pub const MIN_PAYOUT_SATS: i64 = 100;

pub struct PayoutRequest<'a> {
    pub amount_sats: Option<i64>,
    pub fiat_amount: Option<&'a str>,
    pub provider: &'a str,
    pub destination: &'a str,
}

pub async fn create_demo_payout(
    db: &PgPool,
    fx: &RateService,
    merchant: &Merchant,
    req: PayoutRequest<'_>,
) -> ApiResult<Payout> {
    let cur = money::currency(&merchant.currency)?;
    if !cur.payout_providers.contains(&req.provider) {
        return Err(ApiError::Validation(format!(
            "Choose a cash-out method available for {}",
            cur.code
        )));
    }
    let destination = mask_destination(req.destination)
        .ok_or_else(|| ApiError::Validation("Enter a valid phone or account number".into()))?;

    let quote = fx
        .quote(cur.code)
        .await
        .filter(|q| q.fresh_for_pricing())
        .ok_or(ApiError::RateUnavailable)?;
    let amount_sats = match (
        req.amount_sats,
        req.fiat_amount.map(str::trim).filter(|s| !s.is_empty()),
    ) {
        (Some(s), None) => s,
        (None, Some(input)) => {
            let minor = currencies::parse_amount(input, cur.decimals).ok_or_else(|| {
                ApiError::Validation(format!("Enter a valid amount in {}", cur.code))
            })?;
            currencies::sats_for_fiat(minor, cur.decimals, quote.btc_price)
        }
        _ => return Err(ApiError::Validation("Enter an amount to cash out".into())),
    };
    if amount_sats < MIN_PAYOUT_SATS {
        return Err(ApiError::Validation(format!(
            "The minimum cash-out is {MIN_PAYOUT_SATS} sats"
        )));
    }

    // Serialise cash-outs per merchant so two requests can't both spend the same balance.
    let mut tx = db.begin().await?;
    let available = repo::lock_available_sats(&mut tx, merchant.id).await?;
    if amount_sats > available {
        return Err(ApiError::Validation(
            "That's more than your available balance".into(),
        ));
    }
    let payout = repo::insert(
        &mut tx,
        NewPayout {
            merchant_id: merchant.id,
            amount_sats,
            currency: cur.code,
            fiat_amount_minor: currencies::fiat_for_sats(
                amount_sats,
                cur.decimals,
                quote.btc_price,
            ),
            btc_price_fiat: quote.btc_price,
            provider: req.provider,
            destination_masked: &destination,
        },
    )
    .await?;
    tx.commit().await?;
    tracing::info!(payout_id = %payout.id, amount_sats, "demo payout recorded (no funds moved)");
    Ok(payout)
}

/// Keeps only the last 4 digits; the full number is never stored.
pub fn mask_destination(input: &str) -> Option<String> {
    let digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
    let allowed = input
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, ' ' | '+' | '-' | '(' | ')'));
    if !allowed || !(6..=20).contains(&digits.len()) {
        return None;
    }
    Some(format!("•••• {}", &digits[digits.len() - 4..]))
}

#[cfg(test)]
mod tests {
    use super::mask_destination;

    #[test]
    fn masks_numbers() {
        assert_eq!(
            mask_destination("+254 712 345 678").as_deref(),
            Some("•••• 5678")
        );
        assert_eq!(mask_destination("0123456789").as_deref(), Some("•••• 6789"));
        assert!(mask_destination("12345").is_none());
        assert!(mask_destination("0712abc678").is_none());
        assert!(mask_destination(&"1".repeat(25)).is_none());
    }
}
