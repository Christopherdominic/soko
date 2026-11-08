use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PaymentStatus {
    Pending,
    Paid,
    Expired,
    Failed,
}

impl PaymentStatus {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "paid" => Some(Self::Paid),
            "expired" => Some(Self::Expired),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Merchant {
    pub id: Uuid,
    pub business_name: String,
    pub email: String,
    pub password_hash: String,
    pub nostr_pubkey: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Payment {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub payment_hash: String,
    pub payment_request: String,
    pub amount_sats: i64,
    pub description: Option<String>,
    pub status: PaymentStatus,
    pub amount_paid_msat: Option<i64>,
    pub is_demo: bool,
    pub expires_at: DateTime<Utc>,
    pub settled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Receipt {
    pub id: Uuid,
    pub receipt_number: String,
    pub payment_id: Uuid,
    pub merchant_id: Uuid,
    pub amount_sats: i64,
    pub issued_at: DateTime<Utc>,
    /// Private: never published. Lets a payer link the receipt to their payment.
    pub commitment_salt: Option<Vec<u8>>,
    pub nostr_event: Option<serde_json::Value>,
}

/// A payment plus its receipt number, if it has one (history listing).
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct PaymentWithReceipt {
    #[sqlx(flatten)]
    pub payment: Payment,
    pub receipt_number: Option<String>,
}
