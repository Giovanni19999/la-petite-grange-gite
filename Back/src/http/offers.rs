//! Promotions and options routes (F026), mounted under `/api/admin/offers`; owners only.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Serialize;
use serde_json::{Value, json};

use crate::app_state::AppState;
use crate::application::offers::{OffersError, StoredOption, StoredPromotion};
use crate::domain::offers::{Promotion, StayOption};
use crate::http::auth::AdminUser;
use crate::http::error::ApiError;

/// Admin routes; every handler requires a session.
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/promotions", post(create_promotion))
        .route(
            "/promotions/{id}",
            put(update_promotion).delete(delete_promotion),
        )
        .route("/options", post(create_option))
        .route("/options/{id}", put(update_option).delete(delete_option))
}

impl From<OffersError> for ApiError {
    fn from(error: OffersError) -> Self {
        match error {
            OffersError::NotFound => Self::NotFound,
            OffersError::Invalid(errors) => {
                Self::Invalid(errors.iter().map(ToString::to_string).collect())
            }
            OffersError::Internal(error) => Self::Internal(error),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PromotionResponse {
    id: u64,
    #[serde(flatten)]
    promotion: Promotion,
}

#[derive(Debug, Serialize)]
pub struct OptionResponse {
    id: u64,
    #[serde(flatten)]
    option: StayOption,
}

#[derive(Debug, Serialize)]
pub struct OffersResponse {
    promotions: Vec<PromotionResponse>,
    options: Vec<OptionResponse>,
}

async fn list(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<OffersResponse>, ApiError> {
    Ok(Json(OffersResponse {
        promotions: state
            .offers
            .promotions()
            .await?
            .into_iter()
            .map(|StoredPromotion { id, promotion }| PromotionResponse { id, promotion })
            .collect(),
        options: state
            .offers
            .options()
            .await?
            .into_iter()
            .map(|StoredOption { id, option }| OptionResponse { id, option })
            .collect(),
    }))
}

fn body<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    body.map(|Json(value)| value)
        .map_err(|rejection| ApiError::BadRequest(rejection.body_text()))
}

async fn create_promotion(
    user: AdminUser,
    State(state): State<AppState>,
    promotion: Result<Json<Promotion>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let id = state
        .offers
        .write_promotion(None, Some(body(promotion)?), user.0.id)
        .await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

async fn update_promotion(
    user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    promotion: Result<Json<Promotion>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    state
        .offers
        .write_promotion(Some(id), Some(body(promotion)?), user.0.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_promotion(
    user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<StatusCode, ApiError> {
    state
        .offers
        .write_promotion(Some(id), None, user.0.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_option(
    user: AdminUser,
    State(state): State<AppState>,
    option: Result<Json<StayOption>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let id = state
        .offers
        .write_option(None, Some(body(option)?), user.0.id)
        .await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

async fn update_option(
    user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    option: Result<Json<StayOption>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    state
        .offers
        .write_option(Some(id), Some(body(option)?), user.0.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_option(
    user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<StatusCode, ApiError> {
    state.offers.write_option(Some(id), None, user.0.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
