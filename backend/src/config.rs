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
    pub lnd: LndConfig,
}

#[derive(Clone)]
pub struct LndConfig {
    pub rest_url: Url,
    /// PEM bytes of LND's tls.cert. Pinned exactly by the client.
    pub tls_cert_pem: Vec<u8>,
    /// Hex-encoded macaroon. Secret: never log it.
    pub macaroon_hex: String,
}

// Hand-written so the macaroon can never leak through `{:?}`.
impl fmt::Debug for LndConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LndConfig")
            .field("rest_url", &self.rest_url.as_str())
            .field("macaroon_hex", &"<redacted>")
            .finish()
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let bind_addr = env_or("SOKO_BIND_ADDR", "127.0.0.1:4000")
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

        let rest_url: Url = required("LND_REST_URL")?
            .parse()
            .context("LND_REST_URL must be a URL")?;
        if rest_url.scheme() != "https" {
            bail!("LND_REST_URL must use https");
        }
        let cert_path = PathBuf::from(required("LND_TLS_CERT_PATH")?);
        let tls_cert_pem = std::fs::read(&cert_path)
            .with_context(|| format!("reading LND TLS cert at {}", cert_path.display()))?;
        let macaroon_path = PathBuf::from(required("LND_MACAROON_PATH")?);
        let macaroon = std::fs::read(&macaroon_path)
            .with_context(|| format!("reading LND macaroon at {}", macaroon_path.display()))?;

        Ok(Self {
            bind_addr,
            database_url,
            frontend_origin,
            invoice_expiry_secs,
            lnd: LndConfig {
                rest_url,
                tls_cert_pem,
                macaroon_hex: hex::encode(macaroon),
            },
        })
    }
}

fn required(key: &str) -> Result<String> {
    std::env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .with_context(|| format!("missing required environment variable {key}"))
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}
