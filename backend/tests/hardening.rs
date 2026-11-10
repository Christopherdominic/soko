mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use common::TestApp;
use serde_json::json;
use sqlx::PgPool;

fn login_req(email: &str, ip: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", ip)
        .body(Body::from(
            json!({"email": email, "password": "wrong-password"}).to_string(),
        ))
        .unwrap()
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn login_is_limited_per_account_even_across_ips(db: PgPool) {
    let app = TestApp::new(db);
    app.register("victim@example.com").await;

    // 10 attempts from 10 different IPs are allowed (each IP well under its own limit)...
    for i in 0..10 {
        let r = app
            .send(login_req("victim@example.com", &format!("10.0.0.{i}")))
            .await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED, "attempt {i}");
    }
    // ...but the 11th guess at the same account is blocked, whatever the IP.
    let r = app.send(login_req("VICTIM@example.com", "10.0.0.99")).await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(r.body["error"]["code"], "rate_limited");

    // Other accounts are unaffected.
    let r = app
        .send(login_req("someone-else@example.com", "10.0.0.98"))
        .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn per_ip_limit_applies_with_trusted_proxy(db: PgPool) {
    use soko_backend::middleware::rate_limit::{Limits, RateLimiter};
    let limits = Limits {
        per_ip: RateLimiter::new(5, std::time::Duration::from_secs(600)),
        ..Default::default()
    };
    let app = TestApp::with_limits(db, limits);
    let mut limited = false;
    for i in 0..6 {
        // Different accounts, same client IP.
        let r = app
            .send(login_req(&format!("user{i}@example.com"), "203.0.113.7"))
            .await;
        if r.status == StatusCode::TOO_MANY_REQUESTS {
            limited = true;
            break;
        }
    }
    assert!(limited, "one IP must not get unlimited login attempts");
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn invoice_creation_is_limited_per_merchant(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let mut created = 0;
    let mut last = StatusCode::OK;
    for _ in 0..35 {
        let r = app
            .request(
                "POST",
                "/api/payments/invoices",
                Some(&cookie),
                Some(json!({"amount_sats": 10})),
            )
            .await;
        last = r.status;
        if r.status != StatusCode::CREATED {
            break;
        }
        created += 1;
    }
    assert_eq!(created, 30);
    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn api_sets_security_headers_and_limits_body_size(db: PgPool) {
    let app = TestApp::new(db);
    let res = app
        .router
        .clone()
        .oneshot_req(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(res.headers()["cache-control"], "no-store");
    assert_eq!(res.headers()["x-content-type-options"], "nosniff");
    assert_eq!(res.headers()["x-frame-options"], "DENY");

    let big = "x".repeat(100 * 1024);
    let r = app
        .send(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"email":"{big}","password":"x"}}"#)))
                .unwrap(),
        )
        .await;
    assert!(
        r.status.is_client_error(),
        "oversized body rejected, got {}",
        r.status
    );
}

trait OneshotReq {
    async fn oneshot_req(self, req: Request<Body>) -> axum::response::Response;
}

impl OneshotReq for axum::Router {
    async fn oneshot_req(self, req: Request<Body>) -> axum::response::Response {
        use tower::ServiceExt;
        self.oneshot(req).await.unwrap()
    }
}
