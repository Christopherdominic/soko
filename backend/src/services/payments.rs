//! Payment creation and verification.
//!
//! The only way a payment leaves `pending` is through `apply_node_invoice` or
//! `reconcile_payment`, and both act solely on data fetched from the Lightning node.

use chrono::{Duration, Utc};
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ApiError, ApiResult},
    lightning::{InvoiceState, LightningBackend, NodeInvoice},
    models::{Payment, PaymentStatus},
    repositories::payments::{self as repo, NewPayment},
    services::money::FiatSnapshot,
};

pub const MIN_AMOUNT_SATS: i64 = 1;
/// 0.1 BTC. A sanity cap for a market-stall MVP.
pub const MAX_AMOUNT_SATS: i64 = 10_000_000;
pub const MAX_DESCRIPTION_CHARS: usize = 200;
/// An invoice still OPEN this long after its expiry is treated as expired.
const EXPIRY_GRACE: Duration = Duration::seconds(30);

/// Validates and normalises invoice input.
pub fn validate_invoice_input(
    amount_sats: i64,
    description: Option<&str>,
) -> ApiResult<(i64, Option<String>)> {
    if !(MIN_AMOUNT_SATS..=MAX_AMOUNT_SATS).contains(&amount_sats) {
        return Err(ApiError::Validation(format!(
            "Amount must be between {MIN_AMOUNT_SATS} and {MAX_AMOUNT_SATS} sats"
        )));
    }
    let description = description
        .map(|d| d.chars().filter(|c| !c.is_control()).collect::<String>())
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty());
    if let Some(d) = &description
        && d.chars().count() > MAX_DESCRIPTION_CHARS
    {
        return Err(ApiError::Validation(format!(
            "Description must be at most {MAX_DESCRIPTION_CHARS} characters"
        )));
    }
    Ok((amount_sats, description))
}

pub async fn create_invoice(
    db: &PgPool,
    lightning: &dyn LightningBackend,
    merchant_id: Uuid,
    amount_sats: i64,
    description: Option<&str>,
    expiry_secs: u32,
    fiat: Option<FiatSnapshot>,
) -> ApiResult<Payment> {
    let (amount_sats, description) = validate_invoice_input(amount_sats, description)?;
    let created = lightning
        .create_invoice(amount_sats, description.as_deref(), expiry_secs)
        .await
        .map_err(ApiError::LightningUnavailable)?;
    let payment = repo::insert(
        db,
        NewPayment {
            merchant_id,
            payment_hash: &created.payment_hash,
            payment_request: &created.payment_request,
            amount_sats,
            description: description.as_deref(),
            expires_at: Utc::now() + Duration::seconds(i64::from(expiry_secs)),
            fiat: fiat.as_ref().map(|f| (f.currency, f.minor, f.btc_price)),
        },
    )
    .await?;
    tracing::info!(payment_id = %payment.id, amount_sats, "invoice created");
    Ok(payment)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    MarkedPaid,
    MarkedExpired,
    MarkedFailed,
    /// Nothing to do: still open, or already processed (duplicate event).
    Unchanged,
    /// The node knows this invoice but Soko didn't create it. Ignored.
    NotOurs,
}

/// Applies the node's view of an invoice to our record. Idempotent: replaying the same
/// update (stream duplicates, reconnects, concurrent poll) changes nothing.
pub async fn apply_node_invoice(db: &PgPool, inv: &NodeInvoice) -> anyhow::Result<Outcome> {
    let mut tx = db.begin().await?;
    let Some(payment) = repo::lock_by_hash(&mut tx, &inv.payment_hash).await? else {
        return Ok(Outcome::NotOurs);
    };
    if payment.status != PaymentStatus::Pending || payment.is_demo {
        return Ok(Outcome::Unchanged);
    }

    let outcome = match inv.state {
        InvoiceState::Settled => {
            if inv.value_sats != payment.amount_sats {
                tracing::warn!(payment_id = %payment.id, "node invoice amount does not match record");
                repo::close_pending(&mut tx, payment.id, PaymentStatus::Failed).await?;
                Outcome::MarkedFailed
            } else if inv.amount_paid_msat < payment.amount_sats * 1000 {
                tracing::warn!(payment_id = %payment.id, "invoice settled for less than requested");
                repo::close_pending(&mut tx, payment.id, PaymentStatus::Failed).await?;
                Outcome::MarkedFailed
            } else {
                let settled_at = inv.settled_at.unwrap_or_else(Utc::now);
                if repo::mark_paid(&mut tx, payment.id, inv.amount_paid_msat, settled_at).await? {
                    repo::insert_receipt(&mut tx, &new_receipt_number(), &payment).await?;
                    tracing::info!(payment_id = %payment.id, "payment verified as settled by LND");
                    Outcome::MarkedPaid
                } else {
                    Outcome::Unchanged
                }
            }
        }
        InvoiceState::Canceled => {
            repo::close_pending(&mut tx, payment.id, PaymentStatus::Expired).await?;
            Outcome::MarkedExpired
        }
        InvoiceState::Open | InvoiceState::Accepted => Outcome::Unchanged,
    };
    tx.commit().await?;
    Ok(outcome)
}

/// Asks the node about one pending payment and applies the answer.
pub async fn reconcile_payment(
    db: &PgPool,
    lightning: &dyn LightningBackend,
    payment: &Payment,
) -> anyhow::Result<Outcome> {
    if payment.status != PaymentStatus::Pending || payment.is_demo {
        return Ok(Outcome::Unchanged);
    }
    match lightning.lookup_invoice(&payment.payment_hash).await? {
        None => {
            // The node has no such invoice, so it can never be paid.
            tracing::warn!(payment_id = %payment.id, "invoice not found on node");
            let mut conn = db.acquire().await?;
            repo::close_pending(&mut conn, payment.id, PaymentStatus::Failed).await?;
            Ok(Outcome::MarkedFailed)
        }
        Some(inv) => {
            let outcome = apply_node_invoice(db, &inv).await?;
            let still_open = matches!(inv.state, InvoiceState::Open | InvoiceState::Accepted);
            if outcome == Outcome::Unchanged
                && still_open
                && Utc::now() > payment.expires_at + EXPIRY_GRACE
            {
                // LND refuses payments to expired invoices, so this cannot race a settlement.
                let mut conn = db.acquire().await?;
                if repo::close_pending(&mut conn, payment.id, PaymentStatus::Expired).await? {
                    return Ok(Outcome::MarkedExpired);
                }
            }
            Ok(outcome)
        }
    }
}

/// Re-checks every pending payment with the node. Catches anything the stream missed.
pub async fn reconcile_pending(
    db: &PgPool,
    lightning: &dyn LightningBackend,
) -> anyhow::Result<usize> {
    let pending = repo::list_pending(db, 200).await?;
    let mut changed = 0;
    for p in &pending {
        match reconcile_payment(db, lightning, p).await {
            Ok(Outcome::MarkedPaid | Outcome::MarkedExpired | Outcome::MarkedFailed) => {
                changed += 1
            }
            Ok(_) => {}
            Err(e) => tracing::warn!(payment_id = %p.id, error = %e, "reconcile failed"),
        }
    }
    Ok(changed)
}

/// Human-friendly receipt number, e.g. `SK-7F3K9-QX2MD`. No ambiguous characters.
pub fn new_receipt_number() -> String {
    const ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
    let mut rng = rand::rng();
    let mut part = || -> String {
        (0..5)
            .map(|_| ALPHABET[rng.random_range(0..ALPHABET.len())] as char)
            .collect()
    };
    format!("SK-{}-{}", part(), part())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_amount_bounds() {
        assert!(validate_invoice_input(0, None).is_err());
        assert!(validate_invoice_input(-5, None).is_err());
        assert!(validate_invoice_input(MAX_AMOUNT_SATS + 1, None).is_err());
        assert!(validate_invoice_input(1, None).is_ok());
        assert!(validate_invoice_input(MAX_AMOUNT_SATS, None).is_ok());
    }

    #[test]
    fn normalises_description() {
        let (_, d) = validate_invoice_input(10, Some("  Tomatoes\u{0007} x3 ")).unwrap();
        assert_eq!(d.as_deref(), Some("Tomatoes x3"));
        let (_, d) = validate_invoice_input(10, Some("   ")).unwrap();
        assert_eq!(d, None);
        assert!(validate_invoice_input(10, Some(&"a".repeat(201))).is_err());
    }

    #[test]
    fn receipt_number_format() {
        let n = new_receipt_number();
        assert_eq!(n.len(), 14);
        assert!(n.starts_with("SK-"));
        assert_ne!(n, new_receipt_number());
    }
}
