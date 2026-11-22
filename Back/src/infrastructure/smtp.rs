//! SMTP adapter of the [`Mailer`] port (F025, ADR-0019), with `lettre` over rustls.
//!
//! The server, port and sender come from the owners' settings at each sending; the password
//! only from the environment.

use std::time::Duration;

use lettre::message::{Mailbox, MultiPart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::application::health::BoxFuture;
use crate::application::messaging::{Email, Mailer};
use crate::config::Secret;
use crate::domain::messaging::{SendError, SmtpSecurity, SmtpSettings};

/// Longest wait for the server at each step.
const TIMEOUT: Duration = Duration::from_secs(20);

/// Sends through the SMTP server set by the owners.
#[derive(Debug, Clone)]
pub struct SmtpMailer {
    password: Option<Secret>,
}

impl SmtpMailer {
    #[must_use]
    pub fn new(password: Option<Secret>) -> Self {
        Self { password }
    }
}

fn mailbox(name: &str, address: &str) -> Option<Mailbox> {
    let address: Address = address.parse().ok()?;
    Some(Mailbox::new(
        (!name.is_empty()).then(|| name.to_owned()),
        address,
    ))
}

/// Explains a `lettre` error without its text (which may quote the server).
fn send_error(error: &lettre::transport::smtp::Error) -> SendError {
    if let Some(code) = error.status() {
        return match code.to_string().as_str() {
            "530" | "534" | "535" | "538" => SendError::Authentication,
            "550" | "551" | "553" => SendError::RecipientRefused,
            "421" => SendError::Unreachable,
            _ => SendError::Provider,
        };
    }
    if error.is_tls() {
        SendError::Tls
    } else if error.is_client() {
        // Mostly "no compatible authentication mechanism".
        SendError::Authentication
    } else if error.is_response() {
        SendError::Provider
    } else {
        SendError::Unreachable
    }
}

impl SmtpMailer {
    fn message(smtp: &SmtpSettings, email: &Email) -> Result<Message, SendError> {
        let from = mailbox(&smtp.from_name, &smtp.from_address).ok_or(SendError::SenderRefused)?;
        let to = mailbox("", &email.to).ok_or(SendError::RecipientRefused)?;
        let mut builder = Message::builder()
            .from(from)
            .to(to)
            .subject(email.subject.clone());
        if let Some(reply_to) = &smtp.reply_to {
            builder = builder.reply_to(mailbox("", reply_to).ok_or(SendError::SenderRefused)?);
        }
        let body = match &email.html {
            Some(html) => MultiPart::alternative_plain_html(email.text.clone(), html.clone()),
            None => MultiPart::mixed()
                .singlepart(lettre::message::SinglePart::plain(email.text.clone())),
        };
        let body = if email.attachments.is_empty() {
            body
        } else {
            let mut mixed = MultiPart::mixed().multipart(body);
            for attachment in &email.attachments {
                let content_type =
                    lettre::message::header::ContentType::parse(&attachment.content_type)
                        .map_err(|_| SendError::Attachment)?;
                mixed = mixed.singlepart(
                    lettre::message::Attachment::new(attachment.name.clone())
                        .body(attachment.bytes.clone(), content_type),
                );
            }
            mixed
        };
        builder.multipart(body).map_err(|_| SendError::Provider)
    }

    fn transport(
        &self,
        smtp: &SmtpSettings,
    ) -> Result<AsyncSmtpTransport<Tokio1Executor>, SendError> {
        let builder = match smtp.security {
            SmtpSecurity::Starttls => {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp.host)
                    .map_err(|_| SendError::Tls)?
            }
            SmtpSecurity::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp.host)
                .map_err(|_| SendError::Tls)?,
            SmtpSecurity::None => {
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp.host)
            }
        };
        let mut builder = builder.port(smtp.port).timeout(Some(TIMEOUT));
        if !smtp.username.is_empty() {
            let password = self
                .password
                .as_ref()
                .map(|password| password.expose().to_owned())
                .ok_or(SendError::Authentication)?;
            builder = builder.credentials(Credentials::new(smtp.username.clone(), password));
        }
        Ok(builder.build())
    }
}

impl Mailer for SmtpMailer {
    fn password_set(&self) -> bool {
        self.password.is_some()
    }

    fn send<'a>(
        &'a self,
        smtp: &'a SmtpSettings,
        email: &'a Email,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async move {
            if !smtp.configured() {
                return Err(SendError::NotConfigured);
            }
            let message = Self::message(smtp, email)?;
            let transport = self.transport(smtp)?;
            transport
                .send(message)
                .await
                .map(|_| ())
                .map_err(|error| send_error(&error))
        })
    }
}
