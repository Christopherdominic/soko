//! In-memory token-bucket rate limiting (single backend instance).

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Mutex,
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::request::Parts,
};

use crate::{errors::ApiError, state::AppState};

pub struct RateLimiter {
    capacity: f64,
    refill_per_sec: f64,
    buckets: Mutex<HashMap<String, Bucket>>,
}

struct Bucket {
    tokens: f64,
    updated: Instant,
}

const MAX_KEYS: usize = 50_000;

impl RateLimiter {
    /// Allows bursts of `burst` requests, refilling `burst` tokens every `per`.
    pub fn new(burst: u32, per: Duration) -> Self {
        Self {
            capacity: f64::from(burst),
            refill_per_sec: f64::from(burst) / per.as_secs_f64(),
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Takes one token for `key`, or returns how long until one is available.
    pub fn check(&self, key: &str) -> Result<(), Duration> {
        self.check_at(key, Instant::now())
    }

    fn check_at(&self, key: &str, now: Instant) -> Result<(), Duration> {
        let mut buckets = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
        if buckets.len() >= MAX_KEYS {
            // Drop buckets that have fully refilled; they carry no state.
            let full_after = self.capacity / self.refill_per_sec;
            buckets.retain(|_, b| now.duration_since(b.updated).as_secs_f64() < full_after);
        }
        let b = buckets.entry(key.to_owned()).or_insert(Bucket {
            tokens: self.capacity,
            updated: now,
        });
        let elapsed = now.duration_since(b.updated).as_secs_f64();
        b.tokens = (b.tokens + elapsed * self.refill_per_sec).min(self.capacity);
        b.updated = now;
        if b.tokens >= 1.0 {
            b.tokens -= 1.0;
            Ok(())
        } else {
            Err(
                Duration::from_secs_f64((1.0 - b.tokens) / self.refill_per_sec)
                    .max(Duration::from_secs(1)),
            )
        }
    }
}

/// All limits used by the API. Keys that can't be spoofed (email, merchant id,
/// global) are always enforced; per-IP limits only apply when a trusted reverse
/// proxy sets X-Forwarded-For.
pub struct Limits {
    pub login_per_email: RateLimiter,
    pub login_global: RateLimiter,
    pub register_global: RateLimiter,
    pub invoice_per_merchant: RateLimiter,
    pub public_global: RateLimiter,
    pub per_ip: RateLimiter,
}

impl Default for Limits {
    fn default() -> Self {
        let min = Duration::from_secs(60);
        Self {
            login_per_email: RateLimiter::new(10, Duration::from_secs(15 * 60)),
            login_global: RateLimiter::new(300, min),
            register_global: RateLimiter::new(20, Duration::from_secs(60 * 60)),
            invoice_per_merchant: RateLimiter::new(30, min),
            public_global: RateLimiter::new(600, min),
            per_ip: RateLimiter::new(60, min),
        }
    }
}

pub fn enforce(limiter: &RateLimiter, key: &str) -> Result<(), ApiError> {
    limiter
        .check(key)
        .map_err(|wait| ApiError::RateLimited(wait.as_secs().max(1)))
}

/// Client IP, only when it can be trusted (SOKO_TRUST_PROXY=true). Uses the
/// right-most X-Forwarded-For entry, i.e. the address our own proxy saw.
pub struct ClientIp(pub Option<String>);

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if !state.config.trust_proxy {
            return Ok(ClientIp(None));
        }
        let forwarded = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit(',').next())
            .map(|ip| ip.trim().to_string())
            .filter(|ip| ip.parse::<std::net::IpAddr>().is_ok());
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip().to_string());
        Ok(ClientIp(forwarded.or(peer)))
    }
}

impl ClientIp {
    pub fn enforce(&self, limits: &Limits, scope: &str) -> Result<(), ApiError> {
        match &self.0 {
            Some(ip) => enforce(&limits.per_ip, &format!("{scope}:{ip}")),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_burst_then_blocks_then_refills() {
        let rl = RateLimiter::new(3, Duration::from_secs(30));
        let t0 = Instant::now();
        for _ in 0..3 {
            assert!(rl.check_at("k", t0).is_ok());
        }
        let wait = rl.check_at("k", t0).unwrap_err();
        assert!(wait >= Duration::from_secs(9) && wait <= Duration::from_secs(11));
        assert!(rl.check_at("other", t0).is_ok(), "keys are independent");
        assert!(
            rl.check_at("k", t0 + Duration::from_secs(10)).is_ok(),
            "one token refilled"
        );
        assert!(rl.check_at("k", t0 + Duration::from_secs(10)).is_err());
    }
}
