mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use common::TestApp;
use serde_json::json;
use soko_backend::{
    lightning::InvoiceState,
    services::payments::{self, Outcome},
};
use sqlx::PgPool;

// ---------- Creation & validation ----------

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn creating_an_invoice_requires_login(db: PgPool) {
    let app = TestApp::new(db);
    let r = app
        .request(
            "POST",
            "/api/payments/invoices",
            None,
            Some(json!({"amount_sats": 100})),
        )
        .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert!(
        app.ln.invoices.lock().unwrap().is_empty(),
        "no invoice may be created"
    );
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn creates_pending_invoice(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let r = app
        .request(
            "POST",
            "/api/payments/invoices",
            Some(&cookie),
            Some(json!({"amount_sats": 2500, "description": "  2kg rice "})),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert_eq!(r.body["status"], "pending");
    assert_eq!(r.body["amount_sats"], 2500);
    assert_eq!(r.body["description"], "2kg rice");
    assert_eq!(r.body["is_demo"], false);
    assert!(
        r.body["payment_request"]
            .as_str()
            .unwrap()
            .starts_with("lnbcrt")
    );
    assert!(r.body["receipt_number"].is_null());
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn rejects_invalid_invoice_requests(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    for body in [
        json!({"amount_sats": 0}),
        json!({"amount_sats": -10}),
        json!({"amount_sats": 10_000_001}),
        json!({"amount_sats": "100"}),
        json!({"amount_sats": 1.5}),
        json!({"description": "no amount"}),
        json!({"amount_sats": 100, "description": "x".repeat(201)}),
    ] {
        let r = app
            .request(
                "POST",
                "/api/payments/invoices",
                Some(&cookie),
                Some(body.clone()),
            )
            .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(r.body["error"]["code"], "validation_error");
    }

    // Wrong content type (e.g. a cross-site HTML form post) is rejected.
    let req = Request::builder()
        .method("POST")
        .uri("/api/payments/invoices")
        .header(header::COOKIE, &cookie)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from("amount_sats=100"))
        .unwrap();
    assert_eq!(app.send(req).await.status, StatusCode::BAD_REQUEST);

    assert!(app.ln.invoices.lock().unwrap().is_empty());
}

// ---------- Verification ----------

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn payment_is_paid_only_after_node_reports_settlement(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let hash = app.create_invoice(&cookie, 1000).await;
    let path = format!("/api/payments/{hash}");

    let r = app.request("GET", &path, Some(&cookie), None).await;
    assert_eq!(r.body["status"], "pending");
    assert_eq!(app.receipt_count().await, 0);

    app.ln.settle(&hash);
    let r = app.request("GET", &path, Some(&cookie), None).await;
    assert_eq!(r.body["status"], "paid");
    assert!(r.body["settled_at"].is_string());
    let receipt_number = r.body["receipt_number"].as_str().unwrap();
    assert!(receipt_number.starts_with("SK-"));

    let receipt = app
        .request("GET", &format!("{path}/receipt"), Some(&cookie), None)
        .await;
    assert_eq!(receipt.status, StatusCode::OK);
    assert_eq!(receipt.body["receipt_number"], receipt_number);
    assert_eq!(receipt.body["amount_sats"], 1000);
    assert_eq!(receipt.body["denomination"], "sats");
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn client_cannot_set_payment_status(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let r = app
        .request(
            "POST",
            "/api/payments/invoices",
            Some(&cookie),
            Some(
                json!({"amount_sats": 500, "status": "paid", "settled_at": "2026-01-01T00:00:00Z"}),
            ),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert_eq!(r.body["status"], "pending");
    assert!(r.body["settled_at"].is_null());

    // No route accepts writes to an existing payment.
    let hash = r.body["payment_hash"].as_str().unwrap();
    for method in ["POST", "PUT", "PATCH"] {
        let r = app
            .request(
                method,
                &format!("/api/payments/{hash}"),
                Some(&cookie),
                Some(json!({"status": "paid"})),
            )
            .await;
        assert_eq!(r.status, StatusCode::METHOD_NOT_ALLOWED, "{method}");
    }
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn duplicate_settlement_events_issue_one_receipt(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let hash = app.create_invoice(&cookie, 1000).await;
    let inv = app.ln.settle(&hash);

    assert_eq!(
        payments::apply_node_invoice(&app.db, &inv).await.unwrap(),
        Outcome::MarkedPaid
    );
    assert_eq!(
        payments::apply_node_invoice(&app.db, &inv).await.unwrap(),
        Outcome::Unchanged
    );
    payments::reconcile_pending(&app.db, app.ln.as_ref())
        .await
        .unwrap();
    app.request("GET", &format!("/api/payments/{hash}"), Some(&cookie), None)
        .await;

    assert_eq!(app.receipt_count().await, 1);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn concurrent_settlement_processing_is_idempotent(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let hash = app.create_invoice(&cookie, 1000).await;
    let inv = app.ln.settle(&hash);

    let tasks: Vec<_> = (0..8)
        .map(|_| {
            let db = app.db.clone();
            let inv = inv.clone();
            tokio::spawn(async move { payments::apply_node_invoice(&db, &inv).await.unwrap() })
        })
        .collect();
    let mut paid = 0;
    for t in tasks {
        if t.await.unwrap() == Outcome::MarkedPaid {
            paid += 1;
        }
    }
    assert_eq!(paid, 1, "exactly one processor may mark the payment paid");
    assert_eq!(app.receipt_count().await, 1);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn canceled_invoice_becomes_expired(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let hash = app.create_invoice(&cookie, 1000).await;
    app.ln.set_state(&hash, InvoiceState::Canceled, 0);

    let r = app
        .request("GET", &format!("/api/payments/{hash}"), Some(&cookie), None)
        .await;
    assert_eq!(r.body["status"], "expired");
    assert_eq!(app.receipt_count().await, 0);

    // A later (impossible) settled event must not resurrect an expired payment.
    let inv = app.ln.settle(&hash);
    assert_eq!(
        payments::apply_node_invoice(&app.db, &inv).await.unwrap(),
        Outcome::Unchanged
    );
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn open_invoice_past_expiry_is_expired_by_reconciler(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let hash = app.create_invoice(&cookie, 1000).await;
    sqlx::query(
        "UPDATE payments SET expires_at = now() - interval '5 minutes' WHERE payment_hash = $1",
    )
    .bind(&hash)
    .execute(&app.db)
    .await
    .unwrap();

    assert_eq!(
        payments::reconcile_pending(&app.db, app.ln.as_ref())
            .await
            .unwrap(),
        1
    );
    let r = app
        .request("GET", &format!("/api/payments/{hash}"), Some(&cookie), None)
        .await;
    assert_eq!(r.body["status"], "expired");
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn underpaid_or_mismatched_invoices_fail(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;

    let underpaid = app.create_invoice(&cookie, 1000).await;
    let inv = app.ln.set_state(&underpaid, InvoiceState::Settled, 999_999);
    assert_eq!(
        payments::apply_node_invoice(&app.db, &inv).await.unwrap(),
        Outcome::MarkedFailed
    );

    let mismatched = app.create_invoice(&cookie, 1000).await;
    app.ln.set_value(&mismatched, 1);
    let inv = app.ln.settle(&mismatched);
    assert_eq!(
        payments::apply_node_invoice(&app.db, &inv).await.unwrap(),
        Outcome::MarkedFailed
    );

    assert_eq!(app.receipt_count().await, 0);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn invoice_missing_from_node_fails(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let hash = app.create_invoice(&cookie, 1000).await;
    app.ln.forget(&hash);
    let r = app
        .request("GET", &format!("/api/payments/{hash}"), Some(&cookie), None)
        .await;
    assert_eq!(r.body["status"], "failed");
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn ignores_node_invoices_soko_did_not_create(db: PgPool) {
    let app = TestApp::new(db);
    let inv = soko_backend::lightning::NodeInvoice {
        payment_hash: "ab".repeat(32),
        state: InvoiceState::Settled,
        value_sats: 10,
        amount_paid_msat: 10_000,
        settled_at: None,
    };
    assert_eq!(
        payments::apply_node_invoice(&app.db, &inv).await.unwrap(),
        Outcome::NotOurs
    );
}

// ---------- Lookup & authorisation ----------

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn invalid_and_unknown_hashes(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    for bad in ["xyz", "ABCDEF", &"a".repeat(63), &"g".repeat(64)] {
        let r = app
            .request("GET", &format!("/api/payments/{bad}"), Some(&cookie), None)
            .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{bad}");
    }
    let r = app
        .request(
            "GET",
            &format!("/api/payments/{}", "c".repeat(64)),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn merchants_cannot_see_each_others_payments(db: PgPool) {
    let app = TestApp::new(db);
    let alice = app.register("alice@example.com").await;
    let bob = app.register("bob@example.com").await;
    let hash = app.create_invoice(&alice, 1000).await;
    app.ln.settle(&hash);

    for path in [
        format!("/api/payments/{hash}"),
        format!("/api/payments/{hash}/receipt"),
    ] {
        let r = app.request("GET", &path, Some(&bob), None).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{path}");
    }
    let list = app.request("GET", "/api/payments", Some(&bob), None).await;
    assert_eq!(list.body["payments"].as_array().unwrap().len(), 0);

    // Bob's lookups must not have processed Alice's payment either.
    assert_eq!(app.receipt_count().await, 0);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn receipt_not_available_until_paid(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let hash = app.create_invoice(&cookie, 1000).await;
    let r = app
        .request(
            "GET",
            &format!("/api/payments/{hash}/receipt"),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn lists_and_filters_payments(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let paid = app.create_invoice(&cookie, 100).await;
    app.create_invoice(&cookie, 200).await;
    app.create_invoice(&cookie, 300).await;
    let inv = app.ln.settle(&paid);
    payments::apply_node_invoice(&app.db, &inv).await.unwrap();

    let all = app
        .request("GET", "/api/payments", Some(&cookie), None)
        .await;
    assert_eq!(all.body["payments"].as_array().unwrap().len(), 3);
    assert_eq!(all.body["payments"][0]["amount_sats"], 300, "newest first");

    let paid_only = app
        .request("GET", "/api/payments?status=paid", Some(&cookie), None)
        .await;
    let list = paid_only.body["payments"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["payment_hash"], paid);

    let limited = app
        .request("GET", "/api/payments?limit=2", Some(&cookie), None)
        .await;
    assert_eq!(limited.body["payments"].as_array().unwrap().len(), 2);

    let bad = app
        .request("GET", "/api/payments?status=refunded", Some(&cookie), None)
        .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
}
