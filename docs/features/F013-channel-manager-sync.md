---
id: F013
title: "Channel manager synchronisation, connection monitoring, platform retirement"
status: draft
version: 1
adr: [ADR-0007, ADR-0008, ADR-0010]
---

# F013 - Channel manager synchronisation, connection monitoring, platform retirement

## User story

As an owner,
I want my prices, minimum stays, closures and bookings to be synchronised with Airbnb and Booking through one service, and to be warned when a platform is disconnected,
so that every channel stays up to date and I can stop or add a platform without losing anything.

## Business rules

* **One channel manager active**, chosen in `config/channel_manager.toml` (`beds24` | `smoobu` | `channex` | `ical`), secrets in `.env` only; the API refuses to start with an unknown service, a missing block or secret (ADR-0007). `dry_run = true` logs what would be sent without sending (default until the service is chosen).
* The code only knows ports (`AvailabilityPublisher`, `RatePublisher`, `BookingImporter`, `ReviewImporter`, `ListingPublisher`, `ChannelCapabilities`, `ChannelConnectionMonitor`); one adapter per service; changing service changes no screen and no domain rule.
* **Sent automatically** after each change and on demand ("Envoyer les prix"): availability (bookings, holds, blocked periods, booking horizon), listed price per day and per channel (F006), minimum nights, cleaning fee. Status per channel: sent / pending / error with a clear message.
* **Imported**: platform bookings with amounts, commission, promotion, guests, country, booking date and cancellation; idempotent upsert on `(source, external_id)`; changes recorded in the booking history; imported data kept even if the service changes.
* **Effective features** = service capabilities ∩ platforms `active` (or `en_retrait`: closures and import only) ∩ platforms connected (`EffectiveFeatures::compute`, unit tested). The admin greys out what is not available, with the reason and a link to fix it.
* **Connection monitoring** every `interval_minutes` (default 15), from the service API, failed sends, webhooks and token expiry; states `Connected` / `Disconnected` / `Error` / `Unknown`, with history. Alert to the owners (email + admin banner) after `alert_after_minutes` (default 30), and when it is back; one alert per outage.
* **While disconnected**: that platform's prices, minimum nights, messages and reviews are kept "à envoyer"; its bookings are imported with iCal (F014); before each direct payment availability is re-checked with a fresh iCal download (F007). **Back online**: full resynchronisation of that platform, then import of the missed bookings, result shown in the admin.
* **Platform life cycle** (admin, never in the code): `active` → `en_retrait` (all future dates closed on the platform, existing stays honoured and still imported, its buttons removed from the site) → `inactive` automatically after the last departure; summary screen before retiring (upcoming stays, last departure, steps to do on the platform); nothing is deleted; reactivation finds the prices and listing content back. There is always the `direct` channel.
* Admin header shows a light per platform (connected / error / disconnected / not used).

## Acceptance criteria

```gherkin
Feature: Channel manager synchronisation

  Background:
    Given a channel manager is configured with Airbnb and Booking connected

  Scenario: F013-S01 Prices and closures are sent
    When the owner changes the Booking price of 12 July and closes 20 to 22 July
    Then the new price and the closure are sent to the channel manager
    And the status of each channel shows "envoyé"

  Scenario: F013-S02 Dry run sends nothing
    Given dry_run is on
    When the owner changes a price
    Then the change is only logged

  Scenario: F013-S03 Bookings imported once
    When the channel manager reports a Booking stay with its amount and commission
    Then the stay appears in the admin with its amounts
    When the same stay is reported again with new dates
    Then the existing stay is updated, not duplicated, and the change is in its history

  Scenario: F013-S04 Disconnected platform
    Given Booking has been disconnected for 31 minutes
    Then the owners receive one alert
    And Booking prices are greyed out with the reason and kept "à envoyer"
    And Booking stays are imported with iCal meanwhile

  Scenario: F013-S05 Back online
    When Booking is connected again
    Then a full resynchronisation of Booking is sent
    And the owners are told the connection is back

  Scenario: F013-S06 Retire a platform with upcoming stays
    Given two Booking stays are still to come
    When the owner retires Booking and confirms the summary
    Then every future date is closed on Booking
    And the two stays stay valid and imported
    And Booking becomes inactive after the last departure

  Scenario: F013-S07 Invalid configuration
    Given provider = "beds24" without its secret
    Then the API refuses to start with a clear message
```

## Out of scope

* Listing content publication (F019, assisted mode), reviews import (F015), iCal itself (F014), choosing the service (owners, after a trial).

## Tests

| Scenario | Test |
|---|---|
| F013-S01 → S07 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
