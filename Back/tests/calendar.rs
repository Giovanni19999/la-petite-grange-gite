//! F005 acceptance tests against a disposable `MariaDB` container (requires Docker).
#![allow(clippy::unwrap_used, clippy::missing_panics_doc)] // test helpers: a failure is a test failure

mod common;

use axum::http::StatusCode;
use common::{TestApp, json_body};
use serde_json::{Value, json};

async fn unavailable(app: &TestApp, from: &str, to: &str) -> Vec<String> {
    let response = app
        .get(&format!("/api/availability?from={from}&to={to}"), None)
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await["unavailable"]
        .as_array()
        .unwrap()
        .iter()
        .map(|night| night.as_str().unwrap().to_owned())
        .collect()
}

async fn create(app: &TestApp, cookie: &str, period: &Value) -> axum::response::Response {
    app.send_json("POST", "/api/admin/blocked-periods", Some(cookie), period)
        .await
}

#[tokio::test]
async fn f005_s01_close_dates() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let response = create(
        &app,
        &cookie,
        &json!({ "start": "2027-03-10", "end": "2027-03-15", "kind": "closure", "reason": "Travaux" }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    assert_eq!(
        unavailable(&app, "2027-03-08", "2027-03-17").await,
        vec![
            "2027-03-10",
            "2027-03-11",
            "2027-03-12",
            "2027-03-13",
            "2027-03-14"
        ]
    );

    let list = json_body(app.get("/api/admin/blocked-periods", Some(&cookie)).await).await;
    assert_eq!(list[0]["nights"], 5);
    assert_eq!(list[0]["reason"], "Travaux");
}

#[tokio::test]
async fn f005_s02_family_details_never_public() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    create(
        &app,
        &cookie,
        &json!({ "start": "2027-08-01", "end": "2027-08-08", "kind": "family", "label": "Mamie et Papi", "people": 4 }),
    )
    .await;

    let response = app
        .get("/api/availability?from=2027-07-30&to=2027-08-10", None)
        .await;
    let body = json_body(response).await;
    assert_eq!(body["unavailable"].as_array().unwrap().len(), 7);
    let text = body.to_string();
    assert!(!text.contains("Mamie") && !text.contains("family") && !text.contains("people"));
    assert_eq!(body.as_object().unwrap().len(), 1, "only the nights");

    let list = json_body(app.get("/api/admin/blocked-periods", Some(&cookie)).await).await;
    assert_eq!(list[0]["label"], "Mamie et Papi");
    assert_eq!(list[0]["people"], 4);
}

#[tokio::test]
async fn f005_s03_label_required() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let response = create(
        &app,
        &cookie,
        &json!({ "start": "2027-08-01", "end": "2027-08-08", "kind": "private_event" }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = json_body(response).await;
    assert!(
        body["error"]["details"][0]
            .as_str()
            .unwrap()
            .contains("libellé est obligatoire")
    );
}

#[tokio::test]
async fn f005_s05_s06_open_ended_and_overlap() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    create(
        &app,
        &cookie,
        &json!({ "start": "2027-11-01", "end": null, "kind": "closure" }),
    )
    .await;
    assert_eq!(
        unavailable(&app, "2027-10-30", "2027-11-03").await,
        vec!["2027-11-01", "2027-11-02"]
    );

    let overlapping = create(
        &app,
        &cookie,
        &json!({ "start": "2028-01-10", "end": "2028-01-12", "kind": "family", "label": "Cousins" }),
    )
    .await;
    assert_eq!(overlapping.status(), StatusCode::CONFLICT);
    let body = json_body(overlapping).await;
    assert_eq!(body["error"]["code"], "period_overlap");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("à partir du 01/11/2027")
    );
}

#[tokio::test]
async fn f005_s07_change_reopen_and_history() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let id = json_body(
        create(
            &app,
            &cookie,
            &json!({ "start": "2027-03-10", "end": "2027-03-15", "kind": "closure" }),
        )
        .await,
    )
    .await["id"]
        .clone();

    let changed = app
        .send_json(
            "PUT",
            &format!("/api/admin/blocked-periods/{id}"),
            Some(&cookie),
            &json!({ "start": "2027-03-10", "end": "2027-03-12", "kind": "closure" }),
        )
        .await;
    assert_eq!(changed.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        unavailable(&app, "2027-03-01", "2027-04-01").await,
        vec!["2027-03-10", "2027-03-11"]
    );

    let deleted = app
        .send_json(
            "DELETE",
            &format!("/api/admin/blocked-periods/{id}"),
            Some(&cookie),
            &json!({}),
        )
        .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        unavailable(&app, "2027-03-01", "2027-04-01").await,
        Vec::<String>::new()
    );

    let history = json_body(
        app.get("/api/admin/blocked-periods/history", Some(&cookie))
            .await,
    )
    .await;
    let actions: Vec<&str> = history
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["action"].as_str().unwrap())
        .collect();
    assert_eq!(actions, vec!["deleted", "updated", "created"]);
    assert_eq!(history[0]["author"], common::EMAIL);
    assert_eq!(history[1]["before"]["end"], "2027-03-15");
    assert_eq!(history[1]["after"]["end"], "2027-03-12");
}

#[tokio::test]
async fn f005_s08_booking_horizon() {
    let app = TestApp::start().await;
    // Nights more than 24 months ahead are unavailable (seeded horizon).
    let today = time::OffsetDateTime::now_utc().date();
    let far = gite_api::domain::calendar::add_months(today, 25);
    let nights = unavailable(&app, &far.to_string(), &far.next_day().unwrap().to_string()).await;
    assert_eq!(nights, vec![far.to_string()]);
}

#[tokio::test]
async fn availability_range_is_checked() {
    let app = TestApp::start().await;
    let reversed = app
        .get("/api/availability?from=2027-03-10&to=2027-03-01", None)
        .await;
    assert_eq!(reversed.status(), StatusCode::BAD_REQUEST);
    let too_long = app
        .get("/api/availability?from=2027-01-01&to=2028-06-01", None)
        .await;
    assert_eq!(too_long.status(), StatusCode::BAD_REQUEST);
    let not_a_date = app
        .get("/api/availability?from=demain&to=2027-01-01", None)
        .await;
    assert_eq!(not_a_date.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn f005_s10_admin_only() {
    let app = TestApp::start().await;
    assert_eq!(
        app.get("/api/admin/blocked-periods", None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        create(&app, "lpg_session=wrong", &json!({})).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.send_json("DELETE", "/api/admin/blocked-periods/1", None, &json!({}))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.get("/api/admin/blocked-periods/history", None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
