---
id: F014
title: "iCal import / export"
status: draft
version: 1
adr: [ADR-0007]
---

# F014 - iCal import / export

## User story

As an owner,
I want the Airbnb and Booking calendars to be read by the site and the site's calendar to be read by them,
so that dates booked anywhere are closed everywhere, even without a channel manager.

## Business rules

* **Import**: one iCal source per platform (Airbnb, Booking), its private URL stored in `.env` or in the admin, **never shown publicly**; read every `poll_minutes` (default 15) and on demand ("Synchroniser maintenant"), and fresh just before each direct payment (F007).
* Imported events become unavailable periods of source `airbnb` / `booking`, type `closure`, read-only; an event that disappears from the feed frees its dates; events are matched by their UID so a re-import never duplicates them.
* iCal only carries dates: no guest name nor amount (F013 brings them when a channel manager is used).
* **Export**: a secret URL `https://<domain>/api/logements/<slug>/calendar/<token>.ics` (shown in the admin, token can be regenerated) lists every unavailable night (bookings, holds, blocked periods of any type, nights beyond the booking horizon) as "Not available": no name, label, type or price ever leaves.
* The export is valid iCalendar (RFC 5545), all-day events, end date excluded, UTF-8.
* Last successful read and errors are shown per source; an alert is sent when a source fails for longer than a setting (default 2 hours).
* The admin explains the delay of iCal (platforms read it every 30 minutes to a few hours) and the steps to paste the export URL into Airbnb and Booking.

## Acceptance criteria

```gherkin
Feature: iCal import / export

  Scenario: F014-S01 Airbnb booking imported
    Given the Airbnb feed contains a booking from 10 to 14 July
    When the import runs
    Then 10 to 13 July are unavailable on the site, source "airbnb"

  Scenario: F014-S02 Cancelled on the platform
    Given that booking disappears from the Airbnb feed
    When the import runs again
    Then the nights are available again

  Scenario: F014-S03 No duplicates
    When the same feed is imported twice
    Then each event exists once

  Scenario: F014-S04 Export hides every detail
    Given a family period labelled "Grandparents" and a confirmed direct booking
    When a platform reads the export URL
    Then both appear only as "Not available" all-day events

  Scenario: F014-S05 Secret export URL
    When someone requests the export with a wrong token
    Then the response status is 404
    When the owner regenerates the token
    Then the old URL stops working

  Scenario: F014-S06 Failing source
    Given the Booking feed cannot be read for more than 2 hours
    Then the owners are alerted and the admin shows the last successful read
```

## Out of scope

* Amounts and guest details of platform stays (F013), other platforms than those configured.

## Tests

| Scenario | Test |
|---|---|
| F014-S01 → S06 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
