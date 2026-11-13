use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header::SET_COOKIE},
    response::{AppendHeaders, IntoResponse},
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::json::ApiJson;
use crate::{
    errors::{ApiError, ApiResult},
    middleware::{
        auth::{CurrentMerchant, session_token},
        rate_limit::{ClientIp, enforce},
    },
    models::Merchant,
    repositories::{merchants, sessions},
    services::auth as auth_svc,
    state::AppState,
};

#[derive(Deserialize)]
pub struct RegisterRequest {
    business_name: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct MerchantResponse {
    id: Uuid,
    business_name: String,
    email: String,
    /// Public key that signs this merchant's receipts (NIP-19 npub).
    nostr_npub: Option<String>,
    currency: String,
}

impl From<Merchant> for MerchantResponse {
    fn from(m: Merchant) -> Self {
        Self {
            id: m.id,
            nostr_npub: m.nostr_pubkey.as_deref().and_then(crate::nostr::npub),
            business_name: m.business_name,
            email: m.email,
            currency: m.currency,
        }
    }
}

pub async fn register(
    State(state): State<AppState>,
    ip: ClientIp,
    ApiJson(req): ApiJson<RegisterRequest>,
) -> ApiResult<impl IntoResponse> {
    if !state.config.allow_registration {
        return Err(ApiError::Forbidden);
    }
    ip.enforce(&state.limits, "register")?;
    enforce(&state.limits.register_global, "all")?;
    let business_name = req.business_name.trim().to_string();
    let email = req.email.trim().to_lowercase();
    if business_name.is_empty() || business_name.chars().count() > 120 {
        return Err(ApiError::Validation(
            "Business name must be 1–120 characters".into(),
        ));
    }
    if !is_plausible_email(&email) {
        return Err(ApiError::Validation("Enter a valid email address".into()));
    }
    if !(8..=128).contains(&req.password.chars().count()) {
        return Err(ApiError::Validation(
            "Password must be 8–128 characters".into(),
        ));
    }

    let password = req.password;
    let hash = tokio::task::spawn_blocking(move || auth_svc::hash_password(&password))
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .map_err(ApiError::Internal)?;
    let merchant = merchants::insert(&state.db, &business_name, &email, &hash)
        .await?
        .ok_or(ApiError::Conflict(
            "An account with this email already exists".into(),
        ))?;

    let mut merchant = merchant;
    if let Some(signer) = &state.nostr {
        let pubkey = signer.pubkey_hex(merchant.id);
        merchants::set_nostr_pubkey(&state.db, merchant.id, &pubkey).await?;
        merchant.nostr_pubkey = Some(pubkey);
    }

    start_session(&state, merchant, StatusCode::CREATED).await
}

pub async fn login(
    State(state): State<AppState>,
    ip: ClientIp,
    ApiJson(req): ApiJson<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    // Per-account limit stops password guessing even if the attacker rotates IPs.
    ip.enforce(&state.limits, "login")?;
    enforce(&state.limits.login_global, "all")?;
    enforce(
        &state.limits.login_per_email,
        &req.email.trim().to_lowercase(),
    )?;
    let merchant = merchants::find_by_email(&state.db, req.email.trim()).await?;
    let password = req.password;
    let ok_merchant = tokio::task::spawn_blocking(move || match merchant {
        Some(m) if auth_svc::verify_password(&password, &m.password_hash) => Some(m),
        Some(_) => None,
        None => {
            auth_svc::burn_password_check(&password);
            None
        }
    })
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let merchant = ok_merchant.ok_or(ApiError::InvalidCredentials)?;
    start_session(&state, merchant, StatusCode::OK).await
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    if let Some(token) = session_token(&headers) {
        sessions::delete(&state.db, &auth_svc::hash_token(token)).await?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        AppendHeaders([(
            SET_COOKIE,
            auth_svc::clear_session_cookie(state.config.cookie_secure),
        )]),
    ))
}

pub async fn me(CurrentMerchant(m): CurrentMerchant) -> Json<MerchantResponse> {
    Json(m.into())
}

async fn start_session(
    state: &AppState,
    merchant: Merchant,
    status: StatusCode,
) -> ApiResult<impl IntoResponse + use<>> {
    let (token, hash) = auth_svc::new_session_token();
    let ttl = Duration::hours(state.config.session_ttl_hours);
    sessions::insert(&state.db, &hash, merchant.id, Utc::now() + ttl).await?;
    let cookie = auth_svc::session_cookie(&token, ttl.num_seconds(), state.config.cookie_secure);
    Ok((
        status,
        AppendHeaders([(SET_COOKIE, cookie)]),
        Json(MerchantResponse::from(merchant)),
    ))
}

fn is_plausible_email(email: &str) -> bool {
    email.len() <= 254
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty() && domain.contains('.') && !domain.starts_with('.')
        })
        && !email.chars().any(char::is_whitespace)
}
