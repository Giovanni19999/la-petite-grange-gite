//! F000 acceptance tests for the health check and the error format.
#![allow(clippy::unwrap_used)] // test helpers: a failure here is a test failure

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use gite_api::app_state::AppState;
use gite_api::application::health::{BoxFuture, DatabaseProbe};
use gite_api::http::router;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

struct FakeDatabase {
    reachable: bool,
}

impl DatabaseProbe for FakeDatabase {
    fn ping(&self) -> BoxFuture<'_, bool> {
        let reachable = self.reachable;
        Box::pin(async move { reachable })
    }
}

fn app(database_reachable: bool) -> axum::Router {
    router(AppState {
        database: Arc::new(FakeDatabase {
            reachable: database_reachable,
        }),
    })
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn f000_s01_health_ok() {
    let (status, body) = get(app(true), "/api/health").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["database"], "ok");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn f000_s02_health_degraded() {
    let (status, body) = get(app(false), "/api/health").await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["status"], "degraded");
    assert_eq!(body["database"], "unavailable");
}

#[tokio::test]
async fn f000_s05_unknown_route_json_error() {
    let (status, body) = get(app(true), "/api/does-not-exist").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "not_found");
    assert!(body["error"]["message"].is_string());
}
