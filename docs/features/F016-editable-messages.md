---
id: F016
title: "Editable emails and SMS, scheduled and manual messages"
status: draft
version: 1
adr: [ADR-0010, ADR-0015]
---

# F016 - Editable emails and SMS, scheduled and manual messages

## User story

As an owner,
I want to write every email and SMS sent by the site myself, choose when they are sent and see what was sent,
so that guests receive my words at the right moment, in their language.

## Business rules

* Every automatic message is a **template** linked to a **trigger**: booking confirmed, new booking (owners), payment abandoned, balance reminder, imminent cancellation, cancellation for unpaid balance, waiting for transfer (guest and owners), transfer reminder, cancellation for transfer not received, platform conflict, payment received, practical information (default D-2 at 10:00, at once if booked less than 2 days before), after-stay review request (default D+1), cancellation, contact acknowledgement, new contact message (owners, `Reply-To` visitor), new review to moderate (owners), review refused; plus **free templates** (X days before arrival / after departure, or manual).
* **Manual templates** (e.g. "Bon séjour") are never scheduled: sent only on request from the booking, with a prefilled preview editable before sending, once by default (confirmation to send again).
* Per template: recipient (guest / owners), channel (email, SMS, both), offset, active, FR and EN versions (sent in the guest's language, French fallback). Default templates are seeded and can be **restored**; versions are kept with rollback.
* **Editor**: email subject + body with the rich text editor (ADR-0015, plus an image of the media library), fixed layout (header, footer, cottage colours) readable in every mail client; SMS plain text with a **character and segment counter** (GSM-7 160 / 153, Unicode 70 / 67) and a warning above 2 SMS.
* **Variables** inserted with buttons (`{{ voyageur.prenom }}`, `{{ sejour.arrivee }}`, `{{ sejour.reference }}`, `{{ lien_suivi }}`…), dates and amounts formatted per language; unknown or misspelt variables are refused on save; allow-list of variables per trigger.
* Rendering in a **sandbox** (no file or code access, only the given variables), HTML sanitised.
* **Preview** with a sample or a real booking, **test send** to the owner.
* **Sending**: queue in the database processed by a scheduled task, retried on failure, never sent twice (unique per booking, template and trigger); messages are recalculated or cancelled when a booking changes or is cancelled; per-booking log (template, channel, recipient, date, status sent / delivered / failed).
* Senders behind ports (`Mailer`, `SmsSender`); the site address is the `From` (SPF / DKIM), never the visitor's; phone numbers in E.164.

## Acceptance criteria

```gherkin
Feature: Editable messages

  Scenario: F016-S01 Edit a template
    When the owner changes the French text of "infos pratiques" and saves
    Then the next practical information email uses the new text
    And the previous version can be restored

  Scenario: F016-S02 Unknown variable refused
    When the owner saves a template containing "{{ voyageur.prenon }}"
    Then the template is refused with the name of the unknown variable

  Scenario: F016-S03 Language of the guest
    Given an English-speaking guest
    Then they receive the English version
    Given a template without English text
    Then the French version is sent

  Scenario: F016-S04 Scheduled at D-2, recalculated on change
    Given a booking arriving on 20 August
    Then practical information is scheduled on 18 August at 10:00
    When the arrival moves to 25 August
    Then it is rescheduled on 23 August at 10:00
    When the booking is cancelled
    Then it is not sent

  Scenario: F016-S05 SMS counter
    When the owner types a 170-character SMS with an accented "é"
    Then the counter shows the number of Unicode segments
    And warns when it exceeds 2 SMS

  Scenario: F016-S06 Manual "Bon séjour"
    When the owner sends "Bon séjour" from a stay in progress after editing the preview
    Then the edited text is sent once and logged
    And sending it again asks for confirmation

  Scenario: F016-S07 Never sent twice
    When the sending task runs twice in a row
    Then each scheduled message is sent once

  Scenario: F016-S08 Sandboxed rendering
    When a template tries to read a file or call code
    Then rendering refuses it
```

## Out of scope

* Messages through the Airbnb / Booking inbox (optional, if the channel manager allows it), marketing newsletters.

## Tests

| Scenario | Test |
|---|---|
| F016-S01 → S08 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
