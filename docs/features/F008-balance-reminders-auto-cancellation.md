---
id: F008
title: "Balance at D-30, reminders, automatic cancellation"
status: draft
version: 1
adr: [ADR-0005, ADR-0006, ADR-0010]
---

# F008 - Balance at D-30, reminders, automatic cancellation

## User story

As an owner,
I want guests to be reminded to pay the balance and unpaid bookings to be cancelled automatically,
so that I never chase payments by hand and the dates are released in time.

As a guest,
I want to pay my balance online with one link, or by transfer or cheque if the owners allow it,
so that paying is simple.

## Business rules

* Balance due date = arrival − `solde_jours_avant` (default 30), computed from the rules copied on the booking (F007).
* The guest pays online through `/payer/:token` (link in the confirmation email), or offline (transfer, cheque) if enabled; the owners tick "reçu" for offline payments. Balance received → `soldee`. The IBAN is only ever sent to guests who booked, never shown publicly.
* **Reminders**: every `rappel_solde_intervalle_jours` (default 2) days, from D-(`solde_jours_avant` + `avertir_annulation_jours`) to D-`solde_jours_avant` (default D-40, D-38, D-36, D-34, D-32), only while the balance is expected; a booking made inside that window gets a first reminder at once. Each reminder states the cancellation date.
* **Warning to the owners** `avertir_annulation_jours` (default 10) before the due date (trigger `annulation_imminente`); no separate message to the guest (the reminder of the same day says it).
* **Automatic cancellation** when the balance is not received on the due date → `annulee`, reason `solde_non_recu`, dates released on every channel, **deposit kept**. Never while an online payment is in progress at the provider (waiting for its webhook).
* **"Accorder un délai"**: the owners can set a new due date before the cancellation; recorded in the booking history.
* The daily task runs in Europe/Paris time, is idempotent (at most one reminder per day and booking, recorded in the history) and catches up if it missed a day.
* The admin dashboard lists "annulations automatiques à venir" and payments expected / received / late.

## Acceptance criteria

```gherkin
Feature: Balance, reminders and automatic cancellation

  Background:
    Given the rules: balance 30 days before, warning 10 days before, reminder every 2 days
    And a confirmed booking arriving on 20 August with a balance of 260 € expected

  Scenario: F008-S01 Reminders until the due date
    When the daily task runs from 11 July to 21 July
    Then reminders are sent on 11, 13, 15, 17 and 19 July
    And each reminder states the cancellation date of 21 July
    And no reminder is sent twice the same day

  Scenario: F008-S02 Owners warned before cancellation
    When the daily task runs on 11 July
    Then the owners receive the "annulation imminente" warning

  Scenario: F008-S03 Online balance stops the reminders
    When the guest pays the balance online through the tracking link
    And the provider confirms it
    Then the booking is "soldee"
    And no more reminders are sent

  Scenario: F008-S04 Offline balance ticked by the owners
    When the owners tick the balance as received by transfer
    Then the booking is "soldee"

  Scenario: F008-S05 Automatic cancellation keeps the deposit
    Given the balance is still expected on 21 July
    When the daily task runs on 21 July
    Then the booking is "annulee" with the reason "solde_non_recu"
    And the dates are available again everywhere
    And the deposit is not refunded

  Scenario: F008-S06 Payment in progress prevents the cancellation
    Given the guest started paying the balance online and the provider has not answered yet
    When the daily task runs on 21 July
    Then the booking is not cancelled

  Scenario: F008-S07 More time granted
    When the owners grant a new due date of 1 August
    Then the cancellation moves to 1 August and is recorded in the history

  Scenario: F008-S08 Booking made inside the reminder window
    Given a booking made on 12 July for an arrival on 20 August
    Then a first reminder is sent at once, then every 2 days
```

## Out of scope

* Last-minute payment (F009), refunds (F010), message texts (F016).

## Tests

| Scenario | Test |
|---|---|
| F008-S01 → S08 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
