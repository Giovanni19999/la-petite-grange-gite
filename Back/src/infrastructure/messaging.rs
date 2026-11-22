//! Sending settings and queue in `MariaDB` (F025, ADR-0019).

use anyhow::Context;
use sqlx::{FromRow, MySqlExecutor, MySqlPool};
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};

use crate::application::health::BoxFuture;
use crate::application::messaging::{MessageStore, Outcome, QueuedMessage};
use crate::domain::messaging::{
    Channel, Content, MessageStatus, MessagingSettings, OutgoingMessage, SendError, SmtpSecurity,
    SmtpSettings,
};

/// Settings and queue in `MariaDB`.
#[derive(Debug, Clone)]
pub struct MariaDbMessages {
    pool: MySqlPool,
}

impl MariaDbMessages {
    #[must_use]
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn to_db(value: OffsetDateTime) -> PrimitiveDateTime {
    let utc = value.to_offset(UtcOffset::UTC);
    PrimitiveDateTime::new(utc.date(), utc.time())
}

#[derive(FromRow)]
struct SettingsRow {
    smtp_host: String,
    smtp_port: u16,
    smtp_security: String,
    smtp_username: String,
    from_address: String,
    from_name: String,
    reply_to: Option<String>,
    sms_enabled: bool,
    sms_sender: String,
    owner_emails: String,
    owner_phones: String,
    max_attempts: u8,
    first_retry_minutes: u16,
}

impl TryFrom<SettingsRow> for MessagingSettings {
    type Error = anyhow::Error;

    fn try_from(row: SettingsRow) -> anyhow::Result<Self> {
        Ok(Self {
            smtp: SmtpSettings {
                host: row.smtp_host,
                port: row.smtp_port,
                security: SmtpSecurity::parse(&row.smtp_security)
                    .context("unknown SMTP security stored")?,
                username: row.smtp_username,
                from_address: row.from_address,
                from_name: row.from_name,
                reply_to: row.reply_to,
            },
            sms_enabled: row.sms_enabled,
            sms_sender: row.sms_sender,
            owner_emails: serde_json::from_str(&row.owner_emails)
                .context("invalid notification addresses stored")?,
            owner_phones: serde_json::from_str(&row.owner_phones)
                .context("invalid notification phones stored")?,
            max_attempts: row.max_attempts,
            first_retry_minutes: row.first_retry_minutes,
        })
    }
}

#[derive(FromRow)]
struct MessageRow {
    id: u64,
    message_key: String,
    channel: String,
    recipient: String,
    subject: String,
    body_text: String,
    body_html: Option<String>,
    attachments: Option<String>,
    status: String,
    attempts: u8,
    last_error: Option<String>,
    created_at: PrimitiveDateTime,
    sent_at: Option<PrimitiveDateTime>,
}

impl TryFrom<MessageRow> for QueuedMessage {
    type Error = anyhow::Error;

    fn try_from(row: MessageRow) -> anyhow::Result<Self> {
        let content = match Channel::parse(&row.channel).context("unknown channel stored")? {
            Channel::Email => Content::Email {
                subject: row.subject,
                text: row.body_text,
                html: row.body_html,
                attachments: row
                    .attachments
                    .as_deref()
                    .map(serde_json::from_str)
                    .transpose()
                    .context("invalid attachments stored")?
                    .unwrap_or_default(),
            },
            Channel::Sms => Content::Sms {
                text: row.body_text,
            },
        };
        Ok(Self {
            id: row.id,
            key: row.message_key,
            recipient: row.recipient,
            content,
            status: MessageStatus::parse(&row.status).context("unknown message status stored")?,
            attempts: row.attempts,
            last_error: row.last_error.as_deref().and_then(SendError::parse),
            created_at: row.created_at.assume_utc(),
            sent_at: row.sent_at.map(PrimitiveDateTime::assume_utc),
        })
    }
}

/// Queues a message unless its key is already queued; true if added. Used by other
/// repositories inside their own transaction (ADR-0019), with the booking it belongs to.
///
/// # Errors
///
/// Database errors.
pub(crate) async fn insert_message<'e>(
    executor: impl MySqlExecutor<'e>,
    message: &OutgoingMessage,
    now: OffsetDateTime,
    booking: Option<u64>,
) -> anyhow::Result<bool> {
    let (subject, text, html, attachments) = match &message.content {
        Content::Email {
            subject,
            text,
            html,
            attachments,
        } => (
            subject.as_str(),
            text.as_str(),
            html.as_deref(),
            (!attachments.is_empty())
                .then(|| serde_json::to_string(attachments))
                .transpose()?,
        ),
        Content::Sms { text } => ("", text.as_str(), None, None),
    };
    let result = sqlx::query(
        "INSERT IGNORE INTO outgoing_messages (message_key, channel, recipient, subject, \
         body_text, body_html, attachments, booking_id, status, attempts, next_attempt_at, \
         created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'pending', 0, ?, ?)",
    )
    .bind(&message.key)
    .bind(message.content.channel().as_str())
    .bind(&message.recipient)
    .bind(subject)
    .bind(text)
    .bind(html)
    .bind(attachments)
    .bind(booking)
    .bind(to_db(now))
    .bind(to_db(now))
    .execute(executor)
    .await
    .context("cannot queue the message")?;
    Ok(result.rows_affected() == 1)
}

/// Messages of a booking, latest first (F007).
///
/// # Errors
///
/// Database errors.
pub(crate) async fn booking_messages<'e>(
    executor: impl MySqlExecutor<'e>,
    booking: u64,
) -> anyhow::Result<Vec<QueuedMessage>> {
    let rows: Vec<MessageRow> = sqlx::query_as(
        "SELECT id, message_key, channel, recipient, subject, body_text, body_html, \
         attachments, status, attempts, last_error, created_at, sent_at \
         FROM outgoing_messages WHERE booking_id = ? ORDER BY created_at DESC, id DESC",
    )
    .bind(booking)
    .fetch_all(executor)
    .await
    .context("cannot read the messages of the booking")?;
    rows.into_iter().map(QueuedMessage::try_from).collect()
}

impl MessageStore for MariaDbMessages {
    fn load_settings(&self) -> BoxFuture<'_, anyhow::Result<MessagingSettings>> {
        Box::pin(async move {
            let row: SettingsRow = sqlx::query_as(
                "SELECT smtp_host, smtp_port, smtp_security, smtp_username, from_address, \
                 from_name, reply_to, sms_enabled, sms_sender, owner_emails, owner_phones, \
                 max_attempts, first_retry_minutes FROM messaging_settings WHERE id = 1",
            )
            .fetch_one(&self.pool)
            .await
            .context("cannot read the sending settings")?;
            MessagingSettings::try_from(row)
        })
    }

    fn save_settings<'a>(
        &'a self,
        settings: &'a MessagingSettings,
    ) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            let smtp = &settings.smtp;
            sqlx::query(
                "UPDATE messaging_settings SET smtp_host = ?, smtp_port = ?, smtp_security = ?, \
                 smtp_username = ?, from_address = ?, from_name = ?, reply_to = ?, \
                 sms_enabled = ?, sms_sender = ?, owner_emails = ?, owner_phones = ?, \
                 max_attempts = ?, first_retry_minutes = ? WHERE id = 1",
            )
            .bind(&smtp.host)
            .bind(smtp.port)
            .bind(smtp.security.as_str())
            .bind(&smtp.username)
            .bind(&smtp.from_address)
            .bind(&smtp.from_name)
            .bind(&smtp.reply_to)
            .bind(settings.sms_enabled)
            .bind(&settings.sms_sender)
            .bind(serde_json::to_string(&settings.owner_emails)?)
            .bind(serde_json::to_string(&settings.owner_phones)?)
            .bind(settings.max_attempts)
            .bind(settings.first_retry_minutes)
            .execute(&self.pool)
            .await
            .context("cannot save the sending settings")?;
            Ok(())
        })
    }

    fn enqueue<'a>(
        &'a self,
        message: &'a OutgoingMessage,
        now: OffsetDateTime,
    ) -> BoxFuture<'a, anyhow::Result<bool>> {
        Box::pin(async move { insert_message(&self.pool, message, now, None).await })
    }

    fn claim_due(
        &self,
        now: OffsetDateTime,
    ) -> BoxFuture<'_, anyhow::Result<Option<QueuedMessage>>> {
        Box::pin(async move {
            let mut tx = self
                .pool
                .begin()
                .await
                .context("cannot start a transaction")?;
            let row: Option<MessageRow> = sqlx::query_as(
                "SELECT id, message_key, channel, recipient, subject, body_text, body_html, \
                 attachments, status, attempts, last_error, created_at, sent_at \
                 FROM outgoing_messages \
                 WHERE status = 'pending' AND next_attempt_at <= ? \
                 ORDER BY next_attempt_at, id LIMIT 1 FOR UPDATE SKIP LOCKED",
            )
            .bind(to_db(now))
            .fetch_optional(&mut *tx)
            .await
            .context("cannot read the queue")?;
            let Some(row) = row else {
                return Ok(None);
            };
            sqlx::query("UPDATE outgoing_messages SET status = 'sending' WHERE id = ?")
                .bind(row.id)
                .execute(&mut *tx)
                .await
                .context("cannot take the message")?;
            tx.commit().await.context("cannot take the message")?;
            let mut message = QueuedMessage::try_from(row)?;
            message.status = MessageStatus::Sending;
            Ok(Some(message))
        })
    }

    fn finish(&self, id: u64, outcome: Outcome) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            let query = match outcome {
                Outcome::Sent { at } => sqlx::query(
                    "UPDATE outgoing_messages SET status = 'sent', attempts = attempts + 1, \
                     sent_at = ?, last_error = NULL WHERE id = ?",
                )
                .bind(to_db(at)),
                Outcome::Retry {
                    attempts,
                    next_at,
                    error,
                } => sqlx::query(
                    "UPDATE outgoing_messages SET status = 'pending', attempts = ?, \
                     next_attempt_at = ?, last_error = ? WHERE id = ?",
                )
                .bind(attempts)
                .bind(to_db(next_at))
                .bind(error.as_str()),
                Outcome::Failed { attempts, error } => sqlx::query(
                    "UPDATE outgoing_messages SET status = 'failed', attempts = ?, \
                     last_error = ? WHERE id = ?",
                )
                .bind(attempts)
                .bind(error.as_str()),
                Outcome::Disabled => {
                    sqlx::query("UPDATE outgoing_messages SET status = 'disabled' WHERE id = ?")
                }
            };
            query
                .bind(id)
                .execute(&self.pool)
                .await
                .context("cannot record the sending")?;
            Ok(())
        })
    }

    fn recover_interrupted(&self) -> BoxFuture<'_, anyhow::Result<u64>> {
        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE outgoing_messages SET status = 'to_check' WHERE status = 'sending'",
            )
            .execute(&self.pool)
            .await
            .context("cannot recover the interrupted messages")?;
            Ok(result.rows_affected())
        })
    }

    fn recent(&self, limit: u32) -> BoxFuture<'_, anyhow::Result<Vec<QueuedMessage>>> {
        Box::pin(async move {
            let rows: Vec<MessageRow> = sqlx::query_as(
                "SELECT id, message_key, channel, recipient, subject, body_text, body_html, \
                 attachments, status, attempts, last_error, created_at, sent_at \
                 FROM outgoing_messages \
                 ORDER BY status IN ('failed', 'to_check') DESC, created_at DESC, id DESC \
                 LIMIT ?",
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .context("cannot read the messages")?;
            rows.into_iter().map(QueuedMessage::try_from).collect()
        })
    }

    fn resend(&self, id: u64, now: OffsetDateTime) -> BoxFuture<'_, anyhow::Result<bool>> {
        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE outgoing_messages SET status = 'pending', attempts = 0, \
                 next_attempt_at = ? \
                 WHERE id = ? AND status IN ('failed', 'to_check', 'disabled')",
            )
            .bind(to_db(now))
            .bind(id)
            .execute(&self.pool)
            .await
            .context("cannot send the message again")?;
            Ok(result.rows_affected() == 1)
        })
    }

    fn problems(&self) -> BoxFuture<'_, anyhow::Result<u64>> {
        Box::pin(async move {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM outgoing_messages WHERE status IN ('failed', 'to_check')",
            )
            .fetch_one(&self.pool)
            .await
            .context("cannot count the failed messages")?;
            Ok(u64::try_from(count).unwrap_or(0))
        })
    }
}
