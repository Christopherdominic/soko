use axum::{
    Router,
    http::{HeaderValue, Method, header},
    routing::{get, post},
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

    use handlers::{auth, health, payments};
    let api = Router::new()
        .route("/health", get(health::health))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::me))
        .route("/payments/invoices", post(payments::create_invoice))
        .route("/payments", get(payments::list_payments))
        .route("/payments/{payment_hash}", get(payments::get_payment))
        .route(
            "/payments/{payment_hash}/receipt",
            get(payments::get_receipt),
        );

    Router::new()
        .nest("/api", api)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
