---
id: F007
title: "Instant booking with online deposit (percentage or fixed amount)"
status: draft
version: 1
adr: [ADR-0005, ADR-0006, ADR-0010, ADR-0007]
---

# F007 - Instant booking with online deposit (percentage or fixed amount)

## User story

As a visitor,
I want to book the cottage directly on the site and pay a deposit online,
so that my stay is confirmed at once, without commission and without waiting for an answer.

As an owner,
I want direct bookings to be confirmed only when the deposit is really received, and never to overlap another booking,
so that I have nothing to validate and no double booking.

## Business rules

* **House rules** ("Règles du gîte" admin screen, table `booking_rules`, versioned with history): arrival and departure weekdays, arrival from (default 16:00) and departure before (default 10:00), capacity (default 5), pets (default no), booking horizon, minimum delay, payment hold duration (default 20 min), deposit mode `percentage` (default 50 %) or `fixed amount` (capped at the stay price), balance due days before arrival (default 30), cancellation settings (F010), offline payment methods, deposit (caution) amount. **The code holds none of these values** (ADR-0010).
* The rules in force are **copied onto each booking** when it is made: later changes never alter an existing booking.
* **Flow**: 1) dates and guests → quote (F004); 2) contact details, message, expected arrival time (30-minute slots from the minimum arrival time), mandatory acceptance of the rental terms (version and time recorded); 3) `POST /api/bookings` re-checks availability and **holds** the dates (status `en_paiement`) in the same transaction (lock on the cottage), sends the hold to the channel manager, shows a countdown; 4) payment of the deposit with the active provider (ADR-0006); 5) **confirmation only on a verified provider event** (signed webhook or server check, never the browser return) → `confirmee`, dates blocked everywhere, emails (F016).
* Deposit = mode of the rules applied to the **stay price** (nights + cleaning, tourist tax always excluded); balance = stay price − deposit, due on arrival − balance days. Paying the balance together with the deposit is always offered.
* Just before payment, availability is re-checked with the channel manager, or a **fresh** iCal download of each platform (F014); if the check is impossible, online booking is suspended on those dates ("contactez-nous") and the owners are alerted.
* Hold expired, payment failed or abandoned → `paiement_abandonne` (not a booking), dates released everywhere.
* Payment confirmed after the hold expired while the dates were taken meanwhile → automatic full refund and alert (test dedicated).
* Conflict detected afterwards (platform booking arrived at the same time): immediate alert to the owners (email + SMS), "conflit" screen with both bookings, full refund of the site guest in one click, apology email prefilled.
* Each booking has a public **reference** (e.g. `LPG-2027-0042`) and a random **tracking token** (link sent by email, never the id); the guest can change the expected arrival time until the day before.
* Statuses: `en_paiement` → `confirmee` → `soldee` → `sejour_en_cours` → `terminee`, or `paiement_abandonne` / `annulee` (F008, F010). `en_paiement`, `attente_virement` (F009) and `confirmee` block the dates like `soldee`.
* Every event of a booking (creation, payment, email, change, cancellation) is kept in its history.
* Online booking is impossible with `provider = "none"`: the API refuses to start, or online booking is replaced by the contact form, as configured (ADR-0006).
* Admin: list and detail of bookings (direct and imported), filters by status and dates, payments expected / received / late.

## Acceptance criteria

```gherkin
Feature: Instant booking with online deposit

  Background:
    Given the house rules ask for a 50 % deposit and the balance 30 days before arrival
    And the payment provider is in sandbox mode

  Scenario: F007-S01 Booking confirmed by the deposit
    Given a free stay from 10 to 14 July costing 520 € with cleaning
    When a visitor fills in the form, accepts the terms and pays the 260 € deposit
    And the provider sends a valid signed confirmation
    Then the booking is "confirmee" with a reference and a tracking link
    And the dates are unavailable on the site and sent as blocked to the platforms
    And the balance of 260 € is due 30 days before arrival

  Scenario: F007-S02 Browser return alone confirms nothing
    When the visitor comes back from the provider page without a provider confirmation
    Then the booking stays "en_paiement"

  Scenario: F007-S03 Fixed deposit
    Given the house rules ask for a fixed deposit of 150 €
    When a visitor books a stay costing 520 €
    Then the deposit is 150 € and the balance 370 €

  Scenario: F007-S04 Tourist tax outside the deposit
    When a visitor books a stay costing 520 € with 12 € of tourist tax
    Then the deposit and balance are computed on 520 €
    And the tourist tax is shown apart, to pay on arrival

  Scenario: F007-S05 Two visitors on the same dates
    Given visitor A holds 10 to 14 July
    When visitor B tries to book 12 to 15 July
    Then visitor B is told the dates are no longer available

  Scenario: F007-S06 Abandoned payment releases the dates
    Given a hold of 20 minutes
    When the visitor does not pay within 20 minutes
    Then the status is "paiement_abandonne"
    And the dates are available again everywhere

  Scenario: F007-S07 Late confirmation on taken dates is refunded
    Given the hold expired and the dates were booked on Airbnb meanwhile
    When the provider confirms the payment
    Then the payment is refunded in full automatically
    And the owners are alerted

  Scenario: F007-S08 Availability cannot be checked
    Given the platforms cannot be reached just before payment
    When a visitor tries to pay
    Then online booking is suspended for those dates with a "contactez-nous" message
    And the owners are alerted

  Scenario: F007-S09 Rules are frozen on the booking
    Given a booking made with a 50 % deposit
    When the owners change the deposit to 30 %
    Then the existing booking keeps 50 %

  Scenario: F007-S10 Terms must be accepted
    When a visitor submits the form without accepting the rental terms
    Then the booking is refused before any payment
```

## Out of scope

* Balance reminders and automatic cancellation (F008), last minute (F009), cancellation and refunds (F010), tourist tax rates (F011), invoices and VAT (F012), messages content (F016), cancellation insurance (later, `none` by default).

## Tests

| Scenario | Test |
|---|---|
| F007-S01 → S10 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
