use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::json::ApiJson;
use crate::{
    errors::{ApiError, ApiResult},
    fx::currencies::{CURRENCIES, Currency},
    middleware::{auth::CurrentMerchant, rate_limit::enforce},
    models::Payout,
    repositories::{merchants, payouts},
    services::{
        money::{self, FiatValue},
        payouts::{self as payout_svc, PayoutRequest},
    },
    state::AppState,
};

#[derive(Serialize)]
pub struct SettingsResponse {
    currency: String,
    currencies: &'static [Currency],
}

/// GET /api/settings
pub async fn get_settings(CurrentMerchant(m): CurrentMerchant) -> Json<SettingsResponse> {
    Json(SettingsResponse {
        currency: m.currency,
        currencies: CURRENCIES,
    })
}

#[derive(Deserialize)]
pub struct UpdateSettings {
    currency: String,
}

/// PUT /api/settings
pub async fn update_settings(
    State(state): State<AppState>,
    CurrentMerchant(m): CurrentMerchant,
    ApiJson(req): ApiJson<UpdateSettings>,
) -> ApiResult<Json<SettingsResponse>> {
    let cur = money::currency(req.currency.trim())?;
    merchants::set_currency(&state.db, m.id, cur.code).await?;
    Ok(Json(SettingsResponse {
        currency: cur.code.into(),
        currencies: CURRENCIES,
    }))
}

#[derive(Serialize)]
pub struct RateResponse {
    currency: String,
    /// Price of 1 BTC in `currency`.
    btc_price: f64,
    /// How many sats one unit of `currency` buys.
    sats_per_unit: f64,
    updated_at: DateTime<Utc>,
    source: &'static str,
    /// False when the price is too old to price new invoices.
    fresh: bool,
}

/// GET /api/rates — live rate for the merchant's currency.
pub async fn get_rate(
    State(state): State<AppState>,
    CurrentMerchant(m): CurrentMerchant,
) -> ApiResult<Json<RateResponse>> {
    let q = state
        .fx
        .quote(&m.currency)
        .await
        .ok_or(ApiError::RateUnavailable)?;
    Ok(Json(RateResponse {
        currency: m.currency,
        btc_price: q.btc_price,
        sats_per_unit: 100_000_000.0 / q.btc_price,
        updated_at: q.updated_at,
        source: q.source,
        fresh: q.fresh_for_pricing(),
    }))
}

#[derive(Serialize)]
pub struct PayoutResponse {
    id: Uuid,
    amount_sats: i64,
    fiat: FiatValue,
    provider: String,
    destination: String,
    status: String,
    /// Always true: no money moves in this version.
    is_demo: bool,
    created_at: DateTime<Utc>,
}

impl From<Payout> for PayoutResponse {
    fn from(p: Payout) -> Self {
        Self {
            id: p.id,
            amount_sats: p.amount_sats,
            fiat: FiatValue::new(&p.currency, p.fiat_amount_minor, p.btc_price_fiat),
            provider: p.provider,
            destination: p.destination_masked,
            status: p.status,
            is_demo: p.is_demo,
            created_at: p.created_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreatePayout {
    amount_sats: Option<i64>,
    fiat_amount: Option<String>,
    provider: String,
    destination: String,
}

/// POST /api/payouts — DEMO cash-out (recorded only, no funds move).
pub async fn create_payout(
    State(state): State<AppState>,
    CurrentMerchant(m): CurrentMerchant,
    ApiJson(req): ApiJson<CreatePayout>,
) -> ApiResult<(StatusCode, Json<PayoutResponse>)> {
    enforce(
        &state.limits.invoice_per_merchant,
        &format!("payout:{}", m.id),
    )?;
    let payout = payout_svc::create_demo_payout(
        &state.db,
        &state.fx,
        &m,
        PayoutRequest {
            amount_sats: req.amount_sats,
            fiat_amount: req.fiat_amount.as_deref(),
            provider: req.provider.trim(),
            destination: &req.destination,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(payout.into())))
}

#[derive(Serialize)]
pub struct PayoutList {
    payouts: Vec<PayoutResponse>,
}

/// GET /api/payouts
pub async fn list_payouts(
    State(state): State<AppState>,
    CurrentMerchant(m): CurrentMerchant,
) -> ApiResult<Json<PayoutList>> {
    let rows = payouts::list(&state.db, m.id, 50).await?;
    Ok(Json(PayoutList {
        payouts: rows.into_iter().map(Into::into).collect(),
    }))
}
