use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::models::Payout;

pub struct NewPayout<'a> {
    pub merchant_id: Uuid,
    pub amount_sats: i64,
    pub currency: &'a str,
    pub fiat_amount_minor: i64,
    pub btc_price_fiat: f64,
    pub provider: &'a str,
    pub destination_masked: &'a str,
}

/// Locks the merchant row so concurrent cash-outs are serialised, then returns
/// sats available to cash out: paid sales minus previous (demo) payouts.
pub async fn lock_available_sats(conn: &mut PgConnection, merchant_id: Uuid) -> sqlx::Result<i64> {
    sqlx::query("SELECT id FROM merchants WHERE id = $1 FOR UPDATE")
        .bind(merchant_id)
        .execute(&mut *conn)
        .await?;
    available_sats(conn, merchant_id).await
}

pub async fn available_sats(conn: &mut PgConnection, merchant_id: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "SELECT
           COALESCE((SELECT SUM(amount_sats) FROM payments
                     WHERE merchant_id = $1 AND status = 'paid' AND is_demo = false), 0)::BIGINT
         - COALESCE((SELECT SUM(amount_sats) FROM payouts WHERE merchant_id = $1), 0)::BIGINT",
    )
    .bind(merchant_id)
    .fetch_one(conn)
    .await
}

pub async fn paid_out_sats(db: &PgPool, merchant_id: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_sats), 0)::BIGINT FROM payouts WHERE merchant_id = $1",
    )
    .bind(merchant_id)
    .fetch_one(db)
    .await
}

pub async fn insert(conn: &mut PgConnection, p: NewPayout<'_>) -> sqlx::Result<Payout> {
    sqlx::query_as::<_, Payout>(
        "INSERT INTO payouts
           (id, merchant_id, amount_sats, currency, fiat_amount_minor, btc_price_fiat, provider, destination_masked)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id, merchant_id, amount_sats, currency, fiat_amount_minor, btc_price_fiat,
                   provider, destination_masked, status, is_demo, created_at",
    )
    .bind(Uuid::new_v4())
    .bind(p.merchant_id)
    .bind(p.amount_sats)
    .bind(p.currency)
    .bind(p.fiat_amount_minor)
    .bind(p.btc_price_fiat)
    .bind(p.provider)
    .bind(p.destination_masked)
    .fetch_one(conn)
    .await
}

pub async fn list(db: &PgPool, merchant_id: Uuid, limit: i64) -> sqlx::Result<Vec<Payout>> {
    sqlx::query_as::<_, Payout>(
        "SELECT id, merchant_id, amount_sats, currency, fiat_amount_minor, btc_price_fiat,
                provider, destination_masked, status, is_demo, created_at
         FROM payouts WHERE merchant_id = $1 ORDER BY created_at DESC LIMIT $2",
    )
    .bind(merchant_id)
    .bind(limit)
    .fetch_all(db)
    .await
}
