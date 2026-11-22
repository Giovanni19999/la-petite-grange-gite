//! F004 acceptance tests against a disposable `MariaDB` container (requires Docker).
#![allow(clippy::unwrap_used, clippy::missing_panics_doc)] // test helpers: a failure is a test failure

mod common;

use axum::http::StatusCode;
use common::{TestApp, json_body};
use serde_json::{Value, json};
use time::{Date, Duration, OffsetDateTime, Weekday};

/// A day `days` from now (UTC; the tests keep away from the minimum delay and midnight).
fn day(days: i64) -> Date {
    OffsetDateTime::now_utc().date() + Duration::days(days)
}

/// First `weekday` at least `days` from now.
fn next(weekday: Weekday, days: i64) -> Date {
    let mut date = day(days);
    while date.weekday() != weekday {
        date = date.next_day().unwrap();
    }
    date
}

async fn calendar(app: &TestApp, from: Date, to: Date) -> Value {
    json_body(
        app.get(&format!("/api/calendar?from={from}&to={to}"), None)
            .await,
    )
    .await
}

async fn quote(app: &TestApp, body: &Value) -> (StatusCode, Value) {
    let response = app.send_json("POST", "/api/quote", None, body).await;
    let status = response.status();
    (status, json_body(response).await)
}

fn stay(arrival: Date, departure: Date, adults: u8, children: u8) -> Value {
    json!({ "arrival": arrival, "departure": departure, "adults": adults, "children": children })
}

async fn rules(app: &TestApp, cookie: &str) -> Value {
    json_body(app.get("/api/admin/house-rules", Some(cookie)).await).await
}

async fn set_rules(app: &TestApp, cookie: &str, rules: &Value) -> StatusCode {
    app.send_json("PUT", "/api/admin/house-rules", Some(cookie), rules)
        .await
        .status()
}

#[tokio::test]
async fn f004_s01_s02_s08_free_nights_show_their_price_others_no_detail() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let (from, to) = (day(40), day(50));
    let response = app
        .send_json(
            "POST",
            "/api/admin/blocked-periods",
            Some(&cookie),
            &json!({ "start": day(42), "end": day(45), "kind": "family", "label": "Mamie et Papi", "people": 4 }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = calendar(&app, from, to).await;
    let nights = body["nights"].as_array().unwrap();
    assert_eq!(nights.len(), 10);
    // Seed: the site price is 100 € with a minimum of 2 nights.
    assert_eq!(
        nights[0],
        json!({ "date": day(40), "price": 10000, "min_nights": 2 })
    );
    // The family period: unavailable, nothing else.
    assert_eq!(nights[2], json!({ "date": day(42), "price": null }));
    assert_eq!(nights[4], json!({ "date": day(44), "price": null }));
    assert_eq!(nights[5]["price"], 10000);
    assert!(!body.to_string().contains("Mamie"));
    assert_eq!(body["capacity"], 5);
    assert_eq!(body["children_under"], 18);
    assert_eq!(body["arrival_weekdays"], json!([1, 2, 3, 4, 5, 6, 7]));
}

#[tokio::test]
async fn f004_s03_exact_quote() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let status = app
        .send_json(
            "PUT",
            "/api/admin/prices/settings",
            Some(&cookie),
            &json!({ "cleaning_fee": 6000, "default_min_nights": 2, "commission_alert_points": 5 }),
        )
        .await
        .status();
    assert_eq!(status, StatusCode::NO_CONTENT);
    let mut new_rules = rules(&app, &cookie).await;
    new_rules["tourist_tax"]["per_adult_night"] = json!(170);
    new_rules["tourist_tax"]["additional_bp"] = json!(1000);
    assert_eq!(
        set_rules(&app, &cookie, &new_rules).await,
        StatusCode::NO_CONTENT
    );

    let arrival = day(60);
    let (status, body) = quote(&app, &stay(arrival, day(63), 2, 1)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({
            "nights": 3, "nights_total": 30000, "promotion": null, "cleaning_fee": 6000,
            "options": [], "options_total": 0, "stay_total": 36000,
            "deposit": 18000, "balance": 18000, "balance_due": arrival - Duration::days(30),
            "tourist_tax": 1122
        })
    );
}

#[tokio::test]
async fn f004_s04_refused_quotes_explain_why() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    app.send_json(
        "POST",
        "/api/admin/blocked-periods",
        Some(&cookie),
        &json!({ "start": day(41), "end": day(43), "kind": "closure" }),
    )
    .await;

    let (status, body) = quote(&app, &stay(day(40), day(44), 2, 0)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["details"], json!(["dates indisponibles"]));

    let (_, body) = quote(&app, &stay(day(50), day(51), 2, 0)).await;
    assert_eq!(
        body["error"]["details"],
        json!(["2 nuits minimum pour cette date d'arrivée"])
    );

    let (_, body) = quote(&app, &stay(day(50), day(53), 4, 2)).await;
    assert_eq!(body["error"]["details"], json!(["5 personnes au maximum"]));
}

#[tokio::test]
async fn f004_s05_horizon_and_minimum_delay() {
    let app = TestApp::start().await;
    let body = calendar(&app, day(-1), day(3)).await;
    // Yesterday and today are before the earliest arrival (minimum delay of 1 day).
    assert_eq!(body["nights"][0]["price"], Value::Null);
    assert_eq!(body["nights"][1]["price"], Value::Null);
    assert_eq!(body["nights"][3]["price"], 10000);

    let far = OffsetDateTime::now_utc().date() + Duration::days(25 * 31);
    let body = calendar(&app, far, far + Duration::days(28)).await;
    assert!(
        body["nights"]
            .as_array()
            .unwrap()
            .iter()
            .all(|night| night["price"].is_null())
    );
}

#[tokio::test]
async fn f004_s09_house_rules_and_history() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let seeded = rules(&app, &cookie).await;
    assert_eq!(
        seeded,
        json!({
            "capacity": 5, "min_delay_days": 1,
            "arrival_weekdays": [1, 2, 3, 4, 5, 6, 7], "departure_weekdays": [1, 2, 3, 4, 5, 6, 7],
            "deposit": { "mode": "percentage", "percent": 50 }, "balance_days": 30,
            "tourist_tax": { "per_adult_night": 0, "additional_bp": 0, "exempt_under": 18 },
            "horizon": { "months": 24, "until": null },
            "arrival_from": 960, "departure_before": 600, "pets_allowed": false,
            "deposit_alert_days": 7, "terms_page": null
        })
    );

    let mut changed = seeded.clone();
    changed["capacity"] = json!(6);
    changed["tourist_tax"]["per_adult_night"] = json!(170);
    assert_eq!(
        set_rules(&app, &cookie, &changed).await,
        StatusCode::NO_CONTENT
    );
    let (status, body) = quote(&app, &stay(day(60), day(63), 4, 2)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["tourist_tax"], 4 * 3 * 170);

    let history = json_body(
        app.get("/api/admin/house-rules/history", Some(&cookie))
            .await,
    )
    .await;
    assert_eq!(
        history[0]["description"],
        "Capacité : 5 → 6 ; Taxe de séjour : 0 € → 1,70 €"
    );
    assert_eq!(history[0]["author"], common::EMAIL);

    let mut broken = changed.clone();
    broken["capacity"] = json!(0);
    broken["arrival_weekdays"] = json!([]);
    let response = app
        .send_json("PUT", "/api/admin/house-rules", Some(&cookie), &broken)
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json_body(response).await["error"]["details"],
        json!([
            "capacité : de 1 à 50 personnes",
            "choisissez au moins un jour d'arrivée"
        ])
    );
}

#[tokio::test]
async fn f004_s10_allowed_weekdays() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let mut changed = rules(&app, &cookie).await;
    changed["arrival_weekdays"] = json!([6]);
    changed["departure_weekdays"] = json!([6]);
    set_rules(&app, &cookie, &changed).await;

    let wednesday = next(Weekday::Wednesday, 40);
    let (status, body) = quote(&app, &stay(wednesday, wednesday + Duration::days(3), 2, 0)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body["error"]["details"],
        json!(["arrivée possible : samedi"])
    );
    let saturday = next(Weekday::Saturday, 40);
    let (status, _) = quote(&app, &stay(saturday, saturday + Duration::days(7), 2, 0)).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn f004_house_rules_are_admin_only() {
    let app = TestApp::start().await;
    assert_eq!(
        app.get("/api/admin/house-rules", None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.send_json("PUT", "/api/admin/house-rules", None, &json!({}))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
