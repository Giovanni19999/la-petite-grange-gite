---
id: F009
title: "Last-minute booking: 100 % online or bank transfer within 3 days"
status: draft
version: 1
adr: [ADR-0005, ADR-0006, ADR-0010]
---

# F009 - Last-minute booking: 100 % online or bank transfer within 3 days

## User story

As a visitor booking a few days before arrival,
I want to pay the whole stay online, or by bank transfer if there is time,
so that I can still book at the last minute.

As an owner,
I want last-minute bookings to be paid in full before arrival and to be told at once,
so that I can prepare the cottage in time.

## Business rules

* A booking is **last minute** when it is made less than `solde_jours_avant` (default 30) days before arrival: one single payment of 100 % of the stay price (tourist tax excluded).
* Bookable until `delai_min_jours` (default 1 = until the day before; 0 = the same day).
* **Online**: paid at once; provider confirmation → `soldee` directly.
* **Bank transfer within `delai_virement_jours`** (default 3), offered only if `derniere_minute_virement` is on **and** the arrival is after today + that delay: status `attente_virement`, dates blocked everywhere until `attente_virement_jusqu_a`; email to the guest with amount, IBAN, transfer reference (= booking reference) and deadline; email "nouvelle réservation en attente de virement" to the owners.
* The owners tick "virement reçu" → `soldee` and the rental contract is sent; the guest gets a reminder the day before the deadline and the owners an alert on the deadline day (they can also grant more time).
* Deadline passed without the transfer → `annulee`, reason `virement_non_recu`, dates released, nothing to refund.
* Owners are notified with priority (email + SMS) of every last-minute booking.

## Acceptance criteria

```gherkin
Feature: Last-minute booking

  Background:
    Given the rules: balance 30 days before, transfer within 3 days allowed

  Scenario: F009-S01 Full payment online
    Given today is 1 July and a visitor books 10 to 12 July
    When the visitor pays online and the provider confirms
    Then the whole stay price is paid at once
    And the booking is "soldee"
    And the owners get an email and an SMS

  Scenario: F009-S02 Transfer offered when there is time
    Given today is 1 July and a visitor books 10 to 12 July
    Then the "virement sous 3 jours" option is offered
    When the visitor chooses it
    Then the booking is "attente_virement" until 4 July
    And the dates are blocked everywhere
    And the guest receives the IBAN, the reference and the deadline

  Scenario: F009-S03 Transfer not offered too close to arrival
    Given today is 1 July and a visitor books 3 to 5 July
    Then only online payment is offered

  Scenario: F009-S04 Transfer received
    Given a booking waiting for a transfer
    When the owners tick "virement reçu"
    Then the booking is "soldee" and the contract is sent

  Scenario: F009-S05 Transfer not received
    Given a booking waiting for a transfer until 4 July
    When the daily task runs on 4 July without the transfer ticked
    Then the booking is "annulee" with the reason "virement_non_recu"
    And the dates are available again
    And nothing is refunded

  Scenario: F009-S06 Same-day booking
    Given the minimum delay is 0 day
    When a visitor books for tonight
    Then online payment of 100 % is required
```

## Out of scope

* Balance of normal bookings (F008), refunds (F010).

## Tests

| Scenario | Test |
|---|---|
| F009-S01 → S06 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
