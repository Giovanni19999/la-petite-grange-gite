//! Price routes (F006), admin only, mounted under `/api/admin/prices`.

use std::collections::BTreeMap;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::Date;
use time::format_description::well_known::Rfc3339;

use crate::app_state::AppState;
use crate::application::pricing::{
    ChannelCode, NightPrices, PriceBook, PriceBookError, PriceChange, PricedRange, PricingService,
    PricingSettings, TypicalStays,
};
use crate::domain::pricing::{ChannelPrice, ChannelRates, RangeInput};
use crate::http::auth::AdminUser;
use crate::http::error::ApiError;

/// Nights of the typical stays shown next to each price.
const TYPICAL_STAYS: [u64; 2] = [2, 7];
/// Most history lines returned at once.
const HISTORY_LIMIT: u32 = 100;

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(book))
        .route("/settings", put(settings))
        .route("/channels/{code}", put(channel_rates))
        .route("/defaults", put(default_prices))
        .route("/ranges", post(create_range))
        .route("/ranges/{id}", put(update_range).delete(delete_range))
        .route("/copy-year", post(copy_year))
        .route("/copy-channel", post(copy_channel))
        .route("/nights", get(nights))
        .route("/history", get(history))
}

impl From<PriceBookError> for ApiError {
    fn from(error: PriceBookError) -> Self {
        match error {
            PriceBookError::NotFound => Self::NotFound,
            PriceBookError::Invalid(errors) => {
                Self::Invalid(errors.iter().map(ToString::to_string).collect())
            }
            PriceBookError::BadRange(message) => Self::BadRequest(message),
            PriceBookError::Internal(error) => Self::Internal(error),
        }
    }
}

fn json_body<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    body.map(|Json(value)| value)
        .map_err(|rejection| ApiError::BadRequest(rejection.body_text()))
}

fn stays(book: &PriceBook, range: Option<&BTreeMap<ChannelCode, ChannelPrice>>) -> Value {
    let by_nights: BTreeMap<String, TypicalStays> = TYPICAL_STAYS
        .iter()
        .map(|nights| {
            (
                nights.to_string(),
                PricingService::typical_stays(book, *nights, range),
            )
        })
        .collect();
    json!(by_nights)
}

#[derive(Debug, Serialize)]
struct RangeResponse<'a> {
    #[serde(flatten)]
    range: &'a PricedRange,
    typical_stays: Value,
}

async fn book(_user: AdminUser, State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let book = state.pricing.book().await?;
    let channels: Vec<Value> = book
        .channels
        .iter()
        .map(|channel| {
            json!({
                "code": channel.code,
                "name": channel.name,
                "commission_bp": channel.rates.commission_bp,
                "payment_fees_bp": channel.rates.payment_fees_bp,
                "vat_on_commission": channel.rates.vat_on_commission,
                "effective_bp": channel.rates.effective_bp(),
            })
        })
        .collect();
    let ranges: Vec<RangeResponse> = book
        .ranges
        .iter()
        .map(|range| RangeResponse {
            range,
            typical_stays: stays(&book, Some(&range.prices)),
        })
        .collect();
    Ok(Json(json!({
        "channels": channels,
        "settings": book.settings,
        "defaults": book.defaults,
        "default_typical_stays": stays(&book, None),
        "ranges": ranges,
        "incomplete": PricingService::incomplete_channels(&book),
    })))
}

async fn settings(
    user: AdminUser,
    State(state): State<AppState>,
    body: Result<Json<PricingSettings>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    state
        .pricing
        .set_settings(json_body(body)?, user.0.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn channel_rates(
    user: AdminUser,
    State(state): State<AppState>,
    Path(code): Path<String>,
    body: Result<Json<ChannelRates>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    state
        .pricing
        .set_channel_rates(&code, json_body(body)?, user.0.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Body `{"direct": {"listed": …, "net": …}, "airbnb": null, …}`: every channel of the table.
async fn default_prices(
    user: AdminUser,
    State(state): State<AppState>,
    body: Result<Json<BTreeMap<ChannelCode, Option<ChannelPrice>>>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    state
        .pricing
        .set_default_prices(json_body(body)?, user.0.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct RangeRequest {
    #[serde(flatten)]
    range: RangeInput,
    #[serde(default)]
    prices: BTreeMap<ChannelCode, ChannelPrice>,
}

async fn create_range(
    user: AdminUser,
    State(state): State<AppState>,
    body: Result<Json<RangeRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let request = json_body(body)?;
    state
        .pricing
        .create_range(request.range, request.prices, user.0.id)
        .await?;
    Ok(StatusCode::CREATED)
}

async fn update_range(
    user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    body: Result<Json<RangeRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let request = json_body(body)?;
    state
        .pricing
        .update_range(id, request.range, request.prices, user.0.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_range(
    user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<StatusCode, ApiError> {
    state.pricing.delete_range(id, user.0.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct CopyYearRequest {
    year: i32,
}

async fn copy_year(
    user: AdminUser,
    State(state): State<AppState>,
    body: Result<Json<CopyYearRequest>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let request = json_body(body)?;
    let copied = state.pricing.copy_year(request.year, user.0.id).await?;
    Ok(Json(json!({ "copied": copied })))
}

#[derive(Debug, Deserialize)]
pub struct CopyChannelRequest {
    from: String,
    to: String,
}

async fn copy_channel(
    user: AdminUser,
    State(state): State<AppState>,
    body: Result<Json<CopyChannelRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let request = json_body(body)?;
    state
        .pricing
        .copy_channel(&request.from, &request.to, user.0.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct RangeQuery {
    from: Date,
    to: Date,
}

async fn nights(
    _user: AdminUser,
    State(state): State<AppState>,
    query: Result<Query<RangeQuery>, QueryRejection>,
) -> Result<Json<Vec<NightPrices>>, ApiError> {
    let Query(range) = query.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    Ok(Json(state.pricing.nights(range.from, range.to).await?))
}

fn change_json(change: &PriceChange) -> Value {
    json!({
        "description": change.description,
        "before": change.before,
        "after": change.after,
        "author": change.author_email,
        "at": change.at.format(&Rfc3339).unwrap_or_default(),
    })
}

async fn history(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<Value>>, ApiError> {
    Ok(Json(
        state
            .pricing
            .history(HISTORY_LIMIT)
            .await?
            .into_iter()
            .map(|change| change_json(&change))
            .collect(),
    ))
}
