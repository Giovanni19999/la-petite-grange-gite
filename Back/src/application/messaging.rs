//! Email and SMS sending use cases (F025, ADR-0019): settings, the sending queue processed
//! by a background task, test messages.
//!
//! Other features only call [`MessagingService::queue`]; providers sit behind [`Mailer`]
//! and [`SmsSender`].

use std::fmt;
use std::sync::Arc;

use time::OffsetDateTime;

use crate::application::auth::Clock;
use crate::application::health::BoxFuture;
use crate::domain::messaging::{
    Content, MessageStatus, MessagingSettings, OutgoingMessage, SendError, SettingsError,
    SmtpSettings,
};

/// Port: sends one email through the SMTP server of the settings.
pub trait Mailer: Send + Sync {
    /// The SMTP password is set in the environment.
    fn password_set(&self) -> bool;
    fn send<'a>(
        &'a self,
        smtp: &'a SmtpSettings,
        email: &'a Email,
    ) -> BoxFuture<'a, Result<(), SendError>>;
}

/// An email ready to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email {
    pub to: String,
    pub subject: String,
    pub text: String,
    pub html: Option<String>,
}

/// Port: sends one SMS.
pub trait SmsSender: Send + Sync {
    /// A provider is configured (`SMS_PROVIDER` and its key).
    fn available(&self) -> bool;
    fn send<'a>(
        &'a self,
        sender: &'a str,
        to: &'a str,
        text: &'a str,
    ) -> BoxFuture<'a, Result<(), SendError>>;
}

/// A message of the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedMessage {
    pub id: u64,
    pub key: String,
    pub recipient: String,
    pub content: Content,
    pub status: MessageStatus,
    pub attempts: u8,
    pub last_error: Option<SendError>,
    pub created_at: OffsetDateTime,
    pub sent_at: Option<OffsetDateTime>,
}

/// What became of an attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Sent {
        at: OffsetDateTime,
    },
    Retry {
        attempts: u8,
        next_at: OffsetDateTime,
        error: SendError,
    },
    Failed {
        attempts: u8,
        error: SendError,
    },
    Disabled,
}

/// Port: settings and queue in the database.
pub trait MessageStore: Send + Sync {
    fn load_settings(&self) -> BoxFuture<'_, anyhow::Result<MessagingSettings>>;
    fn save_settings<'a>(
        &'a self,
        settings: &'a MessagingSettings,
    ) -> BoxFuture<'a, anyhow::Result<()>>;
    /// Adds the message unless its key is already in the queue; true if added.
    fn enqueue<'a>(
        &'a self,
        message: &'a OutgoingMessage,
        now: OffsetDateTime,
    ) -> BoxFuture<'a, anyhow::Result<bool>>;
    /// Takes the next due pending message and marks it `sending`, in one step.
    fn claim_due(
        &self,
        now: OffsetDateTime,
    ) -> BoxFuture<'_, anyhow::Result<Option<QueuedMessage>>>;
    fn finish(&self, id: u64, outcome: Outcome) -> BoxFuture<'_, anyhow::Result<()>>;
    /// Marks every `sending` message `to_check`; returns how many.
    fn recover_interrupted(&self) -> BoxFuture<'_, anyhow::Result<u64>>;
    /// Failed and to-check messages first, then the latest.
    fn recent(&self, limit: u32) -> BoxFuture<'_, anyhow::Result<Vec<QueuedMessage>>>;
    /// Puts a failed, to-check or disabled message back in the queue, attempts reset;
    /// false if there is no such message.
    fn resend(&self, id: u64, now: OffsetDateTime) -> BoxFuture<'_, anyhow::Result<bool>>;
    /// Failed and to-check messages.
    fn problems(&self) -> BoxFuture<'_, anyhow::Result<u64>>;
}

/// What the screen shows about the configuration done outside the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SenderStatus {
    pub smtp_password_set: bool,
    pub sms_provider: bool,
}

/// Result of a test message for one recipient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResult {
    pub recipient: String,
    pub error: Option<SendError>,
}

/// Why a use case is refused.
#[derive(Debug, thiserror::Error)]
pub enum MessagingError {
    #[error("réglages invalides")]
    Invalid(Vec<SettingsError>),
    #[error("message introuvable")]
    NotFound,
    #[error("aucun destinataire de notification n'est réglé")]
    NoRecipient,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// Texts of the test messages.
const TEST_SUBJECT: &str = "Test d'envoi du site";
const TEST_TEXT: &str = "Ceci est un message de test du site du gîte : l'envoi fonctionne.";

/// Most messages sent in one round of the background task.
const ROUND_LIMIT: usize = 50;

/// Email and SMS sending use cases.
#[derive(Clone)]
pub struct MessagingService {
    store: Arc<dyn MessageStore>,
    mailer: Arc<dyn Mailer>,
    sms: Arc<dyn SmsSender>,
    clock: Arc<dyn Clock>,
}

impl fmt::Debug for MessagingService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MessagingService").finish_non_exhaustive()
    }
}

impl MessagingService {
    #[must_use]
    pub fn new(
        store: Arc<dyn MessageStore>,
        mailer: Arc<dyn Mailer>,
        sms: Arc<dyn SmsSender>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            store,
            mailer,
            sms,
            clock,
        }
    }

    /// Current settings.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn settings(&self) -> anyhow::Result<MessagingSettings> {
        self.store.load_settings().await
    }

    /// Replaces the settings.
    ///
    /// # Errors
    ///
    /// Invalid settings, infrastructure.
    pub async fn update_settings(&self, settings: MessagingSettings) -> Result<(), MessagingError> {
        let settings = settings.validated().map_err(MessagingError::Invalid)?;
        self.store.save_settings(&settings).await?;
        Ok(())
    }

    #[must_use]
    pub fn senders(&self) -> SenderStatus {
        SenderStatus {
            smtp_password_set: self.mailer.password_set(),
            sms_provider: self.sms.available(),
        }
    }

    /// Queues a message; false if a message with the same key is already queued.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn queue(&self, message: &OutgoingMessage) -> anyhow::Result<bool> {
        self.store.enqueue(message, self.clock.now()).await
    }

    /// At startup: messages left `sending` are marked `to_check`, never resent.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn recover_interrupted(&self) -> anyhow::Result<u64> {
        self.store.recover_interrupted().await
    }

    /// One round of the background task: sends the due messages; returns how many were tried.
    ///
    /// # Errors
    ///
    /// Infrastructure errors (provider errors are recorded on the message).
    pub async fn process_due(&self) -> anyhow::Result<usize> {
        let settings = self.store.load_settings().await?;
        let mut tried = 0;
        while tried < ROUND_LIMIT {
            let Some(message) = self.store.claim_due(self.clock.now()).await? else {
                break;
            };
            tried += 1;
            let outcome = match self.deliver(&settings, &message).await {
                None => Outcome::Disabled,
                Some(Ok(())) => Outcome::Sent {
                    at: self.clock.now(),
                },
                Some(Err(error)) => {
                    let attempts = message.attempts.saturating_add(1);
                    if attempts >= settings.max_attempts {
                        Outcome::Failed { attempts, error }
                    } else {
                        Outcome::Retry {
                            attempts,
                            next_at: self.clock.now() + settings.retry_delay(attempts),
                            error,
                        }
                    }
                }
            };
            tracing::info!(
                message = message.id,
                channel = message.content.channel().as_str(),
                outcome = ?outcome_kind(outcome),
                "message processed"
            );
            self.store.finish(message.id, outcome).await?;
        }
        Ok(tried)
    }

    /// Sends one message; `None` when SMS are off.
    async fn deliver(
        &self,
        settings: &MessagingSettings,
        message: &QueuedMessage,
    ) -> Option<Result<(), SendError>> {
        match &message.content {
            Content::Email {
                subject,
                text,
                html,
            } => Some(
                self.mailer
                    .send(
                        &settings.smtp,
                        &Email {
                            to: message.recipient.clone(),
                            subject: subject.clone(),
                            text: text.clone(),
                            html: html.clone(),
                        },
                    )
                    .await,
            ),
            Content::Sms { text } => {
                if !settings.sms_enabled || !self.sms.available() {
                    return None;
                }
                Some(
                    self.sms
                        .send(&settings.sms_sender, &message.recipient, text)
                        .await,
                )
            }
        }
    }

    /// Latest messages, problems first.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn recent(&self, limit: u32) -> anyhow::Result<Vec<QueuedMessage>> {
        self.store.recent(limit).await
    }

    /// Number of failed and to-check messages, for the admin home.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn problems(&self) -> anyhow::Result<u64> {
        self.store.problems().await
    }

    /// Puts a failed or interrupted message back in the queue, on the owner's request.
    ///
    /// # Errors
    ///
    /// Unknown message (or not failed), infrastructure.
    pub async fn resend(&self, id: u64) -> Result<(), MessagingError> {
        if self.store.resend(id, self.clock.now()).await? {
            Ok(())
        } else {
            Err(MessagingError::NotFound)
        }
    }

    /// Sends a test email at once (outside the queue) to every notification address.
    ///
    /// # Errors
    ///
    /// No notification address, infrastructure.
    pub async fn test_email(&self) -> Result<Vec<TestResult>, MessagingError> {
        let settings = self.store.load_settings().await?;
        if settings.owner_emails.is_empty() {
            return Err(MessagingError::NoRecipient);
        }
        let mut results = Vec::new();
        for address in &settings.owner_emails {
            let email = Email {
                to: address.clone(),
                subject: TEST_SUBJECT.to_owned(),
                text: TEST_TEXT.to_owned(),
                html: Some(format!("<p>{TEST_TEXT}</p>")),
            };
            results.push(TestResult {
                recipient: address.clone(),
                error: self.mailer.send(&settings.smtp, &email).await.err(),
            });
        }
        Ok(results)
    }

    /// Sends a test SMS at once to every notification phone (even if SMS are off).
    ///
    /// # Errors
    ///
    /// No notification phone, infrastructure.
    pub async fn test_sms(&self) -> Result<Vec<TestResult>, MessagingError> {
        let settings = self.store.load_settings().await?;
        if settings.owner_phones.is_empty() {
            return Err(MessagingError::NoRecipient);
        }
        let mut results = Vec::new();
        for number in &settings.owner_phones {
            let error = if self.sms.available() {
                self.sms
                    .send(&settings.sms_sender, number, TEST_TEXT)
                    .await
                    .err()
            } else {
                Some(SendError::NotConfigured)
            };
            results.push(TestResult {
                recipient: number.clone(),
                error,
            });
        }
        Ok(results)
    }
}

/// Outcome for the logs, without any content.
fn outcome_kind(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Sent { .. } => "sent",
        Outcome::Retry { error, .. } | Outcome::Failed { error, .. } => error.as_str(),
        Outcome::Disabled => "disabled",
    }
}

#[cfg(test)]
#[path = "messaging_tests.rs"]
mod tests;
