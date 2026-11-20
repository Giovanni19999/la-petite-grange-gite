---
id: F006
title: "Prices per date range and per channel (listed / net), default price"
status: draft
version: 1
adr: [ADR-0008, ADR-0010, ADR-0007]
---

# F006 - Prices per date range and per channel (listed / net), default price

## User story

As an owner,
I want to set by hand, for each date range and each channel (site, Airbnb, Booking…), the price the guest pays and the price I keep,
so that I can be cheaper on my own site and still know what each night really brings me.

## Business rules

* **Default price** per channel covers every date without a range (seed: 100 € per night kept, site listed 100 €; Airbnb / Booking listed price to be typed by the owners).
* **Date ranges** are free (no imposed seasons), chosen with the shared `DateRangeField`; optional label ("Été", "Noël"); a minimum number of nights per range, identical on every channel.
* A new range **overwrites** the ranges it overlaps (they are cut or removed): there is never an overlap in the database, so the price of a night is never ambiguous (domain `RateCalendar::apply`, checked again in a transaction).
* For each range (and the default price) and each active channel, **two prices typed by hand** (ADR-0008): **listed** (what the guest sees and pays on that channel, sent to the platform and used by the site quote) and **net** (what the owners keep). No automatic calculation is ever applied.
* Both prices are required; net ≤ listed. Helper only: when one is typed, the other shows a greyed **suggestion** computed with the channel's commission settings (commission %, payment fees %, VAT on commission); it is saved only if the owner types it or clicks "reprendre la suggestion". A non-blocking warning appears if the implicit commission differs from the known rate by more than the threshold (seed 5 points). All rates and thresholds are owner-editable settings ("Plateformes" screen), never in the code.
* **Cleaning fee**: one fixed amount per stay, the same on every channel, never multiplied by the number of nights.
* A date open for booking without a price for an active channel raises an **alert** and that channel cannot be booked on that date (no invented price).
* The admin shows, per channel, the total of a typical stay (2 and 7 nights, cleaning included) listed and net, to compare channels.
* Tools: apply the same prices to several ranges; copy the year to the next one (same weekdays); copy one channel's prices to another once (editable afterwards).
* Every price change is kept in a history (who, when, range, old → new). Changes are confirmed with the shared confirmation dialog.
* Prices are sent to the platforms only through a channel manager (F013); without one, they only apply to the site and the admin says so.

## Acceptance criteria

```gherkin
Feature: Prices per date range and per channel

  Background:
    Given the owner is logged in
    And the channels site, Airbnb and Booking are active

  Scenario: F006-S01 Default price
    When the owner sets the default price of the site to 80 € listed and 79 € net
    Then a night without range costs 80 € in the site quote

  Scenario: F006-S02 Range with prices per channel
    When the owner creates a range from 6 July to 31 August with 2 nights minimum
    And sets site 110 € / 108 €, Airbnb 130 € / 110 €, Booking 130 € / 109 €
    Then a site quote for 10 to 12 July uses 110 € per night
    And the admin calendar shows the listed and net price of each channel for those nights

  Scenario: F006-S03 A new range cuts the ranges it overlaps
    Given a range from 1 to 31 July
    When the owner creates a range from 10 to 20 July
    Then the ranges are 1 to 10 July, 10 to 20 July and 20 to 31 July
    And no two ranges overlap

  Scenario: F006-S04 Suggestion never applied by itself
    When the owner types 110 € net for Booking
    Then the listed field shows a greyed suggestion
    And saving without typing the listed price is refused as incomplete

  Scenario: F006-S05 Net above listed is refused
    When the owner saves 100 € listed and 120 € net
    Then the prices are refused with a clear message

  Scenario: F006-S06 Missing price blocks the channel, not the site
    Given Booking has no price for 1 to 5 October
    Then the admin shows a "prix incomplet" alert for Booking
    And those nights cannot be booked through Booking
    And they can still be booked on the site if the site has a price

  Scenario: F006-S07 Cleaning fee is fixed per stay
    Given the cleaning fee is 60 €
    When a visitor asks a quote for 7 nights at 100 €
    Then the stay price is 760 €

  Scenario: F006-S08 Minimum nights
    Given the range of 15 July requires 2 nights
    When a visitor asks a quote arriving on 15 July for 1 night
    Then the quote is refused with the minimum stay

  Scenario: F006-S09 Copy to next year
    When the owner copies the 2027 ranges to 2028
    Then each range exists in 2028 on the same weekdays with the same prices

  Scenario: F006-S10 History
    When the owner changes a price
    Then the history shows who changed it, when, and the old and new prices
```

## Out of scope

* Dynamic pricing, promotions and coupons, sending prices to the platforms (F013), taxes (F011, F012), the public calendar display (F004).

## Tests

| Scenario | Test |
|---|---|
| F006-S01 → S10 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
