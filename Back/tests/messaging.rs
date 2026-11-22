//! Acceptance tests of F025: email and SMS sending (requires Docker: `MariaDB` and Mailpit).

mod common;

use std::sync::{Arc, Mutex, PoisonError};

use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use common::{TestApp, json_body};
use gite_api::config::Secret;
use gite_api::domain::messaging::{Content, OutgoingMessage};
use gite_api::infrastructure::memory_senders::MemoryMailer;
use gite_api::infrastructure::sms::BrevoSms;
use gite_api::infrastructure::smtp::SmtpMailer;
use gite_api::wiring::Senders;
use serde_json::{Value, json};
use testcontainers_modules::testcontainers::core::{ContainerPort, WaitFor};
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, GenericImage, ImageExt};

const URL: &str = "/api/admin/messaging";
const SMTP_PASSWORD: &str = "good-password-f025";
const BREVO_KEY: &str = "xkeysib-test-f025";

/// Mailpit: SMTP on 1025 (user `site`, password [`SMTP_PASSWORD`]), API on 8025.
struct Mailpit {
    _container: ContainerAsync<GenericImage>,
    host: String,
    smtp_port: u16,
    api: String,
}

impl Mailpit {
    async fn start() -> Self {
        // The test client reads the Mailpit API with reqwest (ring, like the API).
        let _ = rustls::crypto::ring::default_provider().install_default();
        let container = GenericImage::new("axllent/mailpit", "v1.34.0")
            .with_exposed_port(ContainerPort::Tcp(1025))
            .with_exposed_port(ContainerPort::Tcp(8025))
            .with_wait_for(WaitFor::message_on_stdout("accessible via"))
            .with_env_var("MP_SMTP_AUTH", format!("site:{SMTP_PASSWORD}"))
            .with_env_var("MP_SMTP_AUTH_ALLOW_INSECURE", "true")
            .start()
            .await
            .unwrap();
        let host = container.get_host().await.unwrap().to_string();
        let smtp_port = container.get_host_port_ipv4(1025).await.unwrap();
        let api_port = container.get_host_port_ipv4(8025).await.unwrap();
        Self {
            api: format!("http://{host}:{api_port}/api/v1"),
            _container: container,
            host,
            smtp_port,
        }
    }

    async fn messages(&self) -> Vec<Value> {
        let list: Value = reqwest::get(format!("{}/messages", self.api))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let mut messages = Vec::new();
        for summary in list["messages"].as_array().unwrap() {
            let id = summary["ID"].as_str().unwrap();
            messages.push(
                reqwest::get(format!("{}/message/{id}", self.api))
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap(),
            );
        }
        messages
    }
}

fn settings(host: &str, port: u16) -> Value {
    json!({
        "smtp": {
            "host": host,
            "port": port,
            "security": "none",
            "username": "site",
            "from_address": "site@example.test",
            "from_name": "Le gîte",
            "reply_to": "proprio@example.test"
        },
        "sms_enabled": true,
        "sms_sender": "LaGrange",
        "owner_emails": ["proprio@example.test"],
        "owner_phones": ["+33 6 12 34 56 78"],
        "max_attempts": 1,
        "first_retry_minutes": 5
    })
}

fn email(key: &str) -> OutgoingMessage {
    OutgoingMessage {
        key: key.to_owned(),
        recipient: "guest@example.test".to_owned(),
        content: Content::Email {
            subject: "Réservation confirmée".to_owned(),
            text: "Bonjour, votre séjour est confirmé.".to_owned(),
            html: Some("<p>Bonjour, votre séjour est <strong>confirmé</strong>.</p>".to_owned()),
            attachments: Vec::new(),
        },
    }
}

async fn smtp_app(password: &str) -> TestApp {
    TestApp::start_with(Senders {
        mailer: Arc::new(SmtpMailer::new(Some(Secret::new(password)))),
        sms: Arc::new(gite_api::infrastructure::sms::NoSms),
    })
    .await
}

async fn put_settings(app: &TestApp, cookie: &str, settings: &Value) {
    let response = app.send_json("PUT", URL, Some(cookie), settings).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn f025_s01_email_sent_through_the_smtp_server() {
    let mailpit = Mailpit::start().await;
    let app = smtp_app(SMTP_PASSWORD).await;
    let cookie = app.owner_cookie().await;
    put_settings(&app, &cookie, &settings(&mailpit.host, mailpit.smtp_port)).await;

    assert!(app.messaging.queue(&email("test:1")).await.unwrap());
    assert_eq!(app.messaging.process_due().await.unwrap(), 1);

    let messages = mailpit.messages().await;
    assert_eq!(messages.len(), 1);
    let message = &messages[0];
    assert_eq!(message["From"]["Address"], "site@example.test");
    assert_eq!(message["From"]["Name"], "Le gîte");
    assert_eq!(message["To"][0]["Address"], "guest@example.test");
    assert_eq!(message["ReplyTo"][0]["Address"], "proprio@example.test");
    assert_eq!(message["Subject"], "Réservation confirmée");
    assert!(
        message["Text"]
            .as_str()
            .unwrap()
            .contains("votre séjour est confirmé")
    );
    assert!(
        message["HTML"]
            .as_str()
            .unwrap()
            .contains("<strong>confirmé</strong>")
    );

    let list = json_body(app.get(&format!("{URL}/messages"), Some(&cookie)).await).await;
    assert_eq!(list[0]["status"], "sent");
    assert_eq!(list[0]["summary"], "Réservation confirmée");
}

#[tokio::test]
async fn f025_s02_unreachable_server_then_sent_again_by_the_owner() {
    let mailpit = Mailpit::start().await;
    let app = smtp_app(SMTP_PASSWORD).await;
    let cookie = app.owner_cookie().await;
    // Nothing listens on port 1.
    put_settings(&app, &cookie, &settings("127.0.0.1", 1)).await;
    app.messaging.queue(&email("test:2")).await.unwrap();
    app.messaging.process_due().await.unwrap();

    let list = json_body(app.get(&format!("{URL}/messages"), Some(&cookie)).await).await;
    assert_eq!(list[0]["status"], "failed");
    assert_eq!(list[0]["attempts"], 1);
    assert_eq!(list[0]["error"], "serveur injoignable");
    let problems = json_body(app.get(&format!("{URL}/problems"), Some(&cookie)).await).await;
    assert_eq!(problems["count"], 1);

    put_settings(&app, &cookie, &settings(&mailpit.host, mailpit.smtp_port)).await;
    let id = list[0]["id"].as_u64().unwrap();
    let response = app
        .send_json(
            "POST",
            &format!("{URL}/messages/{id}/resend"),
            Some(&cookie),
            &json!({}),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    app.messaging.process_due().await.unwrap();
    assert_eq!(mailpit.messages().await.len(), 1);
    let list = json_body(app.get(&format!("{URL}/messages"), Some(&cookie)).await).await;
    assert_eq!(list[0]["status"], "sent");
}

#[tokio::test]
async fn f025_s03_test_email_and_wrong_password_explained_in_french() {
    let mailpit = Mailpit::start().await;
    let app = smtp_app(SMTP_PASSWORD).await;
    let cookie = app.owner_cookie().await;
    put_settings(&app, &cookie, &settings(&mailpit.host, mailpit.smtp_port)).await;
    let results = json_body(
        app.send_json(
            "POST",
            &format!("{URL}/test-email"),
            Some(&cookie),
            &json!({}),
        )
        .await,
    )
    .await;
    assert_eq!(
        results,
        json!([{ "recipient": "proprio@example.test", "sent": true, "error": null }])
    );
    assert_eq!(
        mailpit.messages().await[0]["To"][0]["Address"],
        "proprio@example.test"
    );

    let wrong = smtp_app("wrong-password").await;
    let cookie = wrong.owner_cookie().await;
    put_settings(&wrong, &cookie, &settings(&mailpit.host, mailpit.smtp_port)).await;
    let results = json_body(
        wrong
            .send_json(
                "POST",
                &format!("{URL}/test-email"),
                Some(&cookie),
                &json!({}),
            )
            .await,
    )
    .await;
    assert_eq!(results[0]["sent"], false);
    assert_eq!(
        results[0]["error"],
        "authentification refusée par le serveur"
    );
}

#[tokio::test]
async fn f025_s04_no_secret_nor_email_body_in_the_responses() {
    let app = smtp_app(SMTP_PASSWORD).await;
    let cookie = app.owner_cookie().await;
    put_settings(&app, &cookie, &settings("127.0.0.1", 1)).await;
    app.messaging.queue(&email("test:4")).await.unwrap();
    app.messaging.process_due().await.unwrap();

    let mut bodies = String::new();
    for uri in [URL.to_owned(), format!("{URL}/messages")] {
        let response = app.get(&uri, Some(&cookie)).await;
        assert_eq!(response.status(), StatusCode::OK);
        bodies.push_str(&json_body(response).await.to_string());
    }
    assert!(bodies.contains("\"smtp_password_set\":true"));
    assert!(!bodies.contains(SMTP_PASSWORD));
    assert!(!bodies.contains("votre séjour est confirmé"));
}

/// Fake Brevo API: records the requests and answers with the given status.
#[derive(Clone)]
struct FakeBrevo {
    requests: Arc<Mutex<Vec<(String, Value)>>>,
    status: Arc<Mutex<u16>>,
}

impl FakeBrevo {
    async fn start() -> (Self, String) {
        let fake = Self {
            requests: Arc::default(),
            status: Arc::new(Mutex::new(201)),
        };
        let state = fake.clone();
        let router = Router::new().route(
            "/v3/transactionalSMS/send",
            post(
                move |headers: axum::http::HeaderMap, Json(body): Json<Value>| {
                    let state = state.clone();
                    async move {
                        let key = headers["api-key"].to_str().unwrap().to_owned();
                        state
                            .requests
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .push((key, body));
                        let status = *state.status.lock().unwrap_or_else(PoisonError::into_inner);
                        (
                            StatusCode::from_u16(status).unwrap(),
                            Json(json!({ "messageId": 1, "code": "not_enough_credits" })),
                        )
                    }
                },
            ),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        (fake, url)
    }

    fn requests(&self) -> Vec<(String, Value)> {
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[tokio::test]
async fn f025_s05_sms_through_brevo_or_disabled() {
    let (brevo, url) = FakeBrevo::start().await;
    let app = TestApp::start_with(Senders {
        mailer: Arc::new(MemoryMailer::default()),
        sms: Arc::new(BrevoSms::new(&url, Secret::new(BREVO_KEY)).unwrap()),
    })
    .await;
    let cookie = app.owner_cookie().await;
    put_settings(&app, &cookie, &settings("smtp.example.test", 587)).await;
    let sms = |key: &str| OutgoingMessage {
        key: key.to_owned(),
        recipient: "+33612345678".to_owned(),
        content: Content::Sms {
            text: "Nouvelle réservation du 10 au 14 juillet".to_owned(),
        },
    };
    app.messaging.queue(&sms("test:5:a")).await.unwrap();
    app.messaging.process_due().await.unwrap();
    assert_eq!(
        brevo.requests(),
        vec![(
            BREVO_KEY.to_owned(),
            json!({
                "sender": "LaGrange",
                "recipient": "33612345678",
                "content": "Nouvelle réservation du 10 au 14 juillet",
                "type": "transactional",
                "unicodeEnabled": true
            })
        )]
    );

    // Test SMS refused by Brevo: no credit left.
    *brevo.status.lock().unwrap_or_else(PoisonError::into_inner) = 402;
    let results = json_body(
        app.send_json(
            "POST",
            &format!("{URL}/test-sms"),
            Some(&cookie),
            &json!({}),
        )
        .await,
    )
    .await;
    assert_eq!(results[0]["recipient"], "+33612345678");
    assert_eq!(results[0]["error"], "crédit SMS épuisé");

    // SMS off: nothing is sent.
    let mut off = settings("smtp.example.test", 587);
    off["sms_enabled"] = json!(false);
    put_settings(&app, &cookie, &off).await;
    app.messaging.queue(&sms("test:5:b")).await.unwrap();
    app.messaging.process_due().await.unwrap();
    assert_eq!(brevo.requests().len(), 2);
    let list = json_body(app.get(&format!("{URL}/messages"), Some(&cookie)).await).await;
    assert_eq!(list[0]["status"], "disabled");
}

#[tokio::test]
async fn f025_s06_never_queued_nor_sent_twice() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    put_settings(&app, &cookie, &settings("smtp.example.test", 587)).await;
    assert!(
        app.messaging
            .queue(&email("booking:1:guest"))
            .await
            .unwrap()
    );
    assert!(
        !app.messaging
            .queue(&email("booking:1:guest"))
            .await
            .unwrap()
    );

    // The API stopped while a message was being sent.
    sqlx::query("UPDATE outgoing_messages SET status = 'sending'")
        .execute(&app.pool)
        .await
        .unwrap();
    assert_eq!(app.messaging.recover_interrupted().await.unwrap(), 1);
    assert_eq!(app.messaging.process_due().await.unwrap(), 0);
    assert_eq!(app.mailer.sent().len(), 0);
    let list = json_body(app.get(&format!("{URL}/messages"), Some(&cookie)).await).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["status"], "to_check");

    // The owner decides to send it again.
    let id = list[0]["id"].as_u64().unwrap();
    app.send_json(
        "POST",
        &format!("{URL}/messages/{id}/resend"),
        Some(&cookie),
        &json!({}),
    )
    .await;
    app.messaging.process_due().await.unwrap();
    assert_eq!(app.mailer.sent().len(), 1);
}

#[tokio::test]
async fn f025_s07_settings_saved_or_refused_with_every_reason() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let defaults = json_body(app.get(URL, Some(&cookie)).await).await;
    assert_eq!(defaults["settings"]["smtp"]["host"], "");
    assert_eq!(defaults["settings"]["sms_enabled"], false);
    assert_eq!(defaults["sms_provider"], true);

    put_settings(&app, &cookie, &settings("smtp.example.test", 587)).await;
    let saved = json_body(app.get(URL, Some(&cookie)).await).await;
    assert_eq!(saved["settings"]["owner_phones"], json!(["+33612345678"]));

    let mut wrong = settings("smtp.example.test", 587);
    wrong["owner_phones"] = json!(["0612"]);
    wrong["sms_sender"] = json!("PetiteGrange");
    let response = app.send_json("PUT", URL, Some(&cookie), &wrong).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = json_body(response).await;
    assert_eq!(
        body["error"]["details"],
        json!([
            "nom d'expéditeur des SMS : de 3 à 11 lettres ou chiffres, sans espace ni accent",
            "numéro de notification invalide : 0612 (format international, par exemple +33612345678)"
        ])
    );
}

#[tokio::test]
async fn f025_sending_is_admin_only() {
    let app = TestApp::start().await;
    for (method, uri) in [
        ("GET", URL.to_owned()),
        ("PUT", URL.to_owned()),
        ("GET", format!("{URL}/messages")),
        ("POST", format!("{URL}/test-email")),
        ("POST", format!("{URL}/messages/1/resend")),
    ] {
        let response = app.send_json(method, &uri, None, &json!({})).await;
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri}"
        );
    }
}
