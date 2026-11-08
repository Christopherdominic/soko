mod common;

use axum::http::StatusCode;
use common::{TestApp, cookie_from};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn register_sets_secure_session_cookie(db: PgPool) {
    let app = TestApp::new(db);
    let r = app
        .request(
            "POST",
            "/api/auth/register",
            None,
            Some(json!({"business_name": "Duka", "email": "A@Example.com", "password": "longenough"})),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert_eq!(r.body["email"], "a@example.com");
    assert!(r.body.get("password_hash").is_none());
    let sc = r.set_cookie.unwrap();
    assert!(sc.contains("HttpOnly") && sc.contains("SameSite=Strict"));

    let me = app
        .request("GET", "/api/auth/me", Some(&cookie_from(&sc)), None)
        .await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["business_name"], "Duka");
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn register_validates_input_and_rejects_duplicates(db: PgPool) {
    let app = TestApp::new(db);
    app.register("dup@example.com").await;

    for (body, expected) in [
        (
            json!({"business_name": "X", "email": "DUP@example.com", "password": "longenough"}),
            StatusCode::CONFLICT,
        ),
        (
            json!({"business_name": "X", "email": "not-an-email", "password": "longenough"}),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({"business_name": "X", "email": "b@example.com", "password": "short"}),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({"business_name": "  ", "email": "b@example.com", "password": "longenough"}),
            StatusCode::BAD_REQUEST,
        ),
        (json!({"email": "b@example.com"}), StatusCode::BAD_REQUEST),
    ] {
        let r = app
            .request("POST", "/api/auth/register", None, Some(body.clone()))
            .await;
        assert_eq!(r.status, expected, "{body}");
        assert!(r.body["error"]["code"].is_string());
    }
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn login_and_logout(db: PgPool) {
    let app = TestApp::new(db);
    app.register("trader@example.com").await;

    let bad = app
        .request(
            "POST",
            "/api/auth/login",
            None,
            Some(json!({"email": "trader@example.com", "password": "wrong-password"})),
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNAUTHORIZED);
    assert_eq!(bad.body["error"]["code"], "invalid_credentials");

    let unknown = app
        .request(
            "POST",
            "/api/auth/login",
            None,
            Some(json!({"email": "nobody@example.com", "password": "wrong-password"})),
        )
        .await;
    assert_eq!(
        unknown.body, bad.body,
        "unknown email must look the same as wrong password"
    );

    let ok = app
        .request(
            "POST",
            "/api/auth/login",
            None,
            Some(json!({"email": "Trader@Example.com", "password": "correct-horse-battery"})),
        )
        .await;
    assert_eq!(ok.status, StatusCode::OK);
    let cookie = cookie_from(&ok.set_cookie.unwrap());

    assert_eq!(
        app.request("GET", "/api/auth/me", Some(&cookie), None)
            .await
            .status,
        StatusCode::OK
    );
    let out = app
        .request("POST", "/api/auth/logout", Some(&cookie), None)
        .await;
    assert_eq!(out.status, StatusCode::NO_CONTENT);
    assert_eq!(
        app.request("GET", "/api/auth/me", Some(&cookie), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED,
        "session must be invalid after logout"
    );
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn forged_or_missing_session_is_rejected(db: PgPool) {
    let app = TestApp::new(db);
    for cookie in [
        None,
        Some("soko_session=forged-token"),
        Some("soko_session="),
    ] {
        let r = app.request("GET", "/api/payments", cookie, None).await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    }
}
