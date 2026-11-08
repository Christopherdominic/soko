use axum::{
    Router,
    http::{HeaderValue, Method, header},
    routing::get,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{handlers, state::AppState};

pub fn router(state: AppState) -> Router {
    let cors = match state.config.frontend_origin.parse::<HeaderValue>() {
        Ok(origin) => CorsLayer::new()
            .allow_origin(origin)
            .allow_methods([Method::GET, Method::POST])
            .allow_headers([header::CONTENT_TYPE])
            .allow_credentials(true),
        Err(_) => CorsLayer::new(),
    };

    let api = Router::new().route("/health", get(handlers::health::health));

    Router::new()
        .nest("/api", api)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
