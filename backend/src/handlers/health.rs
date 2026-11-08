use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
    database: &'static str,
    lightning: &'static str,
}

/// GET /api/health — 200 when the database and LND are both usable, 503 otherwise.
/// Reports only up/down, never node details.
pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    let lightning = match state.lightning.health().await {
        Ok(h) if h.server_active => "ok",
        Ok(_) => "starting",
        Err(e) => {
            tracing::warn!(error = %e, "LND health check failed");
            "unavailable"
        }
    };

    let healthy = db_ok && lightning == "ok";
    let body = HealthResponse {
        status: if healthy { "ok" } else { "degraded" },
        database: if db_ok { "ok" } else { "unavailable" },
        lightning,
    };
    let code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, Json(body))
}
