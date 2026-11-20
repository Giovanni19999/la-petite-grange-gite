//! `GET /api/health`.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;

use crate::app_state::AppState;
use crate::application::health::{ComponentStatus, check_health};

/// Response body of the health check.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    status: &'static str,
    version: &'static str,
    database: &'static str,
}

/// Returns 200 when every component is up, 503 otherwise.
pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let report = check_health(state.database.as_ref()).await;

    let (status_code, status) = if report.is_healthy() {
        (StatusCode::OK, "ok")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "degraded")
    };
    let database = match report.database {
        ComponentStatus::Up => "ok",
        ComponentStatus::Down => "unavailable",
    };

    (
        status_code,
        Json(HealthResponse {
            status,
            version: env!("CARGO_PKG_VERSION"),
            database,
        }),
    )
}
