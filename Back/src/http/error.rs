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
    /// Another use case is rate limited; the message says which.
    #[error("{0}")]
    TooManyRequests(String),
    /// The submitted content breaks business rules; `details` lists every problem.
    #[error("Contenu invalide")]
    Invalid(Vec<String>),
    /// The request conflicts with the current state (stale version, duplicate…).
    #[error("{message}")]
    Conflict { code: &'static str, message: String },
    #[error("Erreur interne")]
    Internal(#[source] anyhow::Error),
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized | Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::TooManyAttempts | Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Invalid(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::NotFound => "not_found",
            Self::BadRequest(_) => "bad_request",
            Self::Unauthorized => "unauthorized",
            Self::InvalidCredentials => "invalid_credentials",
            Self::TooManyAttempts | Self::TooManyRequests(_) => "too_many_attempts",
            Self::Invalid(_) => "invalid_content",
            Self::Conflict { code, .. } => code,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    details: Vec<String>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let Self::Internal(source) = &self {
            tracing::error!(error = ?source, "internal error");
        }
        let details = match &self {
            Self::Invalid(details) => details.clone(),
            _ => Vec::new(),
        };
        let body = ErrorBody {
            error: ErrorDetail {
                code: self.code(),
                message: self.to_string(),
                details,
            },
        };
        (self.status(), Json(body)).into_response()
    }
}

/// Fallback handler for unknown routes.
pub async fn not_found() -> ApiError {
    ApiError::NotFound
}
