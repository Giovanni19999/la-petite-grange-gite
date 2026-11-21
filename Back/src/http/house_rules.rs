//! House rules routes (F004), mounted under `/api/admin/house-rules`; owners only.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use time::format_description::well_known::Rfc3339;

use crate::app_state::AppState;
use crate::application::house_rules::{HouseRulesError, RuleChange};
use crate::domain::house_rules::HouseRules;
use crate::http::auth::AdminUser;
use crate::http::error::ApiError;

/// Most history lines returned at once.
const HISTORY_LIMIT: u32 = 100;

/// Admin routes; every handler requires a session.
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(rules).put(update))
        .route("/history", get(history))
}

impl From<HouseRulesError> for ApiError {
    fn from(error: HouseRulesError) -> Self {
        match error {
            HouseRulesError::Invalid(errors) => {
                Self::Invalid(errors.iter().map(ToString::to_string).collect())
            }
            HouseRulesError::Internal(error) => Self::Internal(error),
        }
    }
}

async fn rules(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<HouseRules>, ApiError> {
    Ok(Json(state.house_rules.rules().await?))
}

async fn update(
    user: AdminUser,
    State(state): State<AppState>,
    body: Result<Json<HouseRules>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(rules) = body.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    state.house_rules.update(rules, user.0.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct ChangeResponse {
    description: String,
    author: Option<String>,
    at: String,
}

impl From<RuleChange> for ChangeResponse {
    fn from(change: RuleChange) -> Self {
        Self {
            description: change.description,
            author: change.author_email,
            at: change.at.format(&Rfc3339).unwrap_or_default(),
        }
    }
}

async fn history(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<ChangeResponse>>, ApiError> {
    Ok(Json(
        state
            .house_rules
            .history(HISTORY_LIMIT)
            .await?
            .into_iter()
            .map(ChangeResponse::from)
            .collect(),
    ))
}
