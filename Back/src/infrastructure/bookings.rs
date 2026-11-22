//! Bookings in `MariaDB` (F007, ADR-0020): bookings, their payments, history and messages,
//! the nights they occupy, booking attempts, payment instructions, rental terms pages and
//! message templates.

use anyhow::Context;
use sqlx::{FromRow, MySqlPool};
use time::{Date, OffsetDateTime, PrimitiveDateTime, UtcOffset};

use crate::application::bookings::{
    Booking, BookingAttempts, BookingEvent, BookingFilter, BookingRepository, Compose,
    CreateOutcome, Document, NewBooking, NewEvent, NewPayment, Payment, PaymentInfo,
    PaymentInfoRepository, TermsPage, TermsPages,
};
use crate::application::calendar::{CottageId, Stay, StayCalendar};
use crate::application::health::BoxFuture;
use crate::application::messaging::QueuedMessage;
use crate::application::templates::{StoredTemplate, TemplateRepository};
use crate::domain::booking::{BookingStatus, Guest, PaymentMode, reference};
use crate::domain::calendar::DateRange;
use crate::domain::messaging::Channel;
use crate::domain::page::{Lang, Localized};
use crate::domain::templates::TemplateKind;
use crate::infrastructure::messaging::{booking_messages, insert_message};

fn to_db(value: OffsetDateTime) -> PrimitiveDateTime {
    let utc = value.to_offset(UtcOffset::UTC);
    PrimitiveDateTime::new(utc.date(), utc.time())
}

fn lang_code(lang: Lang) -> &'static str {
    match lang {
        Lang::Fr => "fr",
        Lang::En => "en",
    }
}

fn parse_lang(code: &str) -> anyhow::Result<Lang> {
    match code {
        "fr" => Ok(Lang::Fr),
        "en" => Ok(Lang::En),
        other => anyhow::bail!("unknown language stored: {other}"),
    }
}

/// Bookings and what belongs to them in `MariaDB`.
#[derive(Debug, Clone)]
pub struct MariaDbBookings {
    pool: MySqlPool,
}

impl MariaDbBookings {
    #[must_use]
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct BookingRow {
    id: u64,
    reference: String,
    token: String,
    status: String,
    payment_mode: String,
    lang: String,
    arrival: Date,
    departure: Date,
    adults: u8,
    children: u8,
    first_name: String,
    last_name: String,
    email: String,
    phone: String,
    street: String,
    postcode: String,
    town: String,
    country: String,
    arrival_time: u16,
    message: String,
    conditions: String,
    paid_total: u64,
    created_at: PrimitiveDateTime,
    cancelled_at: Option<PrimitiveDateTime>,
    cancel_reason: Option<String>,
    deposit_alert_sent_at: Option<PrimitiveDateTime>,
}

impl TryFrom<BookingRow> for Booking {
    type Error = anyhow::Error;

    fn try_from(row: BookingRow) -> anyhow::Result<Self> {
        Ok(Self {
            id: row.id,
            reference: row.reference,
            token: row.token,
            status: BookingStatus::parse(&row.status).context("unknown booking status stored")?,
            payment_mode: PaymentMode::parse(&row.payment_mode)
                .context("unknown payment mode stored")?,
            lang: parse_lang(&row.lang)?,
            arrival: row.arrival,
            departure: row.departure,
            adults: row.adults,
            children: row.children,
            guest: Guest {
                first_name: row.first_name,
                last_name: row.last_name,
                email: row.email,
                phone: row.phone,
                street: row.street,
                postcode: row.postcode,
                town: row.town,
                country: row.country,
            },
            arrival_time: row.arrival_time,
            message: row.message,
            conditions: serde_json::from_str(&row.conditions)
                .context("invalid booking conditions stored")?,
            paid_total: row.paid_total,
            created_at: row.created_at.assume_utc(),
            cancelled_at: row.cancelled_at.map(PrimitiveDateTime::assume_utc),
            cancel_reason: row.cancel_reason,
            deposit_alert_sent_at: row.deposit_alert_sent_at.map(PrimitiveDateTime::assume_utc),
        })
    }
}

fn bookings(rows: Vec<BookingRow>) -> anyhow::Result<Vec<Booking>> {
    rows.into_iter().map(Booking::try_from).collect()
}

#[derive(FromRow)]
struct PaymentRow {
    amount: u64,
    paid_on: Date,
    method: String,
    note: String,
    author: Option<String>,
    created_at: PrimitiveDateTime,
}

#[derive(FromRow)]
struct EventRow {
    description: String,
    author: Option<String>,
    at: PrimitiveDateTime,
}

async fn insert_event<'e>(
    executor: impl sqlx::MySqlExecutor<'e>,
    booking: u64,
    event: &NewEvent,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO booking_events (booking_id, description, author_id, at) VALUES (?, ?, ?, ?)",
    )
    .bind(booking)
    .bind(&event.description)
    .bind(event.author.map(|author| author.0))
    .bind(to_db(event.at))
    .execute(executor)
    .await
    .context("cannot record the booking history")?;
    Ok(())
}

/// Inserts the booking row; returns its id.
async fn insert_booking<'e>(
    executor: impl sqlx::MySqlExecutor<'e>,
    booking: &NewBooking,
    reference: &str,
    year: i32,
    number: u32,
) -> anyhow::Result<u64> {
    let stay = &booking.stay;
    let guest = &booking.guest;
    let conditions = &booking.conditions;
    let result = sqlx::query(
        "INSERT INTO bookings (cottage_id, reference, reference_year, reference_number, \
         token, status, payment_mode, lang, arrival, departure, adults, children, \
         first_name, last_name, email, phone, street, postcode, town, country, \
         arrival_time, message, conditions, stay_total, deposit, balance_due, \
         paid_total, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
         ?, ?, ?, 0, ?)",
    )
    .bind(booking.cottage.0)
    .bind(reference)
    .bind(year)
    .bind(number)
    .bind(&booking.token)
    .bind(booking.status.as_str())
    .bind(booking.payment_mode.as_str())
    .bind(lang_code(booking.lang))
    .bind(stay.arrival)
    .bind(stay.departure)
    .bind(stay.adults)
    .bind(stay.children)
    .bind(&guest.first_name)
    .bind(&guest.last_name)
    .bind(&guest.email)
    .bind(&guest.phone)
    .bind(&guest.street)
    .bind(&guest.postcode)
    .bind(&guest.town)
    .bind(&guest.country)
    .bind(booking.arrival_time)
    .bind(&booking.message)
    .bind(serde_json::to_string(conditions)?)
    .bind(conditions.stay_total)
    .bind(conditions.deposit)
    .bind(conditions.balance_due)
    .bind(to_db(booking.created_at))
    .execute(executor)
    .await
    .context("cannot store the booking")?;
    Ok(result.last_insert_id())
}

/// The booking just stored.
fn created_booking(booking: &NewBooking, id: u64, reference: String) -> Booking {
    let stay = &booking.stay;
    Booking {
        id,
        reference,
        token: booking.token.clone(),
        status: booking.status,
        payment_mode: booking.payment_mode,
        lang: booking.lang,
        arrival: stay.arrival,
        departure: stay.departure,
        adults: stay.adults,
        children: stay.children,
        guest: booking.guest.clone(),
        arrival_time: booking.arrival_time,
        message: booking.message.clone(),
        conditions: booking.conditions.clone(),
        paid_total: 0,
        created_at: booking.created_at,
        cancelled_at: None,
        cancel_reason: None,
        deposit_alert_sent_at: None,
    }
}

impl BookingRepository for MariaDbBookings {
    fn create<'a>(
        &'a self,
        booking: &'a NewBooking,
        compose: &'a Compose<'a>,
    ) -> BoxFuture<'a, anyhow::Result<CreateOutcome>> {
        Box::pin(async move {
            let stay = &booking.stay;
            let mut tx = self
                .pool
                .begin()
                .await
                .context("cannot start a transaction")?;
            // One booking at a time per cottage: the check below and the insert cannot race.
            sqlx::query("SELECT id FROM cottages WHERE id = ? FOR UPDATE")
                .bind(booking.cottage.0)
                .execute(&mut *tx)
                .await
                .context("cannot lock the cottage")?;
            let taken: i64 = sqlx::query_scalar(
                "SELECT (SELECT COUNT(*) FROM bookings WHERE cottage_id = ? \
                 AND status IN ('en_paiement', 'confirmee', 'acompte_recu', 'soldee') \
                 AND arrival < ? AND departure > ?) \
                 + (SELECT COUNT(*) FROM blocked_periods WHERE cottage_id = ? \
                 AND start_date < ? AND (end_date IS NULL OR end_date > ?))",
            )
            .bind(booking.cottage.0)
            .bind(stay.departure)
            .bind(stay.arrival)
            .bind(booking.cottage.0)
            .bind(stay.departure)
            .bind(stay.arrival)
            .fetch_one(&mut *tx)
            .await
            .context("cannot check the availability")?;
            if taken > 0 {
                return Ok(CreateOutcome::Unavailable);
            }
            let year = stay.arrival.year();
            let number: u64 = sqlx::query_scalar(
                "SELECT CAST(COALESCE(MAX(reference_number), 0) + 1 AS UNSIGNED) \
                 FROM bookings WHERE cottage_id = ? AND reference_year = ?",
            )
            .bind(booking.cottage.0)
            .bind(year)
            .fetch_one(&mut *tx)
            .await
            .context("cannot number the booking")?;
            let number = u32::try_from(number).context("too many bookings")?;
            let reference = reference(year, number);
            let id = insert_booking(&mut *tx, booking, &reference, year, number).await?;
            insert_event(
                &mut *tx,
                id,
                &NewEvent {
                    description: booking.event.clone(),
                    author: None,
                    at: booking.created_at,
                },
            )
            .await?;
            let created = created_booking(booking, id, reference);
            for message in compose(&created)? {
                insert_message(&mut *tx, &message, booking.created_at, Some(id)).await?;
            }
            tx.commit().await.context("cannot commit the booking")?;
            Ok(CreateOutcome::Created(Box::new(created)))
        })
    }

    fn find(&self, id: u64) -> BoxFuture<'_, anyhow::Result<Option<Booking>>> {
        Box::pin(async move {
            sqlx::query_as::<_, BookingRow>(
                "SELECT id, reference, token, status, payment_mode, lang, arrival, departure, \
                 adults, children, first_name, last_name, email, phone, street, postcode, town, \
                 country, arrival_time, message, conditions, paid_total, created_at, \
                 cancelled_at, cancel_reason, deposit_alert_sent_at FROM bookings WHERE id = ?",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("cannot read the booking")?
            .map(Booking::try_from)
            .transpose()
        })
    }

    fn by_token<'a>(&'a self, token: &'a str) -> BoxFuture<'a, anyhow::Result<Option<Booking>>> {
        Box::pin(async move {
            sqlx::query_as::<_, BookingRow>(
                "SELECT id, reference, token, status, payment_mode, lang, arrival, departure, \
                 adults, children, first_name, last_name, email, phone, street, postcode, town, \
                 country, arrival_time, message, conditions, paid_total, created_at, \
                 cancelled_at, cancel_reason, deposit_alert_sent_at FROM bookings \
                 WHERE token = ?",
            )
            .bind(token)
            .fetch_optional(&self.pool)
            .await
            .context("cannot read the booking")?
            .map(Booking::try_from)
            .transpose()
        })
    }

    fn list<'a>(
        &'a self,
        cottage: CottageId,
        filter: &'a BookingFilter,
        today: Date,
    ) -> BoxFuture<'a, anyhow::Result<Vec<Booking>>> {
        Box::pin(async move {
            let status = filter.status.map(BookingStatus::as_str);
            let rows: Vec<BookingRow> = sqlx::query_as(
                "SELECT id, reference, token, status, payment_mode, lang, arrival, departure, \
                 adults, children, first_name, last_name, email, phone, street, postcode, town, \
                 country, arrival_time, message, conditions, paid_total, created_at, \
                 cancelled_at, cancel_reason, deposit_alert_sent_at FROM bookings \
                 WHERE cottage_id = ? AND (? IS NULL OR status = ?) \
                 AND (? IS NULL OR departure > ?) AND (? IS NULL OR arrival < ?) \
                 ORDER BY departure < ? ASC, \
                 CASE WHEN departure >= ? THEN arrival END ASC, arrival DESC, id DESC",
            )
            .bind(cottage.0)
            .bind(status)
            .bind(status)
            .bind(filter.from)
            .bind(filter.from)
            .bind(filter.to)
            .bind(filter.to)
            .bind(today)
            .bind(today)
            .fetch_all(&self.pool)
            .await
            .context("cannot list the bookings")?;
            bookings(rows)
        })
    }

    fn payments(&self, id: u64) -> BoxFuture<'_, anyhow::Result<Vec<Payment>>> {
        Box::pin(async move {
            let rows: Vec<PaymentRow> = sqlx::query_as(
                "SELECT p.amount, p.paid_on, p.method, p.note, u.email AS author, p.created_at \
                 FROM booking_payments p LEFT JOIN users u ON u.id = p.author_id \
                 WHERE p.booking_id = ? ORDER BY p.paid_on, p.id",
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .context("cannot read the payments")?;
            Ok(rows
                .into_iter()
                .map(|row| Payment {
                    amount: row.amount,
                    paid_on: row.paid_on,
                    method: row.method,
                    note: row.note,
                    author: row.author,
                    created_at: row.created_at.assume_utc(),
                })
                .collect())
        })
    }

    fn events(&self, id: u64) -> BoxFuture<'_, anyhow::Result<Vec<BookingEvent>>> {
        Box::pin(async move {
            let rows: Vec<EventRow> = sqlx::query_as(
                "SELECT e.description, u.email AS author, e.at FROM booking_events e \
                 LEFT JOIN users u ON u.id = e.author_id WHERE e.booking_id = ? \
                 ORDER BY e.at DESC, e.id DESC",
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .context("cannot read the booking history")?;
            Ok(rows
                .into_iter()
                .map(|row| BookingEvent {
                    description: row.description,
                    author: row.author,
                    at: row.at.assume_utc(),
                })
                .collect())
        })
    }

    fn messages(&self, id: u64) -> BoxFuture<'_, anyhow::Result<Vec<QueuedMessage>>> {
        Box::pin(async move { booking_messages(&self.pool, id).await })
    }

    fn add_payment<'a>(
        &'a self,
        id: u64,
        payment: &'a NewPayment,
        paid_before: u64,
        status: BookingStatus,
        event: &'a NewEvent,
    ) -> BoxFuture<'a, anyhow::Result<bool>> {
        Box::pin(async move {
            let mut tx = self
                .pool
                .begin()
                .await
                .context("cannot start a transaction")?;
            let updated = sqlx::query(
                "UPDATE bookings SET paid_total = paid_total + ?, status = ? \
                 WHERE id = ? AND paid_total = ? AND status <> 'annulee'",
            )
            .bind(payment.amount)
            .bind(status.as_str())
            .bind(id)
            .bind(paid_before)
            .execute(&mut *tx)
            .await
            .context("cannot record the payment")?;
            if updated.rows_affected() != 1 {
                return Ok(false);
            }
            sqlx::query(
                "INSERT INTO booking_payments \
                 (booking_id, amount, paid_on, method, note, author_id, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(payment.amount)
            .bind(payment.paid_on)
            .bind(&payment.method)
            .bind(&payment.note)
            .bind(payment.author.0)
            .bind(to_db(payment.created_at))
            .execute(&mut *tx)
            .await
            .context("cannot record the payment")?;
            insert_event(&mut *tx, id, event).await?;
            tx.commit().await.context("cannot commit the payment")?;
            Ok(true)
        })
    }

    fn cancel<'a>(
        &'a self,
        id: u64,
        reason: &'a str,
        event: &'a NewEvent,
    ) -> BoxFuture<'a, anyhow::Result<bool>> {
        Box::pin(async move {
            let mut tx = self
                .pool
                .begin()
                .await
                .context("cannot start a transaction")?;
            let updated = sqlx::query(
                "UPDATE bookings SET status = 'annulee', cancelled_at = ?, cancel_reason = ? \
                 WHERE id = ? AND status <> 'annulee'",
            )
            .bind(to_db(event.at))
            .bind(reason)
            .bind(id)
            .execute(&mut *tx)
            .await
            .context("cannot cancel the booking")?;
            if updated.rows_affected() != 1 {
                return Ok(false);
            }
            insert_event(&mut *tx, id, event).await?;
            tx.commit()
                .await
                .context("cannot commit the cancellation")?;
            Ok(true)
        })
    }

    fn update_guest<'a>(
        &'a self,
        id: u64,
        guest: &'a Guest,
        arrival_time: u16,
        event: &'a NewEvent,
        messages: &'a [crate::domain::messaging::OutgoingMessage],
    ) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            let mut tx = self
                .pool
                .begin()
                .await
                .context("cannot start a transaction")?;
            sqlx::query(
                "UPDATE bookings SET first_name = ?, last_name = ?, email = ?, phone = ?, \
                 street = ?, postcode = ?, town = ?, country = ?, arrival_time = ? WHERE id = ?",
            )
            .bind(&guest.first_name)
            .bind(&guest.last_name)
            .bind(&guest.email)
            .bind(&guest.phone)
            .bind(&guest.street)
            .bind(&guest.postcode)
            .bind(&guest.town)
            .bind(&guest.country)
            .bind(arrival_time)
            .bind(id)
            .execute(&mut *tx)
            .await
            .context("cannot change the booking")?;
            insert_event(&mut *tx, id, event).await?;
            for message in messages {
                insert_message(&mut *tx, message, event.at, Some(id)).await?;
            }
            tx.commit().await.context("cannot commit the change")?;
            Ok(())
        })
    }

    fn deposit_alert_candidates(
        &self,
        cottage: CottageId,
    ) -> BoxFuture<'_, anyhow::Result<Vec<Booking>>> {
        Box::pin(async move {
            let rows: Vec<BookingRow> = sqlx::query_as(
                "SELECT id, reference, token, status, payment_mode, lang, arrival, departure, \
                 adults, children, first_name, last_name, email, phone, street, postcode, town, \
                 country, arrival_time, message, conditions, paid_total, created_at, \
                 cancelled_at, cancel_reason, deposit_alert_sent_at FROM bookings \
                 WHERE cottage_id = ? AND status = 'confirmee' AND paid_total < deposit \
                 AND deposit_alert_sent_at IS NULL ORDER BY created_at",
            )
            .bind(cottage.0)
            .fetch_all(&self.pool)
            .await
            .context("cannot read the bookings waiting for their deposit")?;
            bookings(rows)
        })
    }

    fn mark_deposit_alert<'a>(
        &'a self,
        id: u64,
        event: &'a NewEvent,
        messages: &'a [crate::domain::messaging::OutgoingMessage],
    ) -> BoxFuture<'a, anyhow::Result<bool>> {
        Box::pin(async move {
            let mut tx = self
                .pool
                .begin()
                .await
                .context("cannot start a transaction")?;
            let updated = sqlx::query(
                "UPDATE bookings SET deposit_alert_sent_at = ? \
                 WHERE id = ? AND deposit_alert_sent_at IS NULL",
            )
            .bind(to_db(event.at))
            .bind(id)
            .execute(&mut *tx)
            .await
            .context("cannot record the deposit alert")?;
            if updated.rows_affected() != 1 {
                return Ok(false);
            }
            insert_event(&mut *tx, id, event).await?;
            for message in messages {
                insert_message(&mut *tx, message, event.at, Some(id)).await?;
            }
            tx.commit()
                .await
                .context("cannot commit the deposit alert")?;
            Ok(true)
        })
    }
}

#[derive(FromRow)]
struct StayRow {
    reference: String,
    arrival: Date,
    departure: Date,
}

impl StayCalendar for MariaDbBookings {
    fn stays<'a>(
        &'a self,
        cottage: CottageId,
        range: &'a DateRange,
    ) -> BoxFuture<'a, anyhow::Result<Vec<Stay>>> {
        Box::pin(async move {
            let rows: Vec<StayRow> = sqlx::query_as(
                "SELECT reference, arrival, departure FROM bookings WHERE cottage_id = ? \
                 AND status IN ('en_paiement', 'confirmee', 'acompte_recu', 'soldee') \
                 AND departure > ? AND (? IS NULL OR arrival < ?) ORDER BY arrival",
            )
            .bind(cottage.0)
            .bind(range.start)
            .bind(range.end)
            .bind(range.end)
            .fetch_all(&self.pool)
            .await
            .context("cannot read the booked nights")?;
            Ok(rows
                .into_iter()
                .map(|row| Stay {
                    label: row.reference,
                    range: DateRange {
                        start: row.arrival,
                        end: Some(row.departure),
                    },
                })
                .collect())
        })
    }
}

impl BookingAttempts for MariaDbBookings {
    fn record<'a>(
        &'a self,
        ip: &'a str,
        now: OffsetDateTime,
        since: OffsetDateTime,
    ) -> BoxFuture<'a, anyhow::Result<u32>> {
        Box::pin(async move {
            sqlx::query("DELETE FROM booking_attempts WHERE at < ?")
                .bind(to_db(since))
                .execute(&self.pool)
                .await
                .context("cannot forget the old booking attempts")?;
            sqlx::query("INSERT INTO booking_attempts (ip, at) VALUES (?, ?)")
                .bind(ip)
                .bind(to_db(now))
                .execute(&self.pool)
                .await
                .context("cannot record the booking attempt")?;
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM booking_attempts WHERE ip = ? AND at >= ?",
            )
            .bind(ip)
            .bind(to_db(since))
            .fetch_one(&self.pool)
            .await
            .context("cannot count the booking attempts")?;
            Ok(u32::try_from(count).unwrap_or(u32::MAX))
        })
    }
}

#[derive(FromRow)]
struct PaymentInfoRow {
    instructions_fr: String,
    instructions_en: String,
    methods: String,
    document_name: Option<String>,
    document_type: Option<String>,
    document_file: Option<String>,
}

impl PaymentInfoRepository for MariaDbBookings {
    fn load(&self, cottage: CottageId) -> BoxFuture<'_, anyhow::Result<PaymentInfo>> {
        Box::pin(async move {
            let row: PaymentInfoRow = sqlx::query_as(
                "SELECT instructions_fr, instructions_en, methods, document_name, \
                 document_type, document_file FROM payment_settings WHERE cottage_id = ?",
            )
            .bind(cottage.0)
            .fetch_one(&self.pool)
            .await
            .context("cannot read the payment instructions")?;
            Ok(PaymentInfo {
                instructions: Localized {
                    fr: row.instructions_fr,
                    en: row.instructions_en,
                },
                methods: serde_json::from_str(&row.methods)
                    .context("invalid payment methods stored")?,
                document: match (row.document_name, row.document_type, row.document_file) {
                    (Some(name), Some(content_type), Some(file)) => Some(Document {
                        name,
                        content_type,
                        file,
                    }),
                    _ => None,
                },
            })
        })
    }

    fn save<'a>(
        &'a self,
        cottage: CottageId,
        info: &'a PaymentInfo,
    ) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            let document = info.document.as_ref();
            sqlx::query(
                "UPDATE payment_settings SET instructions_fr = ?, instructions_en = ?, \
                 methods = ?, document_name = ?, document_type = ?, document_file = ? \
                 WHERE cottage_id = ?",
            )
            .bind(&info.instructions.fr)
            .bind(&info.instructions.en)
            .bind(serde_json::to_string(&info.methods)?)
            .bind(document.map(|document| document.name.as_str()))
            .bind(document.map(|document| document.content_type.as_str()))
            .bind(document.map(|document| document.file.as_str()))
            .bind(cottage.0)
            .execute(&self.pool)
            .await
            .context("cannot save the payment instructions")?;
            Ok(())
        })
    }
}

#[derive(FromRow)]
struct TermsRow {
    id: u64,
    slug_fr: Option<String>,
    slug_en: Option<String>,
    title_fr: Option<String>,
    title_en: Option<String>,
    version: Option<u32>,
}

impl TermsPages for MariaDbBookings {
    fn published(&self, page: u64) -> BoxFuture<'_, anyhow::Result<Option<TermsPage>>> {
        Box::pin(async move {
            let row: Option<TermsRow> = sqlx::query_as(
                "SELECT p.id, p.published_slug_fr AS slug_fr, p.published_slug_en AS slug_en, \
                 JSON_UNQUOTE(JSON_EXTRACT(p.published, '$.settings.title.fr')) AS title_fr, \
                 JSON_UNQUOTE(JSON_EXTRACT(p.published, '$.settings.title.en')) AS title_en, \
                 (SELECT MAX(v.number) FROM page_versions v WHERE v.page_id = p.id) AS version \
                 FROM pages p WHERE p.id = ? AND p.published IS NOT NULL",
            )
            .bind(page)
            .fetch_optional(&self.pool)
            .await
            .context("cannot read the rental terms page")?;
            Ok(row.and_then(|row| {
                Some(TermsPage {
                    id: row.id,
                    slug: Localized {
                        fr: row.slug_fr?,
                        en: row.slug_en.unwrap_or_default(),
                    },
                    title: Localized {
                        fr: row.title_fr.unwrap_or_default(),
                        en: row.title_en.unwrap_or_default(),
                    },
                    version: row.version.unwrap_or(1),
                })
            }))
        })
    }
}

/// Message templates in `MariaDB` (ADR-0021).
#[derive(Debug, Clone)]
pub struct MariaDbTemplates {
    pool: MySqlPool,
}

impl MariaDbTemplates {
    #[must_use]
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct TemplateRow {
    kind: String,
    channel: String,
    lang: String,
    subject: String,
    body: String,
    default_subject: String,
    default_body: String,
}

impl TemplateRepository for MariaDbTemplates {
    fn list(&self) -> BoxFuture<'_, anyhow::Result<Vec<StoredTemplate>>> {
        Box::pin(async move {
            let rows: Vec<TemplateRow> = sqlx::query_as(
                "SELECT kind, channel, lang, subject, body, default_subject, default_body \
                 FROM message_templates ORDER BY kind, channel, lang",
            )
            .fetch_all(&self.pool)
            .await
            .context("cannot read the message templates")?;
            rows.into_iter()
                .filter_map(|row| {
                    // A template of a later feature is ignored by this version.
                    let kind = TemplateKind::parse(&row.kind)?;
                    Some((kind, row))
                })
                .map(|(kind, row)| {
                    Ok(StoredTemplate {
                        kind,
                        channel: Channel::parse(&row.channel)
                            .context("unknown template channel stored")?,
                        lang: parse_lang(&row.lang)?,
                        subject: row.subject,
                        body: row.body,
                        default_subject: row.default_subject,
                        default_body: row.default_body,
                    })
                })
                .collect()
        })
    }

    fn save<'a>(
        &'a self,
        kind: TemplateKind,
        channel: Channel,
        lang: Lang,
        subject: &'a str,
        body: &'a str,
    ) -> BoxFuture<'a, anyhow::Result<bool>> {
        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE message_templates SET subject = ?, body = ? \
                 WHERE kind = ? AND channel = ? AND lang = ?",
            )
            .bind(subject)
            .bind(body)
            .bind(kind.as_str())
            .bind(channel.as_str())
            .bind(lang_code(lang))
            .execute(&self.pool)
            .await
            .context("cannot save the message template")?;
            let found: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM message_templates WHERE kind = ? AND channel = ? AND lang = ?",
            )
            .bind(kind.as_str())
            .bind(channel.as_str())
            .bind(lang_code(lang))
            .fetch_one(&self.pool)
            .await
            .context("cannot read the message template")?;
            Ok(result.rows_affected() == 1 || found == 1)
        })
    }

    fn restore(
        &self,
        kind: TemplateKind,
        channel: Channel,
        lang: Lang,
    ) -> BoxFuture<'_, anyhow::Result<bool>> {
        Box::pin(async move {
            sqlx::query(
                "UPDATE message_templates SET subject = default_subject, body = default_body \
                 WHERE kind = ? AND channel = ? AND lang = ?",
            )
            .bind(kind.as_str())
            .bind(channel.as_str())
            .bind(lang_code(lang))
            .execute(&self.pool)
            .await
            .context("cannot restore the message template")?;
            let found: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM message_templates WHERE kind = ? AND channel = ? AND lang = ?",
            )
            .bind(kind.as_str())
            .bind(channel.as_str())
            .bind(lang_code(lang))
            .fetch_one(&self.pool)
            .await
            .context("cannot read the message template")?;
            Ok(found == 1)
        })
    }
}
