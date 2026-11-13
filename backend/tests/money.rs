mod common;

use axum::http::StatusCode;
use common::{TestApp, paid_invoice};
use serde_json::json;
use sqlx::PgPool;

// Test rates (tests/common): 1 BTC = 70,000 USD; KES 130/USD -> 1 BTC = 9,100,000 KES.

// ---------- Local-currency pricing ----------

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn invoice_priced_in_local_currency_uses_server_rate(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;

    // KES 182.00 at 9,100,000 KES/BTC = 2,000 sats.
    let r = app
        .request(
            "POST",
            "/api/payments/invoices",
            Some(&cookie),
            Some(json!({"fiat_amount": "182", "description": "Tomatoes", "btc_price": 1})),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{:?}", r.body);
    assert_eq!(
        r.body["amount_sats"], 2000,
        "client-supplied rate must be ignored"
    );
    assert_eq!(r.body["fiat"]["currency"], "KES");
    assert_eq!(r.body["fiat"]["amount"], "182.00");

    // The node invoice is for the converted sats amount.
    let hash = r.body["payment_hash"].as_str().unwrap();
    assert_eq!(app.ln.invoices.lock().unwrap()[hash].value_sats, 2000);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn sats_invoice_records_local_value(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let r = app
        .request(
            "POST",
            "/api/payments/invoices",
            Some(&cookie),
            Some(json!({"amount_sats": 10_000})),
        )
        .await;
    assert_eq!(r.body["fiat"]["amount"], "910.00"); // 10,000 sats = KES 910
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn rejects_ambiguous_or_invalid_amounts(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    for body in [
        json!({"amount_sats": 100, "fiat_amount": "10"}),
        json!({}),
        json!({"fiat_amount": "abc"}),
        json!({"fiat_amount": "0"}),
        json!({"fiat_amount": "-5"}),
        json!({"fiat_amount": "10.123"}),
        json!({"fiat_amount": "999999999"}), // above the sats cap
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
    }
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn currency_setting_and_rates(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;

    let s = app
        .request("GET", "/api/settings", Some(&cookie), None)
        .await;
    assert_eq!(s.body["currency"], "KES");
    assert!(s.body["currencies"].as_array().unwrap().len() >= 20);

    let rate = app.request("GET", "/api/rates", Some(&cookie), None).await;
    assert_eq!(rate.body["currency"], "KES");
    assert!((rate.body["btc_price"].as_f64().unwrap() - 9_100_000.0).abs() < 1e-6);

    let r = app
        .request(
            "PUT",
            "/api/settings",
            Some(&cookie),
            Some(json!({"currency": "UGX"})),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    let me = app
        .request("GET", "/api/auth/me", Some(&cookie), None)
        .await;
    assert_eq!(me.body["currency"], "UGX");

    // UGX has no decimals.
    let inv = app
        .request(
            "POST",
            "/api/payments/invoices",
            Some(&cookie),
            Some(json!({"fiat_amount": "5180"})),
        )
        .await;
    assert_eq!(inv.status, StatusCode::CREATED);
    assert_eq!(inv.body["fiat"]["amount"], "5180");
    assert_eq!(inv.body["amount_sats"], 2000); // 1 BTC = 259,000,000 UGX

    for bad in ["EUR", "XYZ", "kes", ""] {
        let r = app
            .request(
                "PUT",
                "/api/settings",
                Some(&cookie),
                Some(json!({"currency": bad})),
            )
            .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{bad}");
    }
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn missing_rate_blocks_fiat_pricing_but_not_sats(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    // TZS is supported but the test rate service has no quote for it.
    app.request(
        "PUT",
        "/api/settings",
        Some(&cookie),
        Some(json!({"currency": "TZS"})),
    )
    .await;

    let r = app
        .request(
            "POST",
            "/api/payments/invoices",
            Some(&cookie),
            Some(json!({"fiat_amount": "5000"})),
        )
        .await;
    assert_eq!(r.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(r.body["error"]["code"], "rate_unavailable");

    let r = app
        .request(
            "POST",
            "/api/payments/invoices",
            Some(&cookie),
            Some(json!({"amount_sats": 500})),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert!(r.body["fiat"].is_null());
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn receipts_and_dashboard_show_local_values(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let r = app
        .request(
            "POST",
            "/api/payments/invoices",
            Some(&cookie),
            Some(json!({"fiat_amount": "182"})),
        )
        .await;
    let hash = r.body["payment_hash"].as_str().unwrap().to_string();
    let inv = app.ln.settle(&hash);
    soko_backend::services::payments::apply_node_invoice(&app.db, &inv)
        .await
        .unwrap();

    let receipt = app
        .request(
            "GET",
            &format!("/api/payments/{hash}/receipt"),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(receipt.body["fiat"]["amount"], "182.00");
    let number = receipt.body["receipt_number"].as_str().unwrap();
    let public = app
        .request("GET", &format!("/api/public/receipts/{number}"), None, None)
        .await;
    assert_eq!(public.body["fiat"]["currency"], "KES");

    let d = app
        .request("GET", "/api/dashboard/summary", Some(&cookie), None)
        .await;
    assert_eq!(d.body["currency"], "KES");
    assert_eq!(d.body["total_sales_fiat"]["amount"], "182.00");
    assert_eq!(d.body["available_sats"], 2000);
}

// ---------- Demo cash-out ----------

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn demo_payout_reduces_available_balance(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;

    let none = app
        .request(
            "POST",
            "/api/payouts",
            Some(&cookie),
            Some(json!({"amount_sats": 500, "provider": "M-Pesa", "destination": "0712345678"})),
        )
        .await;
    assert_eq!(
        none.status,
        StatusCode::BAD_REQUEST,
        "nothing to cash out yet"
    );

    paid_invoice(&app, &cookie, 2000, "sale").await;
    let p = app
        .request(
            "POST",
            "/api/payouts",
            Some(&cookie),
            Some(json!({"amount_sats": 1500, "provider": "M-Pesa", "destination": "+254 712 345 678"})),
        )
        .await;
    assert_eq!(p.status, StatusCode::CREATED, "{:?}", p.body);
    assert_eq!(p.body["is_demo"], true);
    assert_eq!(p.body["destination"], "•••• 5678");
    assert!(
        !p.body.to_string().contains("712345678"),
        "full number must not be returned"
    );
    assert_eq!(p.body["fiat"]["currency"], "KES");

    let d = app
        .request("GET", "/api/dashboard/summary", Some(&cookie), None)
        .await;
    assert_eq!(d.body["available_sats"], 500);
    assert_eq!(d.body["paid_out_sats"], 1500);
    assert_eq!(
        d.body["total_sales_sats"], 2000,
        "sales totals are unchanged by cash-outs"
    );

    let over = app
        .request(
            "POST",
            "/api/payouts",
            Some(&cookie),
            Some(json!({"amount_sats": 600, "provider": "M-Pesa", "destination": "0712345678"})),
        )
        .await;
    assert_eq!(over.status, StatusCode::BAD_REQUEST);

    let list = app
        .request("GET", "/api/payouts", Some(&cookie), None)
        .await;
    assert_eq!(list.body["payouts"].as_array().unwrap().len(), 1);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn payout_validation(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    paid_invoice(&app, &cookie, 5000, "sale").await;
    for body in [
        json!({"amount_sats": 500, "provider": "Wave", "destination": "0712345678"}), // not a KES method
        json!({"amount_sats": 500, "provider": "M-Pesa", "destination": "12"}),
        json!({"amount_sats": 500, "provider": "M-Pesa", "destination": "07123abc78"}),
        json!({"amount_sats": 50, "provider": "M-Pesa", "destination": "0712345678"}), // below minimum
        json!({"provider": "M-Pesa", "destination": "0712345678"}),
    ] {
        let r = app
            .request("POST", "/api/payouts", Some(&cookie), Some(body.clone()))
            .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{body}");
    }
    // Cash out by local amount: KES 9.10 = 100 sats.
    let r = app
        .request(
            "POST",
            "/api/payouts",
            Some(&cookie),
            Some(json!({"fiat_amount": "9.10", "provider": "Airtel Money", "destination": "0733000111"})),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{:?}", r.body);
    assert_eq!(r.body["amount_sats"], 100);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn concurrent_payouts_never_exceed_balance(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    paid_invoice(&app, &cookie, 2000, "sale").await;

    let tasks: Vec<_> = (0..6)
        .map(|_| {
            let router = app.router.clone();
            let cookie = cookie.clone();
            tokio::spawn(async move {
                use tower::ServiceExt;
                let req = axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/payouts")
                    .header("cookie", cookie)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        json!({"amount_sats": 600, "provider": "M-Pesa", "destination": "0712345678"}).to_string(),
                    ))
                    .unwrap();
                router.oneshot(req).await.unwrap().status()
            })
        })
        .collect();
    let mut ok = 0;
    for t in tasks {
        if t.await.unwrap() == StatusCode::CREATED {
            ok += 1;
        }
    }
    assert_eq!(ok, 3, "2,000 sats allows exactly three 600-sat cash-outs");
    let total: i64 = sqlx::query_scalar("SELECT SUM(amount_sats)::BIGINT FROM payouts")
        .fetch_one(&app.db)
        .await
        .unwrap();
    assert_eq!(total, 1800);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn payouts_are_private_per_merchant(db: PgPool) {
    let app = TestApp::new(db);
    let alice = app.register("alice@example.com").await;
    let bob = app.register("bob@example.com").await;
    paid_invoice(&app, &alice, 2000, "sale").await;
    app.request(
        "POST",
        "/api/payouts",
        Some(&alice),
        Some(json!({"amount_sats": 1000, "provider": "M-Pesa", "destination": "0712345678"})),
    )
    .await;
    let list = app.request("GET", "/api/payouts", Some(&bob), None).await;
    assert!(list.body["payouts"].as_array().unwrap().is_empty());
    // Bob has no balance of his own to cash out.
    let r = app
        .request(
            "POST",
            "/api/payouts",
            Some(&bob),
            Some(json!({"amount_sats": 500, "provider": "M-Pesa", "destination": "0712345678"})),
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
}
