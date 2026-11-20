---
id: F018
title: "Statistics"
status: draft
version: 1
adr: [ADR-0007, ADR-0010]
---

# F018 - Statistics

## User story

As an owner,
I want to see occupancy, revenue, channels, booking lead times and reviews over any period, compared with the previous year,
so that I set my prices and choose my platforms with facts.

## Business rules

* Data: direct bookings, platform stays imported with their amounts (F013) or dates only (F014), blocked periods (F005), reviews (F015), site audience (self-hosted Umami, with events "devis affiché", "paiement commencé", "réservation confirmée").
* **Indicators** (domain `stats`, computed by the API, unit tested): occupancy rate = nights rented / open nights (voluntary closures and private occupancy excluded, private occupancy shown apart); gross / net turnover (stays without tourist tax nor deposit, before / after commission); average price per night per channel; revenue per open night; split by channel (bookings, nights, turnover, commissions); average stay length and guests; booking lead time; cancellation rate per channel; review scores per channel /10 and their trend; site conversion (visits → bookings, abandoned payments); guest countries and returning guests.
* A night belongs to the **month it takes place in** (a stay over two months is split).
* Filters: free period, month, year, channel, cottage; **year-over-year comparison**. Amounts in cents, rounded only when shown. Revenue shown including VAT, and excluding VAT when liable (F012).
* Without channel manager amounts, statistics are limited to occupancy and the screen says so; planned net prices (F006) are labelled "prévu", real imported amounts are the reference.
* **History import**: CSV exports of Airbnb (transactions) and Booking (statements) for the years before the site, without duplicates (formats checked with real files).
* CSV export of the figures. Retired platforms stay in the history (filter "plateformes arrêtées").
* **GDPR**: guest data kept for a configurable duration (default 5 years), then anonymised; the statistics remain.

## Acceptance criteria

```gherkin
Feature: Statistics

  Scenario: F018-S01 Occupancy excludes closures and private stays
    Given in June 30 nights, 5 closed for works, 4 family nights and 14 nights rented
    Then the occupancy rate of June is 14 / 21
    And the private occupancy shows 4 nights apart

  Scenario: F018-S02 Stay over two months
    Given a stay from 29 June to 3 July
    Then 2 nights count in June and 2 in July

  Scenario: F018-S03 Revenue without taxes nor deposit
    Given a stay of 520 € with 12 € of tourist tax and a 200 € deposit
    Then its turnover is 520 €

  Scenario: F018-S04 Split by channel with commissions
    Given an Airbnb stay of 600 € with 93 € commission and a direct stay of 500 €
    Then the channel split shows gross and net turnover per channel

  Scenario: F018-S05 Year-over-year comparison
    When the owner compares July with July of the previous year
    Then each indicator shows both values and the difference

  Scenario: F018-S06 History import without duplicates
    When the owner imports the same Airbnb CSV twice
    Then each stay exists once

  Scenario: F018-S07 Anonymisation after the retention period
    Given a guest whose last stay ended more than 5 years ago
    When the retention task runs
    Then their name and contact details are removed
    And the statistics are unchanged
```

## Out of scope

* Forecasting and dynamic pricing, accounting (exports only).

## Tests

| Scenario | Test |
|---|---|
| F018-S01 → S07 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
