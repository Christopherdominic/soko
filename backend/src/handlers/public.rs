//! Unauthenticated endpoints for customers: view a receipt, verify a signed receipt.

use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use serde::Serialize;

use super::json::ApiJson;
use crate::{
    errors::{ApiError, ApiResult},
    middleware::rate_limit::{ClientIp, enforce},
    nostr::Event,
    repositories::receipts as repo,
    services::{
        money::FiatValue,
        receipts::{self as svc, ReceiptSource, SignedReceipt, VerifyResult},
    },
    state::AppState,
};

#[derive(Serialize)]
pub struct PublicReceipt {
    receipt_number: String,
    business_name: String,
    amount_sats: i64,
    denomination: &'static str,
    paid_at: Option<DateTime<Utc>>,
    issued_at: DateTime<Utc>,
    /// Local-currency value at the time of payment.
    fiat: Option<FiatValue>,
    nostr: Option<SignedReceipt>,
}

/// GET /api/public/receipts/{receipt_number}
/// Shows only what belongs on a paper receipt: no description, invoice or payment hash.
pub async fn get_receipt(
    State(state): State<AppState>,
    ip: ClientIp,
    Path(number): Path<String>,
) -> ApiResult<Json<PublicReceipt>> {
    ip.enforce(&state.limits, "public")?;
    enforce(&state.limits.public_global, "all")?;
    if !is_receipt_number(&number) {
        return Err(ApiError::NotFound);
    }
    let r = repo::find_by_number(&state.db, &number)
        .await?
        .ok_or(ApiError::NotFound)?;
    let nostr = svc::ensure_signed(
        &state.db,
        state.nostr.as_deref(),
        ReceiptSource {
            receipt_id: r.receipt_id,
            merchant_id: r.merchant_id,
            receipt_number: &r.receipt_number,
            amount_sats: r.amount_sats,
            paid_at: r.paid_at.unwrap_or(r.issued_at),
            payment_hash: &r.payment_hash,
            commitment_salt: r.commitment_salt.as_deref(),
            nostr_event: r.nostr_event.as_ref(),
        },
    )
    .await
    .map_err(ApiError::Internal)?;
    Ok(Json(PublicReceipt {
        receipt_number: r.receipt_number,
        business_name: r.business_name,
        amount_sats: r.amount_sats,
        denomination: "sats",
        paid_at: r.paid_at,
        issued_at: r.issued_at,
        fiat: FiatValue::from_parts(
            r.fiat_currency.as_deref(),
            r.fiat_amount_minor,
            r.btc_price_fiat,
        ),
        nostr,
    }))
}

/// POST /api/public/receipts/verify — body is a Nostr event.
pub async fn verify_receipt(
    State(state): State<AppState>,
    ip: ClientIp,
    ApiJson(event): ApiJson<Event>,
) -> ApiResult<Json<VerifyResult>> {
    ip.enforce(&state.limits, "public")?;
    enforce(&state.limits.public_global, "all")?;
    Ok(Json(
        svc::verify(&state.db, &event)
            .await
            .map_err(ApiError::Internal)?,
    ))
}

pub fn is_receipt_number(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 14
        && s.starts_with("SK-")
        && b[8] == b'-'
        && b.iter().enumerate().all(|(i, c)| {
            i < 3 || i == 8 || matches!(c, b'2'..=b'9' | b'A'..=b'H' | b'J'..=b'N' | b'P'..=b'Z')
        })
}

#[cfg(test)]
mod tests {
    #[test]
    fn receipt_number_format() {
        use super::is_receipt_number;
        assert!(is_receipt_number("SK-2DA4W-3X62V"));
        assert!(!is_receipt_number("SK-2DA4W-3X62"));
        assert!(!is_receipt_number("SK-2DA4W_3X62V"));
        assert!(!is_receipt_number("SK-2da4w-3x62v"));
        assert!(!is_receipt_number("SK-10O4W-3X62V"));
        assert!(!is_receipt_number("../etc/passwd!"));
        for _ in 0..100 {
            assert!(is_receipt_number(
                &crate::services::payments::new_receipt_number()
            ));
        }
    }
}
