//! F001 acceptance tests against a disposable `MariaDB` container (requires Docker).
#![allow(clippy::unwrap_used, clippy::missing_panics_doc)] // test helpers: a failure is a test failure

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{EMAIL, PASSWORD, TestApp, cookie_pair, json_body, set_cookie};

#[tokio::test]
async fn f001_s01_successful_login() {
    let app = TestApp::start().await;

    let response = app.login("PROPRIO@example.test ", PASSWORD).await;

    assert_eq!(response.status(), StatusCode::OK);
    let cookie = set_cookie(&response).unwrap();
    assert!(cookie.starts_with("lpg_session="));
    for attribute in ["HttpOnly", "Secure", "SameSite=Strict", "Path=/"] {
        assert!(
            cookie.contains(attribute),
            "missing {attribute} in {cookie}"
        );
    }
    let me = app.me(Some(&cookie_pair(&cookie))).await;
    assert_eq!(me.status(), StatusCode::OK);
    assert_eq!(json_body(me).await["email"], EMAIL);
}

#[tokio::test]
async fn f001_s02_wrong_password() {
    let app = TestApp::start().await;

    let response = app.login(EMAIL, "wrong password!").await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(set_cookie(&response).is_none());
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "invalid_credentials");
    assert_eq!(body["error"]["message"], "Email ou mot de passe incorrect");
}

#[tokio::test]
async fn f001_s03_unknown_email() {
    let app = TestApp::start().await;

    let response = app.login("nobody@example.test", "whatever password").await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = json_body(response).await;
    assert_eq!(body["error"]["message"], "Email ou mot de passe incorrect");
}

#[tokio::test]
async fn f001_s04_rate_limited() {
    let app = TestApp::start().await;
    for _ in 0..5 {
        let response = app.login(EMAIL, "wrong password!").await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let response = app.login(EMAIL, PASSWORD).await;

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        json_body(response).await["error"]["code"],
        "too_many_attempts"
    );
}

#[tokio::test]
async fn f001_s05_logout() {
    let app = TestApp::start().await;
    let login = app.login(EMAIL, PASSWORD).await;
    let cookie = cookie_pair(&set_cookie(&login).unwrap());

    let logout = app
        .send(
            Request::post("/api/auth/logout")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(logout.status(), StatusCode::NO_CONTENT);
    let cleared = set_cookie(&logout).unwrap();
    assert!(cleared.starts_with("lpg_session=;") || cleared.contains("Max-Age=0"));
    assert_eq!(
        app.me(Some(&cookie)).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn me_without_cookie_is_unauthorized() {
    let app = TestApp::start().await;

    let response = app.me(None).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(json_body(response).await["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn invalid_json_is_a_bad_request() {
    let app = TestApp::start().await;

    let response = app
        .send(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{\"email\":"))
                .unwrap(),
        )
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
