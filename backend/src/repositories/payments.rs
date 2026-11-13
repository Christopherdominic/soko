use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgConnection, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::models::{Payment, PaymentStatus, PaymentWithReceipt, Receipt};

macro_rules! payment_columns {
    () => {
        "id, merchant_id, payment_hash, payment_request, amount_sats, description, \
     status, amount_paid_msat, is_demo, expires_at, settled_at, created_at, updated_at, \
     fiat_currency, fiat_amount_minor, btc_price_fiat"
    };
}

pub struct NewPayment<'a> {
    pub merchant_id: Uuid,
    pub payment_hash: &'a str,
    pub payment_request: &'a str,
    pub amount_sats: i64,
    pub description: Option<&'a str>,
    pub expires_at: DateTime<Utc>,
    /// (currency, minor units, BTC price in that currency) at creation time.
    pub fiat: Option<(&'a str, i64, f64)>,
}

pub async fn insert(db: &PgPool, p: NewPayment<'_>) -> sqlx::Result<Payment> {
    sqlx::query_as::<_, Payment>(concat!(
        "INSERT INTO payments
           (id, merchant_id, payment_hash, payment_request, amount_sats, description, expires_at,
            fiat_currency, fiat_amount_minor, btc_price_fiat)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
    .bind(p.fiat.map(|f| f.0))
    .bind(p.fiat.map(|f| f.1))
    .bind(p.fiat.map(|f| f.2))
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

/// History filters. Dates are calendar days in the merchant's time zone (inclusive).
pub struct ListFilter {
    pub status: Option<PaymentStatus>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// Matches description, receipt number, or payment-hash prefix.
    pub search: Option<String>,
    pub tz: String,
    pub limit: i64,
    pub offset: i64,
}

pub async fn list_for_merchant(
    db: &PgPool,
    merchant_id: Uuid,
    f: &ListFilter,
) -> sqlx::Result<Vec<PaymentWithReceipt>> {
    let mut q: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT p.id, p.merchant_id, p.payment_hash, p.payment_request, p.amount_sats, \
         p.description, p.status, p.amount_paid_msat, p.is_demo, p.expires_at, p.settled_at, \
         p.created_at, p.updated_at, p.fiat_currency, p.fiat_amount_minor, p.btc_price_fiat, \
         r.receipt_number \
         FROM payments p LEFT JOIN receipts r ON r.payment_id = p.id WHERE p.merchant_id = ",
    );
    q.push_bind(merchant_id);
    if let Some(status) = f.status {
        q.push(" AND p.status = ").push_bind(status);
    }
    if let Some(from) = f.from {
        q.push(" AND p.created_at >= (")
            .push_bind(from)
            .push("::date)::timestamp AT TIME ZONE ")
            .push_bind(f.tz.clone());
    }
    if let Some(to) = f.to {
        q.push(" AND p.created_at < (")
            .push_bind(to)
            .push("::date + 1)::timestamp AT TIME ZONE ")
            .push_bind(f.tz.clone());
    }
    if let Some(term) = &f.search {
        let like = format!("%{}%", escape_like(term));
        let prefix = format!("{}%", escape_like(&term.to_lowercase()));
        q.push(" AND (p.description ILIKE ")
            .push_bind(like.clone())
            .push(" OR r.receipt_number ILIKE ")
            .push_bind(like)
            .push(" OR p.payment_hash LIKE ")
            .push_bind(prefix)
            .push(")");
    }
    q.push(" ORDER BY p.created_at DESC LIMIT ")
        .push_bind(f.limit)
        .push(" OFFSET ")
        .push_bind(f.offset);
    q.build_query_as::<PaymentWithReceipt>().fetch_all(db).await
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
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
        "SELECT id, receipt_number, payment_id, merchant_id, amount_sats, issued_at,
                commitment_salt, nostr_event
         FROM receipts WHERE payment_id = $1",
    )
    .bind(payment_id)
    .fetch_optional(db)
    .await
}
