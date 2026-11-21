---
id: F005
title: "Blocked periods: closure, family, private event"
status: frozen
version: 1
adr: [ADR-0002, ADR-0003, ADR-0007, ADR-0010, ADR-0011]
---

# F005 - Blocked periods: closure, family, private event

## User story

As an owner,
I want to close dates from the admin (works, holidays, family staying, private event),
so that nobody can book them, without any payment, email or contract.

## Business rules

* **Cottage**: the site manages one cottage today (seeded, slug `la-petite-grange`); the data model already links periods to a cottage so that a second one needs no code change.
* **Booking horizon**: stored per cottage in the database (`booking_rules`: rolling number of months, seed 24, and an optional fixed last date; the nearest applies). Its edit screen comes with the house rules (F007); no value is written in the code (ADR-0010).
* A blocked period has a **start** date and an **end** date, the end excluded like a stay ("Du" = first night closed, "Au" = the day after the last night closed); the admin shows the closed nights in words ("nuits fermées : du 10 au 14 mars, 5 nuits").
* **Type**: `closure` (default), `family` or `private_event`; optional reason; for `family` / `private_event` a mandatory **label** (e.g. "Grandparents", at most 80 characters) and an optional number of people (1 to 99).
* **Closed until further notice**: a period without end date; every night from its start is unavailable, whatever the horizon.
* Periods of the same cottage **never overlap**: an overlapping period is refused with the period it overlaps.
* **Conflict with stays**: a period is refused when a stay (direct or imported booking) already uses one of its nights, with the list of stays. Stays come from a port; until bookings exist (F007, F014) there are none, and the rule is tested with a fake.
* **Availability** (domain service, used by the quote and the public calendar later): a night is unavailable when a blocked period, a stay or a hold covers it, or when it is beyond the booking horizon. `GET /api/availability?from=&to=` returns the unavailable nights of the cottage **without any detail** (no type, label, reason, people): visitors only ever see "unavailable".
* Dates are plain calendar dates of the cottage (Europe/Paris); the range asked to the availability endpoint is limited (setting, default 400 days).
* Admin screen **"Fermetures"**: month calendar showing periods by colour (closure, family and private event in the private colour), list of upcoming and current periods, form with the shared **`DateRangeField`** (calendar + "Du" / "Au" date fields kept in sync both ways, validation as you type: end after start), type, label, people, reason, "jusqu'à nouvel ordre". Edit and delete.
* Creating, changing or deleting a period changes what visitors can book: it is confirmed with a dialog summarising the nights concerned.
* Every creation, change and deletion is kept in a **history** (who, when, before / after).
* The screen reminds that, without a channel manager, the platforms take from 30 minutes to a few hours to read the calendar (F014): an urgent closure should also be done on Airbnb / Booking by hand.
* All admin endpoints require the owner's session (F001).

## Acceptance criteria

```gherkin
Feature: Blocked periods

  Background:
    Given the owner is logged in

  Scenario: F005-S01 Close dates
    When the owner closes from 10 to 15 March for works and confirms
    Then the nights of 10 to 14 March are unavailable
    And the night of 15 March is available

  Scenario: F005-S02 Family stay stays private
    When the owner blocks from 1 to 8 August as "family" with the label "Grandparents" and 4 people
    Then the admin calendar shows "Grandparents" in the private colour
    And the public availability only lists those nights as unavailable, without label, type or people

  Scenario: F005-S03 Label required for family and private event
    When the owner saves a "private_event" period without a label
    Then the period is refused with a clear message

  Scenario: F005-S04 Conflict with an existing stay
    Given a stay from 5 to 9 June
    When the owner tries to close from 7 to 12 June
    Then the closure is refused and the stay is listed

  Scenario: F005-S05 Closed until further notice
    When the owner closes the cottage until further notice from 1 November
    Then every night from 1 November is unavailable, even years later

  Scenario: F005-S06 Overlapping periods are refused
    Given a closure from 10 to 15 March
    When the owner adds a family period from 14 to 20 March
    Then it is refused and the existing closure is named

  Scenario: F005-S07 Change and reopen
    Given a closure from 10 to 15 March
    When the owner changes it to 10 to 12 March and confirms
    Then the nights of 12 to 14 March are available again
    When the owner deletes it and confirms
    Then every night of March is available
    And the history shows the creation, the change and the deletion with their author

  Scenario: F005-S08 Booking horizon
    Given the booking horizon is 24 months
    Then the nights beyond 24 months from today are unavailable

  Scenario: F005-S09 Date fields and calendar stay in sync
    When the owner selects 3 to 6 May on the calendar
    Then the "Du" and "Au" fields show 3 and 6 May
    When the owner types 8 May in "Au"
    Then the calendar highlights 3 to 8 May
    And an "Au" before "Du" is flagged at once

  Scenario: F005-S10 Admin only
    Given the owner is not logged in
    When someone calls any blocked period endpoint
    Then the response status is 401
```

## Out of scope

* iCal export and import of the platforms' calendars (F014), sending closures to a channel manager (F013), quotes and the public calendar (F004), the house rules screen (F007), several cottages in the screens (the model allows it), hiding a listing on a platform (done on the platform).

## Tests

| Scenario | Test |
|---|---|
| F005-S01 → S10 | to be filled in when implemented |

## Changelog

* v1 (2026-11-21): first frozen version (cottage and booking horizon foundation, open-ended closure, availability endpoint; iCal moved to F014).
