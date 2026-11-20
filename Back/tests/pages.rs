//! F002 acceptance tests against a disposable `MariaDB` container (requires Docker).
#![allow(clippy::unwrap_used, clippy::missing_panics_doc)] // test helpers: a failure is a test failure

mod common;

use axum::http::StatusCode;
use common::{TestApp, json_body};
use serde_json::{Value, json};

/// Home page (id 1 in the seed): current draft and its version.
async fn home_draft(app: &TestApp, cookie: &str) -> (Value, u64) {
    let page = json_body(app.get("/api/admin/pages/1", Some(cookie)).await).await;
    (
        page["draft"].clone(),
        page["draft_version"].as_u64().unwrap(),
    )
}

async fn save_draft(app: &TestApp, cookie: &str, version: u64, content: &Value) -> u64 {
    let response = app
        .send_json(
            "PUT",
            "/api/admin/pages/1/draft",
            Some(cookie),
            &json!({ "draft_version": version, "content": content }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await["draft_version"].as_u64().unwrap()
}

async fn public_home_title(app: &TestApp) -> String {
    let page = json_body(app.get("/api/pages/home?lang=fr", None).await).await;
    page["title"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn f002_s01_home_page_in_french_and_english() {
    let app = TestApp::start().await;

    let fr = json_body(app.get("/api/pages/home?lang=fr", None).await).await;
    let en = json_body(app.get("/api/pages/home?lang=en", None).await).await;

    assert_eq!(fr["lang"], "fr");
    assert_eq!(fr["blocks"][0]["type"], "hero");
    assert_eq!(fr["blocks"][0]["button_label"], "Réserver");
    assert_eq!(en["blocks"][0]["button_label"], "Book");
    assert_eq!(fr["alternates"], json!({ "fr": "/", "en": "/en" }));
}

#[tokio::test]
async fn f002_s01_english_falls_back_to_french() {
    let app = TestApp::start().await;

    let page = json_body(
        app.get("/api/pages/by-slug/legal-notice?lang=en", None)
            .await,
    )
    .await;

    assert_eq!(page["title"], "Legal notice");
    assert!(
        page["blocks"][0]["body"]
            .as_str()
            .unwrap()
            .contains("Éditeur")
    );
}

#[tokio::test]
async fn f002_s02_menu() {
    let app = TestApp::start().await;

    let fr = json_body(app.get("/api/pages/menu?lang=fr", None).await).await;
    let en = json_body(app.get("/api/pages/menu?lang=en", None).await).await;

    assert_eq!(
        fr,
        json!([
            { "title": "La Petite Grange", "path": "/" },
            { "title": "Le gîte", "path": "/le-gite" },
            { "title": "La région", "path": "/la-region" },
            { "title": "Accès", "path": "/acces" }
        ])
    );
    assert_eq!(
        en[1],
        json!({ "title": "The cottage", "path": "/en/the-cottage" })
    );
}

#[tokio::test]
async fn f002_s03_unknown_page() {
    let app = TestApp::start().await;

    let response = app
        .get("/api/pages/by-slug/page-inconnue?lang=fr", None)
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn f002_s04_draft_not_public() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let (mut draft, version) = home_draft(&app, &cookie).await;

    draft["settings"]["title"]["fr"] = json!("Nouveau titre");
    save_draft(&app, &cookie, version, &draft).await;

    assert_eq!(public_home_title(&app).await, "La Petite Grange");
}

#[tokio::test]
async fn f002_s05_publish() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let (mut draft, version) = home_draft(&app, &cookie).await;
    draft["settings"]["title"]["fr"] = json!("Nouveau titre");
    let version = save_draft(&app, &cookie, version, &draft).await;

    let response = app
        .send_json(
            "POST",
            "/api/admin/pages/1/publish",
            Some(&cookie),
            &json!({ "draft_version": version }),
        )
        .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(public_home_title(&app).await, "Nouveau titre");
    let versions = json_body(app.get("/api/admin/pages/1/versions", Some(&cookie)).await).await;
    assert_eq!(versions.as_array().unwrap().len(), 2);
    assert_eq!(versions[0]["number"], 2);
    assert_eq!(versions[0]["by_owner"], true);
}

#[tokio::test]
async fn f002_s06_stale_draft() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let (mut draft, opened_version) = home_draft(&app, &cookie).await;
    draft["settings"]["title"]["fr"] = json!("Modifié ailleurs");
    save_draft(&app, &cookie, opened_version, &draft).await;

    let response = app
        .send_json(
            "POST",
            "/api/admin/pages/1/publish",
            Some(&cookie),
            &json!({ "draft_version": opened_version }),
        )
        .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        json_body(response).await["error"]["code"],
        "version_conflict"
    );
    assert_eq!(public_home_title(&app).await, "La Petite Grange");
}

#[tokio::test]
async fn f002_s07_changes_endpoint() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let (mut draft, version) = home_draft(&app, &cookie).await;
    let blocks = draft["blocks"].as_array_mut().unwrap();
    blocks[1]["body"]["en"] = json!("<p>Changed</p>");
    blocks.remove(2);
    blocks.push(json!({ "id": "nouveau", "type": "text", "body": { "fr": "<p>Nouveau</p>" } }));
    save_draft(&app, &cookie, version, &draft).await;

    let changes = json_body(app.get("/api/admin/pages/1/changes", Some(&cookie)).await).await;

    assert_eq!(changes["changes"]["blocks_added"], json!(["nouveau"]));
    assert_eq!(changes["changes"]["blocks_removed"], json!(["atouts"]));
    assert_eq!(changes["changes"]["blocks_changed"], json!(["intro"]));
    assert_eq!(changes["changes"]["languages"], json!(["fr", "en"]));
    assert_eq!(changes["nothing_to_publish"], false);
}

#[tokio::test]
async fn f002_s08_restore() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let (mut draft, version) = home_draft(&app, &cookie).await;
    draft["settings"]["title"]["fr"] = json!("Version 2");
    let version = save_draft(&app, &cookie, version, &draft).await;
    app.send_json(
        "POST",
        "/api/admin/pages/1/publish",
        Some(&cookie),
        &json!({ "draft_version": version }),
    )
    .await;

    let response = app
        .send_json(
            "POST",
            "/api/admin/pages/1/versions/1/restore",
            Some(&cookie),
            &json!({}),
        )
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let (draft, _) = home_draft(&app, &cookie).await;
    assert_eq!(draft["settings"]["title"]["fr"], "La Petite Grange");
    assert_eq!(public_home_title(&app).await, "Version 2");
}

#[tokio::test]
async fn f002_s09_invalid_content_is_rejected_with_details() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let (mut draft, version) = home_draft(&app, &cookie).await;
    draft["settings"]["slug"]["fr"] = json!("Mauvaise Adresse");

    let response = app
        .send_json(
            "PUT",
            "/api/admin/pages/1/draft",
            Some(&cookie),
            &json!({ "draft_version": version, "content": draft }),
        )
        .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "invalid_content");
    assert!(
        body["error"]["details"][0]
            .as_str()
            .unwrap()
            .contains("Mauvaise Adresse")
    );
}

#[tokio::test]
async fn f002_s10_admin_requires_session() {
    let app = TestApp::start().await;

    for uri in [
        "/api/admin/pages",
        "/api/admin/pages/1",
        "/api/admin/pages/1/changes",
        "/api/admin/pages/1/versions",
    ] {
        assert_eq!(
            app.get(uri, None).await.status(),
            StatusCode::UNAUTHORIZED,
            "{uri}"
        );
    }
    let publish = app
        .send_json(
            "POST",
            "/api/admin/pages/1/publish",
            None,
            &json!({ "draft_version": 1 }),
        )
        .await;
    assert_eq!(publish.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn new_page_is_public_only_after_publication() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let content = json!({
        "settings": {
            "slug": { "fr": "tarifs", "en": "prices" },
            "title": { "fr": "Tarifs", "en": "Prices" },
            "in_menu": true, "menu_order": 4
        },
        "blocks": []
    });

    let created = app
        .send_json(
            "POST",
            "/api/admin/pages",
            Some(&cookie),
            &json!({ "content": content }),
        )
        .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let id = json_body(created).await["id"].as_u64().unwrap();
    assert_eq!(
        app.get("/api/pages/by-slug/tarifs?lang=fr", None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );

    app.send_json(
        "POST",
        &format!("/api/admin/pages/{id}/publish"),
        Some(&cookie),
        &json!({ "draft_version": 1 }),
    )
    .await;

    assert_eq!(
        app.get("/api/pages/by-slug/prices?lang=en", None)
            .await
            .status(),
        StatusCode::OK
    );
}
