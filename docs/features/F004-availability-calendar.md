---
id: F004
title: "Availability calendar with the nightly price under each free date"
status: implemented
version: 2
adr: [ADR-0005, ADR-0008, ADR-0009, ADR-0010]
---

# F004 - Availability calendar with the nightly price under each free date

## User story

As a visitor,
I want to see which nights are free and their price, then pick my dates and get an exact quote,
so that I know at once whether I can come and how much it costs.

As an owner,
I want to set the house rules the quote depends on (capacity, minimum delay, arrival and departure days, deposit, balance, tourist tax),
so that the quote shown to visitors is always right without asking a developer.

## Business rules

* **House rules** ("Règles du gîte" admin screen, stored per cottage in `booking_rules`, every change kept in a history with author and time; ADR-0010, no value in the code). F004 adds, with these seed values:
  * capacity: 5 guests (adults and children counted alike, no "babies" category);
  * minimum delay before arrival: 1 day (arrival tomorrow at the earliest);
  * allowed arrival weekdays and departure weekdays: every day;
  * deposit: `percentage` 50 % or `fixed amount` (capped at the stay price);
  * balance due 30 days before arrival;
  * tourist tax: amount per **adult** and per night in euros (seed 0 €, to be set by the owners with the rate of their town), optional additional tax in percent (seed 0 %), children under an age (seed 18) exempted.
  The screen also shows the booking horizon of F005 (rolling months, optional last date) and the minimum stay and cleaning fee of F006 (link to "Tarifs"). Other house rules (times, pets, payment hold, cancellation…) come with F007 and the following features.
* **Public calendar**: a new page block **"Disponibilités"** (F002, F023) with the number of months shown side by side on a computer (1 to 3, default 2); on a phone one month at a time, changed with buttons or by swiping; keyboard accessible (arrow keys move between days, Enter selects).
* It shows the months from the current one up to the booking horizon (F005). Under each **free** night: its **site listed price** (F006). Unavailable nights (blocked periods of any type, and later stays, payment holds and imported dates: F007, F014) are shown unavailable **without any detail**. Nights before the minimum delay, beyond the horizon or without site price are unavailable.
* Choosing an arrival then a departure in the calendar, or typing them in the "Arrivée" / "Départ" fields (kept in sync with the calendar), with the number of adults and of children, asks the API for a **quote**. Prices and amounts are computed **only by the API** (`POST /api/quote`), in cents:
  * nights: sum of the site listed prices of each night;
  * cleaning fee (F006); stay total = nights + cleaning;
  * deposit (mode of the house rules applied to the stay total) and balance = stay total − deposit, with their due dates (deposit at booking, balance on arrival − balance days; if that date is already past, everything is due at booking);
  * tourist tax **apart**, to pay on arrival: adults × nights × amount per adult and night, plus the additional percent, rounded to the cent.
* The quote is refused with a clear French reason when: a night is unavailable ("dates indisponibles"), the stay is shorter than the minimum stay of the arrival night (F006), the guests exceed the capacity, there are no adults, the arrival or departure weekday is not allowed, the arrival is before the minimum delay or beyond the horizon.
* Data are read at each request (no cache): a change of prices, closures or rules is visible at once. The first month is rendered on the server; the quote is asked from the browser.
* **"Réserver"**: under a valid quote, the button leads to the direct booking (F007: instant booking, no approval by the owners, emails to the guest and to the owners through F025). Until F007 is live, the button is not shown.

## Acceptance criteria

```gherkin
Feature: Availability calendar

  Scenario: F004-S01 Free nights show their price
    Given the site price of 12 July is 110 €
    When a visitor opens a page with the "Disponibilités" block
    Then 12 July is shown free with "110 €" under it

  Scenario: F004-S02 Unavailable nights show no detail
    Given 1 to 8 August is a family period
    When a visitor opens the calendar
    Then those nights are shown unavailable without label, type or price

  Scenario: F004-S03 Exact quote
    Given the site price is 100 € per night, the cleaning fee is 60 €
    And the house rules ask for a 50 % deposit, the balance 30 days before arrival
    And a tourist tax of 1.70 € per adult and night with 10 % additional tax
    When a visitor chooses 3 nights for 2 adults and 1 child, more than 30 days ahead
    Then the quote shows 300 € of nights, 60 € of cleaning, a stay total of 360 €
    And a deposit of 180 € due at booking and a balance of 180 € due 30 days before arrival
    And a tourist tax of 11.22 € to pay on arrival, apart from the total

  Scenario: F004-S04 Refused quotes explain why
    When a visitor chooses dates including an unavailable night
    Then the quote is refused with "dates indisponibles"
    When a visitor chooses 1 night where the minimum is 2
    Then the quote is refused with the minimum stay of 2 nights
    When a visitor asks for 6 guests
    Then the quote is refused with the capacity of 5

  Scenario: F004-S05 Booking horizon and minimum delay
    When a visitor goes to the month 25 months from now
    Then every night of that month is unavailable
    And today's night is unavailable with a minimum delay of 1 day

  Scenario: F004-S06 Fields and calendar stay in sync
    When a visitor types arrival 3 May and departure 6 May
    Then the calendar highlights 3 to 6 May and the quote is updated

  Scenario: F004-S07 Phone and keyboard
    Given a 360 px wide screen
    Then one month is shown at a time and can be changed by swiping
    And days can be chosen with the arrow keys and Enter

  Scenario: F004-S08 Changes are visible at once
    When the owner closes 20 to 22 May
    Then the next visitor sees those nights unavailable

  Scenario: F004-S09 House rules
    When the owner sets the capacity to 6 and the tourist tax to 1.70 € on the "Règles du gîte" screen
    Then the next quote accepts 6 guests and shows the new tax
    And the change appears in the history with its author

  Scenario: F004-S10 Weekdays
    Given arrivals are allowed on Saturday only
    When a visitor chooses an arrival on a Wednesday
    Then the quote is refused with the allowed arrival day
```

## Out of scope

* The booking itself, payment and emails (F007, F025, F016), promotions and options by stay length (F026), prices per channel on the public site (only the site price is shown), dated tourist tax rates and the town-hall summary (F011), several cottages on the same calendar.

## Tests

| Scenario | Test |
|---|---|
| F004-S01 | `Back/tests/quote.rs::f004_s01_s02_s08_*`, `Front/src/app/features/public/pages/ui/blocks/availability-block.spec.ts` |
| F004-S02 | `Back/tests/quote.rs::f004_s01_s02_s08_*`, `Front/src/app/features/public/pages/ui/blocks/availability-block.spec.ts` |
| F004-S03 | `Back/src/domain/quote.rs::tests::f004_s03_*`, `Back/src/domain/house_rules.rs::tests::f004_s03_*`, `Back/tests/quote.rs::f004_s03_exact_quote`, `Front/src/app/features/public/pages/ui/blocks/availability-block.spec.ts` |
| F004-S04 | `Back/src/domain/quote.rs::tests::f004_s04_*`, `Back/tests/quote.rs::f004_s04_*`, `Front/src/app/features/public/pages/ui/blocks/availability-block.spec.ts` |
| F004-S05 | `Back/src/domain/quote.rs::tests::f004_s05_*`, `Back/tests/quote.rs::f004_s05_*` |
| F004-S06 | `Front/src/app/features/public/pages/ui/blocks/availability-block.spec.ts` |
| F004-S07 | `Front/src/app/features/public/pages/ui/blocks/availability-block.spec.ts` (swipe checked by hand on a phone) |
| F004-S08 | `Back/tests/quote.rs::f004_s01_s02_s08_*` (no cache: every request reads the database) |
| F004-S09 | `Back/src/domain/house_rules.rs::tests::f004_s09_*`, `Back/tests/quote.rs::f004_s09_*`, `Back/tests/quote.rs::f004_house_rules_are_admin_only`, `Front/src/app/features/admin/house-rules/house-rules.page.spec.ts` |
| F004-S10 | `Back/src/domain/house_rules.rs::tests::f004_s10_*`, `Back/src/domain/quote.rs::tests::f004_s10_*`, `Back/tests/quote.rs::f004_s10_*` |
| "Disponibilités" block | `Back/src/domain/page.rs::tests::f004_availability_block` |

## Changelog

* v1 (2026-11-20): first version (draft).
* v2 (2026-11-21): frozen with the owners' choices: "Règles du gîte" screen now (capacity, minimum delay, weekdays, deposit, balance, tourist tax amount), tourist tax amount in the quote (editable by the owners), "Réserver" leads to the direct booking (F007) and is hidden until then, "Disponibilités" page block, no cache; scenarios S09 and S10.
