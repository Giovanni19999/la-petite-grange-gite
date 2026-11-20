//! F001 acceptance tests against a disposable `MariaDB` container (requires Docker).
#![allow(clippy::unwrap_used, clippy::missing_panics_doc)] // test helpers: a failure is a test failure

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use gite_api::application::auth::LoginThrottle;
use gite_api::domain::auth::{Password, SessionPolicy};
use gite_api::http::router;
use gite_api::infrastructure::mariadb::migrate;
use gite_api::wiring;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::MySqlPool;
use testcontainers_modules::mariadb::Mariadb;
use testcontainers_modules::testcontainers::ContainerAsync;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use time::Duration;
use tower::ServiceExt;

const EMAIL: &str = "proprio@example.test";
const PASSWORD: &str = "correct horse battery";

struct TestApp {
    router: axum::Router,
    _container: ContainerAsync<Mariadb>,
}

impl TestApp {
    /// Starts `MariaDB`, applies the migrations and creates the owner account.
    async fn start() -> Self {
        let container = Mariadb::default().start().await.unwrap();
        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(3306).await.unwrap();
        let pool = MySqlPool::connect(&format!("mysql://root@{host}:{port}/test"))
            .await
            .unwrap();
        migrate(&pool).await.unwrap();

        let policy = SessionPolicy {
            max_age: Duration::days(30),
            idle_timeout: Duration::days(7),
        };
        let throttle = LoginThrottle {
            max_failures: 5,
            window: Duration::minutes(15),
        };
        let state = wiring::app_state(&pool, policy, throttle).unwrap();
        state
            .auth
            .create_admin(EMAIL, &Password::new(PASSWORD))
            .await
            .unwrap();

        Self {
            router: router(state),
            _container: container,
        }
    }

    async fn send(&self, request: Request<Body>) -> Response {
        let mut request = request;
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([192, 0, 2, 1], 50000))));
        self.router.clone().oneshot(request).await.unwrap()
    }

    async fn login(&self, email: &str, password: &str) -> Response {
        self.send(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": password }).to_string(),
                ))
                .unwrap(),
        )
        .await
    }

    async fn me(&self, cookie: Option<&str>) -> Response {
        let mut request = Request::get("/api/auth/me");
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        self.send(request.body(Body::empty()).unwrap()).await
    }
}

fn set_cookie(response: &Response) -> Option<String> {
    response
        .headers()
        .get(header::SET_COOKIE)
        .map(|value| value.to_str().unwrap().to_owned())
}

/// `name=value` part of a `Set-Cookie` header, to send back as a `Cookie` header.
fn cookie_pair(set_cookie: &str) -> String {
    set_cookie.split(';').next().unwrap().to_owned()
}

async fn json_body(response: Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

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
