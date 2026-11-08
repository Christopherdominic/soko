use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::models::{Payment, PaymentStatus, Receipt};

macro_rules! payment_columns {
    () => {
        "id, merchant_id, payment_hash, payment_request, amount_sats, description, \
     status, amount_paid_msat, is_demo, expires_at, settled_at, created_at, updated_at"
    };
}

pub struct NewPayment<'a> {
    pub merchant_id: Uuid,
    pub payment_hash: &'a str,
    pub payment_request: &'a str,
    pub amount_sats: i64,
    pub description: Option<&'a str>,
    pub expires_at: DateTime<Utc>,
}

pub async fn insert(db: &PgPool, p: NewPayment<'_>) -> sqlx::Result<Payment> {
    sqlx::query_as::<_, Payment>(concat!(
        "INSERT INTO payments
           (id, merchant_id, payment_hash, payment_request, amount_sats, description, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING ",
        payment_columns!(),
        ""
    ))
    .bind(Uuid::new_v4())
    .bind(p.merchant_id)
    .bind(p.payment_hash)
    .bind(p.payment_request)
    .bind(p.amount_sats)
    .bind(p.description)
    .bind(p.expires_at)
    .fetch_one(db)
    .await
}

/// Merchant-scoped lookup: another merchant's payment is indistinguishable from a missing one.
pub async fn find_for_merchant(
    db: &PgPool,
    merchant_id: Uuid,
    payment_hash: &str,
) -> sqlx::Result<Option<Payment>> {
    sqlx::query_as::<_, Payment>(concat!(
        "SELECT ",
        payment_columns!(),
        " FROM payments WHERE merchant_id = $1 AND payment_hash = $2"
    ))
    .bind(merchant_id)
    .bind(payment_hash)
    .fetch_optional(db)
    .await
}

/// Locks the payment row for the rest of the transaction.
pub async fn lock_by_hash(
    conn: &mut PgConnection,
    payment_hash: &str,
) -> sqlx::Result<Option<Payment>> {
    sqlx::query_as::<_, Payment>(concat!(
        "SELECT ",
        payment_columns!(),
        " FROM payments WHERE payment_hash = $1 FOR UPDATE"
    ))
    .bind(payment_hash)
    .fetch_optional(conn)
    .await
}

#[derive(Default)]
pub struct ListFilter {
    pub status: Option<PaymentStatus>,
    pub limit: i64,
    pub offset: i64,
}

pub async fn list_for_merchant(
    db: &PgPool,
    merchant_id: Uuid,
    f: &ListFilter,
) -> sqlx::Result<Vec<Payment>> {
    let mut q: QueryBuilder<Postgres> = QueryBuilder::new(concat!(
        "SELECT ",
        payment_columns!(),
        " FROM payments WHERE merchant_id = "
    ));
    q.push_bind(merchant_id);
    if let Some(status) = f.status {
        q.push(" AND status = ").push_bind(status);
    }
    q.push(" ORDER BY created_at DESC LIMIT ")
        .push_bind(f.limit)
        .push(" OFFSET ")
        .push_bind(f.offset);
    q.build_query_as::<Payment>().fetch_all(db).await
}

pub async fn list_pending(db: &PgPool, limit: i64) -> sqlx::Result<Vec<Payment>> {
    sqlx::query_as::<_, Payment>(concat!(
        "SELECT ",
        payment_columns!(),
        " FROM payments
         WHERE status = 'pending' AND is_demo = false
         ORDER BY created_at ASC LIMIT $1"
    ))
    .bind(limit)
    .fetch_all(db)
    .await
}

/// pending → paid. Returns false if the payment was not pending (already processed).
pub async fn mark_paid(
    conn: &mut PgConnection,
    payment_id: Uuid,
    amount_paid_msat: i64,
    settled_at: DateTime<Utc>,
) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "UPDATE payments
         SET status = 'paid', amount_paid_msat = $2, settled_at = $3, updated_at = now()
         WHERE id = $1 AND status = 'pending'",
    )
    .bind(payment_id)
    .bind(amount_paid_msat)
    .bind(settled_at)
    .execute(conn)
    .await?;
    Ok(res.rows_affected() == 1)
}

/// pending → expired/failed. Returns false if the payment was not pending.
pub async fn close_pending(
    conn: &mut PgConnection,
    payment_id: Uuid,
    status: PaymentStatus,
) -> sqlx::Result<bool> {
    debug_assert!(matches!(
        status,
        PaymentStatus::Expired | PaymentStatus::Failed
    ));
    let res = sqlx::query(
        "UPDATE payments SET status = $2, updated_at = now() WHERE id = $1 AND status = 'pending'",
    )
    .bind(payment_id)
    .bind(status)
    .execute(conn)
    .await?;
    Ok(res.rows_affected() == 1)
}

/// Inserts the receipt unless one already exists for this payment.
pub async fn insert_receipt(
    conn: &mut PgConnection,
    receipt_number: &str,
    payment: &Payment,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO receipts (id, receipt_number, payment_id, merchant_id, amount_sats)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (payment_id) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(receipt_number)
    .bind(payment.id)
    .bind(payment.merchant_id)
    .bind(payment.amount_sats)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn find_receipt(db: &PgPool, payment_id: Uuid) -> sqlx::Result<Option<Receipt>> {
    sqlx::query_as::<_, Receipt>(
        "SELECT id, receipt_number, payment_id, merchant_id, amount_sats, issued_at
         FROM receipts WHERE payment_id = $1",
    )
    .bind(payment_id)
    .fetch_optional(db)
    .await
}
