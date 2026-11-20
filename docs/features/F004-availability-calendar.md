---
id: F004
title: "Availability calendar with the nightly price under each free date"
status: draft
version: 1
adr: [ADR-0005, ADR-0008, ADR-0010, ADR-0009]
---

# F004 - Availability calendar with the nightly price under each free date

## User story

As a visitor,
I want to see which nights are free and their price, then pick my dates and get an exact quote,
so that I know at once whether I can come and how much it costs.

## Business rules

* The public calendar shows the months up to the booking horizon (`booking_rules.reservable_mois`, default 24 months, and the optional fixed date; the nearest applies), starting with the current month.
* Under each **free** night: its **site listed price** (F006). Booked, blocked (F005, any type), held for payment (F007) or imported dates (F014) are shown as unavailable, without any detail.
* Nights before the minimum booking delay (`delai_min_jours`, default 1) or beyond the horizon are unavailable.
* Choosing an arrival then a departure (or typing them in the "Arrivée" / "Départ" fields, kept in sync with the calendar) and the number of guests asks the API for a **quote**: nights, fixed cleaning fee, stay total, deposit and balance with their due dates, and apart the tourist tax to pay on arrival (F011). Prices are computed **only by the API**.
* The quote is refused with a clear reason when: a night is unavailable, the minimum stay of the arrival night is not met, the number of guests exceeds the capacity (default 5, no "babies" category), the arrival or departure weekday is not allowed, or a night has no site price.
* Number of months shown at once on the page is an option of the "Disponibilités" block (1 to 3); on phones one month at a time with swipe or buttons; keyboard accessible (arrow keys move between days, Enter selects).
* The calendar is rendered on the server for the first month and refreshed in the browser; data are cached for a short time (setting) and invalidated on any change of prices, closures or bookings.
* A "Réserver" button under a valid quote leads to the booking flow (F007); until F007 is live it leads to the contact form (F017) with the dates filled in.

## Acceptance criteria

```gherkin
Feature: Availability calendar

  Scenario: F004-S01 Free nights show their price
    Given the site price of 12 July is 110 €
    When a visitor opens the "Tarifs & disponibilités" page
    Then 12 July is shown free with "110 €" under it

  Scenario: F004-S02 Unavailable nights show no detail
    Given 1 to 8 August is a family period and 10 to 14 August is booked on Airbnb
    When a visitor opens the calendar
    Then those nights are shown unavailable without label, source or price

  Scenario: F004-S03 Exact quote
    Given the site price is 100 € per night and the cleaning fee is 60 €
    When a visitor chooses 3 nights for 2 adults
    Then the quote shows 300 € of nights, 60 € of cleaning, a stay total of 360 €
    And the deposit and balance with their due dates
    And the tourist tax to pay on arrival, apart from the total

  Scenario: F004-S04 Refused quotes explain why
    When a visitor chooses dates including an unavailable night
    Then the quote is refused with "dates indisponibles"
    When a visitor chooses 1 night where the minimum is 2
    Then the quote is refused with the minimum stay
    When a visitor asks for 6 guests
    Then the quote is refused with the capacity of 5

  Scenario: F004-S05 Booking horizon
    When a visitor goes to the month 25 months from now
    Then every night of that month is unavailable

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
```

## Out of scope

* Payment and booking (F007), prices per channel on the public site (only the site price is shown), several cottages on the same calendar.

## Tests

| Scenario | Test |
|---|---|
| F004-S01 → S08 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
