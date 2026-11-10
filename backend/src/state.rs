use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    config::Config, lightning::LightningBackend, middleware::rate_limit::Limits, nostr::NostrSigner,
};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub lightning: Arc<dyn LightningBackend>,
    pub config: Arc<Config>,
    /// None when SOKO_NOSTR_SECRET is not set; receipts are then unsigned.
    pub nostr: Option<Arc<NostrSigner>>,
    pub limits: Arc<Limits>,
}
