use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Merchant;

macro_rules! merchant_columns {
    () => {
        "id, business_name, email, password_hash, nostr_pubkey, currency, created_at"
    };
}

/// Returns `Ok(None)` if the email is already registered.
pub async fn insert(
    db: &PgPool,
    business_name: &str,
    email: &str,
    password_hash: &str,
) -> sqlx::Result<Option<Merchant>> {
    sqlx::query_as::<_, Merchant>(concat!(
        "INSERT INTO merchants (id, business_name, email, password_hash)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT ((lower(email))) DO NOTHING
         RETURNING ",
        merchant_columns!(),
        ""
    ))
    .bind(Uuid::new_v4())
    .bind(business_name)
    .bind(email)
    .bind(password_hash)
    .fetch_optional(db)
    .await
}

pub async fn find_by_email(db: &PgPool, email: &str) -> sqlx::Result<Option<Merchant>> {
    sqlx::query_as::<_, Merchant>(concat!(
        "SELECT ",
        merchant_columns!(),
        " FROM merchants WHERE lower(email) = lower($1)"
    ))
    .bind(email)
    .fetch_optional(db)
    .await
}

pub async fn find_by_nostr_pubkey(db: &PgPool, pubkey: &str) -> sqlx::Result<Option<Merchant>> {
    sqlx::query_as::<_, Merchant>(concat!(
        "SELECT ",
        merchant_columns!(),
        " FROM merchants WHERE nostr_pubkey = $1"
    ))
    .bind(pubkey)
    .fetch_optional(db)
    .await
}

pub async fn ids_missing_nostr_pubkey(db: &PgPool) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar("SELECT id FROM merchants WHERE nostr_pubkey IS NULL")
        .fetch_all(db)
        .await
}

pub async fn set_nostr_pubkey(db: &PgPool, id: Uuid, pubkey: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE merchants SET nostr_pubkey = $2 WHERE id = $1 AND nostr_pubkey IS NULL")
        .bind(id)
        .bind(pubkey)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn set_currency(db: &PgPool, id: Uuid, currency: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE merchants SET currency = $2 WHERE id = $1")
        .bind(id)
        .bind(currency)
        .execute(db)
        .await?;
    Ok(())
}
