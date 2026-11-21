//! Public calendar and quote (F004): `/api/calendar` and `/api/quote`. Prices are computed
//! here only; unavailable nights never say why.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use time::Date;

use crate::app_state::AppState;
use crate::application::quote::{PublicCalendar, QuoteError};
use crate::domain::house_rules::Weekdays;
use crate::domain::quote::{Quote, StayRequest};
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
}

#[derive(Debug, Serialize)]
pub struct QuoteResponse {
    nights: u64,
    nights_total: u64,
    cleaning_fee: u64,
    stay_total: u64,
    deposit: u64,
    balance: u64,
    /// `null` when everything is due at booking.
    balance_due: Option<Date>,
    /// Paid on arrival, apart from the stay total.
    tourist_tax: u64,
}

impl From<Quote> for QuoteResponse {
    fn from(quote: Quote) -> Self {
        Self {
            nights: quote.nights,
            nights_total: quote.nights_total,
            cleaning_fee: quote.cleaning_fee,
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
    let quote = state
        .quotes
        .quote(StayRequest {
            arrival: request.arrival,
            departure: request.departure,
            adults: request.adults,
            children: request.children,
        })
        .await?;
    Ok(Json(quote.into()))
}
