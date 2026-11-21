//! Public calendar and quote (F004): `/api/calendar` and `/api/quote`. Prices are computed
//! here only; unavailable nights never say why.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use time::Date;

use crate::app_state::AppState;
use crate::application::quote::{PublicCalendar, QuoteError, QuoteResult};
use crate::domain::house_rules::Weekdays;
use crate::domain::page::Lang;
use crate::domain::quote::StayRequest;
use crate::http::error::ApiError;

/// Public routes: `/api/calendar` and `/api/quote`.
pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/api/calendar", get(calendar))
        .route("/api/quote", post(quote))
}

impl From<QuoteError> for ApiError {
    fn from(error: QuoteError) -> Self {
        match error {
            QuoteError::Refused(reasons) => {
                Self::Invalid(reasons.iter().map(ToString::to_string).collect())
            }
            QuoteError::BadRange(message) => Self::BadRequest(message),
            QuoteError::Internal(error) => Self::Internal(error),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RangeQuery {
    from: Date,
    to: Date,
}

#[derive(Debug, Serialize)]
pub struct NightResponse {
    date: Date,
    /// Site listed price in cents; `null` when the night cannot be booked.
    price: Option<u64>,
    /// Minimum stay when arriving this night (only for free nights).
    #[serde(skip_serializing_if = "Option::is_none")]
    min_nights: Option<u8>,
}

#[derive(Debug, Serialize)]
pub struct CalendarResponse {
    earliest_arrival: Date,
    last_night: Date,
    capacity: u8,
    /// Guests under this age are children (exempted from the tourist tax).
    children_under: u8,
    arrival_weekdays: Weekdays,
    departure_weekdays: Weekdays,
    nights: Vec<NightResponse>,
}

impl From<PublicCalendar> for CalendarResponse {
    fn from(calendar: PublicCalendar) -> Self {
        Self {
            earliest_arrival: calendar.earliest_arrival,
            last_night: calendar.last_night,
            capacity: calendar.rules.capacity,
            children_under: calendar.rules.tourist_tax.exempt_under,
            arrival_weekdays: calendar.rules.arrival_weekdays,
            departure_weekdays: calendar.rules.departure_weekdays,
            nights: calendar
                .nights
                .into_iter()
                .map(|night| NightResponse {
                    date: night.date,
                    price: night.price,
                    min_nights: night.price.map(|_| night.min_nights),
                })
                .collect(),
        }
    }
}

async fn calendar(
    State(state): State<AppState>,
    query: Result<Query<RangeQuery>, QueryRejection>,
) -> Result<Json<CalendarResponse>, ApiError> {
    let Query(range) = query.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    Ok(Json(
        state.quotes.calendar(range.from, range.to).await?.into(),
    ))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuoteRequest {
    arrival: Date,
    departure: Date,
    adults: u8,
    #[serde(default)]
    children: u8,
    /// Ids of the ticked options (F026).
    #[serde(default)]
    options: Vec<u64>,
    /// Language of the names of the promotion and options.
    #[serde(default = "french")]
    lang: Lang,
}

fn french() -> Lang {
    Lang::Fr
}

#[derive(Debug, Serialize)]
pub struct PromotionResponse {
    name: String,
    /// Discount on the nights, in cents.
    discount: u64,
}

#[derive(Debug, Serialize)]
pub struct QuotedOptionResponse {
    id: u64,
    name: String,
    description: String,
    /// Price for this stay in cents; 0 when offered.
    price: u64,
    selected: bool,
}

#[derive(Debug, Serialize)]
pub struct QuoteResponse {
    nights: u64,
    nights_total: u64,
    /// Best promotion that applies (F026); `null` when none.
    promotion: Option<PromotionResponse>,
    cleaning_fee: u64,
    /// Options offered for this stay (F026).
    options: Vec<QuotedOptionResponse>,
    options_total: u64,
    stay_total: u64,
    deposit: u64,
    balance: u64,
    /// `null` when everything is due at booking.
    balance_due: Option<Date>,
    /// Paid on arrival, apart from the stay total.
    tourist_tax: u64,
}

impl QuoteResponse {
    fn new(result: QuoteResult, lang: Lang) -> Self {
        let quote = result.quote;
        Self {
            nights: quote.nights,
            nights_total: quote.nights_total,
            promotion: result.promotion.map(|promotion| PromotionResponse {
                name: promotion.name.get(lang).to_owned(),
                discount: quote.discount,
            }),
            cleaning_fee: quote.cleaning_fee,
            options: result
                .options
                .into_iter()
                .map(|(quoted, option)| QuotedOptionResponse {
                    id: quoted.id,
                    name: option.name.get(lang).to_owned(),
                    description: option.description.get(lang).to_owned(),
                    price: quoted.price,
                    selected: quoted.selected,
                })
                .collect(),
            options_total: quote.options_total,
            stay_total: quote.stay_total,
            deposit: quote.deposit,
            balance: quote.balance,
            balance_due: quote.balance_due,
            tourist_tax: quote.tourist_tax,
        }
    }
}

async fn quote(
    State(state): State<AppState>,
    body: Result<Json<QuoteRequest>, JsonRejection>,
) -> Result<Json<QuoteResponse>, ApiError> {
    let Json(request) = body.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    let result = state
        .quotes
        .quote(StayRequest {
            arrival: request.arrival,
            departure: request.departure,
            adults: request.adults,
            children: request.children,
            options: request.options,
        })
        .await?;
    Ok(Json(QuoteResponse::new(result, request.lang)))
}
