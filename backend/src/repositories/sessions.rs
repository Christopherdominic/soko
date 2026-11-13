use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Merchant;

pub async fn insert(
    db: &PgPool,
    token_hash: &[u8],
    merchant_id: Uuid,
    expires_at: DateTime<Utc>,
) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO sessions (token_hash, merchant_id, expires_at) VALUES ($1, $2, $3)")
        .bind(token_hash)
        .bind(merchant_id)
        .bind(expires_at)
        .execute(db)
        .await?;
    Ok(())
}

/// Looks up the merchant for a live (unexpired) session.
pub async fn find_merchant(db: &PgPool, token_hash: &[u8]) -> sqlx::Result<Option<Merchant>> {
    sqlx::query_as::<_, Merchant>(
        "SELECT m.id, m.business_name, m.email, m.password_hash, m.nostr_pubkey, m.currency, m.created_at
         FROM sessions s JOIN merchants m ON m.id = s.merchant_id
         WHERE s.token_hash = $1 AND s.expires_at > now()",
    )
    .bind(token_hash)
    .fetch_optional(db)
    .await
}

pub async fn delete(db: &PgPool, token_hash: &[u8]) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(token_hash)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn delete_expired(db: &PgPool) -> sqlx::Result<u64> {
    Ok(
        sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
            .execute(db)
            .await?
            .rows_affected(),
    )
}
