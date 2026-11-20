---
id: F017
title: "Contact form with owner notification (Reply-To)"
status: draft
version: 1
adr: [ADR-0009, ADR-0010]
---

# F017 - Contact form with owner notification (Reply-To)

## User story

As a visitor,
I want to ask the owners a question from the site,
so that I get an answer before booking.

As an owner,
I want each message by email with the visitor's address as reply address,
so that I answer from my usual mailbox.

## Business rules

* **"Contact" block** of the page editor: name, email, wished dates (optional, prefilled from the calendar when coming from a quote), message, consent mention (data used only to answer, kept for a limited time).
* `POST /api/contact` saves the message, then sends **two emails** (templates of F016): acknowledgement to the visitor, and notification to the owners (addresses from the settings) with `From` = the site address (SPF / DKIM) and **`Reply-To` = the visitor's email** (validated, name cleaned against header injection), the message in the body and a link to the admin.
* Each email is sent even if the other fails; failures are retried by the sending queue.
* **Anti-spam**: hidden trap field, rate limit per IP (setting), length limits; no captcha.
* Admin list "Messages reçus" (newest first, read / unread), each message also already received by email; messages are deleted after the retention period (setting, GDPR).
* Form messages and errors in the page language (FR / EN).

## Acceptance criteria

```gherkin
Feature: Contact form

  Scenario: F017-S01 Message sent
    When a visitor sends a message with a valid email
    Then the message is saved
    And the visitor receives an acknowledgement
    And the owners receive the message with the visitor's email as Reply-To and the site address as sender

  Scenario: F017-S02 Header injection
    When a visitor's name contains a line break and "Bcc:"
    Then the notification headers contain no extra recipient

  Scenario: F017-S03 Spam trap
    When the hidden field is filled
    Then no email is sent and the answer looks successful

  Scenario: F017-S04 Rate limit
    When the same address sends more messages than allowed in the period
    Then the next ones are refused with a clear message

  Scenario: F017-S05 One email fails
    Given the acknowledgement cannot be sent
    Then the owners still receive the notification
    And the acknowledgement is retried

  Scenario: F017-S06 Dates prefilled from a quote
    Given a visitor chose 3 to 6 May in the calendar
    When they open the contact form
    Then the wished dates are prefilled
```

## Out of scope

* Answering from the admin (owners answer from their mailbox), chat.

## Tests

| Scenario | Test |
|---|---|
| F017-S01 → S06 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
