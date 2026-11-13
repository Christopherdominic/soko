//! Live BTC exchange rates for local currencies.
//!
//! price(BTC in X) = price(BTC in USD) × rate(USD→X).
//! - BTC/USD: Kraken public ticker, falling back to CoinGecko (refreshed every 5 min).
//! - USD→X:  open.er-api.com daily reference rates (refreshed every 6 h).
//!
//! Rates are only used for display and for converting a local-currency price into
//! sats when an invoice is created. Payment verification never depends on them.

pub mod currencies;

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tokio::sync::RwLock;

const BTC_REFRESH: Duration = Duration::from_secs(5 * 60);
const FX_REFRESH: Duration = Duration::from_secs(6 * 60 * 60);
/// Older than this, a BTC price is not used to price new invoices.
pub const MAX_PRICING_AGE: chrono::Duration = chrono::Duration::minutes(30);

#[derive(Clone, Debug)]
struct Snapshot {
    btc_usd: Option<(f64, DateTime<Utc>, &'static str)>,
    usd_to: HashMap<String, f64>,
    fx_updated: Option<DateTime<Utc>>,
}

#[derive(Clone, Copy, Debug)]
pub struct Quote {
    /// Price of 1 BTC in the requested currency.
    pub btc_price: f64,
    pub updated_at: DateTime<Utc>,
    pub source: &'static str,
}

impl Quote {
    pub fn fresh_for_pricing(&self) -> bool {
        Utc::now() - self.updated_at <= MAX_PRICING_AGE
    }
}

pub struct RateService {
    state: RwLock<Snapshot>,
    http: Option<reqwest::Client>,
}

impl RateService {
    pub fn new() -> Result<Arc<Self>> {
        let http = reqwest::Client::builder()
            .tls_backend_preconfigured(crate::lightning::lnd::public_roots_tls_config()?)
            .timeout(Duration::from_secs(10))
            .user_agent("soko/0.1")
            .build()?;
        Ok(Arc::new(Self {
            state: RwLock::new(Snapshot {
                btc_usd: None,
                usd_to: HashMap::new(),
                fx_updated: None,
            }),
            http: Some(http),
        }))
    }

    /// Fixed rates, for tests: `btc_usd` plus USD→currency pairs.
    pub fn fixed(btc_usd: f64, pairs: &[(&str, f64)]) -> Arc<Self> {
        let mut usd_to: HashMap<String, f64> =
            pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        usd_to.insert("USD".into(), 1.0);
        Arc::new(Self {
            state: RwLock::new(Snapshot {
                btc_usd: Some((btc_usd, Utc::now(), "fixed")),
                usd_to,
                fx_updated: Some(Utc::now()),
            }),
            http: None,
        })
    }

    /// Price of 1 BTC in `currency`, or None if we have no data yet.
    pub async fn quote(&self, currency: &str) -> Option<Quote> {
        let s = self.state.read().await;
        let (btc_usd, updated_at, source) = s.btc_usd?;
        let rate = if currency == "USD" {
            1.0
        } else {
            *s.usd_to.get(currency)?
        };
        Some(Quote {
            btc_price: btc_usd * rate,
            updated_at,
            source,
        })
    }

    pub fn spawn_refresh(self: &Arc<Self>) {
        if self.http.is_none() {
            return;
        }
        let me = self.clone();
        tokio::spawn(async move {
            let mut btc_tick = tokio::time::interval(BTC_REFRESH);
            let mut fx_tick = tokio::time::interval(FX_REFRESH);
            loop {
                tokio::select! {
                    _ = btc_tick.tick() => me.refresh_btc().await,
                    _ = fx_tick.tick() => me.refresh_fx().await,
                }
            }
        });
    }

    async fn refresh_btc(&self) {
        match self.fetch_btc_usd().await {
            Ok((price, source)) => {
                self.state.write().await.btc_usd = Some((price, Utc::now(), source));
                tracing::debug!(price, source, "BTC/USD updated");
            }
            // Keep the last known price; quotes report its age.
            Err(e) => {
                tracing::warn!(error = %e, "BTC price refresh failed; keeping last known price")
            }
        }
    }

    async fn refresh_fx(&self) {
        match self.fetch_fx().await {
            Ok(rates) => {
                let mut s = self.state.write().await;
                s.usd_to = rates;
                s.fx_updated = Some(Utc::now());
                tracing::info!(currencies = s.usd_to.len(), "FX rates updated");
            }
            Err(e) => tracing::warn!(error = %e, "FX refresh failed; keeping last known rates"),
        }
    }

    async fn fetch_btc_usd(&self) -> Result<(f64, &'static str)> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| anyhow!("no HTTP client"))?;
        match fetch_kraken(http).await {
            Ok(p) => Ok((p, "Kraken")),
            Err(e) => {
                tracing::warn!(error = %e, "Kraken failed, trying CoinGecko");
                Ok((fetch_coingecko(http).await?, "CoinGecko"))
            }
        }
    }

    async fn fetch_fx(&self) -> Result<HashMap<String, f64>> {
        #[derive(Deserialize)]
        struct Resp {
            result: String,
            rates: HashMap<String, f64>,
        }
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| anyhow!("no HTTP client"))?;
        let r: Resp = http
            .get("https://open.er-api.com/v6/latest/USD")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if r.result != "success" {
            return Err(anyhow!("FX API returned {}", r.result));
        }
        Ok(r.rates
            .into_iter()
            .filter(|(_, v)| v.is_finite() && *v > 0.0)
            .collect())
    }
}

async fn fetch_kraken(http: &reqwest::Client) -> Result<f64> {
    #[derive(Deserialize)]
    struct Resp {
        error: Vec<String>,
        result: HashMap<String, Ticker>,
    }
    #[derive(Deserialize)]
    struct Ticker {
        c: Vec<String>, // last trade [price, volume]
    }
    let r: Resp = http
        .get("https://api.kraken.com/0/public/Ticker?pair=XBTUSD")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if !r.error.is_empty() {
        return Err(anyhow!("Kraken error: {:?}", r.error));
    }
    let price: f64 = r
        .result
        .values()
        .next()
        .and_then(|t| t.c.first())
        .context("Kraken: no price")?
        .parse()?;
    sane_price(price)
}

async fn fetch_coingecko(http: &reqwest::Client) -> Result<f64> {
    #[derive(Deserialize)]
    struct Resp {
        bitcoin: HashMap<String, f64>,
    }
    let r: Resp = http
        .get("https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    sane_price(*r.bitcoin.get("usd").context("CoinGecko: no usd price")?)
}

/// Rejects obviously broken quotes rather than pricing invoices with them.
fn sane_price(p: f64) -> Result<f64> {
    if p.is_finite() && (1_000.0..=10_000_000.0).contains(&p) {
        Ok(p)
    } else {
        Err(anyhow!("implausible BTC/USD price {p}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn quotes_cross_rate() {
        let fx = RateService::fixed(70_000.0, &[("KES", 130.0)]);
        let q = fx.quote("KES").await.unwrap();
        assert!((q.btc_price - 9_100_000.0).abs() < 1e-6);
        assert!(q.fresh_for_pricing());
        assert_eq!(fx.quote("USD").await.unwrap().btc_price, 70_000.0);
        assert!(fx.quote("NGN").await.is_none(), "no rate -> no quote");
    }

    #[test]
    fn rejects_implausible_prices() {
        assert!(sane_price(69_000.0).is_ok());
        assert!(sane_price(0.0).is_err());
        assert!(sane_price(f64::NAN).is_err());
        assert!(sane_price(1e12).is_err());
    }
}
