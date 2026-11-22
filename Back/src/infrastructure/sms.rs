//! SMS adapters of the [`SmsSender`] port (F025, ADR-0019): Brevo transactional SMS, or none.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::application::health::BoxFuture;
use crate::application::messaging::SmsSender;
use crate::config::Secret;
use crate::domain::messaging::SendError;

/// Longest wait for Brevo.
const TIMEOUT: Duration = Duration::from_secs(20);

/// No SMS provider (`SMS_PROVIDER=none`): SMS are not sent.
#[derive(Debug, Clone, Copy)]
pub struct NoSms;

impl SmsSender for NoSms {
    fn available(&self) -> bool {
        false
    }

    fn send<'a>(
        &'a self,
        _sender: &'a str,
        _to: &'a str,
        _text: &'a str,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async { Err(SendError::NotConfigured) })
    }
}

/// Brevo transactional SMS (`POST /v3/transactionalSMS/send`).
#[derive(Debug, Clone)]
pub struct BrevoSms {
    client: reqwest::Client,
    base_url: String,
    api_key: Secret,
}

impl BrevoSms {
    /// `base_url` is `https://api.brevo.com` except in tests.
    ///
    /// # Errors
    ///
    /// The HTTP client cannot be built.
    pub fn new(base_url: &str, api_key: Secret) -> anyhow::Result<Self> {
        let roots = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
        let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .context("cannot set up TLS")?
        .with_root_certificates(roots)
        .with_no_client_auth();
        let client = reqwest::Client::builder()
            .tls_backend_preconfigured(tls)
            .timeout(TIMEOUT)
            .build()
            .context("cannot build the HTTP client")?;
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_owned(),
            api_key,
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SmsRequest<'a> {
    sender: &'a str,
    /// Country code and number, without `+`.
    recipient: &'a str,
    content: &'a str,
    #[serde(rename = "type")]
    kind: &'static str,
    unicode_enabled: bool,
}

#[derive(Deserialize, Default)]
struct BrevoError {
    #[serde(default)]
    code: String,
    #[serde(default)]
    message: String,
}

/// Explains a refusal of Brevo (its message is not kept: it may quote the request).
fn refusal(status: reqwest::StatusCode, body: &BrevoError) -> SendError {
    match status.as_u16() {
        401 | 403 => SendError::Authentication,
        402 => SendError::NoCredit,
        400 if body.code == "not_enough_credits" => SendError::NoCredit,
        400 if body.message.to_lowercase().contains("sender") => SendError::SenderRefused,
        400 => SendError::InvalidNumber,
        _ => SendError::Provider,
    }
}

impl SmsSender for BrevoSms {
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
            let request = SmsRequest {
                sender,
                recipient: to.trim_start_matches('+'),
                content: text,
                kind: "transactional",
                unicode_enabled: !text.is_ascii(),
            };
            let response = self
                .client
                .post(format!("{}/v3/transactionalSMS/send", self.base_url))
                .header("api-key", self.api_key.expose())
                .json(&request)
                .send()
                .await
                .map_err(|_| SendError::Unreachable)?;
            let status = response.status();
            if status.is_success() {
                return Ok(());
            }
            let body = response.json::<BrevoError>().await.unwrap_or_default();
            Err(refusal(status, &body))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusals_are_explained() {
        let error = |code: &str, message: &str| BrevoError {
            code: code.to_owned(),
            message: message.to_owned(),
        };
        let status = |code| reqwest::StatusCode::from_u16(code).unwrap();
        assert_eq!(
            refusal(status(401), &BrevoError::default()),
            SendError::Authentication
        );
        assert_eq!(
            refusal(status(402), &BrevoError::default()),
            SendError::NoCredit
        );
        assert_eq!(
            refusal(
                status(400),
                &error("invalid_parameter", "Invalid sender name")
            ),
            SendError::SenderRefused
        );
        assert_eq!(
            refusal(
                status(400),
                &error("invalid_parameter", "Invalid phone number")
            ),
            SendError::InvalidNumber
        );
        assert_eq!(
            refusal(status(503), &BrevoError::default()),
            SendError::Provider
        );
    }
}
