use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::json::ApiJson;
use crate::{
    errors::{ApiError, ApiResult},
    lightning::lnd::is_hex_hash,
    middleware::auth::CurrentMerchant,
    models::{Payment, PaymentStatus, Receipt},
    repositories::payments::{self as repo, ListFilter},
    services::payments as svc,
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateInvoiceRequest {
    amount_sats: i64,
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
        }
    }
}

/// POST /api/payments/invoices
pub async fn create_invoice(
    State(state): State<AppState>,
    CurrentMerchant(merchant): CurrentMerchant,
    ApiJson(req): ApiJson<CreateInvoiceRequest>,
) -> ApiResult<(StatusCode, Json<PaymentResponse>)> {
    let payment = svc::create_invoice(
        &state.db,
        state.lightning.as_ref(),
        merchant.id,
        req.amount_sats,
        req.description.as_deref(),
        state.config.invoice_expiry_secs,
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
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
pub struct ListResponse {
    payments: Vec<PaymentResponse>,
}

/// GET /api/payments?status=&limit=&offset=
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
    let filter = ListFilter {
        status,
        limit: q.limit.unwrap_or(20).clamp(1, 100),
        offset: q.offset.unwrap_or(0).max(0),
    };
    let payments = repo::list_for_merchant(&state.db, merchant.id, &filter).await?;
    Ok(Json(ListResponse {
        payments: payments
            .into_iter()
            .map(|p| PaymentResponse::new(p, None))
            .collect(),
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
    Ok(Json(ReceiptResponse {
        receipt_number: receipt.receipt_number,
        business_name: merchant.business_name,
        amount_sats: receipt.amount_sats,
        denomination: "sats",
        description: payment.description,
        payment_hash: payment.payment_hash,
        paid_at: payment.settled_at,
        issued_at: receipt.issued_at,
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
