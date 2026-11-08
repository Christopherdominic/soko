//! LND REST client.
//!
//! - TLS: LND uses a self-signed certificate. We pin that exact certificate instead
//!   of trusting system roots or disabling verification.
//! - Auth: the macaroon is sent as a header and never logged.

use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use chrono::{DateTime, Utc};
use futures_util::{StreamExt, stream::BoxStream};
use reqwest::{StatusCode, header};
use rustls::{
    DigitallySignedStruct, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::CryptoProvider,
    pki_types::{CertificateDer, ServerName, UnixTime, pem::PemObject},
};
use serde::{Deserialize, Deserializer};
use url::Url;

use super::{CreatedInvoice, InvoiceState, LightningBackend, NodeHealth, NodeInvoice};
use crate::config::LndConfig;

pub struct LndRestClient {
    http: reqwest::Client,
    stream_http: reqwest::Client,
    base: Url,
}

impl LndRestClient {
    pub fn new(cfg: &LndConfig) -> Result<Self> {
        let tls = pinned_tls_config(&cfg.tls_cert_pem)?;
        let mut headers = header::HeaderMap::new();
        let mut mac = header::HeaderValue::from_str(&cfg.macaroon_hex)
            .context("macaroon is not valid header text")?;
        mac.set_sensitive(true);
        headers.insert("Grpc-Metadata-macaroon", mac);

        let build = |timeout: Option<std::time::Duration>| {
            let mut b = reqwest::Client::builder()
                .tls_backend_preconfigured(tls.clone())
                .default_headers(headers.clone())
                .connect_timeout(std::time::Duration::from_secs(5));
            if let Some(t) = timeout {
                b = b.timeout(t);
            }
            b.build()
        };

        Ok(Self {
            http: build(Some(std::time::Duration::from_secs(15)))?,
            // The subscription is long-lived, so it has no overall timeout.
            stream_http: build(None)?,
            base: cfg.rest_url.clone(),
        })
    }

    fn url(&self, path: &str) -> Result<Url> {
        self.base.join(path).context("building LND URL")
    }
}

#[async_trait]
impl LightningBackend for LndRestClient {
    async fn create_invoice(
        &self,
        amount_sats: i64,
        memo: Option<&str>,
        expiry_secs: u32,
    ) -> Result<CreatedInvoice> {
        #[derive(Deserialize)]
        struct AddInvoiceResp {
            r_hash: String,
            payment_request: String,
        }
        let body = serde_json::json!({
            "value": amount_sats.to_string(),
            "memo": memo.unwrap_or(""),
            "expiry": expiry_secs.to_string(),
        });
        let resp = self
            .http
            .post(self.url("/v1/invoices")?)
            .json(&body)
            .send()
            .await?;
        let resp: AddInvoiceResp = check(resp).await?.json().await?;
        let hash = B64
            .decode(&resp.r_hash)
            .context("LND returned invalid r_hash")?;
        if hash.len() != 32 {
            bail!("LND returned r_hash of unexpected length");
        }
        Ok(CreatedInvoice {
            payment_hash: hex::encode(hash),
            payment_request: resp.payment_request,
        })
    }

    async fn lookup_invoice(&self, payment_hash: &str) -> Result<Option<NodeInvoice>> {
        if !is_hex_hash(payment_hash) {
            bail!("invalid payment hash");
        }
        let resp = self
            .http
            .get(self.url(&format!("/v1/invoice/{payment_hash}"))?)
            .send()
            .await?;
        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            // LND reports unknown invoices with a 500 + this message on some versions.
            if text.contains("unable to locate invoice") || text.contains("no existing invoices") {
                return Ok(None);
            }
            bail!("LND lookup failed with HTTP {status}");
        }
        let inv: LndInvoice = resp.json().await?;
        Ok(Some(inv.try_into()?))
    }

    async fn subscribe_invoices(&self) -> Result<BoxStream<'static, Result<NodeInvoice>>> {
        let resp = self
            .stream_http
            .get(self.url("/v1/invoices/subscribe")?)
            .send()
            .await?;
        let bytes = check(resp).await?.bytes_stream();

        // LND streams newline-delimited JSON objects: {"result": Invoice} or {"error": ...}.
        let stream = futures_util::stream::unfold(
            (bytes, Vec::<u8>::new()),
            |(mut bytes, mut buf)| async move {
                loop {
                    if let Some(pos) = buf.iter().position(|b| *b == b'\n') {
                        let line: Vec<u8> = buf.drain(..=pos).collect();
                        if line.iter().all(u8::is_ascii_whitespace) {
                            continue;
                        }
                        return Some((parse_stream_line(&line), (bytes, buf)));
                    }
                    match bytes.next().await {
                        Some(Ok(chunk)) => buf.extend_from_slice(&chunk),
                        Some(Err(e)) => return Some((Err(e.into()), (bytes, buf))),
                        None => return None,
                    }
                }
            },
        );
        Ok(stream.boxed())
    }

    async fn health(&self) -> Result<NodeHealth> {
        #[derive(Deserialize)]
        struct StateResp {
            state: String,
        }
        let state: StateResp = check(self.http.get(self.url("/v1/state")?).send().await?)
            .await?
            .json()
            .await?;
        // Listing one invoice proves the macaroon is valid and has invoice permissions.
        check(
            self.http
                .get(self.url("/v1/invoices?num_max_invoices=1")?)
                .send()
                .await?,
        )
        .await?;
        Ok(NodeHealth {
            server_active: state.state == "SERVER_ACTIVE",
        })
    }
}

fn parse_stream_line(line: &[u8]) -> Result<NodeInvoice> {
    #[derive(Deserialize)]
    struct Envelope {
        result: Option<LndInvoice>,
        error: Option<serde_json::Value>,
    }
    let env: Envelope = serde_json::from_slice(line).context("invalid LND stream message")?;
    match (env.result, env.error) {
        (Some(inv), _) => inv.try_into(),
        (None, Some(err)) => Err(anyhow!("LND stream error: {err}")),
        (None, None) => Err(anyhow!("empty LND stream message")),
    }
}

/// Turn a non-2xx response into an error without echoing the body (it may contain
/// node details we don't want in client-facing errors).
async fn check(resp: reqwest::Response) -> Result<reqwest::Response> {
    if resp.status().is_success() {
        Ok(resp)
    } else {
        Err(anyhow!("LND request failed with HTTP {}", resp.status()))
    }
}

pub fn is_hex_hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[derive(Deserialize)]
struct LndInvoice {
    r_hash: String,
    state: String,
    #[serde(default, deserialize_with = "de_i64")]
    value: i64,
    #[serde(default, deserialize_with = "de_i64")]
    amt_paid_msat: i64,
    #[serde(default, deserialize_with = "de_i64")]
    settle_date: i64,
}

impl TryFrom<LndInvoice> for NodeInvoice {
    type Error = anyhow::Error;
    fn try_from(inv: LndInvoice) -> Result<Self> {
        let hash = B64.decode(&inv.r_hash).context("invalid r_hash")?;
        let state = match inv.state.as_str() {
            "OPEN" => InvoiceState::Open,
            "ACCEPTED" => InvoiceState::Accepted,
            "SETTLED" => InvoiceState::Settled,
            "CANCELED" => InvoiceState::Canceled,
            other => bail!("unknown LND invoice state {other}"),
        };
        let settled_at = (state == InvoiceState::Settled && inv.settle_date > 0)
            .then(|| DateTime::<Utc>::from_timestamp(inv.settle_date, 0))
            .flatten();
        Ok(NodeInvoice {
            payment_hash: hex::encode(hash),
            state,
            value_sats: inv.value,
            amount_paid_msat: inv.amt_paid_msat,
            settled_at,
        })
    }
}

/// LND encodes int64 fields as JSON strings.
fn de_i64<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<i64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StrOrNum {
        S(String),
        N(i64),
    }
    match StrOrNum::deserialize(d)? {
        StrOrNum::S(s) if s.is_empty() => Ok(0),
        StrOrNum::S(s) => s.parse().map_err(serde::de::Error::custom),
        StrOrNum::N(n) => Ok(n),
    }
}

// ---------- TLS pinning ----------

fn pinned_tls_config(pem: &[u8]) -> Result<rustls::ClientConfig> {
    let cert = CertificateDer::from_pem_slice(pem).context("LND tls.cert is not valid PEM")?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = PinnedCertVerifier {
        pinned: cert.into_owned(),
        provider: provider.clone(),
    };
    Ok(rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth())
}

/// Accepts only the exact certificate configured for our LND node. Handshake
/// signatures are still verified, so the server must hold the matching private key.
#[derive(Debug)]
struct PinnedCertVerifier {
    pinned: CertificateDer<'static>,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for PinnedCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        if end_entity.as_ref() == self.pinned.as_ref() {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::UnknownIssuer,
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_settled_invoice_from_lnd_json() {
        let hash = [7u8; 32];
        let json = format!(
            r#"{{"result":{{"r_hash":"{}","state":"SETTLED","value":"1500","amt_paid_msat":"1500000","settle_date":"1762620000"}}}}"#,
            B64.encode(hash)
        );
        let inv = parse_stream_line(json.as_bytes()).unwrap();
        assert_eq!(inv.payment_hash, hex::encode(hash));
        assert_eq!(inv.state, InvoiceState::Settled);
        assert_eq!(inv.value_sats, 1500);
        assert_eq!(inv.amount_paid_msat, 1_500_000);
        assert!(inv.settled_at.is_some());
    }

    #[test]
    fn open_invoice_has_no_settle_time() {
        let json = format!(
            r#"{{"result":{{"r_hash":"{}","state":"OPEN","value":"10","settle_date":"0"}}}}"#,
            B64.encode([1u8; 32])
        );
        let inv = parse_stream_line(json.as_bytes()).unwrap();
        assert_eq!(inv.state, InvoiceState::Open);
        assert!(inv.settled_at.is_none());
    }

    #[test]
    fn rejects_unknown_state_and_stream_errors() {
        let bad = format!(
            r#"{{"result":{{"r_hash":"{}","state":"WEIRD"}}}}"#,
            B64.encode([1u8; 32])
        );
        assert!(parse_stream_line(bad.as_bytes()).is_err());
        assert!(parse_stream_line(br#"{"error":{"code":2}}"#).is_err());
    }

    #[test]
    fn hex_hash_validation() {
        assert!(is_hex_hash(&"a".repeat(64)));
        assert!(!is_hex_hash(&"A".repeat(64)));
        assert!(!is_hex_hash(&"a".repeat(63)));
        assert!(!is_hex_hash("../../v1/getinfo"));
    }
}
