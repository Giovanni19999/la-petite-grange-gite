---
id: F011
title: "Tourist tax (3-star rate) paid on arrival"
status: draft
version: 1
adr: [ADR-0010]
---

# F011 - Tourist tax (3-star rate) paid on arrival

## User story

As an owner,
I want the tourist tax to be computed with the rate of my classification, collected on arrival and summarised for the town hall,
so that I declare the right amount without calculations.

## Business rules

* The cottage is a **classified 3-star furnished tourist accommodation**: classification and its decision date (valid 5 years, renewal alert), registration number if the town requires one (shown in the legal notice).
* Tax per **adult** and per **night** at the rate voted by the town for the category (owner-editable, **dated** to keep the history), plus the departmental additional tax percentage if voted; minors exempted under an editable age (default 18).
* Shown **apart** in the quote, the contract and the emails; never in the stay price, the deposit, the balance or the refunds.
* For direct bookings, a payment line of type `taxe_sejour`, due on the arrival day, with the accepted methods (editable: cash, cheque, payment link), ticked "reçue" by the owners; amount recalculated if the number of adults changes.
* Per platform, `taxe_collectee_par_plateforme` (default Airbnb yes, Booking no, to be checked for the town): if yes, nothing to collect on arrival but the tax still appears in the town-hall summary as "collectée par Airbnb".
* **Town-hall summary** for a period: nights, adults, amounts received / collected by a platform / not received, CSV export.
* Rates and amounts in cents; a rate change applies to bookings made after its start date.

## Acceptance criteria

```gherkin
Feature: Tourist tax

  Background:
    Given the 3-star rate is 1.70 € per adult and night with 10 % additional tax

  Scenario: F011-S01 Tax computed per adult and night
    When a stay of 3 nights for 2 adults and 1 child is quoted
    Then the tourist tax is 11.22 €
    And it is shown apart from the stay total

  Scenario: F011-S02 Paid on arrival
    Given a direct booking
    Then a tourist tax payment of 11.22 € is expected on the arrival day
    When the owners tick it received in cash
    Then it is marked received

  Scenario: F011-S03 Collected by a platform
    Given Airbnb collects the tax
    When an Airbnb stay is imported
    Then nothing is expected on arrival
    And the town-hall summary shows the tax as "collectée par Airbnb"

  Scenario: F011-S04 Dated rates
    Given the rate becomes 1.90 € on 1 January
    Then bookings made before 1 January keep 1.70 €

  Scenario: F011-S05 Town-hall summary
    When the owners ask the summary for the 2nd quarter
    Then it lists nights, adults and amounts received, collected by a platform and not received
    And it can be exported as CSV

  Scenario: F011-S06 Classification renewal
    Given the classification was decided 4 years and 11 months ago
    Then the admin shows a renewal alert
```

## Out of scope

* Paying the tax to the town (done by the owners), other classifications than the one configured (the model keeps the setting).

## Tests

| Scenario | Test |
|---|---|
| F011-S01 → S06 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
