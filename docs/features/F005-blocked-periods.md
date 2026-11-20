---
id: F005
title: "Blocked periods: closure, family, private event"
status: draft
version: 1
adr: [ADR-0007, ADR-0010, ADR-0003]
---

# F005 - Blocked periods: closure, family, private event

## User story

As an owner,
I want to close dates from the admin (works, holidays, family staying, private event),
so that nobody can book them on the site, Airbnb or Booking, without paying or sending anything.

## Business rules

* A blocked period has a start date, an end date (excluded, like a stay), a **type** `closure` (default), `family` or `private_event`, an optional reason, and for `family` / `private_event` a mandatory **label** (e.g. "Grandparents") and an optional number of people.
* Dates are chosen with the shared `DateRangeField` (calendar + "Du" / "Au" fields kept in sync, validation: end after start, within the booking horizon of the rules).
* Blocked dates are unavailable everywhere: site calendar, quote refused, iCal export and channel manager. Outside the admin they are only "not available": the label, reason and type never leave the admin (iCal `SUMMARY:Not available`).
* `family` and `private_event` periods create no payment, no email and no contract; they get their own colour in the admin calendar and are counted apart in the statistics (private occupancy, excluded from commercial occupancy and revenue per night, F018).
* **Closed until further notice**: one action blocks every date from a chosen day up to the booking horizon (`booking_rules.reservable_mois`, default 24 months), extended every day by the daily horizon task.
* **Reopen**: deleting a period frees the dates on the site and on the platforms.
* **Conflict warning**: if a booking (direct, Airbnb or Booking) already exists on the chosen dates, the period is refused with the list of conflicting bookings; the owner cancels or moves the booking first (on the platform for imported ones).
* Periods imported from Airbnb / Booking (F014) are shown in the same calendar with their source, read-only, type `closure`.
* Every creation, change and deletion is recorded (who, when, before / after) and is confirmed with the shared confirmation dialog, as it changes what visitors see.
* iCal synchronisation is not instant (platforms read it every 30 minutes to a few hours): the admin reminds the owner to also block an urgent closure by hand on the platforms when no channel manager is connected.

## Acceptance criteria

```gherkin
Feature: Blocked periods

  Background:
    Given the owner is logged in

  Scenario: F005-S01 Close dates
    When the owner closes 10 to 15 March for works
    Then those nights are not available on the public calendar
    And a quote for 12 to 14 March is refused
    And the iCal export contains a "Not available" event for 10 to 15 March

  Scenario: F005-S02 Family stay stays private
    When the owner blocks 1 to 8 August as "family" with the label "Grandparents" and 4 people
    Then the admin calendar shows "Grandparents" in the family colour
    And the public calendar, the iCal export and the channel manager only see "not available"
    And no email, payment or contract is created

  Scenario: F005-S03 Label required for family and private event
    When the owner saves a "private_event" period without a label
    Then the period is refused with a clear message

  Scenario: F005-S04 Conflict with an existing booking
    Given a confirmed booking from 5 to 9 June
    When the owner tries to close 7 to 12 June
    Then the closure is refused
    And the conflicting booking is listed

  Scenario: F005-S05 Closed until further notice
    When the owner closes the cottage until further notice from 1 November
    Then every date from 1 November to the booking horizon is unavailable
    And the next day the newly opened date is also closed

  Scenario: F005-S06 Reopen
    Given 10 to 15 March are closed
    When the owner deletes the closure and confirms
    Then those nights are available again on the site and in the iCal export

  Scenario: F005-S07 Date fields and calendar stay in sync
    When the owner selects 3 to 6 May on the calendar
    Then the "Du" and "Au" fields show 3 and 6 May
    When the owner types 8 May in "Au"
    Then the calendar highlights 3 to 8 May

  Scenario: F005-S08 Admin only
    Given the owner is not logged in
    When someone calls any blocked period endpoint
    Then the response status is 401
```

## Out of scope

* Import of Airbnb / Booking calendars (F014), channel manager sending (F013), hiding a listing on a platform (done on the platform), several cottages (the model allows it, the screens show one).

## Tests

| Scenario | Test |
|---|---|
| F005-S01 → S08 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
