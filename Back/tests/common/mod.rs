//! Shared helpers of the acceptance tests: a disposable `MariaDB` container (requires
//! Docker), the full router and HTTP helpers.
#![allow(clippy::unwrap_used, clippy::missing_panics_doc, dead_code)] // each test file uses part of it

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, header};
use axum::response::Response;
use gite_api::application::auth::LoginThrottle;
use gite_api::application::media::MediaSettings;
use gite_api::domain::auth::{Password, SessionPolicy};
use gite_api::domain::media::{ImageSettings, UploadLimits};
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

pub const EMAIL: &str = "proprio@example.test";
pub const PASSWORD: &str = "correct horse battery";

pub struct TestApp {
    router: axum::Router,
    _container: ContainerAsync<Mariadb>,
    /// Media files of this test (deleted with the app).
    pub media_dir: tempfile::TempDir,
}

impl TestApp {
    /// Starts `MariaDB`, applies the migrations (with the seed pages) and creates the owner.
    pub async fn start() -> Self {
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
        wiring::admin_accounts(&pool)
            .unwrap()
            .create(EMAIL, &Password::new(PASSWORD))
            .await
            .unwrap();
        let media_dir = tempfile::tempdir().unwrap();
        let media = MediaSettings {
            limits: UploadLimits {
                max_file_bytes: 2 * 1024 * 1024,
                max_files: 5,
                max_pixels: 4_000_000,
            },
            images: ImageSettings {
                widths: vec![32, 64, 1920],
                quality: 80.0,
            },
            public_url: "/uploads".to_owned(),
        };
        let mut state =
            wiring::app_state(&pool, policy, throttle, media_dir.path(), media).unwrap();
        state.serve_media_dir = Some(media_dir.path().join("public"));

        Self {
            router: router(state),
            _container: container,
            media_dir,
        }
    }

    pub async fn send(&self, request: Request<Body>) -> Response {
        let mut request = request;
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([192, 0, 2, 1], 50000))));
        self.router.clone().oneshot(request).await.unwrap()
    }

    pub async fn login(&self, email: &str, password: &str) -> Response {
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

    /// Logs the owner in and returns the `Cookie` header value.
    pub async fn owner_cookie(&self) -> String {
        let response = self.login(EMAIL, PASSWORD).await;
        cookie_pair(&set_cookie(&response).unwrap())
    }

    pub async fn me(&self, cookie: Option<&str>) -> Response {
        let mut request = Request::get("/api/auth/me");
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        self.send(request.body(Body::empty()).unwrap()).await
    }

    pub async fn get(&self, uri: &str, cookie: Option<&str>) -> Response {
        let mut request = Request::get(uri);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        self.send(request.body(Body::empty()).unwrap()).await
    }

    pub async fn send_json(
        &self,
        method: &str,
        uri: &str,
        cookie: Option<&str>,
        body: &Value,
    ) -> Response {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        self.send(request.body(Body::from(body.to_string())).unwrap())
            .await
    }
}

pub fn set_cookie(response: &Response) -> Option<String> {
    response
        .headers()
        .get(header::SET_COOKIE)
        .map(|value| value.to_str().unwrap().to_owned())
}

/// `name=value` part of a `Set-Cookie` header, to send back as a `Cookie` header.
pub fn cookie_pair(set_cookie: &str) -> String {
    set_cookie.split(';').next().unwrap().to_owned()
}

pub async fn json_body(response: Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}
