//! HTTP layer: axum routes, request/response DTOs and error mapping.
//!
//! Handlers stay thin: extract the request, call a use case, map to a DTO.

pub mod auth;
pub mod calendar;
pub mod error;
pub mod health;
pub mod house_rules;
pub mod media;
pub mod messaging;
pub mod offers;
pub mod pages;
pub mod pricing;
pub mod quote;
pub mod server;

use axum::Router;
use axum::routing::get;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::app_state::AppState;

/// Builds the API router with all routes and middlewares.
pub fn router(state: AppState) -> Router {
    let mut router = Router::new()
        .route("/api/health", get(health::health))
        .nest("/api/auth", auth::routes())
        .nest("/api/pages", pages::public_routes())
        .nest("/api/admin/pages", pages::admin_routes())
        .nest("/api/admin/media", media::admin_routes(&state))
        .nest("/api/availability", calendar::public_routes())
        .nest("/api/admin/blocked-periods", calendar::admin_routes())
        .nest("/api/admin/prices", pricing::admin_routes())
        .nest("/api/admin/house-rules", house_rules::admin_routes())
        .nest("/api/admin/offers", offers::admin_routes())
        .nest("/api/admin/messaging", messaging::admin_routes())
        .merge(quote::public_routes());
    // Development only: in production Caddy serves the media files (ADR-0017).
    if let Some(dir) = &state.serve_media_dir {
        let prefix = state
            .media
            .settings()
            .public_url
            .trim_end_matches('/')
            .to_owned();
        router = router.nest_service(&prefix, ServeDir::new(dir));
    }
    router
        .fallback(error::not_found)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
