use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method, header},
    routing::{get, post},
};
use std::time::Duration;

use axum::http::StatusCode;
use tower_http::{
    cors::CorsLayer, set_header::SetResponseHeaderLayer, timeout::TimeoutLayer, trace::TraceLayer,
};

use crate::{handlers, state::AppState};

pub fn router(state: AppState) -> Router {
    let cors = match state.config.frontend_origin.parse::<HeaderValue>() {
        Ok(origin) => CorsLayer::new()
            .allow_origin(origin)
            .allow_methods([Method::GET, Method::POST, Method::PUT])
            .allow_headers([header::CONTENT_TYPE])
            .allow_credentials(true),
        Err(_) => CorsLayer::new(),
    };

    use handlers::{auth, dashboard, health, money, payments, public};
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
        )
        .route("/dashboard/summary", get(dashboard::summary))
        .route(
            "/settings",
            get(money::get_settings).put(money::update_settings),
        )
        .route("/rates", get(money::get_rate))
        .route(
            "/payouts",
            get(money::list_payouts).post(money::create_payout),
        )
        .route(
            "/public/receipts/{receipt_number}",
            get(public::get_receipt),
        )
        .route(
            "/public/receipts/verify",
            post(public::verify_receipt).layer(DefaultBodyLimit::max(16 * 1024)),
        );

    Router::new()
        .nest("/api", api)
        // API responses are private, JSON-only and never framed or cached.
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
