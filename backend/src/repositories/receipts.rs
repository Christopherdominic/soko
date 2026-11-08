use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Receipt;

/// Everything needed to render a receipt publicly (no auth). Deliberately excludes
/// the payment description, payment hash and invoice.
#[derive(Debug, sqlx::FromRow)]
pub struct PublicReceiptRow {
    pub receipt_id: Uuid,
    pub merchant_id: Uuid,
    pub receipt_number: String,
    pub amount_sats: i64,
    pub issued_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
    pub business_name: String,
    pub payment_hash: String,
    pub commitment_salt: Option<Vec<u8>>,
    pub nostr_event: Option<serde_json::Value>,
}

pub async fn find_by_number(
    db: &PgPool,
    receipt_number: &str,
) -> sqlx::Result<Option<PublicReceiptRow>> {
    sqlx::query_as::<_, PublicReceiptRow>(
        "SELECT r.id AS receipt_id, r.merchant_id, r.receipt_number, r.amount_sats, r.issued_at,
                p.settled_at AS paid_at, m.business_name, p.payment_hash,
                r.commitment_salt, r.nostr_event
         FROM receipts r
         JOIN payments p ON p.id = r.payment_id
         JOIN merchants m ON m.id = r.merchant_id
         WHERE r.receipt_number = $1",
    )
    .bind(receipt_number)
    .fetch_optional(db)
    .await
}

/// Stores the signed event and salt the first time only. Returns the stored receipt,
/// so concurrent first views all end up with the same event.
pub async fn attach_nostr(
    db: &PgPool,
    receipt_id: Uuid,
    salt: &[u8],
    event: &serde_json::Value,
) -> sqlx::Result<Receipt> {
    sqlx::query(
        "UPDATE receipts SET commitment_salt = $2, nostr_event = $3
         WHERE id = $1 AND nostr_event IS NULL",
    )
    .bind(receipt_id)
    .bind(salt)
    .bind(event)
    .execute(db)
    .await?;
    sqlx::query_as::<_, Receipt>(
        "SELECT id, receipt_number, payment_id, merchant_id, amount_sats, issued_at,
                commitment_salt, nostr_event
         FROM receipts WHERE id = $1",
    )
    .bind(receipt_id)
    .fetch_one(db)
    .await
}
