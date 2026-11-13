use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::{json::ApiJson, tz};
use crate::{
    errors::{ApiError, ApiResult},
    lightning::lnd::is_hex_hash,
    middleware::{auth::CurrentMerchant, rate_limit::enforce},
    models::{Payment, PaymentStatus, Receipt},
    repositories::payments::{self as repo, ListFilter},
    services::{
        money::{self, FiatValue},
        payments as svc,
        receipts::{self, ReceiptSource, SignedReceipt},
    },
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateInvoiceRequest {
    /// Exactly one of `amount_sats` or `fiat_amount` (merchant's currency).
    amount_sats: Option<i64>,
    fiat_amount: Option<String>,
    description: Option<String>,
}

#[derive(Serialize)]
pub struct PaymentResponse {
    payment_hash: String,
    payment_request: String,
    amount_sats: i64,
    description: Option<String>,
    status: PaymentStatus,
    is_demo: bool,
    expires_at: DateTime<Utc>,
    settled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    receipt_number: Option<String>,
    /// Local-currency value when the invoice was created.
    fiat: Option<FiatValue>,
}

impl PaymentResponse {
    fn new(p: Payment, receipt: Option<&Receipt>) -> Self {
        Self {
            payment_hash: p.payment_hash,
            payment_request: p.payment_request,
            amount_sats: p.amount_sats,
            description: p.description,
            status: p.status,
            is_demo: p.is_demo,
            expires_at: p.expires_at,
            settled_at: p.settled_at,
            created_at: p.created_at,
            receipt_number: receipt.map(|r| r.receipt_number.clone()),
            fiat: FiatValue::from_parts(
                p.fiat_currency.as_deref(),
                p.fiat_amount_minor,
                p.btc_price_fiat,
            ),
        }
    }
}

/// POST /api/payments/invoices
pub async fn create_invoice(
    State(state): State<AppState>,
    CurrentMerchant(merchant): CurrentMerchant,
    ApiJson(req): ApiJson<CreateInvoiceRequest>,
) -> ApiResult<(StatusCode, Json<PaymentResponse>)> {
    enforce(&state.limits.invoice_per_merchant, &merchant.id.to_string())?;
    let (amount_sats, fiat) = money::price_invoice(
        &state.fx,
        &merchant.currency,
        req.amount_sats,
        req.fiat_amount.as_deref(),
    )
    .await?;
    let payment = svc::create_invoice(
        &state.db,
        state.lightning.as_ref(),
        merchant.id,
        amount_sats,
        req.description.as_deref(),
        state.config.invoice_expiry_secs,
        fiat,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(PaymentResponse::new(payment, None)),
    ))
}

/// GET /api/payments/{payment_hash}
/// If the payment is still pending, re-verifies it with LND before responding.
pub async fn get_payment(
    State(state): State<AppState>,
    CurrentMerchant(merchant): CurrentMerchant,
    Path(payment_hash): Path<String>,
) -> ApiResult<Json<PaymentResponse>> {
    let payment = load_payment(&state, merchant.id, &payment_hash).await?;
    let payment = if payment.status == PaymentStatus::Pending {
        if let Err(e) = svc::reconcile_payment(&state.db, state.lightning.as_ref(), &payment).await
        {
            // Fall back to the stored state; the background watcher will retry.
            tracing::warn!(error = %e, "on-demand verification failed");
        }
        load_payment(&state, merchant.id, &payment_hash).await?
    } else {
        payment
    };
    let receipt = repo::find_receipt(&state.db, payment.id).await?;
    Ok(Json(PaymentResponse::new(payment, receipt.as_ref())))
}

#[derive(Deserialize)]
pub struct ListQuery {
    status: Option<String>,
    /// Free-text search: description, receipt number or payment-hash prefix.
    q: Option<String>,
    /// Inclusive calendar dates (YYYY-MM-DD) in `tz`.
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    tz: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
pub struct ListResponse {
    payments: Vec<PaymentResponse>,
    has_more: bool,
}

const MAX_SEARCH_CHARS: usize = 64;

/// GET /api/payments?status=&q=&from=&to=&tz=&limit=&offset=
pub async fn list_payments(
    State(state): State<AppState>,
    CurrentMerchant(merchant): CurrentMerchant,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<ListResponse>> {
    let status = match q.status.as_deref() {
        None | Some("") | Some("all") => None,
        Some(s) => Some(
            PaymentStatus::parse(s)
                .ok_or_else(|| ApiError::Validation("Unknown status filter".into()))?,
        ),
    };
    if let (Some(from), Some(to)) = (q.from, q.to)
        && from > to
    {
        return Err(ApiError::Validation(
            "'From' date must be before 'to' date".into(),
        ));
    }
    let search = q.q.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if search.is_some_and(|s| s.chars().count() > MAX_SEARCH_CHARS) {
        return Err(ApiError::Validation("Search is too long".into()));
    }
    let tz = if q.from.is_some() || q.to.is_some() {
        tz::resolve(&state.db, q.tz.as_deref()).await?
    } else {
        "UTC".into()
    };
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let filter = ListFilter {
        status,
        from: q.from,
        to: q.to,
        search: search.map(str::to_owned),
        tz,
        // Fetch one extra row to know whether there is another page.
        limit: limit + 1,
        offset: q.offset.unwrap_or(0).max(0),
    };
    let mut rows = repo::list_for_merchant(&state.db, merchant.id, &filter).await?;
    let has_more = rows.len() as i64 > limit;
    rows.truncate(limit as usize);
    Ok(Json(ListResponse {
        payments: rows
            .into_iter()
            .map(|r| {
                let mut p = PaymentResponse::new(r.payment, None);
                p.receipt_number = r.receipt_number;
                p
            })
            .collect(),
        has_more,
    }))
}

#[derive(Serialize)]
pub struct ReceiptResponse {
    receipt_number: String,
    business_name: String,
    amount_sats: i64,
    denomination: &'static str,
    description: Option<String>,
    payment_hash: String,
    paid_at: Option<DateTime<Utc>>,
    issued_at: DateTime<Utc>,
    fiat: Option<FiatValue>,
    /// Shareable customer link (no login needed).
    public_path: String,
    nostr: Option<SignedReceipt>,
}

/// GET /api/payments/{payment_hash}/receipt
pub async fn get_receipt(
    State(state): State<AppState>,
    CurrentMerchant(merchant): CurrentMerchant,
    Path(payment_hash): Path<String>,
) -> ApiResult<Json<ReceiptResponse>> {
    let payment = load_payment(&state, merchant.id, &payment_hash).await?;
    let receipt = repo::find_receipt(&state.db, payment.id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let nostr = receipts::ensure_signed(
        &state.db,
        state.nostr.as_deref(),
        ReceiptSource {
            receipt_id: receipt.id,
            merchant_id: merchant.id,
            receipt_number: &receipt.receipt_number,
            amount_sats: receipt.amount_sats,
            paid_at: payment.settled_at.unwrap_or(receipt.issued_at),
            payment_hash: &payment.payment_hash,
            commitment_salt: receipt.commitment_salt.as_deref(),
            nostr_event: receipt.nostr_event.as_ref(),
        },
    )
    .await
    .map_err(ApiError::Internal)?;
    Ok(Json(ReceiptResponse {
        public_path: format!("/r/{}", receipt.receipt_number),
        fiat: FiatValue::from_parts(
            payment.fiat_currency.as_deref(),
            payment.fiat_amount_minor,
            payment.btc_price_fiat,
        ),
        receipt_number: receipt.receipt_number,
        business_name: merchant.business_name,
        amount_sats: receipt.amount_sats,
        denomination: "sats",
        description: payment.description,
        payment_hash: payment.payment_hash,
        paid_at: payment.settled_at,
        issued_at: receipt.issued_at,
        nostr,
    }))
}

async fn load_payment(state: &AppState, merchant_id: uuid::Uuid, hash: &str) -> ApiResult<Payment> {
    if !is_hex_hash(hash) {
        return Err(ApiError::Validation("Invalid payment hash".into()));
    }
    repo::find_for_merchant(&state.db, merchant_id, hash)
        .await?
        .ok_or(ApiError::NotFound)
}
