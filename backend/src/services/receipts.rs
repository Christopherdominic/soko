use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    nostr::{self, Event, NostrSigner, RECEIPT_KIND, RECEIPT_TAG, ReceiptContent},
    repositories::{merchants, receipts},
};

/// Data needed to sign a receipt.
pub struct ReceiptSource<'a> {
    pub receipt_id: Uuid,
    pub merchant_id: Uuid,
    pub receipt_number: &'a str,
    pub amount_sats: i64,
    pub paid_at: DateTime<Utc>,
    pub payment_hash: &'a str,
    pub commitment_salt: Option<&'a [u8]>,
    pub nostr_event: Option<&'a serde_json::Value>,
}

#[derive(Serialize)]
pub struct SignedReceipt {
    pub event: Event,
    pub npub: Option<String>,
    /// Hex salt. With their payment hash, the payer can recompute the commitment.
    pub verification_code: String,
}

/// Returns the receipt's signed Nostr event, creating it on first use.
/// `Ok(None)` when Nostr signing is not configured.
pub async fn ensure_signed(
    db: &PgPool,
    signer: Option<&NostrSigner>,
    src: ReceiptSource<'_>,
) -> anyhow::Result<Option<SignedReceipt>> {
    if let (Some(salt), Some(ev)) = (src.commitment_salt, src.nostr_event) {
        return Ok(Some(to_signed(ev.clone(), salt)?));
    }
    let Some(signer) = signer else {
        return Ok(None);
    };

    let mut salt = [0u8; 32];
    rand::rng().fill_bytes(&mut salt);
    let content = ReceiptContent {
        kind: "soko.receipt".into(),
        version: 1,
        receipt_number: src.receipt_number.to_string(),
        amount: src.amount_sats,
        unit: "sat".into(),
        paid_at: src.paid_at.timestamp(),
        commitment: nostr::receipt_commitment(&salt, src.payment_hash)?,
    };
    let event = signer.sign_receipt(src.merchant_id, &content);
    let stored =
        receipts::attach_nostr(db, src.receipt_id, &salt, &serde_json::to_value(&event)?).await?;
    // Another request may have signed first; always return what is stored.
    match (stored.commitment_salt, stored.nostr_event) {
        (Some(salt), Some(ev)) => Ok(Some(to_signed(ev, &salt)?)),
        _ => anyhow::bail!("receipt signing did not persist"),
    }
}

fn to_signed(ev: serde_json::Value, salt: &[u8]) -> anyhow::Result<SignedReceipt> {
    let event: Event = serde_json::from_value(ev)?;
    Ok(SignedReceipt {
        npub: nostr::npub(&event.pubkey),
        event,
        verification_code: hex::encode(salt),
    })
}

/// Fills in public keys for merchants created before Nostr was configured.
pub async fn backfill_merchant_pubkeys(db: &PgPool, signer: &NostrSigner) -> anyhow::Result<()> {
    for id in merchants::ids_missing_nostr_pubkey(db).await? {
        merchants::set_nostr_pubkey(db, id, &signer.pubkey_hex(id)).await?;
    }
    Ok(())
}

#[derive(Debug, Serialize, Default)]
pub struct VerifyResult {
    /// The event is well-formed and signed by the key in `pubkey`.
    pub signature_valid: bool,
    /// The signing key belongs to a merchant on this Soko server.
    pub issued_by_known_merchant: bool,
    /// The event is the exact receipt this server issued.
    pub matches_records: bool,
    pub business_name: Option<String>,
    pub receipt_number: Option<String>,
    pub amount_sats: Option<i64>,
    pub paid_at: Option<i64>,
    pub npub: Option<String>,
    pub problem: Option<String>,
}

pub async fn verify(db: &PgPool, event: &Event) -> anyhow::Result<VerifyResult> {
    let mut out = VerifyResult::default();
    if let Err(e) = nostr::verify_event(event) {
        out.problem = Some(format!("Invalid signature: {e}"));
        return Ok(out);
    }
    out.signature_valid = true;
    out.npub = nostr::npub(&event.pubkey);

    let is_receipt = event.kind == RECEIPT_KIND
        && event
            .tags
            .iter()
            .any(|t| t.len() >= 2 && t[0] == "t" && t[1] == RECEIPT_TAG);
    let content: Option<ReceiptContent> = serde_json::from_str(&event.content).ok();
    let Some(content) = content.filter(|_| is_receipt) else {
        out.problem = Some("This is not a Soko receipt".into());
        return Ok(out);
    };
    out.receipt_number = Some(content.receipt_number.clone());
    out.amount_sats = Some(content.amount);
    out.paid_at = Some(content.paid_at);

    let Some(merchant) = merchants::find_by_nostr_pubkey(db, &event.pubkey.to_lowercase()).await?
    else {
        out.problem = Some("Signed by a key that is not a merchant on this Soko server".into());
        return Ok(out);
    };
    out.issued_by_known_merchant = true;
    out.business_name = Some(merchant.business_name);

    let stored_id = receipts::find_by_number(db, &content.receipt_number)
        .await?
        .filter(|r| r.merchant_id == merchant.id)
        .and_then(|r| r.nostr_event)
        .and_then(|ev| ev.get("id").and_then(|v| v.as_str()).map(str::to_owned));
    out.matches_records = stored_id.as_deref() == Some(event.id.to_lowercase().as_str());
    if !out.matches_records {
        out.problem = Some("No matching receipt in this merchant's records".into());
    }
    Ok(out)
}
