use std::sync::OnceLock;

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use sha2::{Digest, Sha256};

pub const SESSION_COOKIE: &str = "soko_session";

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("hashing password: {e}"))
}

pub fn verify_password(password: &str, phc: &str) -> bool {
    PasswordHash::new(phc)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

/// Verifies against a throwaway hash so a login for an unknown email takes about as
/// long as one for a known email (prevents account discovery by timing).
pub fn burn_password_check(password: &str) {
    static DUMMY: OnceLock<String> = OnceLock::new();
    let dummy = DUMMY.get_or_init(|| hash_password("soko-dummy-password").unwrap_or_default());
    let _ = verify_password(password, dummy);
}

/// New random session token (sent to the browser) and its hash (stored in the DB).
pub fn new_session_token() -> (String, [u8; 32]) {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let token = URL_SAFE_NO_PAD.encode(bytes);
    let hash = hash_token(&token);
    (token, hash)
}

pub fn hash_token(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

pub fn session_cookie(token: &str, max_age_secs: i64, secure: bool) -> String {
    format!(
        "{SESSION_COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={max_age_secs}{}",
        if secure { "; Secure" } else { "" }
    )
}

pub fn clear_session_cookie(secure: bool) -> String {
    session_cookie("", 0, secure)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_round_trip() {
        let h = hash_password("correct horse").unwrap();
        assert!(h.starts_with("$argon2id$"));
        assert!(verify_password("correct horse", &h));
        assert!(!verify_password("wrong", &h));
        assert!(!verify_password("x", "not-a-hash"));
    }

    #[test]
    fn session_tokens_are_unique_and_hashed() {
        let (t1, h1) = new_session_token();
        let (t2, _) = new_session_token();
        assert_ne!(t1, t2);
        assert_eq!(hash_token(&t1), h1);
        assert!(t1.len() >= 43);
    }
}
