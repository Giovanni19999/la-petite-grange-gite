---
id: F010
title: "Cancellation and refunds"
status: draft
version: 1
adr: [ADR-0005, ADR-0006, ADR-0010]
---

# F010 - Cancellation and refunds

## User story

As a guest,
I want to know exactly what is refunded if I cancel,
so that I book with confidence.

As an owner,
I want refunds to follow the policy accepted by the guest, with one click for online payments,
so that cancellations are fair and quick to handle.

## Business rules

* The cancellation policy is **copied onto the booking** when it is made (F007); later changes never apply to it.
* Defaults (owner-editable settings, none in the code): cancelled by the guest **≥ 30 days** before arrival → deposit refunded at `remboursement_acompte_pct` (default 100 %) and the balance if already paid; **< 30 days** → 100 % due, nothing refunded; last-minute bookings have no free period.
* Deadlines are computed in Europe/Paris time (`CancellationPolicy::refund_for(paid, cancelled_at, arrival)`), tested at the limits (the day before, the last day at 23:59 and 00:00) with several rule sets.
* The tourist tax (paid on arrival) never enters the calculation.
* Cancelled by the owners → 100 % refunded.
* **Commercial gesture**: the owners can refund more (partial or total) by hand; recorded in the history.
* Online payment refund: button in the admin → provider API (`PaymentGateway::refund`); payment made with a previous provider: refunded through it if it is kept in "refunds only" mode, otherwise the admin shows the link to do it on the provider's dashboard and the owners note it. Offline payment: the owners make the transfer and note it.
* Cancellation releases the dates on every channel and cancels the scheduled messages (F016); the guest and the owners are informed.
* Guests ask to cancel through the contact details given in the confirmation (no self-service cancellation button in v1); the owners cancel from the admin.

## Acceptance criteria

```gherkin
Feature: Cancellation and refunds

  Background:
    Given the policy: free cancellation until 30 days before, deposit refunded at 100 %
    And a booking arriving on 20 August with a 260 € deposit paid online

  Scenario: F010-S01 Early cancellation refunds the deposit
    When the owners cancel at the guest's request on 15 July
    Then 260 € are refunded through the provider
    And the dates are available again everywhere

  Scenario: F010-S02 Late cancellation refunds nothing
    Given the balance was paid
    When the owners cancel at the guest's request on 1 August
    Then nothing is refunded

  Scenario: F010-S03 Limit at midnight Paris time
    When the guest cancels on 21 July at 23:59 Paris time
    Then the deposit is refunded
    When the guest cancels on 22 July at 00:00 Paris time
    Then nothing is refunded

  Scenario: F010-S04 Cancelled by the owners
    When the owners cancel on their side
    Then everything paid is refunded

  Scenario: F010-S05 Policy frozen on the booking
    Given the owners change the free period to 15 days
    When the guest of the existing booking cancels 20 days before arrival
    Then the 30-day policy of the booking applies

  Scenario: F010-S06 Commercial gesture
    When the owners refund 100 € more by hand
    Then the refund and its author are recorded in the history

  Scenario: F010-S07 Offline refund noted
    Given the balance was paid by transfer
    When the owners note the refund transfer
    Then the payment is marked "rembourse"
```

## Out of scope

* Cancellation insurance (later, `none` by default), self-service cancellation by the guest, cancellations of Airbnb / Booking stays (handled on the platforms, imported by F013 / F014).

## Tests

| Scenario | Test |
|---|---|
| F010-S01 → S07 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
