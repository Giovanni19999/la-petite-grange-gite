---
id: F025
title: "Email and SMS sending: SMTP server and SMS provider set by the owners"
status: draft
version: 1
adr: [ADR-0002, ADR-0010]
---

# F025 - Email and SMS sending: SMTP server and SMS provider set by the owners

## User story

As an owner,
I want the site to send its emails through my own mail server and its SMS through a provider I choose,
so that booking confirmations reach the guests and us without depending on the developer.

## Business rules

* Two ports in the application layer: `Mailer` (email with HTML and text versions, `From`, `Reply-To`, attachments later for invoices) and `SmsSender` (text, E.164 number). F016, F017 and F007 only use these ports.
* **SMTP adapter**: host, port, security (`STARTTLS` or `TLS`), user, password, sender address and name. Settings that are secrets (password) come from the environment or a secret file, never from the database or the public repository; the others are editable in an admin "Envoi des messages" screen.
* **SMS adapter**: one provider behind `SmsSender` (choice of the provider at freeze time, with an API key from the environment); sender name editable; SMS can be disabled.
* **Sending queue**: messages are saved first, then sent by a background task with retries (setting: attempts and delay); a message that still fails is marked failed and shown to the owners. A sent message is never sent twice.
* **Test button**: "Envoyer un e-mail de test" and "Envoyer un SMS de test" to the owners' address and phone, with the provider's error in clear French when it fails.
* Owners' notification addresses and phone numbers are settings (several allowed).
* Logs never contain message bodies, passwords or API keys; personal data are kept for the retention period (F018 GDPR setting).
* In development and tests, a fake sender keeps the messages in memory (or a local mail catcher) instead of sending them.

## Acceptance criteria

```gherkin
Feature: Email and SMS sending

  Scenario: F025-S01 Email sent through the owners' SMTP server
    Given the SMTP settings are filled in
    When the site sends a booking confirmation
    Then the guest receives it from the site address

  Scenario: F025-S02 Failure is retried then reported
    Given the SMTP server is unreachable
    When a message is sent
    Then it is retried as set
    And after the last attempt it is shown as failed to the owners

  Scenario: F025-S03 Test message
    When the owner clicks "Envoyer un e-mail de test"
    Then the owners receive the test email, or the error is shown in French

  Scenario: F025-S04 No secret leaks
    Then no password, API key or message body appears in the logs or in the API responses
```

## Out of scope

* Writing the messages and choosing when they are sent (F016), the contact form (F017), marketing emails.

## Tests

| Scenario | Test |
|---|---|
| F025-S01 → S04 | to be filled in when implemented |

## Changelog

* v1 (2026-11-21): first version (draft), at the owners' request: booking emails to the guest and to the owners need an SMTP server and SMS.
