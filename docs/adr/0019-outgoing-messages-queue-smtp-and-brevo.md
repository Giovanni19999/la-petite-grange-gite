---
status: accepted
date: 2026-11-22
---

# Outgoing emails and SMS go through a queue in the database, sent by SMTP and Brevo adapters

## Context and Problem Statement

Bookings (F007), templates (F016) and the contact form (F017) send emails and SMS to the guests and the owners. A mail server or an SMS provider can be down; a message must then be retried, never lost, and never sent twice. The owners have not chosen their providers yet: emails must work with any mail server, and SMS are prepared for Brevo.

## Decision Drivers

* No message lost nor sent twice, even if the API restarts
* A booking never fails because a mail server is down
* Change of provider without touching the domain nor the use cases (ADR-0002)
* Secrets never in the database nor in the repository
* Low maintenance: no extra service in production

## Considered Options

* Queue table in MariaDB + background task in the API, `Mailer` and `SmsSender` ports, SMTP adapter (`lettre`) and Brevo HTTP adapter (`reqwest`)
* Send directly in the request of the booking
* An external queue (Redis, RabbitMQ) or a transactional email service API for emails

## Decision Outcome

Chosen option: "Queue table in MariaDB + background task, SMTP and Brevo adapters".

* Use cases only **queue** messages (in the same transaction as the booking when there is one). A unique key per message prevents queuing it twice.
* A background task started by `wiring` takes due messages one by one (status `sending`), sends them through the port, and records `sent`, or a new attempt with a doubling delay, or `failed` after the last attempt. A message found in `sending` at startup is marked `to_check`, never resent automatically.
* `Mailer` adapters: SMTP with `lettre` (rustls, `starttls` / `tls` / `none`); in tests, an in-memory fake. `SmsSender` adapters: Brevo transactional SMS API with `reqwest` (rustls); `none` (SMS disabled); in tests, an in-memory fake. The Brevo base URL is configurable for the tests only.
* Non-secret settings (host, port, sender, recipients, retries) are in the database and editable by the owners; the SMTP password and the Brevo API key come only from the environment or a secret file. The provider of SMS is chosen by configuration (`SMS_PROVIDER`).
* In development, Mailpit (official image) catches the emails.

### Consequences

* Good, because a provider outage only delays messages, and the owners see the failed ones
* Good, because no new service in production: the queue is a table
* Good, because adding another SMS provider is one adapter and one value of `SMS_PROVIDER`
* Bad, because the background task polls the table (every few seconds) instead of being woken up
* Bad, because two new dependencies (`lettre`, `reqwest`) and their licences to check

### Confirmation

Acceptance tests send through Mailpit (testcontainers) and a fake Brevo server; tests of the queue cover the unique key, the doubling delay, the failed state and the `to_check` state after a restart; a test checks that the API responses and the logs never contain the password, the key or a message body.
