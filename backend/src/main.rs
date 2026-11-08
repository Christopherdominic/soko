use std::sync::Arc;

use anyhow::Context;
use soko_backend::{
    MIGRATOR,
    config::Config,
    lightning::lnd::LndRestClient,
    nostr::NostrSigner,
    routes,
    services::{receipts, watcher},
    state::AppState,
};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env from the backend dir or the repo root, whichever exists.
    dotenvy::dotenv()
        .or_else(|_| dotenvy::from_filename("../.env"))
        .ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "soko_backend=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env()?;
    tracing::info!(lnd = ?config.lnd, "starting Soko backend");

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .context("connecting to PostgreSQL")?;
    MIGRATOR.run(&db).await.context("running migrations")?;

    let lightning = Arc::new(LndRestClient::new(&config.lnd)?);
    let nostr = match &config.nostr_secret {
        Some(secret) => {
            let signer = Arc::new(NostrSigner::from_hex(&secret.0)?);
            receipts::backfill_merchant_pubkeys(&db, &signer).await?;
            Some(signer)
        }
        None => {
            tracing::warn!("SOKO_NOSTR_SECRET not set: receipts will not be Nostr-signed");
            None
        }
    };
    let bind_addr = config.bind_addr;
    let state = AppState {
        db,
        lightning,
        config: Arc::new(config),
        nostr,
    };

    watcher::spawn(state.clone());

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    tracing::info!(%bind_addr, "listening");
    axum::serve(listener, routes::router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
