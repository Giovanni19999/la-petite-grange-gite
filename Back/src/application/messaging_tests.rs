//! Unit tests of the sending use cases (F025), with an in-memory queue and fake senders.

use std::sync::{Arc, Mutex, PoisonError};

use time::macros::datetime;
use time::{Duration, OffsetDateTime};

use super::*;
use crate::domain::messaging::{SmtpSecurity, SmtpSettings};
use crate::infrastructure::memory_senders::{MemoryMailer, MemorySms};

/// A clock the test moves forward.
#[derive(Default)]
struct TestClock(Mutex<Duration>);

impl TestClock {
    fn advance(&self, by: Duration) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) += by;
    }
}

impl Clock for TestClock {
    fn now(&self) -> OffsetDateTime {
        datetime!(2027-03-01 10:00 UTC) + *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Default)]
struct State {
    settings: Option<MessagingSettings>,
    messages: Vec<(QueuedMessage, OffsetDateTime)>,
}

/// In-memory [`MessageStore`].
#[derive(Default)]
struct MemoryStore(Mutex<State>);

impl MemoryStore {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn statuses(&self) -> Vec<(MessageStatus, u8)> {
        self.state()
            .messages
            .iter()
            .map(|(message, _)| (message.status, message.attempts))
            .collect()
    }
}

impl MessageStore for MemoryStore {
    fn load_settings(&self) -> BoxFuture<'_, anyhow::Result<MessagingSettings>> {
        Box::pin(async move { Ok(self.state().settings.clone().unwrap_or_else(settings)) })
    }

    fn save_settings<'a>(
        &'a self,
        settings: &'a MessagingSettings,
    ) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            self.state().settings = Some(settings.clone());
            Ok(())
        })
    }

    fn enqueue<'a>(
        &'a self,
        message: &'a OutgoingMessage,
        now: OffsetDateTime,
    ) -> BoxFuture<'a, anyhow::Result<bool>> {
        Box::pin(async move {
            let mut state = self.state();
            if state
                .messages
                .iter()
                .any(|(queued, _)| queued.key == message.key)
            {
                return Ok(false);
            }
            let id = state.messages.len() as u64 + 1;
            state.messages.push((
                QueuedMessage {
                    id,
                    key: message.key.clone(),
                    recipient: message.recipient.clone(),
                    content: message.content.clone(),
                    status: MessageStatus::Pending,
                    attempts: 0,
                    last_error: None,
                    created_at: now,
                    sent_at: None,
                },
                now,
            ));
            Ok(true)
        })
    }

    fn claim_due(
        &self,
        now: OffsetDateTime,
    ) -> BoxFuture<'_, anyhow::Result<Option<QueuedMessage>>> {
        Box::pin(async move {
            let mut state = self.state();
            let due = state
                .messages
                .iter_mut()
                .find(|(message, at)| message.status == MessageStatus::Pending && *at <= now);
            Ok(due.map(|(message, _)| {
                message.status = MessageStatus::Sending;
                message.clone()
            }))
        })
    }

    fn finish(&self, id: u64, outcome: Outcome) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            let mut state = self.state();
            let (message, next) = state
                .messages
                .iter_mut()
                .find(|(message, _)| message.id == id)
                .expect("known message");
            match outcome {
                Outcome::Sent { at } => {
                    message.status = MessageStatus::Sent;
                    message.attempts += 1;
                    message.sent_at = Some(at);
                }
                Outcome::Retry {
                    attempts,
                    next_at,
                    error,
                } => {
                    message.status = MessageStatus::Pending;
                    message.attempts = attempts;
                    message.last_error = Some(error);
                    *next = next_at;
                }
                Outcome::Failed { attempts, error } => {
                    message.status = MessageStatus::Failed;
                    message.attempts = attempts;
                    message.last_error = Some(error);
                }
                Outcome::Disabled => message.status = MessageStatus::Disabled,
            }
            Ok(())
        })
    }

    fn recover_interrupted(&self) -> BoxFuture<'_, anyhow::Result<u64>> {
        Box::pin(async move {
            let mut count = 0;
            for (message, _) in &mut self.state().messages {
                if message.status == MessageStatus::Sending {
                    message.status = MessageStatus::ToCheck;
                    count += 1;
                }
            }
            Ok(count)
        })
    }

    fn recent(&self, _limit: u32) -> BoxFuture<'_, anyhow::Result<Vec<QueuedMessage>>> {
        Box::pin(async move {
            Ok(self
                .state()
                .messages
                .iter()
                .map(|(message, _)| message.clone())
                .collect())
        })
    }

    fn resend(&self, id: u64, now: OffsetDateTime) -> BoxFuture<'_, anyhow::Result<bool>> {
        Box::pin(async move {
            let mut state = self.state();
            let Some((message, next)) = state.messages.iter_mut().find(|(message, _)| {
                message.id == id
                    && matches!(
                        message.status,
                        MessageStatus::Failed | MessageStatus::ToCheck | MessageStatus::Disabled
                    )
            }) else {
                return Ok(false);
            };
            message.status = MessageStatus::Pending;
            message.attempts = 0;
            *next = now;
            Ok(true)
        })
    }

    fn problems(&self) -> BoxFuture<'_, anyhow::Result<u64>> {
        Box::pin(async move {
            Ok(self
                .state()
                .messages
                .iter()
                .filter(|(message, _)| {
                    matches!(
                        message.status,
                        MessageStatus::Failed | MessageStatus::ToCheck
                    )
                })
                .count() as u64)
        })
    }
}

fn settings() -> MessagingSettings {
    MessagingSettings {
        smtp: SmtpSettings {
            host: "smtp.example.test".to_owned(),
            port: 587,
            security: SmtpSecurity::Starttls,
            username: String::new(),
            from_address: "site@example.test".to_owned(),
            from_name: "Gîte".to_owned(),
            reply_to: None,
        },
        sms_enabled: true,
        sms_sender: "LaGrange".to_owned(),
        owner_emails: vec!["proprio@example.test".to_owned()],
        owner_phones: vec!["+33612345678".to_owned()],
        max_attempts: 3,
        first_retry_minutes: 5,
    }
}

struct Fixture {
    service: MessagingService,
    store: Arc<MemoryStore>,
    mailer: MemoryMailer,
    sms: MemorySms,
    clock: Arc<TestClock>,
}

fn fixture() -> Fixture {
    let store = Arc::new(MemoryStore::default());
    let mailer = MemoryMailer::default();
    let sms = MemorySms::default();
    let clock = Arc::new(TestClock::default());
    Fixture {
        service: MessagingService::new(
            store.clone(),
            Arc::new(mailer.clone()),
            Arc::new(sms.clone()),
            clock.clone(),
        ),
        store,
        mailer,
        sms,
        clock,
    }
}

fn email(key: &str) -> OutgoingMessage {
    OutgoingMessage {
        key: key.to_owned(),
        recipient: "guest@example.test".to_owned(),
        content: Content::Email {
            subject: "Réservation confirmée".to_owned(),
            text: "Bonjour".to_owned(),
            html: Some("<p>Bonjour</p>".to_owned()),
        },
    }
}

fn sms(key: &str) -> OutgoingMessage {
    OutgoingMessage {
        key: key.to_owned(),
        recipient: "+33612345678".to_owned(),
        content: Content::Sms {
            text: "Nouvelle réservation".to_owned(),
        },
    }
}

#[tokio::test]
async fn f025_s01_queued_email_is_sent_with_both_versions() {
    let f = fixture();
    assert!(f.service.queue(&email("booking:1:guest")).await.unwrap());
    assert_eq!(f.service.process_due().await.unwrap(), 1);
    let sent = f.mailer.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "guest@example.test");
    assert_eq!(sent[0].html.as_deref(), Some("<p>Bonjour</p>"));
    assert_eq!(f.store.statuses(), vec![(MessageStatus::Sent, 1)]);
}

#[tokio::test]
async fn f025_s02_failures_are_retried_with_a_doubling_delay_then_failed() {
    let f = fixture();
    f.mailer.fail_with(Some(SendError::Unreachable));
    f.service.queue(&email("booking:1:guest")).await.unwrap();

    f.service.process_due().await.unwrap();
    assert_eq!(f.store.statuses(), vec![(MessageStatus::Pending, 1)]);
    // Not due before 5 minutes.
    f.clock.advance(Duration::minutes(4));
    assert_eq!(f.service.process_due().await.unwrap(), 0);
    f.clock.advance(Duration::minutes(1));
    assert_eq!(f.service.process_due().await.unwrap(), 1);
    // Then 10 minutes.
    f.clock.advance(Duration::minutes(9));
    assert_eq!(f.service.process_due().await.unwrap(), 0);
    f.clock.advance(Duration::minutes(1));
    f.service.process_due().await.unwrap();
    assert_eq!(f.store.statuses(), vec![(MessageStatus::Failed, 3)]);
    assert_eq!(f.service.problems().await.unwrap(), 1);

    // Sent again by the owner once the server answers.
    f.mailer.fail_with(None);
    f.service.resend(1).await.unwrap();
    f.service.process_due().await.unwrap();
    assert_eq!(f.store.statuses(), vec![(MessageStatus::Sent, 1)]);
    assert!(matches!(
        f.service.resend(1).await,
        Err(MessagingError::NotFound)
    ));
}

#[tokio::test]
async fn f025_s05_sms_sent_with_the_sender_name_or_disabled() {
    let f = fixture();
    f.service.queue(&sms("booking:1:owners:sms")).await.unwrap();
    f.service.process_due().await.unwrap();
    let sent = f.sms.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].sender, "LaGrange");
    assert_eq!(sent[0].to, "+33612345678");

    f.service
        .update_settings(MessagingSettings {
            sms_enabled: false,
            ..settings()
        })
        .await
        .unwrap();
    f.service.queue(&sms("booking:2:owners:sms")).await.unwrap();
    f.service.process_due().await.unwrap();
    assert_eq!(f.sms.sent().len(), 1);
    assert_eq!(f.store.statuses()[1], (MessageStatus::Disabled, 0));
}

#[tokio::test]
async fn f025_s06_never_queued_nor_sent_twice() {
    let f = fixture();
    assert!(f.service.queue(&email("booking:1:guest")).await.unwrap());
    assert!(!f.service.queue(&email("booking:1:guest")).await.unwrap());
    assert_eq!(f.store.statuses().len(), 1);

    // Taken by a round, then the API stops before the result is recorded.
    f.store.claim_due(f.clock.now()).await.unwrap().unwrap();
    assert_eq!(f.service.recover_interrupted().await.unwrap(), 1);
    assert_eq!(f.service.process_due().await.unwrap(), 0);
    assert_eq!(f.mailer.sent().len(), 0);
    assert_eq!(f.store.statuses(), vec![(MessageStatus::ToCheck, 0)]);
}

#[tokio::test]
async fn f025_s03_test_messages_report_each_recipient() {
    let f = fixture();
    f.mailer.fail_with(Some(SendError::Authentication));
    let results = f.service.test_email().await.unwrap();
    assert_eq!(
        results,
        vec![TestResult {
            recipient: "proprio@example.test".to_owned(),
            error: Some(SendError::Authentication),
        }]
    );
    assert_eq!(
        SendError::Authentication.to_string(),
        "authentification refusée par le serveur"
    );
    // Test messages never go through the queue.
    assert_eq!(f.store.statuses().len(), 0);

    let results = f.service.test_sms().await.unwrap();
    assert_eq!(results[0].error, None);

    f.service
        .update_settings(MessagingSettings {
            owner_phones: vec![],
            ..settings()
        })
        .await
        .unwrap();
    assert!(matches!(
        f.service.test_sms().await,
        Err(MessagingError::NoRecipient)
    ));
}

#[tokio::test]
async fn f025_s07_invalid_settings_are_not_saved() {
    let f = fixture();
    let result = f
        .service
        .update_settings(MessagingSettings {
            sms_sender: "PetiteGrange".to_owned(),
            ..settings()
        })
        .await;
    assert!(matches!(result, Err(MessagingError::Invalid(_))));
    assert_eq!(f.service.settings().await.unwrap(), settings());
}
