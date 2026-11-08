mod common;

use axum::http::StatusCode;
use common::{TestApp, paid_invoice};
use serde_json::{Value, json};
use sqlx::PgPool;

// ---------- Dashboard ----------

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn dashboard_summary_counts_only_real_paid_sales(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;

    let empty = app
        .request("GET", "/api/dashboard/summary", Some(&cookie), None)
        .await;
    assert_eq!(empty.status, StatusCode::OK);
    assert_eq!(empty.body["total_sales_sats"], 0);
    assert_eq!(empty.body["last_7_days"].as_array().unwrap().len(), 7);

    paid_invoice(&app, &cookie, 1000, "rice").await;
    paid_invoice(&app, &cookie, 2500, "beans").await;
    app.create_invoice(&cookie, 999).await; // pending

    // A demo row must never count towards sales.
    let demo = paid_invoice(&app, &cookie, 50_000, "demo").await;
    sqlx::query("UPDATE payments SET is_demo = true WHERE payment_hash = $1")
        .bind(&demo)
        .execute(&app.db)
        .await
        .unwrap();

    let r = app
        .request(
            "GET",
            "/api/dashboard/summary?tz=Africa/Nairobi",
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{:?}", r.body);
    assert_eq!(r.body["time_zone"], "Africa/Nairobi");
    assert_eq!(r.body["total_sales_sats"], 3500);
    assert_eq!(r.body["paid_count"], 2);
    assert_eq!(r.body["pending_count"], 1);
    assert_eq!(r.body["today_sales_sats"], 3500);
    assert_eq!(r.body["today_count"], 2);
    let days = r.body["last_7_days"].as_array().unwrap();
    assert_eq!(days.last().unwrap()["sales_sats"], 3500);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn dashboard_is_per_merchant_and_validates_time_zone(db: PgPool) {
    let app = TestApp::new(db);
    let alice = app.register("alice@example.com").await;
    let bob = app.register("bob@example.com").await;
    paid_invoice(&app, &alice, 1000, "x").await;

    let r = app
        .request("GET", "/api/dashboard/summary", Some(&bob), None)
        .await;
    assert_eq!(r.body["total_sales_sats"], 0);

    for bad in [
        "Mars/Olympus",
        "UTC';DROP TABLE payments;--",
        "a".repeat(80).as_str(),
    ] {
        let r = app
            .request(
                "GET",
                &format!("/api/dashboard/summary?tz={}", urlencode(bad)),
                Some(&bob),
                None,
            )
            .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{bad}");
    }
    assert_eq!(
        app.request("GET", "/api/dashboard/summary", None, None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

// ---------- History ----------

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn history_search_and_filters(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let rice = paid_invoice(&app, &cookie, 1000, "Basmati rice 2kg").await;
    paid_invoice(&app, &cookie, 2000, "Tomatoes").await;
    app.create_invoice(&cookie, 3000).await;

    let list = |q: &str| {
        let app = &app;
        let cookie = cookie.clone();
        let q = q.to_string();
        async move {
            let r = app
                .request("GET", &format!("/api/payments?{q}"), Some(&cookie), None)
                .await;
            assert_eq!(r.status, StatusCode::OK, "{q}: {:?}", r.body);
            r.body
        }
    };

    // Receipt numbers are included in the list.
    let all = list("").await;
    let rice_row = all["payments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["payment_hash"] == rice)
        .unwrap();
    let receipt = rice_row["receipt_number"].as_str().unwrap().to_string();

    assert_eq!(
        count(&list("q=RICE").await),
        1,
        "description, case-insensitive"
    );
    assert_eq!(
        count(&list(&format!("q={}", receipt.to_lowercase())).await),
        1,
        "receipt number"
    );
    assert_eq!(
        count(&list(&format!("q={}", &rice[..10])).await),
        1,
        "hash prefix"
    );
    assert_eq!(count(&list("q=%25").await), 0, "LIKE wildcards are literal");
    assert_eq!(count(&list("q=_").await), 0);
    assert_eq!(count(&list("status=paid&q=tomato").await), 1);
    assert_eq!(count(&list("status=pending&q=rice").await), 0);

    let today = chrono::Utc::now().date_naive();
    let tomorrow = today.succ_opt().unwrap();
    assert_eq!(
        count(&list(&format!("from={today}&to={today}&tz=UTC")).await),
        3
    );
    assert_eq!(count(&list(&format!("from={tomorrow}")).await), 0);
    assert_eq!(count(&list("to=2020-01-01").await), 0);

    let page1 = list("limit=2").await;
    assert_eq!(count(&page1), 2);
    assert_eq!(page1["has_more"], true);
    let page2 = list("limit=2&offset=2").await;
    assert_eq!(count(&page2), 1);
    assert_eq!(page2["has_more"], false);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn history_rejects_bad_filters(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    for q in [
        "from=2026-02-30",
        "from=yesterday",
        "from=2026-05-02&to=2026-05-01",
        "from=2026-05-01&tz=Not/AZone",
        &format!("q={}", "x".repeat(65)),
    ] {
        let r = app
            .request("GET", &format!("/api/payments?{q}"), Some(&cookie), None)
            .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{q}");
    }
}

// ---------- Receipts & Nostr ----------

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn receipt_is_nostr_signed_without_leaking_payment_details(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let hash = paid_invoice(&app, &cookie, 4200, "Customer: Jane, 0712 345678").await;

    let r = app
        .request(
            "GET",
            &format!("/api/payments/{hash}/receipt"),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    let number = r.body["receipt_number"].as_str().unwrap().to_string();
    assert_eq!(r.body["public_path"], format!("/r/{number}"));
    let event = r.body["nostr"]["event"].clone();
    assert_eq!(event["kind"], 30078);
    assert!(
        r.body["nostr"]["npub"]
            .as_str()
            .unwrap()
            .starts_with("npub1")
    );

    // Nothing private in the public event.
    let event_text = event.to_string();
    assert!(!event_text.contains(&hash));
    assert!(!event_text.contains("Jane"));
    assert!(!event_text.contains("lnbcrt"));
    let content: Value = serde_json::from_str(event["content"].as_str().unwrap()).unwrap();
    assert_eq!(content["amount"], 4200);
    assert_eq!(content["receipt_number"], number);

    // The payer can link it to their payment with the verification code + payment hash.
    let salt = hex::decode(r.body["nostr"]["verification_code"].as_str().unwrap()).unwrap();
    assert_eq!(
        content["commitment"],
        soko_backend::nostr::receipt_commitment(&salt, &hash).unwrap()
    );

    // Signed once: repeated views return the identical event.
    let again = app
        .request(
            "GET",
            &format!("/api/payments/{hash}/receipt"),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(again.body["nostr"]["event"], event);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn public_receipt_page_shows_only_receipt_fields(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let hash = paid_invoice(&app, &cookie, 4200, "Customer: Jane").await;
    let merchant_view = app
        .request(
            "GET",
            &format!("/api/payments/{hash}/receipt"),
            Some(&cookie),
            None,
        )
        .await;
    let number = merchant_view.body["receipt_number"].as_str().unwrap();

    let r = app
        .request("GET", &format!("/api/public/receipts/{number}"), None, None)
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body["business_name"], "Mama Njeri Groceries");
    assert_eq!(r.body["amount_sats"], 4200);
    assert_eq!(
        r.body["nostr"]["event"],
        merchant_view.body["nostr"]["event"]
    );
    let text = r.body.to_string();
    assert!(!text.contains(&hash), "no payment hash");
    assert!(!text.contains("Jane"), "no description");
    assert!(!text.contains("lnbcrt"), "no invoice");
    assert!(!text.contains("m@example.com"), "no merchant email");

    for bad in ["SK-ZZZZZ-ZZZZZ", "not-a-receipt", "SK-2DA4W-3X62v"] {
        let r = app
            .request("GET", &format!("/api/public/receipts/{bad}"), None, None)
            .await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{bad}");
    }
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn verify_endpoint_accepts_genuine_and_rejects_forged_receipts(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let hash = paid_invoice(&app, &cookie, 4200, "x").await;
    let r = app
        .request(
            "GET",
            &format!("/api/payments/{hash}/receipt"),
            Some(&cookie),
            None,
        )
        .await;
    let event = r.body["nostr"]["event"].clone();

    let ok = app
        .request(
            "POST",
            "/api/public/receipts/verify",
            None,
            Some(event.clone()),
        )
        .await;
    assert_eq!(ok.status, StatusCode::OK);
    assert_eq!(ok.body["signature_valid"], true);
    assert_eq!(ok.body["issued_by_known_merchant"], true);
    assert_eq!(ok.body["matches_records"], true);
    assert_eq!(ok.body["business_name"], "Mama Njeri Groceries");
    assert_eq!(ok.body["amount_sats"], 4200);

    // Edited amount: signature breaks.
    let mut tampered = event.clone();
    tampered["content"] =
        Value::String(event["content"].as_str().unwrap().replace("4200", "42000"));
    let r = app
        .request("POST", "/api/public/receipts/verify", None, Some(tampered))
        .await;
    assert_eq!(r.body["signature_valid"], false);
    assert_eq!(r.body["matches_records"], false);

    // Validly signed, but by a key that is not one of our merchants.
    let outsider = soko_backend::nostr::NostrSigner::new([1u8; 32]);
    let content: soko_backend::nostr::ReceiptContent =
        serde_json::from_str(event["content"].as_str().unwrap()).unwrap();
    let forged =
        serde_json::to_value(outsider.sign_receipt(uuid::Uuid::new_v4(), &content)).unwrap();
    let r = app
        .request("POST", "/api/public/receipts/verify", None, Some(forged))
        .await;
    assert_eq!(r.body["signature_valid"], true);
    assert_eq!(r.body["issued_by_known_merchant"], false);
    assert_eq!(r.body["matches_records"], false);

    // Garbage input.
    let r = app
        .request(
            "POST",
            "/api/public/receipts/verify",
            None,
            Some(json!({"hello": 1})),
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "soko_backend::MIGRATOR")]
async fn me_includes_merchant_nostr_key(db: PgPool) {
    let app = TestApp::new(db);
    let cookie = app.register("m@example.com").await;
    let me = app
        .request("GET", "/api/auth/me", Some(&cookie), None)
        .await;
    assert!(me.body["nostr_npub"].as_str().unwrap().starts_with("npub1"));
}

fn count(body: &Value) -> usize {
    body["payments"].as_array().unwrap().len()
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'/' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
