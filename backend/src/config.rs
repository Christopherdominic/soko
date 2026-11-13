use std::{fmt, net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result, bail};
use url::Url;

/// Runtime configuration, loaded from environment variables.
#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub frontend_origin: String,
    pub invoice_expiry_secs: u32,
    /// Allow new merchant sign-ups. Disable once the merchant account exists.
    pub allow_registration: bool,
    /// Mark the session cookie `Secure` (required when served over HTTPS).
    pub cookie_secure: bool,
    pub session_ttl_hours: i64,
    /// Trust X-Forwarded-For for per-IP rate limits. Only enable when a reverse
    /// proxy you control overwrites that header.
    pub trust_proxy: bool,
    /// Hex master secret for deriving merchant Nostr keys. None disables signing.
    pub nostr_secret: Option<Redacted>,
    /// None when no Lightning node is configured; invoices can't be created.
    pub lnd: Option<LndConfig>,
}

/// A secret string whose `Debug` output never shows the value.
#[derive(Clone)]
pub struct Redacted(pub String);

impl fmt::Debug for Redacted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

#[derive(Clone)]
pub struct LndConfig {
    pub rest_url: Url,
    /// PEM bytes of LND's tls.cert, pinned exactly. `None` = use public CA roots.
    pub tls_cert_pem: Option<Vec<u8>>,
    /// Hex-encoded macaroon. Secret: never log it.
    pub macaroon_hex: String,
}

// Hand-written so the macaroon can never leak through `{:?}`.
impl fmt::Debug for LndConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LndConfig")
            .field("rest_url", &self.rest_url.as_str())
            .field(
                "tls",
                &if self.tls_cert_pem.is_some() {
                    "pinned certificate"
                } else {
                    "public CA roots"
                },
            )
            .field("macaroon_hex", &"<redacted>")
            .finish()
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Hosting platforms such as Render assign a port via $PORT.
        let default_bind = match std::env::var("PORT") {
            Ok(port) if !port.trim().is_empty() => format!("0.0.0.0:{}", port.trim()),
            _ => "127.0.0.1:4000".to_string(),
        };
        let bind_addr = env_or("SOKO_BIND_ADDR", &default_bind)
            .parse()
            .context("SOKO_BIND_ADDR must be host:port")?;
        let database_url = required("DATABASE_URL")?;
        let frontend_origin = env_or("SOKO_FRONTEND_ORIGIN", "http://localhost:3000");
        let invoice_expiry_secs: u32 = env_or("SOKO_INVOICE_EXPIRY_SECS", "900")
            .parse()
            .context("SOKO_INVOICE_EXPIRY_SECS must be a positive integer")?;
        if !(60..=86_400).contains(&invoice_expiry_secs) {
            bail!("SOKO_INVOICE_EXPIRY_SECS must be between 60 and 86400");
        }

        let allow_registration = env_bool("SOKO_ALLOW_REGISTRATION", true)?;
        let cookie_secure = env_bool("SOKO_COOKIE_SECURE", false)?;
        let trust_proxy = env_bool("SOKO_TRUST_PROXY", false)?;
        let session_ttl_hours: i64 = env_or("SOKO_SESSION_TTL_HOURS", "168")
            .parse()
            .context("SOKO_SESSION_TTL_HOURS must be an integer")?;
        if !(1..=720).contains(&session_ttl_hours) {
            bail!("SOKO_SESSION_TTL_HOURS must be between 1 and 720");
        }

        let nostr_secret = std::env::var("SOKO_NOSTR_SECRET")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map(Redacted);

        let lnd = lnd_from_env()?;

        Ok(Self {
            bind_addr,
            database_url,
            frontend_origin,
            invoice_expiry_secs,
            allow_registration,
            cookie_secure,
            session_ttl_hours,
            trust_proxy,
            nostr_secret,
            lnd,
        })
    }
}

/// `Ok(None)` when LND_REST_URL is unset: the app runs without taking payments.
fn lnd_from_env() -> Result<Option<LndConfig>> {
    if optional("LND_REST_URL").is_none() {
        return Ok(None);
    }
    let rest_url: Url = required("LND_REST_URL")?
        .parse()
        .context("LND_REST_URL must be a URL")?;
    if rest_url.scheme() != "https" {
        bail!("LND_REST_URL must use https");
    }
    // With a cert file we pin LND's self-signed certificate (Polar, self-hosted).
    // Without one, LND must present a publicly trusted certificate (e.g. Voltage).
    let tls_cert_pem = match optional("LND_TLS_CERT_PATH") {
        Some(path) => {
            let path = PathBuf::from(path);
            Some(
                std::fs::read(&path)
                    .with_context(|| format!("reading LND TLS cert at {}", path.display()))?,
            )
        }
        None => None,
    };
    let macaroon_hex = match (optional("LND_MACAROON_HEX"), optional("LND_MACAROON_PATH")) {
        (Some(hex_value), _) => {
            let hex_value = hex_value.trim().to_lowercase();
            if hex::decode(&hex_value).map_or(true, |b| b.is_empty()) {
                bail!("LND_MACAROON_HEX must be a hex-encoded macaroon");
            }
            hex_value
        }
        (None, Some(path)) => {
            let path = PathBuf::from(path);
            let bytes = std::fs::read(&path)
                .with_context(|| format!("reading LND macaroon at {}", path.display()))?;
            hex::encode(bytes)
        }
        (None, None) => bail!("set LND_MACAROON_HEX or LND_MACAROON_PATH"),
    };

    Ok(Some(LndConfig {
        rest_url,
        tls_cert_pem,
        macaroon_hex,
    }))
}

fn required(key: &str) -> Result<String> {
    std::env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .with_context(|| format!("missing required environment variable {key}"))
}

fn optional(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn env_bool(key: &str, default: bool) -> Result<bool> {
    match std::env::var(key).ok().as_deref().map(str::trim) {
        None | Some("") => Ok(default),
        Some("true" | "1" | "yes") => Ok(true),
        Some("false" | "0" | "no") => Ok(false),
        Some(_) => bail!("{key} must be true or false"),
    }
}
