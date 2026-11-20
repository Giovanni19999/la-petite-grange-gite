---
status: accepted
date: 2026-11-20
---

# Channel manager selected by configuration, iCal as permanent fallback

## Context and Problem Statement

Airbnb and Booking.com APIs are reserved to certified partners. Prices per platform, near-instant availability and booking amounts require a channel manager; the choice (Beds24 is the likely candidate, to be confirmed after a trial) may change, and a platform may be connected, disconnected or retired at any time.

## Decision Drivers

* Different prices per platform
* Near-instant availability to limit double bookings
* Keep working (degraded) when a platform or the service is disconnected

## Considered Options

* Ports (`RatePublisher`, `AvailabilityPublisher`, `BookingImporter`, `ReviewImporter`, `ListingPublisher`, `ChannelConnectionMonitor`) + one adapter per service, one active in `config/channel_manager.toml`, iCal adapter always available
* iCal only
* Scraping platforms (rejected: forbidden by their terms)

## Decision Outcome

Chosen option: "Ports + adapter per service, iCal fallback".

* Effective features = service capabilities ∩ platform status (`active` / `retiring` / `inactive`) ∩ live connection status; disabled features are shown greyed out with an explanation.
* Changes made during a disconnection are queued and resent on reconnection.
* `dry_run = true` logs instead of sending (development, trials).

### Consequences

* Good, because the service can be swapped by configuration
* Bad, because listing texts and photos cannot be pushed by affordable services: "assisted mode" (copy + direct link)

### Confirmation

Unit tests of `EffectiveFeatures::compute`; adapter tests against recorded responses (`wiremock`).
