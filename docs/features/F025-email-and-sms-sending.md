---
id: F025
title: "Email and SMS sending: SMTP server and SMS provider set by the owners"
status: frozen
version: 2
adr: [ADR-0002, ADR-0010, ADR-0019]
---

# F025 - Email and SMS sending: SMTP server and SMS provider set by the owners

## User story

As an owner,
I want the site to send its emails through a mail server I choose and its SMS through Brevo,
so that booking confirmations reach the guests and us without depending on the developer.

## Business rules

* Two ports in the application layer (ADR-0019): `Mailer` (subject, text and HTML versions, `From`, `Reply-To`) and `SmsSender` (plain text to an E.164 number). The features that send messages (F007, F016, F017) only put messages in the **sending queue**; they never call a provider.
* **Email: SMTP.** Host, port, security (`starttls`, `tls`, or `none` for a local mail catcher only, with a warning on the screen), user name, sender address and name, optional `Reply-To`: editable in the admin screen **"Envoi des messages"**. The **password** comes only from the environment (`SMTP_PASSWORD` or `SMTP_PASSWORD_FILE`), never from the database, the API or the repository. Any SMTP service works (the owners' provider, or Brevo's SMTP relay).
* **SMS: Brevo.** The provider is chosen by configuration (`SMS_PROVIDER`: `brevo` or `none`); the Brevo API key comes only from the environment (`BREVO_API_KEY` or `BREVO_API_KEY_FILE`). The startup fails if `brevo` is chosen without a key. On the screen: SMS on / off and the sender name (3 to 11 letters or digits, the name shown on the phone). With `none` or SMS off, SMS are not sent and are shown as "SMS désactivés" in the log; emails are not affected.
* **Notification recipients**: the owners' email addresses and phone numbers (several of each, E.164 for phones, e.g. `+33612345678`), used for the owners' messages (new booking…) and the test messages.
* **Sending queue** (table in the database): a message is saved with its channel, recipient, content and a **unique key** (e.g. `booking:42:confirmation:guest:email`), then sent by a background task. A key already in the queue is never added again, so a message is never queued twice.
* **Retries**: a failed attempt is retried after a delay that doubles each time (setting: number of attempts, default 5, first delay, default 5 minutes). After the last attempt the message is **failed** and shown to the owners on the screen and on the admin home. The owner can **send again** a failed message by hand.
* **Never sent twice**: a message is taken by the task before sending (status `sending`); a message found in `sending` after a restart is **not** sent again automatically but marked "à vérifier" (the owner decides to send it again or not).
* **Test buttons**: "Envoyer un e-mail de test" and "Envoyer un SMS de test" send at once (outside the queue) to the notification recipients and show the result, or the error of the server or provider explained in French (address refused, authentication failed, server unreachable, invalid number, no credit…).
* **Log** on the screen: the last messages (date, channel, recipient, subject or start of the SMS, status, attempts, last error), the failed ones first.
* **No secret leaks**: the logs and the API responses never contain the SMTP password, the API key nor the body of the messages. The logs only keep the id, channel, status and error kind.
* Personal data of the queue (recipients, bodies) are kept for the GDPR retention period (F018); until F018, they are kept.
* In development, a local mail catcher (Mailpit, `Back/docker-compose.yml`) receives the emails; in tests, fake senders keep the messages in memory, and the SMTP and Brevo adapters are tested against Mailpit and a fake Brevo server.
* The texts of the messages and when they are sent belong to F016 (editable templates); F007 brings the first two (booking confirmed for the guest, new booking for the owners).

## Acceptance criteria

```gherkin
Feature: Email and SMS sending

  Scenario: F025-S01 Email sent through the SMTP server
    Given the SMTP settings point to the mail server
    When a message for "guest@example.test" is queued
    Then the mail server receives it from the site address, with the text and HTML versions

  Scenario: F025-S02 Failure is retried then reported
    Given the mail server is unreachable and 3 attempts are set
    When a message is queued
    Then it is tried 3 times with a growing delay
    And it is then shown as failed to the owners
    When the owner sends it again and the server answers
    Then it is sent

  Scenario: F025-S03 Test messages
    When the owner clicks "Envoyer un e-mail de test"
    Then the notification addresses receive the test email
    Given the SMTP password is wrong
    Then the screen shows "authentification refusée par le serveur" in French

  Scenario: F025-S04 No secret leaks
    Then no password, API key or message body appears in the logs or in the API responses

  Scenario: F025-S05 SMS through Brevo
    Given SMS are on with the sender name "LaGrange"
    When an SMS for "+33612345678" is queued
    Then Brevo receives a transactional SMS from "LaGrange" to that number
    Given SMS are off
    Then the SMS is shown as "SMS désactivés" and nothing is sent

  Scenario: F025-S06 Never queued nor sent twice
    When the same message key is queued twice
    Then only one message is in the queue
    Given a message was being sent when the API stopped
    When the API starts again
    Then the message is marked "à vérifier" and not sent again

  Scenario: F025-S07 Invalid settings are refused
    When the owner saves a phone number "0612" or a sender name of 12 letters
    Then the settings are refused with the reason of each mistake
```

## Out of scope

* Writing the messages and choosing when they are sent (F016; F007 brings the two booking messages), the contact form (F017), marketing emails, delivery reports from Brevo, attachments (invoices, F012).

## Tests

| Scenario | Test |
|---|---|
| F025-S01 → S07 | to be filled in when implemented |

## Changelog

* v1 (2026-11-21): first version (draft), at the owners' request: booking emails to the guest and to the owners need an SMTP server and SMS.
* v2 (2026-11-22): frozen. SMS through Brevo (prepared, not yet subscribed: `SMS_PROVIDER=none` until then), secrets only from the environment, queue with unique key, doubling delay and "à vérifier" after a crash, test buttons, Mailpit in development.
