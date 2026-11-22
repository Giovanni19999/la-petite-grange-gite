//! Direct booking routes (F007): the public booking and tracking page, the owners' bookings,
//! payment instructions and message templates. Personal data only go to signed-in owners and
//! to the guest holding the tracking token.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use time::Date;
use time::format_description::well_known::Rfc3339;

use crate::app_state::AppState;
use crate::application::bookings::{
    Booking, BookingDetail, BookingError, BookingFilter, BookingInfo, BookingRequest,
    MAX_DOCUMENT_BYTES, PaymentInfo, PaymentInput,
};
use crate::application::messaging::QueuedMessage;
use crate::application::templates::{Layout, StoredTemplate, TemplatesError};
use crate::domain::booking::{BookingStatus, Guest};
use crate::domain::house_rules::arrival_slots;
use crate::domain::messaging::{Channel, Content};
use crate::domain::page::{Lang, Localized};
use crate::domain::quote::StayRequest;
use crate::domain::templates::TemplateKind;
use crate::http::auth::{AdminUser, ClientIp};
use crate::http::error::ApiError;

/// Room for the multipart envelope around the document.
const MULTIPART_OVERHEAD_BYTES: usize = 64 * 1024;

/// Public routes: booking information, booking, tracking.
pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/api/booking-info", get(info))
        .route("/api/bookings", post(create))
        .route("/api/bookings/{token}", get(track))
        .route(
            "/api/bookings/{token}/arrival-time",
            put(change_arrival_time),
        )
}

/// Admin routes under `/api/admin/bookings`.
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/{id}", get(detail))
        .route("/{id}/payments", post(record_payment))
        .route("/{id}/cancel", post(cancel))
        .route("/{id}/guest", put(update_guest))
}

/// Admin routes under `/api/admin/payment-info`.
pub fn payment_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(payment_info).put(update_payment_info))
        .route(
            "/document",
            get(document)
                .put(upload_document)
                .delete(remove_document)
                .layer(DefaultBodyLimit::max(
                    MAX_DOCUMENT_BYTES + MULTIPART_OVERHEAD_BYTES,
                )),
        )
}

/// Admin routes under `/api/admin/templates`.
pub fn template_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(templates))
        .route("/preview", post(preview))
        .route("/{kind}/{channel}/{lang}", put(save_template))
        .route("/{kind}/{channel}/{lang}/restore", post(restore_template))
}

impl From<BookingError> for ApiError {
    fn from(error: BookingError) -> Self {
        match error {
            BookingError::Invalid(details) => Self::Invalid(details),
            BookingError::Unavailable => Self::Conflict {
                code: "unavailable",
                message: error.to_string(),
            },
            BookingError::Closed => Self::Conflict {
                code: "booking_closed",
                message: error.to_string(),
            },
            BookingError::Changed => Self::Conflict {
                code: "stale",
                message: error.to_string(),
            },
            BookingError::TooManyAttempts => Self::TooManyRequests(error.to_string()),
            BookingError::NotFound => Self::NotFound,
            BookingError::Internal(error) => Self::Internal(error),
        }
    }
}

impl From<TemplatesError> for ApiError {
    fn from(error: TemplatesError) -> Self {
        match error {
            TemplatesError::Invalid(errors) => {
                Self::Invalid(errors.iter().map(ToString::to_string).collect())
            }
            TemplatesError::NotFound => Self::NotFound,
            TemplatesError::Internal(error) => Self::Internal(error),
        }
    }
}

fn bad_json(rejection: &JsonRejection) -> ApiError {
    ApiError::BadRequest(rejection.body_text())
}

fn time_text(at: time::OffsetDateTime) -> String {
    at.format(&Rfc3339).unwrap_or_default()
}

#[derive(Debug, Deserialize)]
pub struct LangQuery {
    #[serde(default = "french")]
    lang: Lang,
}

fn french() -> Lang {
    Lang::Fr
}

#[derive(Debug, Serialize)]
pub struct TermsResponse {
    slug: String,
    title: String,
}

#[derive(Debug, Serialize)]
pub struct InfoResponse {
    /// Booking is possible (rental terms page chosen and published).
    open: bool,
    capacity: u8,
    pets_allowed: bool,
    /// Minutes after midnight.
    arrival_from: u16,
    departure_before: u16,
    /// Arrival times the guest may choose.
    arrival_slots: Vec<u16>,
    /// Payment instructions in the language, sanitised HTML.
    instructions: String,
    /// A bank details document is attached to the confirmation email.
    document: bool,
    terms: Option<TermsResponse>,
}

fn info_response(info: BookingInfo, lang: Lang) -> InfoResponse {
    InfoResponse {
        open: info.open,
        capacity: info.rules.capacity,
        pets_allowed: info.rules.pets_allowed,
        arrival_from: info.rules.arrival_from,
        departure_before: info.rules.departure_before,
        arrival_slots: arrival_slots(info.rules.arrival_from),
        instructions: info.instructions.get(lang).to_owned(),
        document: info.document,
        terms: info.terms.map(|terms| TermsResponse {
            slug: terms.slug.get(lang).to_owned(),
            title: terms.title.get(lang).to_owned(),
        }),
    }
}

async fn info(
    State(state): State<AppState>,
    query: Result<Query<LangQuery>, QueryRejection>,
) -> Result<Json<InfoResponse>, ApiError> {
    let Query(query) = query.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    Ok(Json(info_response(
        state.bookings.info().await?,
        query.lang,
    )))
}

#[derive(Debug, Deserialize)]
// Unknown fields (a price sent by the browser…) are ignored: amounts come from the API.
pub struct CreateRequest {
    arrival: Date,
    departure: Date,
    adults: u8,
    #[serde(default)]
    children: u8,
    #[serde(default)]
    options: Vec<u64>,
    #[serde(default = "french")]
    lang: Lang,
    guest: Guest,
    arrival_time: u16,
    #[serde(default)]
    message: String,
    #[serde(default)]
    terms_accepted: bool,
    /// Hidden field of the form: filled only by robots.
    #[serde(default)]
    website: String,
}

#[derive(Debug, Serialize)]
pub struct CreatedResponse {
    reference: String,
    token: String,
}

async fn create(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    body: Result<Json<CreateRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(request) = body.map_err(|rejection| bad_json(&rejection))?;
    if !request.website.is_empty() {
        // A robot: nothing is stored, nothing is explained.
        tracing::warn!("booking refused: hidden field filled");
        return Ok(StatusCode::NO_CONTENT.into_response());
    }
    let created = state
        .bookings
        .create(
            BookingRequest {
                stay: StayRequest {
                    arrival: request.arrival,
                    departure: request.departure,
                    adults: request.adults,
                    children: request.children,
                    options: request.options,
                },
                lang: request.lang,
                guest: request.guest,
                arrival_time: request.arrival_time,
                message: request.message,
                terms_accepted: request.terms_accepted,
            },
            ip,
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedResponse {
            reference: created.reference,
            token: created.token,
        }),
    )
        .into_response())
}

#[derive(Debug, Serialize)]
pub struct OptionResponse {
    name: String,
    price: u64,
}

#[derive(Debug, Serialize)]
pub struct AmountsResponse {
    nights: u64,
    nights_total: u64,
    promotion: Option<String>,
    discount: u64,
    cleaning_fee: u64,
    options: Vec<OptionResponse>,
    options_total: u64,
    stay_total: u64,
    deposit: u64,
    balance: u64,
    /// `null` when everything was due at booking.
    balance_due: Option<Date>,
    tourist_tax: u64,
    paid: u64,
}

fn amounts(booking: &Booking, lang: Lang) -> AmountsResponse {
    let conditions = &booking.conditions;
    AmountsResponse {
        nights: conditions.nights,
        nights_total: conditions.nights_total,
        promotion: conditions
            .promotion
            .as_ref()
            .map(|promotion| promotion.name.get(lang).to_owned()),
        discount: conditions.discount,
        cleaning_fee: conditions.cleaning_fee,
        options: conditions
            .options
            .iter()
            .map(|booked| OptionResponse {
                name: booked.option.name.get(lang).to_owned(),
                price: booked.price,
            })
            .collect(),
        options_total: conditions.options_total,
        stay_total: conditions.stay_total,
        deposit: conditions.deposit,
        balance: conditions.balance,
        balance_due: conditions.balance_due,
        tourist_tax: conditions.tourist_tax,
        paid: booking.paid_total,
    }
}

#[derive(Debug, Serialize)]
pub struct TrackResponse {
    reference: String,
    status: &'static str,
    lang: Lang,
    arrival: Date,
    departure: Date,
    adults: u8,
    children: u8,
    first_name: String,
    arrival_time: u16,
    arrival_slots: Vec<u16>,
    arrival_time_changeable: bool,
    arrival_from: u16,
    departure_before: u16,
    pets_allowed: bool,
    amounts: AmountsResponse,
    /// Payment instructions copied on the booking, sanitised HTML.
    instructions: String,
}

async fn track(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<TrackResponse>, ApiError> {
    let (booking, changeable) = state.bookings.track(&token).await?;
    let lang = booking.lang;
    let rules = &booking.conditions.rules;
    Ok(Json(TrackResponse {
        reference: booking.reference.clone(),
        status: booking.status.as_str(),
        lang,
        arrival: booking.arrival,
        departure: booking.departure,
        adults: booking.adults,
        children: booking.children,
        first_name: booking.guest.first_name.clone(),
        arrival_time: booking.arrival_time,
        arrival_slots: arrival_slots(rules.arrival_from),
        arrival_time_changeable: changeable,
        arrival_from: rules.arrival_from,
        departure_before: rules.departure_before,
        pets_allowed: rules.pets_allowed,
        amounts: amounts(&booking, lang),
        instructions: booking.conditions.payment_instructions.get(lang).to_owned(),
    }))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArrivalTimeRequest {
    arrival_time: u16,
}

async fn change_arrival_time(
    State(state): State<AppState>,
    Path(token): Path<String>,
    body: Result<Json<ArrivalTimeRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(request) = body.map_err(|rejection| bad_json(&rejection))?;
    state
        .bookings
        .change_arrival_time(&token, request.arrival_time)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    status: Option<String>,
    from: Option<Date>,
    to: Option<Date>,
}

#[derive(Debug, Serialize)]
pub struct ListItem {
    id: u64,
    reference: String,
    status: &'static str,
    arrival: Date,
    departure: Date,
    guests: u16,
    guest: String,
    stay_total: u64,
    deposit: u64,
    paid: u64,
    /// Deposit not received after the alert delay (the owners were alerted).
    deposit_late: bool,
    /// Balance not received and its due date passed.
    balance_late: bool,
    created_at: String,
}

fn list_item(booking: &Booking, today: Date) -> ListItem {
    let conditions = &booking.conditions;
    let active = booking.status != BookingStatus::Cancelled;
    ListItem {
        id: booking.id,
        reference: booking.reference.clone(),
        status: booking.status.as_str(),
        arrival: booking.arrival,
        departure: booking.departure,
        guests: u16::from(booking.adults) + u16::from(booking.children),
        guest: format!("{} {}", booking.guest.first_name, booking.guest.last_name),
        stay_total: conditions.stay_total,
        deposit: conditions.deposit,
        paid: booking.paid_total,
        deposit_late: active
            && booking.deposit_alert_sent_at.is_some()
            && booking.deposit_expected(),
        balance_late: active
            && booking.paid_total < conditions.stay_total
            && conditions.balance_due.is_none_or(|due| due < today),
        created_at: time_text(booking.created_at),
    }
}

async fn list(
    _user: AdminUser,
    State(state): State<AppState>,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<Vec<ListItem>>, ApiError> {
    let Query(query) = query.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    let status = match query.status.as_deref() {
        None | Some("") => None,
        Some(value) => Some(
            BookingStatus::parse(value)
                .ok_or_else(|| ApiError::BadRequest(format!("statut inconnu : {value}")))?,
        ),
    };
    let today = state.bookings.today();
    Ok(Json(
        state
            .bookings
            .list(BookingFilter {
                status,
                from: query.from,
                to: query.to,
            })
            .await?
            .iter()
            .map(|booking| list_item(booking, today))
            .collect(),
    ))
}

#[derive(Debug, Serialize)]
pub struct PaymentResponse {
    amount: u64,
    paid_on: Date,
    method: String,
    note: String,
    author: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize)]
pub struct EventResponse {
    description: String,
    author: Option<String>,
    at: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    id: u64,
    channel: &'static str,
    recipient: String,
    summary: String,
    status: &'static str,
    error: Option<String>,
    created_at: String,
    sent_at: Option<String>,
}

fn message_response(message: QueuedMessage) -> MessageResponse {
    MessageResponse {
        id: message.id,
        channel: message.content.channel().as_str(),
        recipient: message.recipient,
        summary: match &message.content {
            Content::Email { subject, .. } => subject.clone(),
            Content::Sms { text } => text.chars().take(60).collect(),
        },
        status: message.status.as_str(),
        error: message.last_error.map(|error| error.to_string()),
        created_at: time_text(message.created_at),
        sent_at: message.sent_at.map(time_text),
    }
}

#[derive(Debug, Serialize)]
pub struct DetailResponse {
    id: u64,
    reference: String,
    status: &'static str,
    payment_mode: &'static str,
    lang: Lang,
    arrival: Date,
    departure: Date,
    adults: u8,
    children: u8,
    guest: Guest,
    arrival_time: u16,
    arrival_slots: Vec<u16>,
    message: String,
    amounts: AmountsResponse,
    deposit_percent: Option<u8>,
    terms: TermsAcceptanceResponse,
    tracking_link: String,
    created_at: String,
    cancelled_at: Option<String>,
    cancel_reason: Option<String>,
    deposit_alert_sent_at: Option<String>,
    payments: Vec<PaymentResponse>,
    events: Vec<EventResponse>,
    messages: Vec<MessageResponse>,
}

#[derive(Debug, Serialize)]
pub struct TermsAcceptanceResponse {
    page: u64,
    version: u32,
    accepted_at: String,
}

async fn detail(
    _user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<DetailResponse>, ApiError> {
    let BookingDetail {
        booking,
        payments,
        events,
        messages,
    } = state.bookings.detail(id).await?;
    let conditions = &booking.conditions;
    Ok(Json(DetailResponse {
        id: booking.id,
        reference: booking.reference.clone(),
        status: booking.status.as_str(),
        payment_mode: booking.payment_mode.as_str(),
        lang: booking.lang,
        arrival: booking.arrival,
        departure: booking.departure,
        adults: booking.adults,
        children: booking.children,
        guest: booking.guest.clone(),
        arrival_time: booking.arrival_time,
        arrival_slots: arrival_slots(conditions.rules.arrival_from),
        message: booking.message.clone(),
        amounts: amounts(&booking, Lang::Fr),
        deposit_percent: match conditions.rules.deposit {
            crate::domain::house_rules::Deposit::Percentage { percent } => Some(percent),
            crate::domain::house_rules::Deposit::Fixed { .. } => None,
        },
        terms: TermsAcceptanceResponse {
            page: conditions.terms.page,
            version: conditions.terms.version,
            accepted_at: time_text(conditions.terms.accepted_at),
        },
        tracking_link: state.bookings.tracking_link(&booking),
        created_at: time_text(booking.created_at),
        cancelled_at: booking.cancelled_at.map(time_text),
        cancel_reason: booking.cancel_reason.clone(),
        deposit_alert_sent_at: booking.deposit_alert_sent_at.map(time_text),
        payments: payments
            .into_iter()
            .map(|payment| PaymentResponse {
                amount: payment.amount,
                paid_on: payment.paid_on,
                method: payment.method,
                note: payment.note,
                author: payment.author,
                created_at: time_text(payment.created_at),
            })
            .collect(),
        events: events
            .into_iter()
            .map(|event| EventResponse {
                description: event.description,
                author: event.author,
                at: time_text(event.at),
            })
            .collect(),
        messages: messages.into_iter().map(message_response).collect(),
    }))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentRequest {
    amount: u64,
    paid_on: Date,
    method: String,
    #[serde(default)]
    note: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    status: &'static str,
}

async fn record_payment(
    user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    body: Result<Json<PaymentRequest>, JsonRejection>,
) -> Result<Json<StatusResponse>, ApiError> {
    let Json(request) = body.map_err(|rejection| bad_json(&rejection))?;
    let status = state
        .bookings
        .record_payment(
            id,
            PaymentInput {
                amount: request.amount,
                paid_on: request.paid_on,
                method: request.method,
                note: request.note,
            },
            user.0.id,
        )
        .await?;
    Ok(Json(StatusResponse {
        status: status.as_str(),
    }))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelRequest {
    reason: String,
}

async fn cancel(
    user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    body: Result<Json<CancelRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(request) = body.map_err(|rejection| bad_json(&rejection))?;
    state
        .bookings
        .cancel(id, &request.reason, user.0.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuestRequest {
    guest: Guest,
    arrival_time: u16,
}

async fn update_guest(
    user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    body: Result<Json<GuestRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(request) = body.map_err(|rejection| bad_json(&rejection))?;
    state
        .bookings
        .update_guest(id, request.guest, request.arrival_time, user.0.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct DocumentResponse {
    name: String,
    content_type: String,
}

#[derive(Debug, Serialize)]
pub struct PaymentInfoResponse {
    instructions: Localized,
    methods: Vec<String>,
    document: Option<DocumentResponse>,
    /// Online payment provider configured on the server (`none`).
    provider: &'static str,
}

fn payment_info_response(info: PaymentInfo, provider: &'static str) -> PaymentInfoResponse {
    PaymentInfoResponse {
        instructions: info.instructions,
        methods: info.methods,
        document: info.document.map(|document| DocumentResponse {
            name: document.name,
            content_type: document.content_type,
        }),
        provider,
    }
}

async fn payment_info(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<PaymentInfoResponse>, ApiError> {
    Ok(Json(payment_info_response(
        state.bookings.payment_info().await?,
        state.bookings.payment_provider(),
    )))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentInfoRequest {
    instructions: Localized,
    methods: Vec<String>,
}

async fn update_payment_info(
    _user: AdminUser,
    State(state): State<AppState>,
    body: Result<Json<PaymentInfoRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(request) = body.map_err(|rejection| bad_json(&rejection))?;
    state
        .bookings
        .update_payment_info(request.instructions, request.methods)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn upload_document(
    _user: AdminUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<StatusCode, ApiError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::BadRequest(error.body_text()))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let name = field.file_name().unwrap_or("RIB").to_owned();
        let bytes = field
            .bytes()
            .await
            .map_err(|error| ApiError::BadRequest(error.body_text()))?;
        state.bookings.set_document(&name, &bytes).await?;
        return Ok(StatusCode::NO_CONTENT);
    }
    Err(ApiError::BadRequest("fichier manquant".to_owned()))
}

async fn remove_document(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    state.bookings.remove_document().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn document(_user: AdminUser, State(state): State<AppState>) -> Result<Response, ApiError> {
    let Some((document, bytes)) = state.bookings.document().await? else {
        return Err(ApiError::NotFound);
    };
    let disposition = format!(
        "attachment; filename=\"{}\"",
        document.name.replace(['"', '\\'], "")
    );
    Ok((
        [
            (header::CONTENT_TYPE, document.content_type),
            (header::CONTENT_DISPOSITION, disposition),
            (header::CACHE_CONTROL, "private, no-store".to_owned()),
        ],
        bytes,
    )
        .into_response())
}

#[derive(Debug, Serialize)]
pub struct VariableResponse {
    name: &'static str,
    label: &'static str,
}

#[derive(Debug, Serialize)]
pub struct TemplateText {
    lang: Lang,
    subject: String,
    body: String,
    /// Differs from the default text.
    changed: bool,
}

#[derive(Debug, Serialize)]
pub struct TemplateChannel {
    channel: &'static str,
    texts: Vec<TemplateText>,
}

#[derive(Debug, Serialize)]
pub struct TemplateResponse {
    kind: &'static str,
    label: &'static str,
    recipient: &'static str,
    variables: Vec<VariableResponse>,
    channels: Vec<TemplateChannel>,
}

fn template_responses(templates: &[StoredTemplate]) -> Vec<TemplateResponse> {
    TemplateKind::ALL
        .into_iter()
        .map(|kind| TemplateResponse {
            kind: kind.as_str(),
            label: kind.label(),
            recipient: match kind.recipient() {
                crate::domain::templates::Recipient::Guest => "guest",
                crate::domain::templates::Recipient::Owners => "owners",
            },
            variables: kind
                .variables()
                .into_iter()
                .map(|variable| VariableResponse {
                    name: variable.name,
                    label: variable.label,
                })
                .collect(),
            channels: [Channel::Email, Channel::Sms]
                .into_iter()
                .map(|channel| TemplateChannel {
                    channel: channel.as_str(),
                    texts: kind
                        .languages()
                        .iter()
                        .filter_map(|lang| {
                            templates.iter().find(|template| {
                                template.kind == kind
                                    && template.channel == channel
                                    && template.lang == *lang
                            })
                        })
                        .map(|template| TemplateText {
                            lang: template.lang,
                            subject: template.subject.clone(),
                            body: template.body.clone(),
                            changed: template.subject != template.default_subject
                                || template.body != template.default_body,
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

async fn templates(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<TemplateResponse>>, ApiError> {
    Ok(Json(template_responses(&state.templates.list().await?)))
}

fn template_path(
    kind: &str,
    channel: &str,
    lang: &str,
) -> Result<(TemplateKind, Channel, Lang), ApiError> {
    let kind = TemplateKind::parse(kind).ok_or(ApiError::NotFound)?;
    let channel = Channel::parse(channel).ok_or(ApiError::NotFound)?;
    let lang = match lang {
        "fr" => Lang::Fr,
        "en" => Lang::En,
        _ => return Err(ApiError::NotFound),
    };
    Ok((kind, channel, lang))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateRequest {
    #[serde(default)]
    subject: String,
    body: String,
}

async fn save_template(
    _user: AdminUser,
    State(state): State<AppState>,
    Path((kind, channel, lang)): Path<(String, String, String)>,
    body: Result<Json<TemplateRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let (kind, channel, lang) = template_path(&kind, &channel, &lang)?;
    let Json(request) = body.map_err(|rejection| bad_json(&rejection))?;
    state
        .templates
        .save(kind, channel, lang, &request.subject, &request.body)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn restore_template(
    _user: AdminUser,
    State(state): State<AppState>,
    Path((kind, channel, lang)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    let (kind, channel, lang) = template_path(&kind, &channel, &lang)?;
    state.templates.restore(kind, channel, lang).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewRequest {
    kind: String,
    channel: String,
    lang: String,
    #[serde(default)]
    subject: String,
    body: String,
}

#[derive(Debug, Serialize)]
pub struct PreviewResponse {
    subject: String,
    html: Option<String>,
    text: String,
}

async fn preview(
    _user: AdminUser,
    State(state): State<AppState>,
    body: Result<Json<PreviewRequest>, JsonRejection>,
) -> Result<Json<PreviewResponse>, ApiError> {
    let Json(request) = body.map_err(|rejection| bad_json(&rejection))?;
    let (kind, channel, lang) = template_path(&request.kind, &request.channel, &request.lang)?;
    let cottage = state.calendar.cottage().await?;
    let rendered = state.templates.preview(
        kind,
        channel,
        lang,
        &request.subject,
        &request.body,
        &Layout {
            cottage: cottage.name,
            site_url: state.bookings.site_url().to_owned(),
        },
    )?;
    Ok(Json(PreviewResponse {
        subject: rendered.subject,
        html: rendered.html,
        text: rendered.text,
    }))
}
