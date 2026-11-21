//! F006 acceptance tests against a disposable `MariaDB` container (requires Docker).
#![allow(clippy::unwrap_used, clippy::missing_panics_doc)] // test helpers: a failure is a test failure

mod common;

use axum::http::StatusCode;
use common::{TestApp, json_body};
use serde_json::{Value, json};

const PRICES: &str = "/api/admin/prices";

async fn book(app: &TestApp, cookie: &str) -> Value {
    json_body(app.get(PRICES, Some(cookie)).await).await
}

async fn night(app: &TestApp, cookie: &str, date: &str, next: &str) -> Value {
    let nights = json_body(
        app.get(
            &format!("{PRICES}/nights?from={date}&to={next}"),
            Some(cookie),
        )
        .await,
    )
    .await;
    nights[0].clone()
}

async fn send(app: &TestApp, cookie: &str, method: &str, path: &str, body: &Value) -> StatusCode {
    app.send_json(method, &format!("{PRICES}{path}"), Some(cookie), body)
        .await
        .status()
}

#[tokio::test]
async fn f006_s01_s06_seed_and_default_prices() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;

    let seeded = book(&app, &cookie).await;
    assert_eq!(
        seeded["defaults"]["direct"],
        json!({ "listed": 10000, "net": 10000 })
    );
    assert_eq!(seeded["incomplete"], json!(["airbnb", "booking"]));
    assert_eq!(seeded["settings"]["default_min_nights"], 2);
    assert_eq!(seeded["channels"][2]["effective_bp"], 1968);

    let status = send(
        &app,
        &cookie,
        "PUT",
        "/defaults",
        &json!({ "direct": { "listed": 8000, "net": 7900 }, "booking": { "listed": 13700, "net": 11000 } }),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let after = book(&app, &cookie).await;
    assert_eq!(after["incomplete"], json!(["airbnb"]));
    let june = night(&app, &cookie, "2027-06-01", "2027-06-02").await;
    assert_eq!(
        june["prices"]["direct"],
        json!({ "listed": 8000, "net": 7900 })
    );
    assert_eq!(june["prices"]["airbnb"], Value::Null);
}

#[tokio::test]
async fn f006_s02_s03_ranges_cut_each_other() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let status = send(
        &app,
        &cookie,
        "POST",
        "/ranges",
        &json!({
            "start": "2027-07-01", "end": "2027-07-31", "label": "Juillet", "min_nights": 2,
            "prices": { "direct": { "listed": 11000, "net": 10800 }, "airbnb": { "listed": 13000, "net": 11000 } }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    send(
        &app,
        &cookie,
        "POST",
        "/ranges",
        &json!({ "start": "2027-07-10", "end": "2027-07-20", "min_nights": 3,
                 "prices": { "direct": { "listed": 15000, "net": 15000 } } }),
    )
    .await;

    let ranges = book(&app, &cookie).await["ranges"].clone();
    let dates: Vec<(Value, Value)> = ranges
        .as_array()
        .unwrap()
        .iter()
        .map(|range| (range["start"].clone(), range["end"].clone()))
        .collect();
    assert_eq!(
        dates,
        vec![
            (json!("2027-07-01"), json!("2027-07-10")),
            (json!("2027-07-10"), json!("2027-07-20")),
            (json!("2027-07-20"), json!("2027-07-31")),
        ]
    );
    // The cut range keeps its prices; the new one falls back to the default for Airbnb.
    assert_eq!(ranges[2]["prices"]["airbnb"]["listed"], 13000);
    let middle = night(&app, &cookie, "2027-07-12", "2027-07-13").await;
    assert_eq!(middle["prices"]["direct"]["listed"], 15000);
    assert_eq!(
        middle["prices"]["airbnb"],
        Value::Null,
        "no Airbnb default price"
    );
    assert_eq!(middle["min_nights"], 3);
}

#[tokio::test]
async fn f006_s05_invalid_prices_are_refused() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let response = app
        .send_json(
            "PUT",
            &format!("{PRICES}/defaults"),
            Some(&cookie),
            &json!({ "direct": { "listed": 10000, "net": 12000 } }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = json_body(response).await;
    assert_eq!(
        body["error"]["details"][0],
        "le prix touché ne peut pas dépasser le prix affiché"
    );
    // Nothing was saved.
    assert_eq!(
        book(&app, &cookie).await["defaults"]["direct"]["listed"],
        10000
    );
}

#[tokio::test]
async fn f006_s07_cleaning_fee_and_typical_stays() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let status = send(
        &app,
        &cookie,
        "PUT",
        "/settings",
        &json!({ "cleaning_fee": 6000, "default_min_nights": 2, "commission_alert_points": 5 }),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let after = book(&app, &cookie).await;
    assert_eq!(
        after["default_typical_stays"]["7"]["direct"],
        json!([76000, 76000])
    );
    assert_eq!(after["default_typical_stays"]["2"]["airbnb"], Value::Null);
}

#[tokio::test]
async fn f006_s09_s10_copy_year_and_channel() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    send(
        &app,
        &cookie,
        "POST",
        "/ranges",
        &json!({ "start": "2027-07-06", "end": "2027-09-01", "label": "Été", "min_nights": 2,
                 "prices": { "direct": { "listed": 11000, "net": 10800 } } }),
    )
    .await;
    let copied = app
        .send_json(
            "POST",
            &format!("{PRICES}/copy-year"),
            Some(&cookie),
            &json!({ "year": 2027 }),
        )
        .await;
    assert_eq!(json_body(copied).await["copied"], 1);
    let status = send(
        &app,
        &cookie,
        "POST",
        "/copy-channel",
        &json!({ "from": "direct", "to": "booking" }),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let after = book(&app, &cookie).await;
    assert_eq!(after["ranges"][1]["start"], "2028-07-04");
    assert_eq!(after["ranges"][1]["label"], "Été");
    assert_eq!(after["ranges"][1]["prices"]["booking"]["listed"], 11000);
    assert_eq!(after["defaults"]["booking"]["listed"], 10000);
}

#[tokio::test]
async fn f006_s11_s13_history_and_rates_never_change_prices() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    send(
        &app,
        &cookie,
        "PUT",
        "/defaults",
        &json!({ "booking": { "listed": 14000, "net": 11500 } }),
    )
    .await;
    let status = send(
        &app,
        &cookie,
        "PUT",
        "/channels/booking",
        &json!({ "commission_bp": 1700, "payment_fees_bp": 140, "vat_on_commission": true }),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let after = book(&app, &cookie).await;
    assert_eq!(
        after["defaults"]["booking"],
        json!({ "listed": 14000, "net": 11500 })
    );
    assert_eq!(after["channels"][2]["commission_bp"], 1700);

    let history = json_body(app.get(&format!("{PRICES}/history"), Some(&cookie)).await).await;
    assert_eq!(history[0]["description"], "Taux de Booking modifiés");
    assert_eq!(
        history[1]["description"],
        "Prix par défaut : Booking 140 € / 115 €"
    );
    assert_eq!(history[1]["author"], common::EMAIL);
}

#[tokio::test]
async fn f006_s12_admin_only() {
    let app = TestApp::start().await;
    for path in ["", "/nights?from=2027-01-01&to=2027-01-02", "/history"] {
        assert_eq!(
            app.get(&format!("{PRICES}{path}"), None).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        app.send_json("PUT", &format!("{PRICES}/defaults"), None, &json!({}))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.send_json("POST", &format!("{PRICES}/ranges"), None, &json!({}))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
