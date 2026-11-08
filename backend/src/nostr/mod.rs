//! Nostr-signed receipts (NIP-01 events, BIP-340 Schnorr signatures).
//!
//! Privacy rules for the public event:
//! - Included: receipt number, amount, unit, paid-at time, and a salted commitment.
//! - Never included: payment hash, preimage, invoice, description, customer data.
//!
//! The commitment is SHA-256(domain || salt || payment_hash). Only someone who knows
//! both the salt (shown on the receipt page) and the payment hash (in the payer's
//! wallet) can link the event to a Lightning payment.
//!
//! Merchant signing keys are derived from a server secret with HMAC, so no private
//! key is ever stored in the database.

use anyhow::{Context, Result, anyhow, bail};
use hmac::{Hmac, Mac};
use secp256k1::{Keypair, Secp256k1, XOnlyPublicKey, schnorr::Signature};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// NIP-78 application-specific data (addressable).
pub const RECEIPT_KIND: u32 = 30078;
pub const RECEIPT_TAG: &str = "soko-receipt";
const KEY_DOMAIN: &[u8] = b"soko/nostr/merchant-key/v1/";
const COMMITMENT_DOMAIN: &[u8] = b"soko/receipt-commitment/v1/";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub id: String,
    pub pubkey: String,
    pub created_at: i64,
    pub kind: u32,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
}

/// Public receipt fields carried in the event content.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReceiptContent {
    #[serde(rename = "type")]
    pub kind: String,
    pub version: u32,
    pub receipt_number: String,
    pub amount: i64,
    pub unit: String,
    pub paid_at: i64,
    pub commitment: String,
}

pub struct NostrSigner {
    master: [u8; 32],
    secp: Secp256k1<secp256k1::All>,
}

impl NostrSigner {
    pub fn new(master: [u8; 32]) -> Self {
        Self {
            master,
            secp: Secp256k1::new(),
        }
    }

    pub fn from_hex(hex_secret: &str) -> Result<Self> {
        let bytes = hex::decode(hex_secret.trim()).context("SOKO_NOSTR_SECRET must be hex")?;
        let master: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow!("SOKO_NOSTR_SECRET must be 32 bytes (64 hex characters)"))?;
        Ok(Self::new(master))
    }

    fn keypair_for(&self, merchant_id: Uuid) -> Keypair {
        // A counter makes derivation total; an invalid scalar is ~2^-128 likely.
        for counter in 0u8.. {
            let mut mac = Hmac::<Sha256>::new_from_slice(&self.master).expect("any key length");
            mac.update(KEY_DOMAIN);
            mac.update(merchant_id.as_bytes());
            mac.update(&[counter]);
            let seed: [u8; 32] = mac.finalize().into_bytes().into();
            if let Ok(kp) = Keypair::from_seckey_byte_array(&self.secp, seed) {
                return kp;
            }
        }
        unreachable!("no valid key after 256 attempts")
    }

    pub fn pubkey_hex(&self, merchant_id: Uuid) -> String {
        hex::encode(
            self.keypair_for(merchant_id)
                .x_only_public_key()
                .0
                .serialize(),
        )
    }

    pub fn sign(
        &self,
        merchant_id: Uuid,
        created_at: i64,
        kind: u32,
        tags: Vec<Vec<String>>,
        content: String,
    ) -> Event {
        let kp = self.keypair_for(merchant_id);
        let pubkey = hex::encode(kp.x_only_public_key().0.serialize());
        let id = event_id(&pubkey, created_at, kind, &tags, &content);
        let sig = self.secp.sign_schnorr(&id, &kp);
        Event {
            id: hex::encode(id),
            pubkey,
            created_at,
            kind,
            tags,
            content,
            sig: hex::encode(sig.to_byte_array()),
        }
    }

    pub fn sign_receipt(&self, merchant_id: Uuid, content: &ReceiptContent) -> Event {
        let tags = vec![
            vec![
                "d".into(),
                format!("{RECEIPT_TAG}:{}", content.receipt_number),
            ],
            vec!["t".into(), RECEIPT_TAG.into()],
            vec![
                "alt".into(),
                format!(
                    "Soko payment receipt {} for {} sats",
                    content.receipt_number, content.amount
                ),
            ],
        ];
        let body = serde_json::to_string(content).expect("serialisable");
        self.sign(merchant_id, content.paid_at, RECEIPT_KIND, tags, body)
    }
}

/// NIP-01 event id: SHA-256 of the compact JSON `[0, pubkey, created_at, kind, tags, content]`.
pub fn event_id(
    pubkey: &str,
    created_at: i64,
    kind: u32,
    tags: &[Vec<String>],
    content: &str,
) -> [u8; 32] {
    let serialized = json!([0, pubkey, created_at, kind, tags, content]).to_string();
    Sha256::digest(serialized.as_bytes()).into()
}

/// Checks the event id and Schnorr signature. Says nothing about who the key belongs to.
pub fn verify_event(event: &Event) -> Result<()> {
    let expected = event_id(
        &event.pubkey,
        event.created_at,
        event.kind,
        &event.tags,
        &event.content,
    );
    if hex::encode(expected) != event.id.to_lowercase() {
        bail!("event id does not match its contents");
    }
    let pk_bytes: [u8; 32] = hex::decode(&event.pubkey)
        .ok()
        .and_then(|b| b.try_into().ok())
        .context("invalid pubkey")?;
    let pubkey = XOnlyPublicKey::from_byte_array(pk_bytes).context("invalid pubkey")?;
    let sig_bytes: [u8; 64] = hex::decode(&event.sig)
        .ok()
        .and_then(|b| b.try_into().ok())
        .context("invalid signature encoding")?;
    Secp256k1::verification_only()
        .verify_schnorr(&Signature::from_byte_array(sig_bytes), &expected, &pubkey)
        .map_err(|_| anyhow!("signature is not valid"))
}

pub fn receipt_commitment(salt: &[u8], payment_hash_hex: &str) -> Result<String> {
    let hash = hex::decode(payment_hash_hex).context("invalid payment hash")?;
    let mut h = Sha256::new();
    h.update(COMMITMENT_DOMAIN);
    h.update(salt);
    h.update(&hash);
    Ok(hex::encode(h.finalize()))
}

/// bech32 `npub` form of a hex public key (NIP-19).
pub fn npub(pubkey_hex: &str) -> Option<String> {
    let bytes = hex::decode(pubkey_hex).ok()?;
    let hrp = bech32::Hrp::parse("npub").ok()?;
    bech32::encode::<bech32::Bech32>(hrp, &bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signer() -> NostrSigner {
        NostrSigner::new([42u8; 32])
    }

    fn content() -> ReceiptContent {
        ReceiptContent {
            kind: "soko.receipt".into(),
            version: 1,
            receipt_number: "SK-ABCDE-23456".into(),
            amount: 5000,
            unit: "sat".into(),
            paid_at: 1_762_620_000,
            commitment: "00".repeat(32),
        }
    }

    #[test]
    fn keys_are_deterministic_and_per_merchant() {
        let s = signer();
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        assert_eq!(s.pubkey_hex(a), s.pubkey_hex(a));
        assert_ne!(s.pubkey_hex(a), s.pubkey_hex(b));
        assert_ne!(NostrSigner::new([7u8; 32]).pubkey_hex(a), s.pubkey_hex(a));
    }

    #[test]
    fn signed_receipt_verifies_and_tampering_is_detected() {
        let ev = signer().sign_receipt(Uuid::from_u128(1), &content());
        verify_event(&ev).unwrap();
        assert_eq!(ev.kind, RECEIPT_KIND);

        let mut tampered = ev.clone();
        tampered.content = tampered.content.replace("5000", "50000");
        assert!(verify_event(&tampered).is_err(), "changed content");

        let mut forged = ev.clone();
        forged.pubkey = signer().pubkey_hex(Uuid::from_u128(2));
        forged.id = hex::encode(event_id(
            &forged.pubkey,
            forged.created_at,
            forged.kind,
            &forged.tags,
            &forged.content,
        ));
        assert!(
            verify_event(&forged).is_err(),
            "signature from a different key"
        );
    }

    #[test]
    fn public_event_contains_no_payment_secrets() {
        let hash = "ab".repeat(32);
        let mut c = content();
        c.commitment = receipt_commitment(&[9u8; 32], &hash).unwrap();
        let ev = signer().sign_receipt(Uuid::from_u128(1), &c);
        let json = serde_json::to_string(&ev).unwrap();
        assert!(!json.contains(&hash), "payment hash must not appear");
        assert!(!json.contains("lnbc"), "invoice must not appear");
    }

    #[test]
    fn commitment_depends_on_salt_and_hash() {
        let h1 = "ab".repeat(32);
        let h2 = "cd".repeat(32);
        let c = receipt_commitment(&[1; 32], &h1).unwrap();
        assert_eq!(c, receipt_commitment(&[1; 32], &h1).unwrap());
        assert_ne!(c, receipt_commitment(&[2; 32], &h1).unwrap());
        assert_ne!(c, receipt_commitment(&[1; 32], &h2).unwrap());
    }

    #[test]
    fn event_id_matches_nip01_serialisation() {
        // Escaping per NIP-01: quotes, backslashes and newlines escaped; UTF-8 kept raw.
        let id = event_id(
            "ab",
            1,
            1,
            &[vec!["t".into(), "x".into()]],
            "hi \"there\"\n😀",
        );
        let expected = Sha256::digest(r#"[0,"ab",1,1,[["t","x"]],"hi \"there\"\n😀"]"#.as_bytes());
        assert_eq!(id, <[u8; 32]>::from(expected));
    }

    #[test]
    fn npub_encoding() {
        let n = npub(&"00".repeat(32)).unwrap();
        assert!(n.starts_with("npub1"));
        assert_eq!(n.len(), 63);
    }
}
