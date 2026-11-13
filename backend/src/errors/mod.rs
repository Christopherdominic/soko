use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// Errors returned by API handlers. Internal details are logged, never sent to clients.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    Validation(String),
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("forbidden")]
    Forbidden,
    #[error("{0}")]
    Conflict(String),
    /// Seconds until the client may retry.
    #[error("rate limited")]
    RateLimited(u64),
    #[error("exchange rate unavailable")]
    RateUnavailable,
    #[error("lightning node unavailable")]
    LightningUnavailable(#[source] anyhow::Error),
    #[error("internal error")]
    Internal(#[source] anyhow::Error),
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        ApiError::Internal(e.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            ApiError::Validation(msg) => (StatusCode::BAD_REQUEST, "validation_error", msg.clone()),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not_found", "Not found".into()),
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Authentication required".into(),
            ),
            ApiError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "Invalid email or password".into(),
            ),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "Not allowed".into()),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg.clone()),
            ApiError::RateLimited(secs) => {
                let body = json!({ "error": {
                    "code": "rate_limited",
                    "message": "Too many attempts. Please wait a moment and try again."
                }});
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    [(axum::http::header::RETRY_AFTER, secs.to_string())],
                    Json(body),
                )
                    .into_response();
            }
            ApiError::RateUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "rate_unavailable",
                "The exchange rate is temporarily unavailable. Enter the amount in sats instead."
                    .into(),
            ),
            ApiError::LightningUnavailable(e) => {
                tracing::error!(error = %e, "lightning backend error");
                (
                    StatusCode::BAD_GATEWAY,
                    "lightning_unavailable",
                    "The Lightning node is unavailable. Please try again.".into(),
                )
            }
            ApiError::Internal(e) => {
                tracing::error!(error = ?e, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Something went wrong".into(),
                )
            }
        };
        (
            status,
            Json(json!({ "error": { "code": code, "message": message } })),
        )
            .into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
