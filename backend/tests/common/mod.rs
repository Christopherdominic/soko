#![allow(dead_code)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use futures_util::stream::{self, BoxStream, StreamExt};
use http_body_util::BodyExt;
use rand::RngCore;
use serde_json::Value;
use soko_backend::{
    config::{Config, LndConfig},
    lightning::{CreatedInvoice, InvoiceState, LightningBackend, NodeHealth, NodeInvoice},
    nostr::NostrSigner,
    routes,
    state::AppState,
};
use sqlx::PgPool;
use tower::ServiceExt;

/// In-memory stand-in for LND. Tests drive invoice state explicitly.
#[derive(Default)]
pub struct FakeLightning {
    pub invoices: Mutex<HashMap<String, NodeInvoice>>,
}

impl FakeLightning {
    pub fn set_state(&self, hash: &str, state: InvoiceState, paid_msat: i64) -> NodeInvoice {
        let mut map = self.invoices.lock().unwrap();
        let inv = map.get_mut(hash).expect("unknown invoice");
        inv.state = state;
        inv.amount_paid_msat = paid_msat;
        inv.settled_at = (state == InvoiceState::Settled).then(chrono::Utc::now);
        inv.clone()
    }

    pub fn settle(&self, hash: &str) -> NodeInvoice {
        let sats = self.invoices.lock().unwrap()[hash].value_sats;
        self.set_state(hash, InvoiceState::Settled, sats * 1000)
    }

    pub fn set_value(&self, hash: &str, value_sats: i64) {
        self.invoices
            .lock()
            .unwrap()
            .get_mut(hash)
            .unwrap()
            .value_sats = value_sats;
    }

    pub fn forget(&self, hash: &str) {
        self.invoices.lock().unwrap().remove(hash);
    }
}

#[async_trait]
impl LightningBackend for FakeLightning {
    async fn create_invoice(
        &self,
        amount_sats: i64,
        _memo: Option<&str>,
        _expiry_secs: u32,
    ) -> anyhow::Result<CreatedInvoice> {
        let mut h = [0u8; 32];
        rand::rng().fill_bytes(&mut h);
        let payment_hash = hex::encode(h);
        self.invoices.lock().unwrap().insert(
            payment_hash.clone(),
            NodeInvoice {
                payment_hash: payment_hash.clone(),
                state: InvoiceState::Open,
                value_sats: amount_sats,
                amount_paid_msat: 0,
                settled_at: None,
            },
        );
        Ok(CreatedInvoice {
            payment_request: format!("lnbcrt{amount_sats}fake{}", &payment_hash[..8]),
            payment_hash,
        })
    }

    async fn lookup_invoice(&self, payment_hash: &str) -> anyhow::Result<Option<NodeInvoice>> {
        Ok(self.invoices.lock().unwrap().get(payment_hash).cloned())
    }

    async fn subscribe_invoices(
        &self,
    ) -> anyhow::Result<BoxStream<'static, anyhow::Result<NodeInvoice>>> {
        Ok(stream::pending().boxed())
    }

    async fn health(&self) -> anyhow::Result<NodeHealth> {
        Ok(NodeHealth {
            server_active: true,
        })
    }
}

pub struct TestApp {
    pub router: Router,
    pub db: PgPool,
    pub ln: Arc<FakeLightning>,
}

pub struct Resp {
    pub status: StatusCode,
    pub body: Value,
    pub set_cookie: Option<String>,
}

impl TestApp {
    pub fn new(db: PgPool) -> Self {
        let ln = Arc::new(FakeLightning::default());
        let config = Config {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            database_url: String::new(),
            frontend_origin: "http://localhost:3000".into(),
            invoice_expiry_secs: 900,
            allow_registration: true,
            cookie_secure: false,
            session_ttl_hours: 24,
            nostr_secret: None,
            lnd: LndConfig {
                rest_url: "https://localhost:1".parse().unwrap(),
                tls_cert_pem: vec![],
                macaroon_hex: String::new(),
            },
        };
        let state = AppState {
            db: db.clone(),
            lightning: ln.clone(),
            config: Arc::new(config),
            nostr: Some(Arc::new(NostrSigner::new([42u8; 32]))),
        };
        Self {
            router: routes::router(state),
            db,
            ln,
        }
    }

    pub async fn request(
        &self,
        method: &str,
        uri: &str,
        cookie: Option<&str>,
        body: Option<Value>,
    ) -> Resp {
        let mut req = Request::builder().method(method).uri(uri);
        if let Some(c) = cookie {
            req = req.header(header::COOKIE, c);
        }
        let req = match body {
            Some(b) => req
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(b.to_string())),
            None => req.body(Body::empty()),
        }
        .unwrap();
        self.send(req).await
    }

    pub async fn send(&self, req: Request<Body>) -> Resp {
        let res = self.router.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let set_cookie = res
            .headers()
            .get(header::SET_COOKIE)
            .map(|v| v.to_str().unwrap().to_string());
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Resp {
            status,
            body,
            set_cookie,
        }
    }

    /// Registers a merchant and returns the `Cookie` header value for its session.
    pub async fn register(&self, email: &str) -> String {
        let r = self
            .request(
                "POST",
                "/api/auth/register",
                None,
                Some(serde_json::json!({
                    "business_name": "Mama Njeri Groceries",
                    "email": email,
                    "password": "correct-horse-battery"
                })),
            )
            .await;
        assert_eq!(r.status, StatusCode::CREATED, "{:?}", r.body);
        cookie_from(&r.set_cookie.expect("no session cookie"))
    }

    pub async fn create_invoice(&self, cookie: &str, amount: i64) -> String {
        let r = self
            .request(
                "POST",
                "/api/payments/invoices",
                Some(cookie),
                Some(serde_json::json!({ "amount_sats": amount, "description": "Tomatoes" })),
            )
            .await;
        assert_eq!(r.status, StatusCode::CREATED, "{:?}", r.body);
        r.body["payment_hash"].as_str().unwrap().to_string()
    }

    pub async fn receipt_count(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM receipts")
            .fetch_one(&self.db)
            .await
            .unwrap()
    }
}

/// "soko_session=abc; HttpOnly; ..." -> "soko_session=abc"
pub fn cookie_from(set_cookie: &str) -> String {
    set_cookie.split(';').next().unwrap().to_string()
}

/// Creates an invoice and marks it paid as if LND reported settlement.
pub async fn paid_invoice(app: &TestApp, cookie: &str, amount: i64, description: &str) -> String {
    let r = app
        .request(
            "POST",
            "/api/payments/invoices",
            Some(cookie),
            Some(serde_json::json!({ "amount_sats": amount, "description": description })),
        )
        .await;
    let hash = r.body["payment_hash"].as_str().unwrap().to_string();
    let inv = app.ln.settle(&hash);
    soko_backend::services::payments::apply_node_invoice(&app.db, &inv)
        .await
        .unwrap();
    hash
}
