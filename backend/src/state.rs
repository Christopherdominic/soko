use std::sync::Arc;

use sqlx::PgPool;

use crate::{config::Config, lightning::LightningBackend};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub lightning: Arc<dyn LightningBackend>,
    pub config: Arc<Config>,
}
