//! Admin authentication routes (F001, ADR-0011) and the [`AdminUser`] extractor.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, FromRequestParts, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::application::auth::{AuthenticatedUser, LoginError};
use crate::domain::auth::{Password, SessionToken};
use crate::http::error::ApiError;

/// Name of the session cookie.
pub const SESSION_COOKIE: &str = "lpg_session";

/// Authentication routes, mounted under `/api/auth`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    email: String,
}

impl From<AuthenticatedUser> for UserResponse {
    fn from(user: AuthenticatedUser) -> Self {
        Self {
            email: user.email.to_string(),
        }
    }
}

fn session_cookie(value: String, max_age: time::Duration) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, value))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(max_age)
        .build()
}

/// Client IP from the TCP connection (forwarding by the reverse proxy is handled at deployment).
#[derive(Debug, Clone, Copy)]
pub struct ClientIp(pub IpAddr);

impl<S: Send + Sync> FromRequestParts<S> for ClientIp {
    type Rejection = std::convert::Infallible;

    #[allow(clippy::unused_async_trait_impl)] // the trait requires an async function
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let ip = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), |info| info.0.ip());
        Ok(Self(ip))
    }
}

async fn login(
    State(state): State<AppState>,
    ClientIp(client_ip): ClientIp,
    jar: CookieJar,
    body: Result<Json<LoginRequest>, JsonRejection>,
) -> Result<(CookieJar, Json<UserResponse>), ApiError> {
    let Json(request) = body.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    let password = Password::new(request.password);

    let (token, user) = state
        .auth
        .login(&request.email, &password, client_ip)
        .await
        .map_err(|error| match error {
            LoginError::InvalidCredentials => ApiError::InvalidCredentials,
            LoginError::TooManyAttempts => ApiError::TooManyAttempts,
            LoginError::Internal(error) => ApiError::Internal(error),
        })?;

    let cookie = session_cookie(token.expose().to_owned(), state.auth.policy().max_age);
    Ok((jar.add(cookie), Json(user.into())))
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, StatusCode), ApiError> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        state
            .auth
            .logout(&SessionToken::from_cookie(cookie.value()))
            .await?;
    }
    let expired = session_cookie(String::new(), time::Duration::ZERO);
    Ok((jar.add(expired), StatusCode::NO_CONTENT))
}

async fn me(user: AdminUser) -> Json<UserResponse> {
    Json(user.0.into())
}

/// Extractor: the logged-in owner. Rejects the request with 401 otherwise.
#[derive(Debug, Clone)]
pub struct AdminUser(pub AuthenticatedUser);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(SESSION_COOKIE)
            .map(|cookie| SessionToken::from_cookie(cookie.value()))
            .ok_or(ApiError::Unauthorized)?;
        state
            .auth
            .current_user(&token)
            .await?
            .map(AdminUser)
            .ok_or(ApiError::Unauthorized)
    }
}
