//! Local-currency pricing. The server always does the conversion with its own live
//! rate; a client-supplied rate is never trusted.

use serde::Serialize;

use crate::{
    errors::{ApiError, ApiResult},
    fx::{
        RateService,
        currencies::{self, Currency},
    },
};

/// Local-currency value of an amount at a point in time.
#[derive(Clone, Debug)]
pub struct FiatSnapshot {
    pub currency: &'static str,
    pub minor: i64,
    pub btc_price: f64,
}

/// What the API returns for a local-currency amount.
#[derive(Clone, Debug, Serialize)]
pub struct FiatValue {
    pub currency: String,
    /// Decimal string in major units, e.g. "1500.50".
    pub amount: String,
    /// Price of 1 BTC in `currency` used for the conversion.
    pub btc_price: f64,
}

impl FiatValue {
    pub fn new(currency: &str, minor: i64, btc_price: f64) -> Self {
        let decimals = currencies::find(currency).map_or(2, |c| c.decimals);
        Self {
            currency: currency.to_string(),
            amount: currencies::format_amount(minor, decimals),
            btc_price,
        }
    }

    pub fn from_parts(
        currency: Option<&str>,
        minor: Option<i64>,
        btc_price: Option<f64>,
    ) -> Option<Self> {
        Some(Self::new(currency?, minor?, btc_price?))
    }
}

pub fn currency(code: &str) -> ApiResult<&'static Currency> {
    currencies::find(code).ok_or_else(|| ApiError::Validation("Unsupported currency".into()))
}

/// Decides the invoice amount in sats from exactly one of `amount_sats` or
/// `fiat_amount` (in the merchant's currency), and the local-currency snapshot.
pub async fn price_invoice(
    fx: &RateService,
    merchant_currency: &str,
    amount_sats: Option<i64>,
    fiat_amount: Option<&str>,
) -> ApiResult<(i64, Option<FiatSnapshot>)> {
    let cur = currency(merchant_currency)?;
    let quote = fx.quote(cur.code).await;
    match (
        amount_sats,
        fiat_amount.map(str::trim).filter(|s| !s.is_empty()),
    ) {
        (Some(sats), None) => {
            // Record the local value too, if we have a fresh rate. Optional.
            let snapshot = quote
                .filter(|q| q.fresh_for_pricing())
                .map(|q| FiatSnapshot {
                    currency: cur.code,
                    minor: currencies::fiat_for_sats(sats, cur.decimals, q.btc_price),
                    btc_price: q.btc_price,
                });
            Ok((sats, snapshot))
        }
        (None, Some(input)) => {
            let minor = currencies::parse_amount(input, cur.decimals)
                .filter(|m| *m > 0)
                .ok_or_else(|| {
                    ApiError::Validation(format!("Enter a valid amount in {}", cur.code))
                })?;
            let q = quote
                .filter(|q| q.fresh_for_pricing())
                .ok_or(ApiError::RateUnavailable)?;
            let sats = currencies::sats_for_fiat(minor, cur.decimals, q.btc_price);
            Ok((
                sats,
                Some(FiatSnapshot {
                    currency: cur.code,
                    minor,
                    btc_price: q.btc_price,
                }),
            ))
        }
        _ => Err(ApiError::Validation(format!(
            "Enter the amount either in sats or in {}",
            cur.code
        ))),
    }
}

/// Current local value of an amount in sats (for totals), if a rate is known.
pub async fn current_value(fx: &RateService, currency: &str, sats: i64) -> Option<FiatValue> {
    let cur = currencies::find(currency)?;
    let q = fx.quote(cur.code).await?;
    Some(FiatValue::new(
        cur.code,
        currencies::fiat_for_sats(sats, cur.decimals, q.btc_price),
        q.btc_price,
    ))
}
