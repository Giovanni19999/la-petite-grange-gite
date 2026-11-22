//! Sending routes (F025), mounted under `/api/admin/messaging`; owners only.
//!
//! Responses never contain a secret nor the body of an email (F025-S04).

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use time::format_description::well_known::Rfc3339;

use crate::app_state::AppState;
use crate::application::messaging::{MessagingError, QueuedMessage, TestResult};
use crate::domain::messaging::{Channel, Content, MessageStatus, MessagingSettings};
use crate::http::auth::AdminUser;
use crate::http::error::ApiError;

/// Most messages listed at once.
const LIST_LIMIT: u32 = 100;
/// Characters of an SMS shown in the list.
const SMS_PREVIEW: usize = 40;

/// Admin routes; every handler requires a session.
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(settings).put(update))
        .route("/problems", get(problems))
        .route("/messages", get(messages))
        .route("/messages/{id}/resend", post(resend))
        .route("/test-email", post(test_email))
        .route("/test-sms", post(test_sms))
}

impl From<MessagingError> for ApiError {
    fn from(error: MessagingError) -> Self {
        match error {
            MessagingError::Invalid(errors) => {
                Self::Invalid(errors.iter().map(ToString::to_string).collect())
            }
            MessagingError::NotFound => Self::NotFound,
            error @ MessagingError::NoRecipient => Self::Conflict {
                code: "no_recipient",
                message: error.to_string(),
            },
            MessagingError::Internal(error) => Self::Internal(error),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    settings: MessagingSettings,
    /// The SMTP password is set in the environment (its value is never returned).
    smtp_password_set: bool,
    /// An SMS provider is configured (`SMS_PROVIDER`).
    sms_provider: bool,
}

async fn settings(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<SettingsResponse>, ApiError> {
    let senders = state.messaging.senders();
    Ok(Json(SettingsResponse {
        settings: state.messaging.settings().await?,
        smtp_password_set: senders.smtp_password_set,
        sms_provider: senders.sms_provider,
    }))
}

async fn update(
    _user: AdminUser,
    State(state): State<AppState>,
    body: Result<Json<MessagingSettings>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(settings) = body.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    state.messaging.update_settings(settings).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct ProblemsResponse {
    count: u64,
}

async fn problems(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<ProblemsResponse>, ApiError> {
    Ok(Json(ProblemsResponse {
        count: state.messaging.problems().await?,
    }))
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    id: u64,
    channel: Channel,
    recipient: String,
    /// Subject of an email, or the start of an SMS.
    summary: String,
    status: MessageStatus,
    attempts: u8,
    /// Last error, in French.
    error: Option<String>,
    created_at: String,
    sent_at: Option<String>,
}

impl From<QueuedMessage> for MessageResponse {
    fn from(message: QueuedMessage) -> Self {
        let summary = match &message.content {
            Content::Email { subject, .. } => subject.clone(),
            Content::Sms { text } => {
                let mut start: String = text.chars().take(SMS_PREVIEW).collect();
                if text.chars().count() > SMS_PREVIEW {
                    start.push('…');
                }
                start
            }
        };
        Self {
            id: message.id,
            channel: message.content.channel(),
            recipient: message.recipient,
            summary,
            status: message.status,
            attempts: message.attempts,
            error: message.last_error.map(|error| error.to_string()),
            created_at: message.created_at.format(&Rfc3339).unwrap_or_default(),
            sent_at: message.sent_at.and_then(|at| at.format(&Rfc3339).ok()),
        }
    }
}

async fn messages(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<MessageResponse>>, ApiError> {
    Ok(Json(
        state
            .messaging
            .recent(LIST_LIMIT)
            .await?
            .into_iter()
            .map(MessageResponse::from)
            .collect(),
    ))
}

async fn resend(
    _user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<StatusCode, ApiError> {
    state.messaging.resend(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct TestResponse {
    recipient: String,
    sent: bool,
    /// Why it failed, in French.
    error: Option<String>,
}

impl From<TestResult> for TestResponse {
    fn from(result: TestResult) -> Self {
        Self {
            recipient: result.recipient,
            sent: result.error.is_none(),
            error: result.error.map(|error| error.to_string()),
        }
    }
}

async fn test_email(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<TestResponse>>, ApiError> {
    Ok(Json(
        state
            .messaging
            .test_email()
            .await?
            .into_iter()
            .map(TestResponse::from)
            .collect(),
    ))
}

async fn test_sms(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<TestResponse>>, ApiError> {
    Ok(Json(
        state
            .messaging
            .test_sms()
            .await?
            .into_iter()
            .map(TestResponse::from)
            .collect(),
    ))
}
