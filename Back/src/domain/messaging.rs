//! Email and SMS sending (F025, ADR-0019): settings, messages of the queue, retries.
//!
//! Pure rules: no provider, no database. Secrets are not part of the domain.

use serde::{Deserialize, Serialize};
use time::Duration;

/// Channel of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Email,
    Sms,
}

impl Channel {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Sms => "sms",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "email" => Some(Self::Email),
            "sms" => Some(Self::Sms),
            _ => None,
        }
    }
}

/// Security of the connection to the SMTP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmtpSecurity {
    /// Plain connection upgraded with `STARTTLS` (usually port 587).
    Starttls,
    /// TLS from the start (usually port 465).
    Tls,
    /// No encryption: a local mail catcher only.
    None,
}

impl SmtpSecurity {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Starttls => "starttls",
            Self::Tls => "tls",
            Self::None => "none",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "starttls" => Some(Self::Starttls),
            "tls" => Some(Self::Tls),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

/// SMTP server and sender, as the owners set them (the password comes from the environment).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmtpSettings {
    /// Empty: emails are not configured yet.
    pub host: String,
    pub port: u16,
    pub security: SmtpSecurity,
    /// Empty: no authentication.
    pub username: String,
    pub from_address: String,
    pub from_name: String,
    pub reply_to: Option<String>,
}

impl SmtpSettings {
    /// A host and a sender address are set.
    #[must_use]
    pub fn configured(&self) -> bool {
        !self.host.is_empty() && !self.from_address.is_empty()
    }
}

/// Settings of the sending, editable on the "Envoi des messages" screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessagingSettings {
    pub smtp: SmtpSettings,
    pub sms_enabled: bool,
    /// Name shown on the phone (3 to 11 letters or digits).
    pub sms_sender: String,
    /// Notification addresses of the owners.
    pub owner_emails: Vec<String>,
    /// Notification phones of the owners, E.164.
    pub owner_phones: Vec<String>,
    /// Attempts before a message is failed (1 to 20).
    pub max_attempts: u8,
    /// Delay before the first retry, doubled each time (1 to 1440 minutes).
    pub first_retry_minutes: u16,
}

/// A mistake in the settings, in French for the owners.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    #[error("serveur SMTP : un nom d'hôte sans espace")]
    Host,
    #[error("port SMTP : un nombre entre 1 et 65535")]
    Port,
    #[error("adresse d'expédition invalide : {0}")]
    FromAddress(String),
    #[error("le nom d'expéditeur fait au plus 80 caractères")]
    FromName,
    #[error("adresse de réponse invalide : {0}")]
    ReplyTo(String),
    #[error("nom d'expéditeur des SMS : de 3 à 11 lettres ou chiffres, sans espace ni accent")]
    SmsSender,
    #[error("adresse de notification invalide : {0}")]
    OwnerEmail(String),
    #[error(
        "numéro de notification invalide : {0} (format international, par exemple +33612345678)"
    )]
    OwnerPhone(String),
    #[error("nombre d'essais : entre 1 et 20")]
    Attempts,
    #[error("délai avant un nouvel essai : entre 1 et 1440 minutes")]
    RetryDelay,
}

/// Longest stored email address.
const MAX_ADDRESS: usize = 254;

/// A plausible email address: one `@`, a dot in the domain, no space.
#[must_use]
pub fn valid_email(address: &str) -> bool {
    let Some((local, domain)) = address.split_once('@') else {
        return false;
    };
    address.len() <= MAX_ADDRESS
        && !local.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !address.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// An E.164 phone number: `+`, then 8 to 15 digits, not starting with 0.
#[must_use]
pub fn valid_phone(number: &str) -> bool {
    number.strip_prefix('+').is_some_and(|digits| {
        (8..=15).contains(&digits.len())
            && !digits.starts_with('0')
            && digits.bytes().all(|b| b.is_ascii_digit())
    })
}

/// A phone number typed with spaces, dots or dashes, kept as digits.
fn clean_phone(number: &str) -> String {
    number
        .chars()
        .filter(|c| !matches!(c, ' ' | '.' | '-'))
        .collect()
}

impl MessagingSettings {
    /// The settings with trimmed values, or every mistake found.
    ///
    /// # Errors
    ///
    /// Each rule broken.
    pub fn validated(self) -> Result<Self, Vec<SettingsError>> {
        let mut errors = Vec::new();
        let smtp = SmtpSettings {
            host: self.smtp.host.trim().to_owned(),
            username: self.smtp.username.trim().to_owned(),
            from_address: self.smtp.from_address.trim().to_owned(),
            from_name: self.smtp.from_name.trim().to_owned(),
            reply_to: self
                .smtp
                .reply_to
                .map(|address| address.trim().to_owned())
                .filter(|address| !address.is_empty()),
            ..self.smtp
        };
        if smtp.host.chars().any(char::is_whitespace) || smtp.host.len() > 255 {
            errors.push(SettingsError::Host);
        }
        if smtp.port == 0 {
            errors.push(SettingsError::Port);
        }
        if !smtp.from_address.is_empty() && !valid_email(&smtp.from_address) {
            errors.push(SettingsError::FromAddress(smtp.from_address.clone()));
        }
        if smtp.from_name.chars().count() > 80 {
            errors.push(SettingsError::FromName);
        }
        if let Some(reply_to) = &smtp.reply_to
            && !valid_email(reply_to)
        {
            errors.push(SettingsError::ReplyTo(reply_to.clone()));
        }
        let sms_sender = self.sms_sender.trim().to_owned();
        if !(3..=11).contains(&sms_sender.len())
            || !sms_sender.bytes().all(|b| b.is_ascii_alphanumeric())
        {
            errors.push(SettingsError::SmsSender);
        }
        let owner_emails: Vec<String> = self
            .owner_emails
            .iter()
            .map(|address| address.trim().to_owned())
            .filter(|address| !address.is_empty())
            .collect();
        for address in &owner_emails {
            if !valid_email(address) {
                errors.push(SettingsError::OwnerEmail(address.clone()));
            }
        }
        let owner_phones: Vec<String> = self
            .owner_phones
            .iter()
            .map(|number| clean_phone(number))
            .filter(|number| !number.is_empty())
            .collect();
        for number in &owner_phones {
            if !valid_phone(number) {
                errors.push(SettingsError::OwnerPhone(number.clone()));
            }
        }
        if !(1..=20).contains(&self.max_attempts) {
            errors.push(SettingsError::Attempts);
        }
        if !(1..=1440).contains(&self.first_retry_minutes) {
            errors.push(SettingsError::RetryDelay);
        }
        if errors.is_empty() {
            Ok(Self {
                smtp,
                sms_sender,
                owner_emails,
                owner_phones,
                ..self
            })
        } else {
            Err(errors)
        }
    }

    /// Delay before the attempt following `attempts_done` failed attempts: the first
    /// delay, doubled each time (capped at 30 days).
    #[must_use]
    pub fn retry_delay(&self, attempts_done: u8) -> Duration {
        let doublings = u32::from(attempts_done.saturating_sub(1)).min(16);
        let minutes = i64::from(self.first_retry_minutes) * (1_i64 << doublings);
        Duration::minutes(minutes.min(30 * 24 * 60))
    }
}

/// Content of a message to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    Email {
        subject: String,
        text: String,
        html: Option<String>,
    },
    Sms {
        text: String,
    },
}

impl Content {
    #[must_use]
    pub fn channel(&self) -> Channel {
        match self {
            Self::Email { .. } => Channel::Email,
            Self::Sms { .. } => Channel::Sms,
        }
    }
}

/// A message to put in the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingMessage {
    /// Unique key: a message with a key already queued is never queued again.
    pub key: String,
    /// Email address or E.164 phone number.
    pub recipient: String,
    pub content: Content,
}

/// Status of a message of the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    Pending,
    Sending,
    Sent,
    Failed,
    /// Being sent when the API stopped: never resent automatically.
    ToCheck,
    /// SMS off or no SMS provider.
    Disabled,
}

impl MessageStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Sending => "sending",
            Self::Sent => "sent",
            Self::Failed => "failed",
            Self::ToCheck => "to_check",
            Self::Disabled => "disabled",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "sending" => Some(Self::Sending),
            "sent" => Some(Self::Sent),
            "failed" => Some(Self::Failed),
            "to_check" => Some(Self::ToCheck),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }
}

/// Why a message could not be sent, explained in French; never contains a secret nor the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum SendError {
    #[error("envoi non configuré")]
    NotConfigured,
    #[error("serveur injoignable")]
    Unreachable,
    #[error("connexion sécurisée impossible avec le serveur")]
    Tls,
    #[error("authentification refusée par le serveur")]
    Authentication,
    #[error("destinataire refusé par le serveur")]
    RecipientRefused,
    #[error("expéditeur refusé par le serveur")]
    SenderRefused,
    #[error("numéro de téléphone refusé")]
    InvalidNumber,
    #[error("crédit SMS épuisé")]
    NoCredit,
    #[error("erreur du serveur ou du prestataire")]
    Provider,
}

impl SendError {
    /// Stored name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotConfigured => "not_configured",
            Self::Unreachable => "unreachable",
            Self::Tls => "tls",
            Self::Authentication => "authentication",
            Self::RecipientRefused => "recipient_refused",
            Self::SenderRefused => "sender_refused",
            Self::InvalidNumber => "invalid_number",
            Self::NoCredit => "no_credit",
            Self::Provider => "provider",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        [
            Self::NotConfigured,
            Self::Unreachable,
            Self::Tls,
            Self::Authentication,
            Self::RecipientRefused,
            Self::SenderRefused,
            Self::InvalidNumber,
            Self::NoCredit,
            Self::Provider,
        ]
        .into_iter()
        .find(|error| error.as_str() == value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> MessagingSettings {
        MessagingSettings {
            smtp: SmtpSettings {
                host: "smtp.example.test".to_owned(),
                port: 587,
                security: SmtpSecurity::Starttls,
                username: "site".to_owned(),
                from_address: "site@example.test".to_owned(),
                from_name: "La Petite Grange".to_owned(),
                reply_to: None,
            },
            sms_enabled: true,
            sms_sender: "LaGrange".to_owned(),
            owner_emails: vec!["proprio@example.test".to_owned()],
            owner_phones: vec!["+33612345678".to_owned()],
            max_attempts: 5,
            first_retry_minutes: 5,
        }
    }

    #[test]
    fn f025_s07_invalid_settings_are_refused_with_every_reason() {
        let errors = MessagingSettings {
            sms_sender: "PetiteGrange".to_owned(),
            owner_phones: vec!["0612".to_owned()],
            owner_emails: vec!["proprio@".to_owned()],
            max_attempts: 0,
            ..settings()
        }
        .validated()
        .unwrap_err();
        assert_eq!(
            errors,
            vec![
                SettingsError::SmsSender,
                SettingsError::OwnerEmail("proprio@".to_owned()),
                SettingsError::OwnerPhone("0612".to_owned()),
                SettingsError::Attempts,
            ]
        );
    }

    #[test]
    fn valid_settings_are_trimmed_and_phones_cleaned() {
        let settings = MessagingSettings {
            owner_phones: vec!["+33 6 12 34 56 78".to_owned(), " ".to_owned()],
            owner_emails: vec![" proprio@example.test ".to_owned()],
            smtp: SmtpSettings {
                reply_to: Some("  ".to_owned()),
                ..settings().smtp
            },
            ..settings()
        }
        .validated()
        .unwrap();
        assert_eq!(settings.owner_phones, vec!["+33612345678"]);
        assert_eq!(settings.owner_emails, vec!["proprio@example.test"]);
        assert_eq!(settings.smtp.reply_to, None);
    }

    #[test]
    fn emails_and_phones() {
        assert!(valid_email("a.b@example.test"));
        assert!(!valid_email("a b@example.test"));
        assert!(!valid_email("a@example"));
        assert!(!valid_email("@example.test"));
        assert!(valid_phone("+33612345678"));
        assert!(!valid_phone("0612345678"));
        assert!(!valid_phone("+0612345678"));
        assert!(!valid_phone("+33 612"));
    }

    #[test]
    fn f025_s02_retry_delay_doubles() {
        let settings = settings();
        assert_eq!(settings.retry_delay(1), Duration::minutes(5));
        assert_eq!(settings.retry_delay(2), Duration::minutes(10));
        assert_eq!(settings.retry_delay(3), Duration::minutes(20));
        assert_eq!(settings.retry_delay(200), Duration::days(30));
    }

    #[test]
    fn stored_names_round_trip() {
        for error in [SendError::Authentication, SendError::NoCredit] {
            assert_eq!(SendError::parse(error.as_str()), Some(error));
        }
        for status in [MessageStatus::ToCheck, MessageStatus::Disabled] {
            assert_eq!(MessageStatus::parse(status.as_str()), Some(status));
        }
    }
}
