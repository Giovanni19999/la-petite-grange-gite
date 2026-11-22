//! Fake senders that keep the messages in memory (F025): tests only, never wired in
//! production.

use std::sync::{Arc, Mutex, PoisonError};

use crate::application::health::BoxFuture;
use crate::application::messaging::{Email, Mailer, SmsSender};
use crate::domain::messaging::{SendError, SmtpSettings};

/// An SMS kept by [`MemorySms`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentSms {
    pub sender: String,
    pub to: String,
    pub text: String,
}

#[derive(Debug)]
struct Outbox<T> {
    sent: Vec<T>,
    /// Error returned by the next sendings, if any.
    failure: Option<SendError>,
}

impl<T> Default for Outbox<T> {
    fn default() -> Self {
        Self {
            sent: Vec::new(),
            failure: None,
        }
    }
}

/// Keeps the emails; can be told to fail.
#[derive(Debug, Clone, Default)]
pub struct MemoryMailer {
    inner: Arc<Mutex<Outbox<Email>>>,
}

impl MemoryMailer {
    #[must_use]
    pub fn sent(&self) -> Vec<Email> {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .sent
            .clone()
    }

    /// Every next sending fails with `failure` (`None`: they succeed again).
    pub fn fail_with(&self, failure: Option<SendError>) {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .failure = failure;
    }
}

impl Mailer for MemoryMailer {
    fn password_set(&self) -> bool {
        true
    }

    fn send<'a>(
        &'a self,
        _smtp: &'a SmtpSettings,
        email: &'a Email,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async move {
            let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(failure) = inner.failure {
                return Err(failure);
            }
            inner.sent.push(email.clone());
            Ok(())
        })
    }
}

/// Keeps the SMS; can be told to fail.
#[derive(Debug, Clone, Default)]
pub struct MemorySms {
    inner: Arc<Mutex<Outbox<SentSms>>>,
}

impl MemorySms {
    #[must_use]
    pub fn sent(&self) -> Vec<SentSms> {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .sent
            .clone()
    }

    pub fn fail_with(&self, failure: Option<SendError>) {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .failure = failure;
    }
}

impl SmsSender for MemorySms {
    fn available(&self) -> bool {
        true
    }

    fn send<'a>(
        &'a self,
        sender: &'a str,
        to: &'a str,
        text: &'a str,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async move {
            let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(failure) = inner.failure {
                return Err(failure);
            }
            inner.sent.push(SentSms {
                sender: sender.to_owned(),
                to: to.to_owned(),
                text: text.to_owned(),
            });
            Ok(())
        })
    }
}
