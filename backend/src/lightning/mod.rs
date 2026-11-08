//! Lightning node abstraction. The payment service depends on this trait, not on LND
//! directly, so verification logic can be unit-tested with a fake node.

pub mod lnd;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures_util::stream::BoxStream;

/// Invoice state as reported by the Lightning node — the only source of truth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvoiceState {
    Open,
    /// HTLCs held but not yet settled (hold invoices). Treated as not paid.
    Accepted,
    Settled,
    /// Expired or cancelled by the node.
    Canceled,
}

#[derive(Clone, Debug)]
pub struct CreatedInvoice {
    /// Lowercase hex payment hash.
    pub payment_hash: String,
    pub payment_request: String,
}

#[derive(Clone, Debug)]
pub struct NodeInvoice {
    pub payment_hash: String,
    pub state: InvoiceState,
    pub value_sats: i64,
    pub amount_paid_msat: i64,
    pub settled_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug)]
pub struct NodeHealth {
    pub server_active: bool,
}

#[async_trait]
pub trait LightningBackend: Send + Sync + 'static {
    async fn create_invoice(
        &self,
        amount_sats: i64,
        memo: Option<&str>,
        expiry_secs: u32,
    ) -> anyhow::Result<CreatedInvoice>;

    /// Returns `Ok(None)` if the node has no invoice with this hash.
    async fn lookup_invoice(&self, payment_hash: &str) -> anyhow::Result<Option<NodeInvoice>>;

    /// Stream of invoice updates (new and settled) from the node.
    async fn subscribe_invoices(
        &self,
    ) -> anyhow::Result<BoxStream<'static, anyhow::Result<NodeInvoice>>>;

    /// Checks connectivity and that our macaroon is accepted.
    async fn health(&self) -> anyhow::Result<NodeHealth>;
}
