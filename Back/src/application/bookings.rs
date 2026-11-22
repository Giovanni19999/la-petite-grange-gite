//! Direct booking use cases (F007, ADR-0020): booking from the site without online payment,
//! the guest's tracking page, the owners' screen (payments, cancellation, details), the
//! payment instructions and the alert when the deposit is late.
//!
//! Prices are always recomputed here ([`QuoteService`]); the dates are locked and checked
//! again by [`BookingRepository::create`] in the transaction that stores the booking and
//! queues its messages.

use std::fmt;
use std::net::IpAddr;
use std::sync::Arc;

use base64::Engine;
use time::{Date, Duration, OffsetDateTime};

use crate::application::auth::Clock;
use crate::application::calendar::{CottageId, CottageRepository};
use crate::application::health::BoxFuture;
use crate::application::house_rules::HouseRulesService;
use crate::application::messaging::{DocumentStore, MessagingService, QueuedMessage};
use crate::application::quote::{QuoteError, QuoteResult, QuoteService};
use crate::application::templates::{Layout, StoredTemplate, TemplatesService, content, pick};
use crate::domain::auth::UserId;
use crate::domain::booking::{
    BookedOption, BookingRuleError, BookingStatus, Conditions, Guest, MAX_MESSAGE, MAX_NOTE,
    PaymentMode, TermsAcceptance, date_text, money_text, valid_arrival_time,
};
use crate::domain::calendar::paris_date;
use crate::domain::house_rules::{HouseRules, clock_time};
use crate::domain::messaging::{Attachment, Channel, Content, MessagingSettings, OutgoingMessage};
use crate::domain::page::{Lang, Localized, sanitize_rich_text};
use crate::domain::quote::StayRequest;
use crate::domain::templates::{MessageValues, TemplateKind, html_to_text};

/// Largest bank details document.
pub const MAX_DOCUMENT_BYTES: usize = 5 * 1024 * 1024;
/// Most payment methods, and their longest name.
const MAX_METHODS: usize = 20;
const MAX_METHOD: usize = 100;

/// A booking as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Booking {
    pub id: u64,
    pub reference: String,
    pub token: String,
    pub status: BookingStatus,
    pub payment_mode: PaymentMode,
    pub lang: Lang,
    pub arrival: Date,
    pub departure: Date,
    pub adults: u8,
    pub children: u8,
    pub guest: Guest,
    /// Expected arrival time, minutes after midnight.
    pub arrival_time: u16,
    pub message: String,
    pub conditions: Conditions,
    pub paid_total: u64,
    pub created_at: OffsetDateTime,
    pub cancelled_at: Option<OffsetDateTime>,
    pub cancel_reason: Option<String>,
    pub deposit_alert_sent_at: Option<OffsetDateTime>,
}

impl Booking {
    /// The deposit is still expected after its due date… or just expected.
    #[must_use]
    pub fn deposit_expected(&self) -> bool {
        self.status != BookingStatus::Cancelled && self.paid_total < self.conditions.deposit
    }
}

/// A booking to store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBooking {
    pub cottage: CottageId,
    pub token: String,
    pub status: BookingStatus,
    pub payment_mode: PaymentMode,
    pub lang: Lang,
    pub stay: StayRequest,
    pub guest: Guest,
    pub arrival_time: u16,
    pub message: String,
    pub conditions: Conditions,
    pub created_at: OffsetDateTime,
    /// First line of the history.
    pub event: String,
}

/// A line of the history of a booking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookingEvent {
    pub description: String,
    /// Email of the owner who did it; `None` for the guest or the site.
    pub author: Option<String>,
    pub at: OffsetDateTime,
}

/// A line of history to record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEvent {
    pub description: String,
    pub author: Option<UserId>,
    pub at: OffsetDateTime,
}

/// A payment recorded by the owners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payment {
    pub amount: u64,
    pub paid_on: Date,
    pub method: String,
    pub note: String,
    pub author: Option<String>,
    pub created_at: OffsetDateTime,
}

/// A payment to record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPayment {
    pub amount: u64,
    pub paid_on: Date,
    pub method: String,
    pub note: String,
    pub author: UserId,
    pub created_at: OffsetDateTime,
}

/// What became of a booking attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateOutcome {
    Created(Box<Booking>),
    /// A night is taken by another booking or a blocked period.
    Unavailable,
}

/// Filter of the owners' list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BookingFilter {
    pub status: Option<BookingStatus>,
    /// Stays ending after this date.
    pub from: Option<Date>,
    /// Stays starting before this date.
    pub to: Option<Date>,
}

/// Messages of a new booking, composed once its id and reference are known.
pub type Compose<'a> = dyn Fn(&Booking) -> anyhow::Result<Vec<OutgoingMessage>> + Send + Sync + 'a;

/// Port: bookings in the database.
pub trait BookingRepository: Send + Sync {
    /// Locks the cottage, checks that no booking blocking the dates nor blocked period shares a
    /// night with the stay, gives the next reference of the arrival year, stores the booking,
    /// its first history line and its messages, in one transaction.
    fn create<'a>(
        &'a self,
        booking: &'a NewBooking,
        compose: &'a Compose<'a>,
    ) -> BoxFuture<'a, anyhow::Result<CreateOutcome>>;
    fn find(&self, id: u64) -> BoxFuture<'_, anyhow::Result<Option<Booking>>>;
    fn by_token<'a>(&'a self, token: &'a str) -> BoxFuture<'a, anyhow::Result<Option<Booking>>>;
    /// Stays to come first, then the past ones.
    fn list<'a>(
        &'a self,
        cottage: CottageId,
        filter: &'a BookingFilter,
        today: Date,
    ) -> BoxFuture<'a, anyhow::Result<Vec<Booking>>>;
    fn payments(&self, id: u64) -> BoxFuture<'_, anyhow::Result<Vec<Payment>>>;
    /// Latest first.
    fn events(&self, id: u64) -> BoxFuture<'_, anyhow::Result<Vec<BookingEvent>>>;
    fn messages(&self, id: u64) -> BoxFuture<'_, anyhow::Result<Vec<QueuedMessage>>>;
    /// Records the payment if the paid total is still `paid_before`; false otherwise.
    fn add_payment<'a>(
        &'a self,
        id: u64,
        payment: &'a NewPayment,
        paid_before: u64,
        status: BookingStatus,
        event: &'a NewEvent,
    ) -> BoxFuture<'a, anyhow::Result<bool>>;
    /// False if the booking is already cancelled.
    fn cancel<'a>(
        &'a self,
        id: u64,
        reason: &'a str,
        event: &'a NewEvent,
    ) -> BoxFuture<'a, anyhow::Result<bool>>;
    /// Replaces the guest's details and arrival time, records the event, queues `messages`.
    fn update_guest<'a>(
        &'a self,
        id: u64,
        guest: &'a Guest,
        arrival_time: u16,
        event: &'a NewEvent,
        messages: &'a [OutgoingMessage],
    ) -> BoxFuture<'a, anyhow::Result<()>>;
    /// Confirmed bookings whose deposit is not covered and whose alert was not sent.
    fn deposit_alert_candidates(
        &self,
        cottage: CottageId,
    ) -> BoxFuture<'_, anyhow::Result<Vec<Booking>>>;
    /// Marks the alert sent and queues its messages; false if it was already sent.
    fn mark_deposit_alert<'a>(
        &'a self,
        id: u64,
        event: &'a NewEvent,
        messages: &'a [OutgoingMessage],
    ) -> BoxFuture<'a, anyhow::Result<bool>>;
}

/// The bank details document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub name: String,
    pub content_type: String,
    /// Name in the private directory.
    pub file: String,
}

/// How to pay offline (house rules screen, F007).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaymentInfo {
    /// Sanitised HTML.
    pub instructions: Localized,
    pub methods: Vec<String>,
    pub document: Option<Document>,
}

/// Port: payment instructions of the cottage.
pub trait PaymentInfoRepository: Send + Sync {
    fn load(&self, cottage: CottageId) -> BoxFuture<'_, anyhow::Result<PaymentInfo>>;
    fn save<'a>(
        &'a self,
        cottage: CottageId,
        info: &'a PaymentInfo,
    ) -> BoxFuture<'a, anyhow::Result<()>>;
}

/// The published page of the rental terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermsPage {
    pub id: u64,
    pub slug: Localized,
    pub title: Localized,
    /// Number of its published version.
    pub version: u32,
}

/// Port: pages that can hold the rental terms.
pub trait TermsPages: Send + Sync {
    /// The page if it is published.
    fn published(&self, page: u64) -> BoxFuture<'_, anyhow::Result<Option<TermsPage>>>;
}

/// Port: booking attempts per IP address.
pub trait BookingAttempts: Send + Sync {
    /// Records an attempt and returns how many there were from `ip` since `since`, this one
    /// included; forgets the older ones.
    fn record<'a>(
        &'a self,
        ip: &'a str,
        now: OffsetDateTime,
        since: OffsetDateTime,
    ) -> BoxFuture<'a, anyhow::Result<u32>>;
}

/// Port: the online payment provider (ADR-0006). Only `none` exists while the owners have
/// not chosen one (ADR-0020); the checkout, confirmation, refund and webhook operations
/// arrive with the first provider adapter.
pub trait PaymentGateway: Send + Sync {
    /// Configured name (`PAYMENT_PROVIDER`).
    fn name(&self) -> &'static str;
    /// Whether guests pay online.
    fn online(&self) -> bool;
}

/// No online payment: bookings are confirmed at once and paid offline (ADR-0020).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoPayment;

impl PaymentGateway for NoPayment {
    fn name(&self) -> &'static str {
        "none"
    }

    fn online(&self) -> bool {
        false
    }
}

/// Technical settings of the bookings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookingSettings {
    /// Public address of the site, for the links of the messages.
    pub site_url: String,
    /// Most booking attempts per IP address within `attempt_window`.
    pub max_attempts: u32,
    pub attempt_window: Duration,
}

/// Adapters of the booking use cases.
#[derive(Clone)]
pub struct BookingPorts {
    pub bookings: Arc<dyn BookingRepository>,
    pub payment_info: Arc<dyn PaymentInfoRepository>,
    pub terms: Arc<dyn TermsPages>,
    pub attempts: Arc<dyn BookingAttempts>,
    pub documents: Arc<dyn DocumentStore>,
    pub payments: Arc<dyn PaymentGateway>,
    pub cottages: Arc<dyn CottageRepository>,
    pub clock: Arc<dyn Clock>,
}

/// Use cases the bookings rely on.
#[derive(Clone)]
pub struct BookingServices {
    pub quotes: Arc<QuoteService>,
    pub rules: Arc<HouseRulesService>,
    pub messaging: Arc<MessagingService>,
    pub templates: Arc<TemplatesService>,
}

impl fmt::Debug for BookingPorts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BookingPorts").finish_non_exhaustive()
    }
}

impl fmt::Debug for BookingServices {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BookingServices").finish_non_exhaustive()
    }
}

/// What a visitor sends to book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookingRequest {
    pub stay: StayRequest,
    pub lang: Lang,
    pub guest: Guest,
    pub arrival_time: u16,
    pub message: String,
    pub terms_accepted: bool,
}

/// Answer to a booking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookingCreated {
    pub reference: String,
    /// Tracking token of the guest's link.
    pub token: String,
}

/// What the booking page shows besides the quote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookingInfo {
    /// Booking is possible (rental terms page chosen and published).
    pub open: bool,
    pub rules: HouseRules,
    pub instructions: Localized,
    pub document: bool,
    pub terms: Option<TermsPage>,
}

/// A payment typed by the owners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentInput {
    pub amount: u64,
    pub paid_on: Date,
    pub method: String,
    pub note: String,
}

/// Everything the detail screen shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookingDetail {
    pub booking: Booking,
    pub payments: Vec<Payment>,
    pub events: Vec<BookingEvent>,
    pub messages: Vec<QueuedMessage>,
}

/// Why a booking use case is refused.
#[derive(Debug, thiserror::Error)]
pub enum BookingError {
    /// Every reason, in French.
    #[error("réservation invalide")]
    Invalid(Vec<String>),
    #[error("ces dates ne sont plus disponibles")]
    Unavailable,
    #[error("la réservation en ligne n'est pas encore ouverte : contactez-nous")]
    Closed,
    #[error("trop de tentatives de réservation, réessayez plus tard")]
    TooManyAttempts,
    #[error("réservation introuvable")]
    NotFound,
    #[error("la réservation vient d'être modifiée : rechargez la page")]
    Changed,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl From<BookingRuleError> for BookingError {
    fn from(error: BookingRuleError) -> Self {
        Self::Invalid(vec![error.to_string()])
    }
}

/// Direct booking use cases.
#[derive(Clone)]
pub struct BookingService {
    ports: BookingPorts,
    services: BookingServices,
    settings: BookingSettings,
}

impl fmt::Debug for BookingService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BookingService").finish_non_exhaustive()
    }
}

/// Random tracking token: 32 bytes, URL-safe base64.
fn new_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::fill(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

impl BookingService {
    #[must_use]
    pub fn new(ports: BookingPorts, services: BookingServices, settings: BookingSettings) -> Self {
        Self {
            ports,
            services,
            settings,
        }
    }

    fn site(&self) -> &str {
        self.settings.site_url.trim_end_matches('/')
    }

    /// Public address of the site.
    #[must_use]
    pub fn site_url(&self) -> &str {
        self.site()
    }

    /// Today in the cottage's time zone.
    #[must_use]
    pub fn today(&self) -> Date {
        paris_date(self.ports.clock.now())
    }

    /// Link of the guest's tracking page.
    #[must_use]
    pub fn tracking_link(&self, booking: &Booking) -> String {
        match booking.lang {
            Lang::Fr => format!("{}/reservation/{}", self.site(), booking.token),
            Lang::En => format!("{}/en/booking/{}", self.site(), booking.token),
        }
    }

    fn admin_link(&self, booking: &Booking) -> String {
        format!("{}/admin/reservations/{}", self.site(), booking.id)
    }

    /// Name of the online payment provider (`none`).
    #[must_use]
    pub fn payment_provider(&self) -> &'static str {
        self.ports.payments.name()
    }

    /// The published rental terms page of the rules, if any.
    async fn terms(&self, rules: &HouseRules) -> anyhow::Result<Option<TermsPage>> {
        match rules.terms_page {
            Some(page) => self.ports.terms.published(page).await,
            None => Ok(None),
        }
    }

    /// What the booking page shows besides the quote.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn info(&self) -> anyhow::Result<BookingInfo> {
        let cottage = self.ports.cottages.default_cottage().await?;
        let rules = self.services.rules.rules().await?;
        let payment = self.ports.payment_info.load(cottage.id).await?;
        let terms = self.terms(&rules).await?;
        Ok(BookingInfo {
            open: terms.is_some(),
            rules,
            instructions: payment.instructions,
            document: payment.document.is_some(),
            terms,
        })
    }

    /// Books a stay from the site: confirmed at once, paid offline (ADR-0020).
    ///
    /// # Errors
    ///
    /// Too many attempts, booking closed, invalid details or stay, dates taken meanwhile,
    /// infrastructure.
    pub async fn create(
        &self,
        request: BookingRequest,
        ip: IpAddr,
    ) -> Result<BookingCreated, BookingError> {
        let now = self.ports.clock.now();
        let attempts = self
            .ports
            .attempts
            .record(&ip.to_string(), now, now - self.settings.attempt_window)
            .await?;
        if attempts > self.settings.max_attempts {
            return Err(BookingError::TooManyAttempts);
        }
        let cottage = self.ports.cottages.default_cottage().await?;
        let rules = self.services.rules.rules().await?;
        let Some(terms) = self.terms(&rules).await? else {
            return Err(BookingError::Closed);
        };
        if self.ports.payments.online() {
            // ADR-0005 applies to an online provider; no adapter exists yet (ADR-0020).
            return Err(BookingError::Internal(anyhow::anyhow!(
                "online payment is not implemented"
            )));
        }

        let (guest, quoted, message) = self.checked(&request, &rules).await?;
        let payment = self.ports.payment_info.load(cottage.id).await?;
        let conditions = conditions_of(
            &quoted,
            rules,
            payment.instructions.clone(),
            TermsAcceptance {
                page: terms.id,
                version: terms.version,
                accepted_at: now,
            },
        );
        let booking = NewBooking {
            cottage: cottage.id,
            token: new_token(),
            status: BookingStatus::Confirmed,
            payment_mode: PaymentMode::Offline,
            lang: request.lang,
            stay: request.stay,
            guest,
            arrival_time: request.arrival_time,
            message,
            conditions,
            created_at: now,
            event: "Réservation faite en ligne par le voyageur, confirmée (paiement hors ligne)"
                .to_owned(),
        };
        let settings = self.services.messaging.settings().await?;
        let templates = self.services.templates.list().await?;
        let layout = Layout {
            cottage: cottage.name.clone(),
            site_url: self.settings.site_url.clone(),
        };
        let attachment = payment.document.as_ref().map(|document| Attachment {
            name: document.name.clone(),
            content_type: document.content_type.clone(),
            file: document.file.clone(),
        });
        let compose = |booking: &Booking| {
            let mut messages = self.compose(
                booking,
                TemplateKind::BookingConfirmed,
                &templates,
                &settings,
                &layout,
            )?;
            if let Some(attachment) = &attachment {
                for message in &mut messages {
                    if let Content::Email { attachments, .. } = &mut message.content {
                        attachments.push(attachment.clone());
                    }
                }
            }
            messages.extend(self.compose(
                booking,
                TemplateKind::NewBooking,
                &templates,
                &settings,
                &layout,
            )?);
            Ok(messages)
        };
        match self.ports.bookings.create(&booking, &compose).await? {
            CreateOutcome::Created(created) => {
                tracing::info!(booking = created.id, "direct booking created");
                Ok(BookingCreated {
                    reference: created.reference,
                    token: created.token,
                })
            }
            CreateOutcome::Unavailable => Err(BookingError::Unavailable),
        }
    }

    /// The guest's details cleaned, the quote recomputed and the message trimmed, or every
    /// reason to refuse the booking.
    async fn checked(
        &self,
        request: &BookingRequest,
        rules: &HouseRules,
    ) -> Result<(Guest, QuoteResult, String), BookingError> {
        let mut errors = Vec::new();
        if !request.terms_accepted {
            errors.push(BookingRuleError::TermsNotAccepted.to_string());
        }
        let guest = match request.guest.clone().validated() {
            Ok(guest) => Some(guest),
            Err(guest_errors) => {
                errors.extend(guest_errors.iter().map(ToString::to_string));
                None
            }
        };
        if !valid_arrival_time(request.arrival_time, rules) {
            errors.push(BookingRuleError::ArrivalTime(rules.arrival_from).to_string());
        }
        let message = request.message.trim().to_owned();
        if message.chars().count() > MAX_MESSAGE {
            errors.push(BookingRuleError::MessageTooLong.to_string());
        }
        let quoted = match self.services.quotes.quote(request.stay.clone()).await {
            Ok(quoted) => Some(quoted),
            Err(QuoteError::Refused(reasons)) => {
                errors.extend(reasons.iter().map(ToString::to_string));
                None
            }
            Err(QuoteError::BadRange(reason)) => {
                errors.push(reason);
                None
            }
            Err(QuoteError::Internal(error)) => return Err(BookingError::Internal(error)),
        };
        match (guest, quoted, errors.is_empty()) {
            (Some(guest), Some(quoted), true) => Ok((guest, quoted, message)),
            _ => Err(BookingError::Invalid(errors)),
        }
    }

    /// Messages of a template for a booking: to the guest (their email and phone) or to the
    /// owners (the notification addresses and phones of F025).
    fn compose(
        &self,
        booking: &Booking,
        kind: TemplateKind,
        templates: &[StoredTemplate],
        settings: &MessagingSettings,
        layout: &Layout,
    ) -> anyhow::Result<Vec<OutgoingMessage>> {
        let guest = kind.recipient() == crate::domain::templates::Recipient::Guest;
        let lang = if guest { booking.lang } else { Lang::Fr };
        let values = self.values(booking, lang);
        let mut messages = Vec::new();
        for channel in [Channel::Email, Channel::Sms] {
            let Some(template) = pick(templates, kind, channel, lang) else {
                continue;
            };
            let recipients: Vec<String> = match (guest, channel) {
                (true, Channel::Email) => vec![booking.guest.email.clone()],
                (true, Channel::Sms) => vec![booking.guest.phone.clone()],
                (false, Channel::Email) => settings.owner_emails.clone(),
                (false, Channel::Sms) => settings.owner_phones.clone(),
            };
            let content = content(template, &values, layout)?;
            for recipient in recipients {
                messages.push(OutgoingMessage {
                    key: format!(
                        "booking:{}:{}:{}:{recipient}",
                        booking.id,
                        kind.as_str(),
                        channel.as_str()
                    ),
                    recipient,
                    content: content.clone(),
                });
            }
        }
        Ok(messages)
    }

    /// Values of the variables for a booking, formatted in `lang`.
    #[must_use]
    pub fn values(&self, booking: &Booking, lang: Lang) -> MessageValues {
        let conditions = &booking.conditions;
        let english = lang == Lang::En;
        let at_booking = if english {
            "at booking"
        } else {
            "à la réservation"
        };
        let none = if english { "(none)" } else { "(aucun)" };
        let money = |cents| money_text(cents, lang);
        let texts = [
            ("voyageur.prenom", booking.guest.first_name.clone()),
            ("voyageur.nom", booking.guest.last_name.clone()),
            ("voyageur.email", booking.guest.email.clone()),
            ("voyageur.telephone", booking.guest.phone.clone()),
            (
                "voyageur.message",
                if booking.message.is_empty() {
                    none.to_owned()
                } else {
                    booking.message.clone()
                },
            ),
            ("sejour.reference", booking.reference.clone()),
            ("sejour.arrivee", date_text(booking.arrival, lang)),
            ("sejour.depart", date_text(booking.departure, lang)),
            ("sejour.nuits", conditions.nights.to_string()),
            (
                "sejour.voyageurs",
                (u16::from(booking.adults) + u16::from(booking.children)).to_string(),
            ),
            ("sejour.heure_arrivee", clock_time(booking.arrival_time)),
            ("montants.total", money(conditions.stay_total)),
            ("montants.acompte", money(conditions.deposit)),
            ("montants.acompte_echeance", at_booking.to_owned()),
            ("montants.solde", money(conditions.balance)),
            (
                "montants.solde_echeance",
                match conditions.balance_due {
                    Some(date) if english => format!("on {}", date_text(date, lang)),
                    Some(date) => format!("le {}", date_text(date, lang)),
                    None => at_booking.to_owned(),
                },
            ),
            ("montants.taxe_sejour", money(conditions.tourist_tax)),
            (
                "regles.heure_arrivee",
                clock_time(conditions.rules.arrival_from),
            ),
            (
                "regles.heure_depart",
                clock_time(conditions.rules.departure_before),
            ),
            ("lien_suivi", self.tracking_link(booking)),
            ("lien_admin", self.admin_link(booking)),
        ];
        MessageValues {
            texts: texts.into_iter().collect(),
            payment_html: conditions.payment_instructions.get(lang).to_owned(),
        }
    }

    /// The guest's booking, from the token of their link.
    ///
    /// # Errors
    ///
    /// Unknown token, infrastructure.
    pub async fn track(&self, token: &str) -> Result<(Booking, bool), BookingError> {
        let booking = self
            .ports
            .bookings
            .by_token(token)
            .await?
            .ok_or(BookingError::NotFound)?;
        let changeable = self.arrival_time_changeable(&booking);
        Ok((booking, changeable))
    }

    fn arrival_time_changeable(&self, booking: &Booking) -> bool {
        booking.status != BookingStatus::Cancelled
            && paris_date(self.ports.clock.now()) < booking.arrival
    }

    /// The guest changes their expected arrival time; the owners get an email.
    ///
    /// # Errors
    ///
    /// Unknown token, cancelled booking, arrival day reached, invalid time, infrastructure.
    pub async fn change_arrival_time(&self, token: &str, minutes: u16) -> Result<(), BookingError> {
        let (booking, changeable) = self.track(token).await?;
        if booking.status == BookingStatus::Cancelled {
            return Err(BookingRuleError::Cancelled.into());
        }
        if !changeable {
            return Err(BookingRuleError::TooLateToChange.into());
        }
        if !valid_arrival_time(minutes, &booking.conditions.rules) {
            return Err(
                BookingRuleError::ArrivalTime(booking.conditions.rules.arrival_from).into(),
            );
        }
        if minutes == booking.arrival_time {
            return Ok(());
        }
        let now = self.ports.clock.now();
        let description = format!(
            "Heure d'arrivée changée par le voyageur : {} → {}",
            clock_time(booking.arrival_time),
            clock_time(minutes)
        );
        let settings = self.services.messaging.settings().await?;
        let text = format!(
            "{description}.\n\nRéservation {} ({} {}, du {} au {}) : {}",
            booking.reference,
            booking.guest.first_name,
            booking.guest.last_name,
            date_text(booking.arrival, Lang::Fr),
            date_text(booking.departure, Lang::Fr),
            self.admin_link(&booking)
        );
        let messages: Vec<OutgoingMessage> = settings
            .owner_emails
            .iter()
            .map(|address| OutgoingMessage {
                key: format!(
                    "booking:{}:arrival_time:{}:{address}",
                    booking.id,
                    now.unix_timestamp_nanos()
                ),
                recipient: address.clone(),
                content: Content::Email {
                    subject: format!("Heure d'arrivée changée : {}", booking.reference),
                    text: text.clone(),
                    html: None,
                    attachments: Vec::new(),
                },
            })
            .collect();
        self.ports
            .bookings
            .update_guest(
                booking.id,
                &booking.guest,
                minutes,
                &NewEvent {
                    description,
                    author: None,
                    at: now,
                },
                &messages,
            )
            .await?;
        Ok(())
    }

    /// The owners' list.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn list(&self, filter: BookingFilter) -> anyhow::Result<Vec<Booking>> {
        let cottage = self.ports.cottages.default_cottage().await?;
        let today = paris_date(self.ports.clock.now());
        self.ports.bookings.list(cottage.id, &filter, today).await
    }

    /// Detail of a booking for the owners.
    ///
    /// # Errors
    ///
    /// Unknown booking, infrastructure.
    pub async fn detail(&self, id: u64) -> Result<BookingDetail, BookingError> {
        let booking = self.find(id).await?;
        Ok(BookingDetail {
            payments: self.ports.bookings.payments(id).await?,
            events: self.ports.bookings.events(id).await?,
            messages: self.ports.bookings.messages(id).await?,
            booking,
        })
    }

    async fn find(&self, id: u64) -> Result<Booking, BookingError> {
        self.ports
            .bookings
            .find(id)
            .await?
            .ok_or(BookingError::NotFound)
    }

    /// The owners record a payment received; the status follows the paid total.
    ///
    /// # Errors
    ///
    /// Unknown or cancelled booking, invalid payment, concurrent change, infrastructure.
    pub async fn record_payment(
        &self,
        id: u64,
        input: PaymentInput,
        author: UserId,
    ) -> Result<BookingStatus, BookingError> {
        let booking = self.find(id).await?;
        if booking.status == BookingStatus::Cancelled {
            return Err(BookingRuleError::Cancelled.into());
        }
        let cottage = self.ports.cottages.default_cottage().await?;
        let methods = self.ports.payment_info.load(cottage.id).await?.methods;
        let method = input.method.trim().to_owned();
        let note = input.note.trim().to_owned();
        let mut errors = Vec::new();
        if input.amount == 0 {
            errors.push(BookingRuleError::AmountZero.to_string());
        }
        if !methods.contains(&method) {
            errors.push(BookingRuleError::UnknownMethod(method.clone()).to_string());
        }
        if note.chars().count() > MAX_NOTE {
            errors.push(BookingRuleError::NoteTooLong.to_string());
        }
        if !errors.is_empty() {
            return Err(BookingError::Invalid(errors));
        }
        let paid = booking.paid_total + input.amount;
        let status = BookingStatus::from_payments(
            paid,
            booking.conditions.deposit,
            booking.conditions.stay_total,
        );
        let now = self.ports.clock.now();
        let description = format!(
            "Paiement reçu : {} par {} le {} (réservation {})",
            money_text(input.amount, Lang::Fr),
            method,
            date_text(input.paid_on, Lang::Fr),
            status.label()
        );
        let recorded = self
            .ports
            .bookings
            .add_payment(
                id,
                &NewPayment {
                    amount: input.amount,
                    paid_on: input.paid_on,
                    method,
                    note,
                    author,
                    created_at: now,
                },
                booking.paid_total,
                status,
                &NewEvent {
                    description,
                    author: Some(author),
                    at: now,
                },
            )
            .await?;
        if recorded {
            Ok(status)
        } else {
            Err(BookingError::Changed)
        }
    }

    /// The owners cancel a booking: its nights are free at once (refunds are done offline).
    ///
    /// # Errors
    ///
    /// Unknown or already cancelled booking, missing reason, infrastructure.
    pub async fn cancel(&self, id: u64, reason: &str, author: UserId) -> Result<(), BookingError> {
        let reason = reason.trim();
        if reason.is_empty() || reason.chars().count() > MAX_NOTE {
            return Err(BookingRuleError::CancelReason.into());
        }
        self.find(id).await?;
        let cancelled = self
            .ports
            .bookings
            .cancel(
                id,
                reason,
                &NewEvent {
                    description: format!("Réservation annulée : {reason}"),
                    author: Some(author),
                    at: self.ports.clock.now(),
                },
            )
            .await?;
        if cancelled {
            Ok(())
        } else {
            Err(BookingRuleError::Cancelled.into())
        }
    }

    /// The owners correct the guest's details or expected arrival time.
    ///
    /// # Errors
    ///
    /// Unknown booking, invalid details, infrastructure.
    pub async fn update_guest(
        &self,
        id: u64,
        guest: Guest,
        arrival_time: u16,
        author: UserId,
    ) -> Result<(), BookingError> {
        let booking = self.find(id).await?;
        let mut errors = Vec::new();
        let guest = match guest.validated() {
            Ok(guest) => Some(guest),
            Err(guest_errors) => {
                errors.extend(guest_errors.iter().map(ToString::to_string));
                None
            }
        };
        if !valid_arrival_time(arrival_time, &booking.conditions.rules) {
            errors.push(
                BookingRuleError::ArrivalTime(booking.conditions.rules.arrival_from).to_string(),
            );
        }
        let (Some(guest), true) = (guest, errors.is_empty()) else {
            return Err(BookingError::Invalid(errors));
        };
        self.ports
            .bookings
            .update_guest(
                id,
                &guest,
                arrival_time,
                &NewEvent {
                    description: "Coordonnées du voyageur modifiées".to_owned(),
                    author: Some(author),
                    at: self.ports.clock.now(),
                },
                &[],
            )
            .await?;
        Ok(())
    }

    /// Alerts the owners, once, of every confirmed booking whose deposit is not received
    /// after the delay of the house rules; returns how many alerts were queued.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn alert_late_deposits(&self) -> anyhow::Result<usize> {
        let now = self.ports.clock.now();
        let cottage = self.ports.cottages.default_cottage().await?;
        let rules = self.services.rules.rules().await?;
        let delay = Duration::days(i64::from(rules.deposit_alert_days));
        let late: Vec<Booking> = self
            .ports
            .bookings
            .deposit_alert_candidates(cottage.id)
            .await?
            .into_iter()
            .filter(|booking| booking.deposit_expected() && booking.created_at + delay <= now)
            .collect();
        if late.is_empty() {
            return Ok(0);
        }
        let settings = self.services.messaging.settings().await?;
        let templates = self.services.templates.list().await?;
        let layout = Layout {
            cottage: cottage.name.clone(),
            site_url: self.settings.site_url.clone(),
        };
        let mut alerted = 0;
        for booking in late {
            let messages = self.compose(
                &booking,
                TemplateKind::DepositLate,
                &templates,
                &settings,
                &layout,
            )?;
            let event = NewEvent {
                description: "Alerte aux propriétaires : acompte non reçu".to_owned(),
                author: None,
                at: now,
            };
            if self
                .ports
                .bookings
                .mark_deposit_alert(booking.id, &event, &messages)
                .await?
            {
                alerted += 1;
            }
        }
        Ok(alerted)
    }

    /// Payment instructions, methods and document.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn payment_info(&self) -> anyhow::Result<PaymentInfo> {
        let cottage = self.ports.cottages.default_cottage().await?;
        self.ports.payment_info.load(cottage.id).await
    }

    /// Replaces the payment instructions (sanitised) and methods.
    ///
    /// # Errors
    ///
    /// Invalid methods, infrastructure.
    pub async fn update_payment_info(
        &self,
        instructions: Localized,
        methods: Vec<String>,
    ) -> Result<(), BookingError> {
        let mut cleaned: Vec<String> = Vec::new();
        let mut errors = Vec::new();
        for method in methods {
            let method = method.trim().to_owned();
            if method.is_empty() {
                continue;
            }
            if method.chars().count() > MAX_METHOD {
                errors.push(format!(
                    "mode de paiement : {MAX_METHOD} caractères au maximum"
                ));
            } else if !cleaned.contains(&method) {
                cleaned.push(method);
            }
        }
        if cleaned.is_empty() {
            errors.push("indiquez au moins un mode de paiement".to_owned());
        }
        if cleaned.len() > MAX_METHODS {
            errors.push(format!("{MAX_METHODS} modes de paiement au maximum"));
        }
        if !errors.is_empty() {
            return Err(BookingError::Invalid(errors));
        }
        let clean_html = |html: &str| {
            let html = sanitize_rich_text(html);
            if html_to_text(&html).trim().is_empty() {
                String::new()
            } else {
                html
            }
        };
        let cottage = self.ports.cottages.default_cottage().await?;
        let mut info = self.ports.payment_info.load(cottage.id).await?;
        info.instructions = Localized {
            fr: clean_html(&instructions.fr),
            en: clean_html(&instructions.en),
        };
        info.methods = cleaned;
        self.ports.payment_info.save(cottage.id, &info).await?;
        Ok(())
    }

    /// Stores the bank details document (PDF or image), replacing the previous one.
    ///
    /// # Errors
    ///
    /// Not a PDF nor an image, too large, infrastructure.
    pub async fn set_document(&self, name: &str, bytes: &[u8]) -> Result<(), BookingError> {
        if bytes.len() > MAX_DOCUMENT_BYTES {
            return Err(BookingError::Invalid(vec![format!(
                "document : {} Mo au maximum",
                MAX_DOCUMENT_BYTES / 1024 / 1024
            )]));
        }
        let Some((extension, content_type)) = document_kind(bytes) else {
            return Err(BookingError::Invalid(vec![
                "document : un PDF ou une image (JPEG, PNG, WebP)".to_owned(),
            ]));
        };
        let stem: String = name
            .rsplit_once('.')
            .map_or(name, |(stem, _)| stem)
            .chars()
            .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
            .take(100)
            .collect();
        let stem = if stem.trim().is_empty() {
            "RIB".to_owned()
        } else {
            stem.trim().to_owned()
        };
        let file = self.ports.documents.save(bytes, extension).await?;
        let cottage = self.ports.cottages.default_cottage().await?;
        let mut info = self.ports.payment_info.load(cottage.id).await?;
        let previous = info.document.replace(Document {
            name: format!("{stem}.{extension}"),
            content_type: content_type.to_owned(),
            file,
        });
        self.ports.payment_info.save(cottage.id, &info).await?;
        if let Some(previous) = previous {
            self.remove_file_later(previous.file);
        }
        Ok(())
    }

    /// Removes the bank details document.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn remove_document(&self) -> anyhow::Result<()> {
        let cottage = self.ports.cottages.default_cottage().await?;
        let mut info = self.ports.payment_info.load(cottage.id).await?;
        if let Some(previous) = info.document.take() {
            self.ports.payment_info.save(cottage.id, &info).await?;
            self.remove_file_later(previous.file);
        }
        Ok(())
    }

    /// The bank details document, for the owners to check it.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn document(&self) -> anyhow::Result<Option<(Document, Vec<u8>)>> {
        let info = self.payment_info().await?;
        let Some(document) = info.document else {
            return Ok(None);
        };
        Ok(self
            .ports
            .documents
            .read(&document.file)
            .await?
            .map(|bytes| (document, bytes)))
    }

    /// Queued emails may still attach an old document: it is kept a day before removal.
    fn remove_file_later(&self, file: String) {
        let documents = self.ports.documents.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(24 * 3600)).await;
            if let Err(error) = documents.delete(&file).await {
                tracing::warn!(%error, "cannot remove an old document");
            }
        });
    }
}

/// Copy of the quote, rules, payment instructions and terms in force, kept on the booking.
fn conditions_of(
    quoted: &QuoteResult,
    rules: HouseRules,
    payment_instructions: Localized,
    terms: TermsAcceptance,
) -> Conditions {
    let quote = &quoted.quote;
    Conditions {
        nights: quote.nights,
        nights_total: quote.nights_total,
        discount: quote.discount,
        promotion: quoted.promotion.clone(),
        cleaning_fee: quote.cleaning_fee,
        options: quoted
            .options
            .iter()
            .filter(|(quoted, _)| quoted.selected)
            .map(|(quoted, option)| BookedOption {
                option: option.clone(),
                price: quoted.price,
            })
            .collect(),
        options_total: quote.options_total,
        stay_total: quote.stay_total,
        deposit: quote.deposit,
        balance: quote.balance,
        balance_due: quote.balance_due,
        tourist_tax: quote.tourist_tax,
        rules,
        payment_instructions,
        terms,
    }
}

/// Extension and content type of a PDF or an image, from its first bytes.
#[must_use]
pub fn document_kind(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(b"%PDF-") {
        Some(("pdf", "application/pdf"))
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some(("png", "image/png"))
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(("jpg", "image/jpeg"))
    } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(("webp", "image/webp"))
    } else {
        None
    }
}

#[cfg(test)]
#[path = "bookings_tests.rs"]
mod tests;
