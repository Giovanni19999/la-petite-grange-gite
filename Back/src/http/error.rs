//! API error type and its JSON representation.
//!
//! Every error response has the shape `{"error": {"code": "...", "message": "..."}}`.
//! Messages shown to users are in French; internal details are logged, never sent.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Errors returned by the HTTP layer.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Ressource introuvable")]
    NotFound,
    #[error("Requête invalide : {0}")]
    BadRequest(String),
    #[error("Connexion requise")]
    Unauthorized,
    #[error("Email ou mot de passe incorrect")]
    InvalidCredentials,
    #[error("Trop de tentatives de connexion, réessayez dans quelques minutes")]
    TooManyAttempts,
    #[error("Erreur interne")]
    Internal(#[source] anyhow::Error),
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized | Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::TooManyAttempts => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::NotFound => "not_found",
            Self::BadRequest(_) => "bad_request",
            Self::Unauthorized => "unauthorized",
            Self::InvalidCredentials => "invalid_credentials",
            Self::TooManyAttempts => "too_many_attempts",
            Self::Internal(_) => "internal_error",
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let Self::Internal(source) = &self {
            tracing::error!(error = ?source, "internal error");
        }
        let body = ErrorBody {
            error: ErrorDetail {
                code: self.code(),
                message: self.to_string(),
            },
        };
        (self.status(), Json(body)).into_response()
    }
}

/// Fallback handler for unknown routes.
pub async fn not_found() -> ApiError {
    ApiError::NotFound
}
