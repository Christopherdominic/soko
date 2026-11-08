//! Background tasks that keep payment state in sync with the Lightning node.

use std::time::Duration;

use futures_util::StreamExt;

use crate::{repositories::sessions, services::payments, state::AppState};

const RECONCILE_EVERY: Duration = Duration::from_secs(15);
const RECONNECT_DELAY: Duration = Duration::from_secs(5);

pub fn spawn(state: AppState) {
    tokio::spawn(invoice_stream_loop(state.clone()));
    tokio::spawn(reconcile_loop(state));
}

/// Follows LND's invoice subscription; reconnects forever on failure.
async fn invoice_stream_loop(state: AppState) {
    loop {
        match state.lightning.subscribe_invoices().await {
            Ok(mut stream) => {
                tracing::info!("subscribed to LND invoice updates");
                // Catch up on anything that happened while we were disconnected.
                if let Err(e) =
                    payments::reconcile_pending(&state.db, state.lightning.as_ref()).await
                {
                    tracing::warn!(error = %e, "catch-up reconcile failed");
                }
                while let Some(item) = stream.next().await {
                    match item {
                        Ok(inv) => match payments::apply_node_invoice(&state.db, &inv).await {
                            Ok(outcome) => tracing::debug!(?outcome, "invoice update applied"),
                            Err(e) => tracing::error!(error = %e, "applying invoice update failed"),
                        },
                        Err(e) => {
                            tracing::warn!(error = %e, "invoice stream error");
                            break;
                        }
                    }
                }
                tracing::warn!("LND invoice stream ended; reconnecting");
            }
            Err(e) => tracing::warn!(error = %e, "could not subscribe to LND invoices"),
        }
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

/// Periodic safety net: re-verifies pending payments and expires stale ones.
async fn reconcile_loop(state: AppState) {
    let mut tick = tokio::time::interval(RECONCILE_EVERY);
    loop {
        tick.tick().await;
        match payments::reconcile_pending(&state.db, state.lightning.as_ref()).await {
            Ok(0) => {}
            Ok(n) => tracing::info!(changed = n, "reconciled pending payments"),
            Err(e) => tracing::warn!(error = %e, "reconcile loop failed"),
        }
        if let Err(e) = sessions::delete_expired(&state.db).await {
            tracing::warn!(error = %e, "session cleanup failed");
        }
    }
}
