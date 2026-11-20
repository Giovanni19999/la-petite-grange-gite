//! F003 acceptance tests against a disposable `MariaDB` container (requires Docker).
#![allow(clippy::unwrap_used, clippy::missing_panics_doc)] // test helpers: a failure is a test failure

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{TestApp, json_body};
use http_body_util::BodyExt;
use image::codecs::jpeg::JpegEncoder;
use image::{ImageEncoder, Rgb, RgbImage};
use serde_json::{Value, json};

const BOUNDARY: &str = "gite-test-boundary";

/// A real JPEG photo `width` x `height`.
fn jpeg(width: u32, height: u32, shade: u8) -> Vec<u8> {
    let image = RgbImage::from_pixel(width, height, Rgb([shade, 120, 200]));
    let mut bytes = Vec::new();
    JpegEncoder::new(&mut bytes)
        .write_image(
            image.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    bytes
}

async fn upload(
    app: &TestApp,
    cookie: Option<&str>,
    files: &[(&str, Vec<u8>)],
) -> axum::response::Response {
    let mut body = Vec::new();
    for (name, bytes) in files {
        body.extend(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"files\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend(bytes);
        body.extend(b"\r\n");
    }
    body.extend(format!("--{BOUNDARY}--\r\n").as_bytes());
    let mut request = Request::post("/api/admin/media").header(
        header::CONTENT_TYPE,
        format!("multipart/form-data; boundary={BOUNDARY}"),
    );
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    app.send(request.body(Body::from(body)).unwrap()).await
}

async fn upload_ok(app: &TestApp, cookie: &str, files: &[(&str, Vec<u8>)]) -> Value {
    let response = upload(app, Some(cookie), files).await;
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

/// Puts `image` in the hero of the home page draft and publishes it.
async fn publish_home_with_hero_image(app: &TestApp, cookie: &str, image: &Value) {
    let page = json_body(app.get("/api/admin/pages/1", Some(cookie)).await).await;
    let mut draft = page["draft"].clone();
    draft["blocks"][0]["image"] = image.clone();
    let saved = app
        .send_json(
            "PUT",
            "/api/admin/pages/1/draft",
            Some(cookie),
            &json!({ "draft_version": page["draft_version"], "content": draft }),
        )
        .await;
    assert_eq!(saved.status(), StatusCode::OK);
    let version = json_body(saved).await["draft_version"].clone();
    let published = app
        .send_json(
            "POST",
            "/api/admin/pages/1/publish",
            Some(cookie),
            &json!({ "draft_version": version }),
        )
        .await;
    assert_eq!(published.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn f003_s01_s03_upload_photos_some_refused() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;

    let result = upload_ok(
        &app,
        &cookie,
        &[
            ("grange.jpg", jpeg(100, 50, 10)),
            ("jardin.jpg", jpeg(40, 40, 90)),
            ("anim.gif", b"GIF89a....".to_vec()),
        ],
    )
    .await;

    let uploaded = result["uploaded"].as_array().unwrap();
    assert_eq!(uploaded.len(), 2);
    let widths: Vec<u64> = uploaded[0]["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|source| source["width"].as_u64().unwrap())
        .collect();
    assert_eq!(widths, vec![32, 64], "never wider than the 100 px original");
    assert_eq!(uploaded[1]["sources"][0]["width"], 32);
    assert_eq!(uploaded[0]["missing_alt"], true);
    assert_eq!(result["refused"][0]["file_name"], "anim.gif");
    assert!(
        result["refused"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("JPEG, PNG ou WebP")
    );

    // Files are on disk: public WebP versions only, the original kept privately.
    let url = uploaded[0]["sources"][0]["url"].as_str().unwrap();
    let file = app
        .media_dir
        .path()
        .join("public")
        .join(url.trim_start_matches("/uploads/"));
    assert!(std::fs::read(file).unwrap().starts_with(b"RIFF"));
    assert_eq!(
        std::fs::read_dir(app.media_dir.path().join("originals"))
            .unwrap()
            .count(),
        2
    );

    let list = json_body(app.get("/api/admin/media", Some(&cookie)).await).await;
    assert_eq!(list.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn f003_s04_too_many_pixels() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let result = upload_ok(&app, &cookie, &[("immense.jpg", jpeg(2100, 2000, 0))]).await;
    assert_eq!(result["uploaded"], json!([]));
    assert!(
        result["refused"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("mégapixels")
    );
}

#[tokio::test]
async fn f003_s05_describe_a_photo() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let id =
        upload_ok(&app, &cookie, &[("grange.jpg", jpeg(64, 64, 10))]).await["uploaded"][0]["id"]
            .clone();

    let response = app
        .send_json(
            "PUT",
            &format!("/api/admin/media/{id}"),
            Some(&cookie),
            &json!({
                "alt": { "fr": "La grange en été", "en": "" },
                "caption": { "fr": "", "en": "The barn in summer" },
                "focal": { "x": 30, "y": 70 }
            }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let media = json_body(response).await;
    assert_eq!(media["missing_alt"], false);
    assert_eq!(media["focal"], json!({ "x": 30, "y": 70 }));

    let invalid = app
        .send_json(
            "PUT",
            &format!("/api/admin/media/{id}"),
            Some(&cookie),
            &json!({ "focal": { "x": 101, "y": 0 } }),
        )
        .await;
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // The public page uses the texts and the focal point (F003-S05, S09).
    publish_home_with_hero_image(&app, &cookie, &id).await;
    let page = json_body(app.get("/api/pages/home?lang=en", None).await).await;
    let image = &page["blocks"][0]["image"];
    assert_eq!(
        image["alt"], "La grange en été",
        "English falls back to French"
    );
    assert_eq!(image["caption"], "The barn in summer");
    assert_eq!(
        (image["focal_x"].as_u64(), image["focal_y"].as_u64()),
        (Some(30), Some(70))
    );
}

#[tokio::test]
async fn f003_s06_s07_delete_unused_refuse_used() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let uploaded = upload_ok(
        &app,
        &cookie,
        &[("a.jpg", jpeg(64, 64, 10)), ("b.jpg", jpeg(64, 64, 200))],
    )
    .await;
    let used = uploaded["uploaded"][0]["id"].clone();
    let unused = uploaded["uploaded"][1]["id"].clone();
    publish_home_with_hero_image(&app, &cookie, &used).await;

    let refused = app
        .send_json(
            "DELETE",
            &format!("/api/admin/media/{used}"),
            Some(&cookie),
            &json!({}),
        )
        .await;
    assert_eq!(refused.status(), StatusCode::CONFLICT);
    let body = json_body(refused).await;
    assert_eq!(body["error"]["code"], "media_in_use");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("La Petite Grange")
    );

    let list = json_body(app.get("/api/admin/media", Some(&cookie)).await).await;
    let used_entry = list
        .as_array()
        .unwrap()
        .iter()
        .find(|media| media["id"] == used)
        .unwrap();
    assert_eq!(used_entry["used_in"][0]["page_id"], 1);

    let deleted = app
        .send_json(
            "DELETE",
            &format!("/api/admin/media/{unused}"),
            Some(&cookie),
            &json!({}),
        )
        .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    let gone = app
        .get(&format!("/api/admin/media/{unused}"), Some(&cookie))
        .await;
    assert_eq!(gone.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        std::fs::read_dir(app.media_dir.path().join("originals"))
            .unwrap()
            .count(),
        1
    );
}

#[tokio::test]
async fn f003_s08_unknown_media_in_a_draft_is_refused() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let page = json_body(app.get("/api/admin/pages/1", Some(&cookie)).await).await;
    let mut draft = page["draft"].clone();
    draft["blocks"][0]["image"] = json!(9999);
    let response = app
        .send_json(
            "PUT",
            "/api/admin/pages/1/draft",
            Some(&cookie),
            &json!({ "draft_version": page["draft_version"], "content": draft }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = json_body(response).await;
    assert!(
        body["error"]["details"][0]
            .as_str()
            .unwrap()
            .contains("9999")
    );
}

#[tokio::test]
async fn f003_s09_public_page_image_and_file() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let id =
        upload_ok(&app, &cookie, &[("grange.jpg", jpeg(100, 50, 10))]).await["uploaded"][0]["id"]
            .clone();
    publish_home_with_hero_image(&app, &cookie, &id).await;

    let page = json_body(app.get("/api/pages/home?lang=fr", None).await).await;
    let image = &page["blocks"][0]["image"];
    assert_eq!(
        (image["width"].as_u64(), image["height"].as_u64()),
        (Some(100), Some(50))
    );
    assert_eq!(image["sources"].as_array().unwrap().len(), 2);

    let url = image["sources"][0]["url"].as_str().unwrap().to_owned();
    assert!(url.starts_with("/uploads/") && url.ends_with("-32.webp"));
    let file = app.get(&url, None).await;
    assert_eq!(file.status(), StatusCode::OK);
    let bytes = file.into_body().collect().await.unwrap().to_bytes();
    assert!(bytes.starts_with(b"RIFF"));
}

#[tokio::test]
async fn f003_s10_admin_only() {
    let app = TestApp::start().await;
    assert_eq!(
        app.get("/api/admin/media", None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        upload(&app, None, &[("a.jpg", jpeg(10, 10, 0))])
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.send_json("PUT", "/api/admin/media/1", None, &json!({}))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.send_json("DELETE", "/api/admin/media/1", None, &json!({}))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn f024_s01_s02_s04_s05_hero_carousel_image_and_carousel_blocks() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let uploaded = upload_ok(
        &app,
        &cookie,
        &[
            ("a.jpg", jpeg(64, 48, 10)),
            ("b.jpg", jpeg(64, 48, 80)),
            ("c.jpg", jpeg(64, 48, 160)),
            ("d.jpg", jpeg(64, 48, 240)),
        ],
    )
    .await;
    let ids: Vec<Value> = uploaded["uploaded"]
        .as_array()
        .unwrap()
        .iter()
        .map(|media| media["id"].clone())
        .collect();

    let page = json_body(app.get("/api/admin/pages/1", Some(&cookie)).await).await;
    let mut draft = page["draft"].clone();
    draft["blocks"][0]["mode"] = json!("carousel");
    draft["blocks"][0]["images"] = json!([ids[0], ids[1], ids[2]]);
    draft["blocks"][0]["image"] = ids[3].clone();
    let blocks = draft["blocks"].as_array_mut().unwrap();
    blocks.push(json!({ "id": "photo", "type": "image", "image": ids[3], "width": "full" }));
    blocks.push(json!({ "id": "defile", "type": "carousel", "images": ids, "height": "large" }));
    let saved = app
        .send_json(
            "PUT",
            "/api/admin/pages/1/draft",
            Some(&cookie),
            &json!({ "draft_version": page["draft_version"], "content": draft }),
        )
        .await;
    assert_eq!(saved.status(), StatusCode::OK);
    let version = json_body(saved).await["draft_version"].clone();
    let published = app
        .send_json(
            "POST",
            "/api/admin/pages/1/publish",
            Some(&cookie),
            &json!({ "draft_version": version }),
        )
        .await;
    assert_eq!(published.status(), StatusCode::NO_CONTENT);

    let public = json_body(app.get("/api/pages/home?lang=fr", None).await).await;
    let hero = &public["blocks"][0];
    assert_eq!(hero["mode"], "carousel");
    assert_eq!(
        hero["image"],
        Value::Null,
        "the fixed image is not sent in carousel mode"
    );
    assert_eq!(hero["images"].as_array().unwrap().len(), 3);
    assert_eq!(hero["autoplay"], true);
    assert_eq!(hero["interval"], 6);

    let blocks = public["blocks"].as_array().unwrap();
    let image = blocks
        .iter()
        .find(|block| block["type"] == "image")
        .unwrap();
    assert_eq!(image["width"], "full");
    assert_eq!(image["image"]["id"], ids[3]);
    let carousel = blocks
        .iter()
        .find(|block| block["type"] == "carousel")
        .unwrap();
    assert_eq!(carousel["images"].as_array().unwrap().len(), 4);
    assert_eq!(carousel["autoplay"], false);
    assert_eq!(carousel["height"], "large");

    // Every photo of a carousel counts as used (F003-S07).
    let refused = app
        .send_json(
            "DELETE",
            &format!("/api/admin/media/{}", ids[1]),
            Some(&cookie),
            &json!({}),
        )
        .await;
    assert_eq!(refused.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn f024_s06_invalid_carousel_is_refused_with_a_message() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let id =
        upload_ok(&app, &cookie, &[("a.jpg", jpeg(64, 48, 10))]).await["uploaded"][0]["id"].clone();
    let page = json_body(app.get("/api/admin/pages/1", Some(&cookie)).await).await;
    let mut draft = page["draft"].clone();
    draft["blocks"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "id": "c", "type": "carousel", "images": [id] }));
    let response = app
        .send_json(
            "PUT",
            "/api/admin/pages/1/draft",
            Some(&cookie),
            &json!({ "draft_version": page["draft_version"], "content": draft }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = json_body(response).await;
    assert_eq!(
        body["error"]["details"][0],
        "le carrousel « c » doit contenir de 2 à 20 photos"
    );
}
