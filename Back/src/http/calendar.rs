//! Calendar routes (F005): public availability without any detail, admin blocked periods.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::Date;
use time::format_description::well_known::Rfc3339;

use crate::app_state::AppState;
use crate::application::calendar::{CalendarError, PeriodEvent, PeriodId, StoredPeriod};
use crate::domain::calendar::BlockedPeriodInput;
use crate::http::auth::AdminUser;
use crate::http::error::ApiError;

/// Most history lines returned at once.
const HISTORY_LIMIT: u32 = 100;

/// Public routes, mounted under `/api/availability`.
pub fn public_routes() -> Router<AppState> {
    Router::new().route("/", get(availability))
}

/// Admin routes, mounted under `/api/admin/blocked-periods`; every handler requires a session.
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/history", get(history))
        .route("/{id}", axum::routing::put(update).delete(remove))
}

impl From<CalendarError> for ApiError {
    fn from(error: CalendarError) -> Self {
        match error {
            CalendarError::NotFound => Self::NotFound,
            CalendarError::Invalid(errors) => {
                Self::Invalid(errors.iter().map(ToString::to_string).collect())
            }
            CalendarError::Overlap(_) => Self::Conflict {
                code: "period_overlap",
                message: error.to_string(),
            },
            CalendarError::StayConflict(_) => Self::Conflict {
                code: "stay_conflict",
                message: error.to_string(),
            },
            CalendarError::BadRange(message) => Self::BadRequest(message),
            CalendarError::Internal(error) => Self::Internal(error),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RangeQuery {
    from: Date,
    to: Date,
}

/// Nights visitors cannot book; never says why.
async fn availability(
    State(state): State<AppState>,
    query: Result<Query<RangeQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let Query(range) = query.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    let nights = state.calendar.availability(range.from, range.to).await?;
    Ok(Json(json!({ "unavailable": nights })))
}

#[derive(Debug, Serialize)]
pub struct PeriodResponse {
    id: u64,
    #[serde(flatten)]
    period: BlockedPeriodInput,
    /// `None` when closed until further notice.
    nights: Option<i64>,
    updated_at: String,
}

impl From<StoredPeriod> for PeriodResponse {
    fn from(stored: StoredPeriod) -> Self {
        Self {
            id: stored.id.0,
            nights: stored.period.range().nights(),
            period: stored.period,
            updated_at: stored.updated_at.format(&Rfc3339).unwrap_or_default(),
        }
    }
}

async fn list(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<PeriodResponse>>, ApiError> {
    Ok(Json(
        state
            .calendar
            .periods()
            .await?
            .into_iter()
            .map(PeriodResponse::from)
            .collect(),
    ))
}

fn json_body<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    body.map(|Json(value)| value)
        .map_err(|rejection| ApiError::BadRequest(rejection.body_text()))
}

async fn create(
    user: AdminUser,
    State(state): State<AppState>,
    body: Result<Json<BlockedPeriodInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let id = state.calendar.create(json_body(body)?, user.0.id).await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id.0 }))))
}

async fn update(
    user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    body: Result<Json<BlockedPeriodInput>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    state
        .calendar
        .update(PeriodId(id), json_body(body)?, user.0.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove(
    user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<StatusCode, ApiError> {
    state.calendar.delete(PeriodId(id), user.0.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct EventResponse {
    period_id: u64,
    action: &'static str,
    before: Option<BlockedPeriodInput>,
    after: Option<BlockedPeriodInput>,
    author: Option<String>,
    at: String,
}

impl From<PeriodEvent> for EventResponse {
    fn from(event: PeriodEvent) -> Self {
        Self {
            period_id: event.period.0,
            action: event.action.as_str(),
            before: event.before,
            after: event.after,
            author: event.author_email,
            at: event.at.format(&Rfc3339).unwrap_or_default(),
        }
    }
}

async fn history(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<EventResponse>>, ApiError> {
    Ok(Json(
        state
            .calendar
            .history(HISTORY_LIMIT)
            .await?
            .into_iter()
            .map(EventResponse::from)
            .collect(),
    ))
}
