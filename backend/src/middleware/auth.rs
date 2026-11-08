use axum::{extract::FromRequestParts, http::request::Parts};

use crate::{
    errors::ApiError,
    models::Merchant,
    repositories::sessions,
    services::auth::{SESSION_COOKIE, hash_token},
    state::AppState,
};

/// The logged-in merchant. Adding this to a handler's arguments makes the route
/// require a valid session; every query in that handler is scoped to this merchant.
pub struct CurrentMerchant(pub Merchant);

impl FromRequestParts<AppState> for CurrentMerchant {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = session_token(&parts.headers).ok_or(ApiError::Unauthorized)?;
        sessions::find_merchant(&state.db, &hash_token(token))
            .await?
            .map(CurrentMerchant)
            .ok_or(ApiError::Unauthorized)
    }
}

pub fn session_token(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|kv| kv.trim().split_once('='))
        .find(|(k, v)| *k == SESSION_COOKIE && !v.is_empty())
        .map(|(_, v)| v)
}
