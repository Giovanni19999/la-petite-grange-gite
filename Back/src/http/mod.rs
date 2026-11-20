//! HTTP layer: axum routes, request/response DTOs and error mapping.
//!
//! Handlers stay thin: extract the request, call a use case, map to a DTO.

pub mod auth;
pub mod error;
pub mod health;

use axum::Router;
use axum::routing::get;
use tower_http::trace::TraceLayer;

use crate::app_state::AppState;

/// Builds the API router with all routes and middlewares.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health::health))
        .nest("/api/auth", auth::routes())
        .fallback(error::not_found)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
