//! F026 acceptance tests against a disposable `MariaDB` container (requires Docker).
#![allow(clippy::unwrap_used, clippy::missing_panics_doc)] // test helpers: a failure is a test failure

mod common;

use axum::http::StatusCode;
use common::{TestApp, json_body};
use serde_json::{Value, json};
use time::{Date, Duration, OffsetDateTime};

const OFFERS: &str = "/api/admin/offers";

/// A day `days` from now (away from the minimum delay).
fn day(days: i64) -> Date {
    OffsetDateTime::now_utc().date() + Duration::days(days)
}

async fn create(app: &TestApp, cookie: &str, path: &str, body: &Value) -> u64 {
    let response = app
        .send_json("POST", &format!("{OFFERS}{path}"), Some(cookie), body)
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    json_body(response).await["id"].as_u64().unwrap()
}

async fn quote(app: &TestApp, body: &Value) -> (StatusCode, Value) {
    let response = app.send_json("POST", "/api/quote", None, body).await;
    let status = response.status();
    (status, json_body(response).await)
}

fn stay(nights: i64, adults: u8, options: &[u64]) -> Value {
    json!({
        "arrival": day(60), "departure": day(60 + nights),
        "adults": adults, "children": 0, "options": options
    })
}

#[tokio::test]
async fn f026_s01_s03_best_promotion_only() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    create(
        &app,
        &cookie,
        "/promotions",
        &json!({ "name": { "fr": "Semaine", "en": "Week" },
                 "discount": { "kind": "percent", "bp": 1000 }, "min_nights": 7 }),
    )
    .await;
    create(
        &app,
        &cookie,
        "/promotions",
        &json!({ "name": { "fr": "Quinzaine" },
                 "discount": { "kind": "percent", "bp": 1500 }, "min_nights": 14 }),
    )
    .await;

    // Seed: 100 € per night, no cleaning fee.
    let (status, body) = quote(&app, &stay(7, 2, &[])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["nights_total"], 70000);
    assert_eq!(
        body["promotion"],
        json!({ "name": "Semaine", "discount": 7000 })
    );
    assert_eq!(body["stay_total"], 63000);

    let (_, body) = quote(&app, &stay(14, 2, &[])).await;
    assert_eq!(
        body["promotion"],
        json!({ "name": "Quinzaine", "discount": 21000 })
    );

    let mut english = stay(7, 2, &[]);
    english["lang"] = json!("en");
    let (_, body) = quote(&app, &english).await;
    assert_eq!(body["promotion"]["name"], "Week");

    let (_, body) = quote(&app, &stay(6, 2, &[])).await;
    assert_eq!(body["promotion"], Value::Null);
}

#[tokio::test]
async fn f026_s04_option_offered_by_stay_length() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let linen = create(
        &app,
        &cookie,
        "/options",
        &json!({ "name": { "fr": "Linge de maison" }, "unit": "guest", "price": 1500, "min_nights": 3 }),
    )
    .await;
    let cot = create(
        &app,
        &cookie,
        "/options",
        &json!({ "name": { "fr": "Lit bébé" }, "unit": "stay", "price": 0, "position": 1 }),
    )
    .await;

    let (_, body) = quote(&app, &stay(2, 2, &[])).await;
    assert_eq!(
        body["options"],
        json!([{ "id": cot, "name": "Lit bébé", "description": "", "price": 0, "selected": false }])
    );
    let (status, body) = quote(&app, &stay(2, 2, &[linen])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body["error"]["details"],
        json!(["une option choisie n'est pas proposée pour ce séjour"])
    );

    let (status, body) = quote(&app, &stay(3, 2, &[linen])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["options"][0]["price"], 3000);
    assert_eq!(body["options"][0]["selected"], true);
    assert_eq!(body["options_total"], 3000);
    assert_eq!(body["stay_total"], 30000 + 3000);
    assert_eq!(body["deposit"], 16500);
}

#[tokio::test]
async fn f026_owners_manage_offers_with_history() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let id = create(
        &app,
        &cookie,
        "/promotions",
        &json!({ "name": { "fr": "Semaine" }, "discount": { "kind": "per_night", "amount": 500 },
                 "min_nights": 7, "valid_from": day(1), "valid_until": day(200) }),
    )
    .await;
    let listed = json_body(app.get(OFFERS, Some(&cookie)).await).await;
    assert_eq!(
        listed["promotions"][0],
        json!({
            "id": id, "name": { "fr": "Semaine", "en": "" },
            "discount": { "kind": "per_night", "amount": 500 },
            "min_nights": 7, "max_nights": null,
            "valid_from": day(1), "valid_until": day(200), "active": true
        })
    );

    let status = app
        .send_json(
            "PUT",
            &format!("{OFFERS}/promotions/{id}"),
            Some(&cookie),
            &json!({ "name": { "fr": "Semaine" }, "discount": { "kind": "percent", "bp": 1000 },
                     "min_nights": 7, "active": false }),
        )
        .await
        .status();
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body) = quote(&app, &stay(7, 2, &[])).await;
    assert_eq!(body["promotion"], Value::Null, "inactive");

    let status = app
        .send_json(
            "DELETE",
            &format!("{OFFERS}/promotions/{id}"),
            Some(&cookie),
            &json!({}),
        )
        .await
        .status();
    assert_eq!(status, StatusCode::NO_CONTENT);

    let history = json_body(app.get("/api/admin/prices/history", Some(&cookie)).await).await;
    assert_eq!(
        history[0]["description"],
        "Promotion supprimée : Semaine : −10 %, dès 7 nuits (inactive)"
    );
    assert_eq!(
        history[2]["description"],
        "Promotion ajoutée : Semaine : −5 € par nuit, dès 7 nuits"
    );
    assert_eq!(history[0]["author"], common::EMAIL);

    let response = app
        .send_json(
            "POST",
            &format!("{OFFERS}/options"),
            Some(&cookie),
            &json!({ "name": { "fr": "" }, "unit": "night", "price": 100, "min_nights": 5, "max_nights": 2 }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json_body(response).await["error"]["details"],
        json!([
            "le nom en français est obligatoire",
            "le minimum de nuits doit être d'au moins 1 et ne pas dépasser le maximum"
        ])
    );
    let missing = app
        .send_json(
            "PUT",
            &format!("{OFFERS}/options/999"),
            Some(&cookie),
            &json!({ "name": { "fr": "X" }, "unit": "stay", "price": 0 }),
        )
        .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn f026_offers_are_admin_only() {
    let app = TestApp::start().await;
    assert_eq!(
        app.get(OFFERS, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.send_json("POST", &format!("{OFFERS}/promotions"), None, &json!({}))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
