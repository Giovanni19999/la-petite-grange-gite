//! F007 acceptance tests against a disposable `MariaDB` container (requires Docker): direct
//! booking without online payment, messages, payments, cancellation, templates, tracking.
#![allow(clippy::unwrap_used, clippy::missing_panics_doc)] // test helpers: a failure is a test failure

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{EMAIL, SITE_URL, TestApp, json_body};
use gite_api::domain::messaging::Content;
use serde_json::{Value, json};
use time::{Date, Duration, OffsetDateTime};

const BOUNDARY: &str = "gite-test-boundary";
/// Seed page "Le gîte", published: stands for the rental terms page.
const TERMS_PAGE: u64 = 2;
const OWNER_PHONE: &str = "+33600000001";

/// A day `days` from now.
fn day(days: i64) -> Date {
    OffsetDateTime::now_utc().date() + Duration::days(days)
}

/// Rental terms chosen, owners' contacts, SMS on, 12 € tourist tax for 2 adults and
/// 4 nights, 120 € cleaning: 4 nights at 100 € (seed) + cleaning = 520 €.
async fn ready(app: &TestApp, cookie: &str) {
    let mut rules = json_body(app.get("/api/admin/house-rules", Some(cookie)).await).await;
    rules["terms_page"] = json!(TERMS_PAGE);
    rules["tourist_tax"]["per_adult_night"] = json!(150);
    let status = app
        .send_json("PUT", "/api/admin/house-rules", Some(cookie), &rules)
        .await
        .status();
    assert_eq!(status, StatusCode::NO_CONTENT);
    let status = app
        .send_json(
            "PUT",
            "/api/admin/prices/settings",
            Some(cookie),
            &json!({ "cleaning_fee": 12000, "default_min_nights": 2, "commission_alert_points": 5 }),
        )
        .await
        .status();
    assert_eq!(status, StatusCode::NO_CONTENT);
    sqlx::query(
        "UPDATE messaging_settings SET smtp_host = 'smtp.example.test', \
         from_address = 'site@example.test', sms_enabled = TRUE, owner_emails = ?, \
         owner_phones = ? WHERE id = 1",
    )
    .bind(json!([EMAIL]).to_string())
    .bind(json!([OWNER_PHONE]).to_string())
    .execute(&app.pool)
    .await
    .unwrap();
}

fn booking(arrival: Date, departure: Date) -> Value {
    json!({
        "arrival": arrival, "departure": departure, "adults": 2, "children": 0,
        "lang": "fr",
        "guest": {
            "first_name": "Léa", "last_name": "Martin", "email": "lea@example.test",
            "phone": "+33 6 12 34 56 78", "street": "1 rue des Lilas", "postcode": "75001",
            "town": "Paris", "country": "France"
        },
        "arrival_time": 1080, "message": "Nous arriverons avec un vélo.",
        "terms_accepted": true
    })
}

async fn book(app: &TestApp, body: &Value) -> (StatusCode, Value) {
    let response = app.send_json("POST", "/api/bookings", None, body).await;
    let status = response.status();
    let bytes = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, value)
}

async fn book_ok(app: &TestApp, body: &Value) -> Value {
    let (status, created) = book(app, body).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    created
}

async fn booking_id(app: &TestApp, reference: &str) -> u64 {
    sqlx::query_scalar("SELECT id FROM bookings WHERE reference = ?")
        .bind(reference)
        .fetch_one(&app.pool)
        .await
        .unwrap()
}

async fn detail(app: &TestApp, cookie: &str, id: u64) -> Value {
    let response = app
        .get(&format!("/api/admin/bookings/{id}"), Some(cookie))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn upload_document(app: &TestApp, cookie: &str, name: &str, bytes: &[u8]) -> StatusCode {
    let mut body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    body.extend(bytes);
    body.extend(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    app.send(
        Request::put("/api/admin/payment-info/document")
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .header(header::COOKIE, cookie)
            .body(Body::from(body))
            .unwrap(),
    )
    .await
    .status()
}

async fn nights(app: &TestApp, from: Date, to: Date) -> Vec<Value> {
    json_body(
        app.get(&format!("/api/calendar?from={from}&to={to}"), None)
            .await,
    )
    .await["nights"]
        .as_array()
        .unwrap()
        .clone()
}

#[tokio::test]
async fn f007_s01_s02_s04_booking_confirmed_at_once_and_everyone_is_told() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    ready(&app, &cookie).await;
    assert_eq!(
        upload_document(&app, &cookie, "RIB famille.pdf", b"%PDF-1.4 bank details").await,
        StatusCode::NO_CONTENT
    );
    let (arrival, departure) = (day(60), day(64));

    let created = book_ok(&app, &booking(arrival, departure)).await;
    let reference = created["reference"].as_str().unwrap().to_owned();
    assert_eq!(reference, format!("LPG-{}-0001", arrival.year()));
    let token = created["token"].as_str().unwrap();
    assert!(token.len() >= 43);

    // F007-S01: the nights are unavailable at once.
    let calendar = nights(&app, day(59), day(65)).await;
    assert_eq!(calendar[0]["price"], 10000);
    for night in &calendar[1..5] {
        assert_eq!(night["price"], Value::Null);
    }
    assert_eq!(calendar[5]["price"], 10000);

    let tracking = json_body(app.get(&format!("/api/bookings/{token}"), None).await).await;
    assert_eq!(tracking["status"], "confirmee");
    assert_eq!(tracking["reference"], reference.as_str());
    let amounts = &tracking["amounts"];
    assert_eq!(amounts["stay_total"], 52000);
    assert_eq!(amounts["deposit"], 26000);
    assert_eq!(amounts["balance"], 26000);
    assert_eq!(amounts["balance_due"], json!(arrival - Duration::days(30)));
    // F007-S04: the tourist tax stays apart.
    assert_eq!(amounts["tourist_tax"], 1200);
    assert!(!tracking.to_string().contains("lea@example.test"));

    // F007-S02: guest and owners, email and SMS, each queued once.
    assert_eq!(app.messaging.process_due().await.unwrap(), 4);
    assert_eq!(app.messaging.process_due().await.unwrap(), 0);
    let emails = app.mailer.sent();
    assert_eq!(emails.len(), 2);
    let guest = emails
        .iter()
        .find(|email| email.to == "lea@example.test")
        .unwrap();
    assert_eq!(
        guest.subject,
        format!("Votre réservation {reference} est confirmée")
    );
    assert_eq!(guest.attachments.len(), 1);
    assert_eq!(guest.attachments[0].name, "RIB famille.pdf");
    assert_eq!(guest.attachments[0].bytes, b"%PDF-1.4 bank details");
    assert!(guest.text.contains("260,00 €"));
    assert!(
        guest
            .text
            .contains(&format!("{SITE_URL}/reservation/{token}"))
    );
    let owner = emails.iter().find(|email| email.to == EMAIL).unwrap();
    assert!(
        owner
            .subject
            .starts_with(&format!("Nouvelle réservation {reference}"))
    );
    assert!(owner.text.contains("Airbnb"));
    assert_eq!(owner.attachments.len(), 0);
    let sms = app.sms.sent();
    assert_eq!(sms.len(), 2);
    assert!(sms.iter().any(|sms| sms.to == "+33612345678"));
    assert!(sms.iter().any(|sms| sms.to == OWNER_PHONE));

    let id = booking_id(&app, &reference).await;
    let detail = detail(&app, &cookie, id).await;
    assert_eq!(detail["messages"].as_array().unwrap().len(), 4);
    assert_eq!(detail["guest"]["phone"], "+33612345678");
}

#[tokio::test]
async fn f007_s03_fixed_deposit() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    ready(&app, &cookie).await;
    let mut rules = json_body(app.get("/api/admin/house-rules", Some(&cookie)).await).await;
    rules["deposit"] = json!({ "mode": "fixed", "amount": 15000 });
    app.send_json("PUT", "/api/admin/house-rules", Some(&cookie), &rules)
        .await;

    let created = book_ok(&app, &booking(day(60), day(64))).await;
    let token = created["token"].as_str().unwrap();
    let tracking = json_body(app.get(&format!("/api/bookings/{token}"), None).await).await;
    assert_eq!(tracking["amounts"]["deposit"], 15000);
    assert_eq!(tracking["amounts"]["balance"], 37000);
}

#[tokio::test]
async fn f007_s05_two_visitors_on_the_same_dates_make_one_booking() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    ready(&app, &cookie).await;
    let first = booking(day(60), day(64));
    let second = booking(day(62), day(65));
    let ((status_a, _), (status_b, refused)) =
        tokio::join!(book(&app, &first), book(&app, &second));
    let statuses = [status_a, status_b];
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::CREATED)
            .count(),
        1,
        "{statuses:?}"
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bookings")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);

    // Later, the same dates are refused with a reason.
    let (status, body) = book(&app, &booking(day(61), day(63))).await;
    assert!(
        status == StatusCode::CONFLICT || status == StatusCode::UNPROCESSABLE_ENTITY,
        "{status} {refused}"
    );
    assert!(body.to_string().contains("indisponibles") || body.to_string().contains("disponibles"));
}

#[tokio::test]
async fn f007_s06_prices_come_from_the_server() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    ready(&app, &cookie).await;
    let mut body = booking(day(60), day(64));
    body["stay_total"] = json!(100);
    body["deposit"] = json!(1);
    let created = book_ok(&app, &body).await;
    let id = booking_id(&app, created["reference"].as_str().unwrap()).await;
    assert_eq!(
        detail(&app, &cookie, id).await["amounts"]["stay_total"],
        52000
    );
}

#[tokio::test]
async fn f007_s07_owners_are_alerted_once_when_the_deposit_is_late() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    ready(&app, &cookie).await;
    let created = book_ok(&app, &booking(day(60), day(64))).await;
    let reference = created["reference"].as_str().unwrap();
    assert_eq!(app.bookings.alert_late_deposits().await.unwrap(), 0);
    app.messaging.process_due().await.unwrap();

    sqlx::query("UPDATE bookings SET created_at = created_at - INTERVAL 8 DAY")
        .execute(&app.pool)
        .await
        .unwrap();
    assert_eq!(app.bookings.alert_late_deposits().await.unwrap(), 1);
    assert_eq!(app.bookings.alert_late_deposits().await.unwrap(), 0);
    assert_eq!(app.messaging.process_due().await.unwrap(), 2);
    let alert = app
        .mailer
        .sent()
        .into_iter()
        .find(|email| email.subject.starts_with("Acompte non reçu"))
        .unwrap();
    assert_eq!(alert.to, EMAIL);
    assert!(alert.text.contains(reference));

    let id = booking_id(&app, reference).await;
    let detail = detail(&app, &cookie, id).await;
    assert_eq!(detail["status"], "confirmee");
    let list = json_body(app.get("/api/admin/bookings", Some(&cookie)).await).await;
    assert_eq!(list[0]["deposit_late"], true);
}

#[tokio::test]
async fn f007_s08_payments_recorded_by_the_owners() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    ready(&app, &cookie).await;
    let created = book_ok(&app, &booking(day(60), day(64))).await;
    let id = booking_id(&app, created["reference"].as_str().unwrap()).await;
    let pay = |amount: u64, method: &str| json!({ "amount": amount, "paid_on": day(1), "method": method, "note": "" });
    let uri = format!("/api/admin/bookings/{id}/payments");

    let response = app
        .send_json("POST", &uri, Some(&cookie), &pay(26000, "Bitcoin"))
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let response = app
        .send_json("POST", &uri, Some(&cookie), &pay(26000, "Virement"))
        .await;
    assert_eq!(json_body(response).await["status"], "acompte_recu");
    let response = app
        .send_json("POST", &uri, Some(&cookie), &pay(26000, "Chèque"))
        .await;
    assert_eq!(json_body(response).await["status"], "soldee");

    let detail = detail(&app, &cookie, id).await;
    assert_eq!(detail["status"], "soldee");
    assert_eq!(detail["amounts"]["paid"], 52000);
    assert_eq!(detail["payments"].as_array().unwrap().len(), 2);
    assert_eq!(detail["payments"][0]["author"], EMAIL);
    assert!(
        detail["events"][0]["description"]
            .as_str()
            .unwrap()
            .starts_with("Paiement reçu : 260,00 € par Chèque")
    );
}

#[tokio::test]
async fn f007_s09_f026_s05_conditions_are_frozen_on_the_booking() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    ready(&app, &cookie).await;
    let response = app
        .send_json(
            "POST",
            "/api/admin/offers/promotions",
            Some(&cookie),
            &json!({ "name": { "fr": "Bienvenue" },
                     "discount": { "kind": "percent", "bp": 1000 }, "min_nights": 2 }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let promotion = json_body(response).await["id"].as_u64().unwrap();
    let created = book_ok(&app, &booking(day(60), day(64))).await;
    let id = booking_id(&app, created["reference"].as_str().unwrap()).await;

    let mut rules = json_body(app.get("/api/admin/house-rules", Some(&cookie)).await).await;
    rules["deposit"] = json!({ "mode": "percentage", "percent": 30 });
    app.send_json("PUT", "/api/admin/house-rules", Some(&cookie), &rules)
        .await;
    let response = app
        .send_json(
            "DELETE",
            &format!("/api/admin/offers/promotions/{promotion}"),
            Some(&cookie),
            &json!({}),
        )
        .await;
    assert!(response.status().is_success());

    let detail = detail(&app, &cookie, id).await;
    // 400 € − 10 % + 120 € of cleaning = 480 €, deposit 50 %.
    assert_eq!(detail["amounts"]["promotion"], "Bienvenue");
    assert_eq!(detail["amounts"]["discount"], 4000);
    assert_eq!(detail["amounts"]["stay_total"], 48000);
    assert_eq!(detail["amounts"]["deposit"], 24000);
    assert_eq!(detail["deposit_percent"], 50);
}

#[tokio::test]
async fn f007_s10_terms_must_be_accepted_and_chosen() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;

    // No rental terms page chosen yet: booking is closed.
    let (status, body) = book(&app, &booking(day(60), day(64))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "booking_closed");
    let info = json_body(app.get("/api/booking-info", None).await).await;
    assert_eq!(info["open"], false);

    ready(&app, &cookie).await;
    let info = json_body(app.get("/api/booking-info?lang=en", None).await).await;
    assert_eq!(info["open"], true);
    assert_eq!(info["terms"]["slug"], "the-cottage");
    assert_eq!(info["arrival_slots"][0], 960);

    let mut body = booking(day(60), day(64));
    body["terms_accepted"] = json!(false);
    body["guest"]["email"] = json!("lea");
    let (status, refused) = book(&app, &body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let details = refused["error"]["details"].to_string();
    assert!(details.contains("conditions de location"), "{details}");
    assert!(details.contains("adresse e-mail invalide"), "{details}");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bookings")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn f007_s11_cancellation_frees_the_dates() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    ready(&app, &cookie).await;
    let created = book_ok(&app, &booking(day(60), day(64))).await;
    let id = booking_id(&app, created["reference"].as_str().unwrap()).await;
    let uri = format!("/api/admin/bookings/{id}/cancel");

    let response = app
        .send_json("POST", &uri, Some(&cookie), &json!({ "reason": " " }))
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = app
        .send_json(
            "POST",
            &uri,
            Some(&cookie),
            &json!({ "reason": "Fausse réservation" }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let detail = detail(&app, &cookie, id).await;
    assert_eq!(detail["status"], "annulee");
    assert_eq!(detail["cancel_reason"], "Fausse réservation");
    for night in nights(&app, day(60), day(64)).await {
        assert_eq!(night["price"], 10000);
    }
    let response = app
        .send_json("POST", &uri, Some(&cookie), &json!({ "reason": "encore" }))
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn f007_s12_templates_are_edited_with_known_variables_only() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    ready(&app, &cookie).await;
    let uri = "/api/admin/templates/booking_confirmed/email/fr";

    let response = app
        .send_json(
            "PUT",
            uri,
            Some(&cookie),
            &json!({ "subject": "Réservation", "body": "<p>Bonjour {{ voyageur.prenon }}</p>" }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json_body(response).await["error"]["details"],
        json!(["variable inconnue : voyageur.prenon"])
    );

    let response = app
        .send_json(
            "PUT",
            uri,
            Some(&cookie),
            &json!({ "subject": "Merci {{ voyageur.prenom }}",
                     "body": "<p>Bonjour <strong>{{ voyageur.prenom }}</strong></p><script>x</script>" }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let templates = json_body(app.get("/api/admin/templates", Some(&cookie)).await).await;
    let confirmed = &templates[0];
    assert_eq!(confirmed["kind"], "booking_confirmed");
    assert_eq!(confirmed["channels"][0]["texts"][0]["changed"], true);
    assert!(
        !confirmed["channels"][0]["texts"][0]["body"]
            .to_string()
            .contains("script")
    );

    let preview = app
        .send_json(
            "POST",
            "/api/admin/templates/preview",
            Some(&cookie),
            &json!({ "kind": "booking_confirmed", "channel": "email", "lang": "fr",
                     "subject": "Merci {{ voyageur.prenom }}", "body": "<p>{{ sejour.reference }}</p>" }),
        )
        .await;
    let preview = json_body(preview).await;
    assert_eq!(preview["subject"], "Merci Camille");
    assert_eq!(preview["text"], "LPG-2027-0001");

    book_ok(&app, &booking(day(60), day(64))).await;
    app.messaging.process_due().await.unwrap();
    let guest = app
        .mailer
        .sent()
        .into_iter()
        .find(|email| email.to == "lea@example.test")
        .unwrap();
    assert_eq!(guest.subject, "Merci Léa");
    assert!(guest.html.unwrap().contains("<strong>Léa</strong>"));

    let response = app
        .send_json("POST", &format!("{uri}/restore"), Some(&cookie), &json!({}))
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let templates = json_body(app.get("/api/admin/templates", Some(&cookie)).await).await;
    assert_eq!(templates[0]["channels"][0]["texts"][0]["changed"], false);
}

#[tokio::test]
async fn f007_s13_guests_get_their_language_with_french_fallback() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    ready(&app, &cookie).await;
    let response = app
        .send_json(
            "PUT",
            "/api/admin/templates/booking_confirmed/sms/en",
            Some(&cookie),
            &json!({ "body": "" }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let mut body = booking(day(60), day(64));
    body["lang"] = json!("en");
    let created = book_ok(&app, &body).await;
    app.messaging.process_due().await.unwrap();

    let guest = app
        .mailer
        .sent()
        .into_iter()
        .find(|email| email.to == "lea@example.test")
        .unwrap();
    assert!(guest.subject.starts_with("Your booking"));
    assert!(guest.text.contains("€260.00"));
    assert!(guest.text.contains(&format!(
        "{SITE_URL}/en/booking/{}",
        created["token"].as_str().unwrap()
    )));
    let sms = app
        .sms
        .sent()
        .into_iter()
        .find(|sms| sms.to == "+33612345678")
        .unwrap();
    assert!(sms.text.starts_with("Réservation"), "{}", sms.text);
    // Owners always read French.
    let owner = app
        .mailer
        .sent()
        .into_iter()
        .find(|email| email.to == EMAIL)
        .unwrap();
    assert!(owner.subject.starts_with("Nouvelle réservation"));
}

#[tokio::test]
async fn f007_s14_tracking_page_and_arrival_time() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    ready(&app, &cookie).await;
    let created = book_ok(&app, &booking(day(60), day(64))).await;
    let token = created["token"].as_str().unwrap();
    app.messaging.process_due().await.unwrap();

    let tracking = json_body(app.get(&format!("/api/bookings/{token}"), None).await).await;
    assert_eq!(tracking["arrival_time"], 1080);
    assert_eq!(tracking["arrival_time_changeable"], true);
    assert_eq!(tracking["first_name"], "Léa");

    let uri = format!("/api/bookings/{token}/arrival-time");
    let response = app
        .send_json("PUT", &uri, None, &json!({ "arrival_time": 900 }))
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = app
        .send_json("PUT", &uri, None, &json!({ "arrival_time": 1170 }))
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let tracking = json_body(app.get(&format!("/api/bookings/{token}"), None).await).await;
    assert_eq!(tracking["arrival_time"], 1170);
    app.messaging.process_due().await.unwrap();
    let changed = app
        .mailer
        .sent()
        .into_iter()
        .find(|email| email.subject.starts_with("Heure d'arrivée changée"))
        .unwrap();
    assert_eq!(changed.to, EMAIL);
    assert!(changed.text.contains("18:00 → 19:30"));

    let response = app.get("/api/bookings/unknown-token", None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn f007_s15_too_many_attempts_from_one_address() {
    let app = TestApp::start().await;
    let mut body = booking(day(60), day(64));
    body["terms_accepted"] = json!(false);
    for _ in 0..5 {
        let (status, _) = book(&app, &body).await;
        assert_ne!(status, StatusCode::TOO_MANY_REQUESTS);
    }
    let (status, refused) = book(&app, &body).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(refused["error"]["code"], "too_many_attempts");
}

#[tokio::test]
async fn f007_robots_and_strangers_get_nothing() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    ready(&app, &cookie).await;
    let mut body = booking(day(60), day(64));
    body["website"] = json!("https://spam.example");
    let (status, _) = book(&app, &body).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bookings")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);

    for uri in [
        "/api/admin/bookings",
        "/api/admin/bookings/1",
        "/api/admin/payment-info",
        "/api/admin/payment-info/document",
        "/api/admin/templates",
    ] {
        assert_eq!(
            app.get(uri, None).await.status(),
            StatusCode::UNAUTHORIZED,
            "{uri}"
        );
    }
}

#[tokio::test]
async fn f007_payment_instructions_and_document() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    let response = app
        .send_json(
            "PUT",
            "/api/admin/payment-info",
            Some(&cookie),
            &json!({ "instructions": { "fr": "<p>Virement</p><script>x</script>", "en": "" },
                     "methods": [] }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = app
        .send_json(
            "PUT",
            "/api/admin/payment-info",
            Some(&cookie),
            &json!({ "instructions": { "fr": "<p>Virement</p><script>x</script>", "en": "<p> </p>" },
                     "methods": [" Virement ", "Chèque", "Virement"] }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let info = json_body(app.get("/api/admin/payment-info", Some(&cookie)).await).await;
    assert_eq!(
        info["instructions"],
        json!({ "fr": "<p>Virement</p>", "en": "" })
    );
    assert_eq!(info["methods"], json!(["Virement", "Chèque"]));
    assert_eq!(info["document"], Value::Null);
    assert_eq!(info["provider"], "none");

    assert_eq!(
        upload_document(&app, &cookie, "virus.exe", b"MZ\x90\x00").await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        upload_document(&app, &cookie, "rib.pdf", b"%PDF-1.4").await,
        StatusCode::NO_CONTENT
    );
    let info = json_body(app.get("/api/admin/payment-info", Some(&cookie)).await).await;
    assert_eq!(info["document"]["name"], "rib.pdf");
    let response = app
        .get("/api/admin/payment-info/document", Some(&cookie))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    // The document is not in the public directory served by Caddy.
    let public = std::fs::read_dir(app.media_dir.path().join("public")).map_or(0, Iterator::count);
    assert_eq!(public, 0);

    let response = app
        .send(
            Request::delete("/api/admin/payment-info/document")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let info = json_body(app.get("/api/admin/payment-info", Some(&cookie)).await).await;
    assert_eq!(info["document"], Value::Null);
}

/// The content type of an attachment survives the queue.
#[tokio::test]
async fn f007_attachments_are_kept_in_the_queue() {
    let app = TestApp::start().await;
    let cookie = app.owner_cookie().await;
    ready(&app, &cookie).await;
    upload_document(&app, &cookie, "rib.png", &[0x89, b'P', b'N', b'G', 1, 2]).await;
    book_ok(&app, &booking(day(60), day(64))).await;
    let id: u64 = sqlx::query_scalar(
        "SELECT id FROM outgoing_messages WHERE recipient = 'lea@example.test' AND channel = 'email'",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    let messages = app.messaging.recent(10).await.unwrap();
    let message = messages.iter().find(|message| message.id == id).unwrap();
    let Content::Email { attachments, .. } = &message.content else {
        panic!("an email");
    };
    assert_eq!(attachments[0].content_type, "image/png");
    assert_eq!(attachments[0].name, "rib.png");
}
