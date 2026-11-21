---
id: F006
title: "Prices per date range and per channel (listed / net), default price"
status: frozen
version: 2
adr: [ADR-0002, ADR-0003, ADR-0007, ADR-0008, ADR-0010, ADR-0011, ADR-0018]
---

# F006 - Prices per date range and per channel (listed / net), default price

## User story

As an owner,
I want to set by hand, for each date range and each channel (site, Airbnb, Booking), the price the guest pays and the price I keep,
so that I can be cheaper on my own site and still know what each night really brings me.

## Business rules

* **Channels**: `direct` (the site, always present), `airbnb`, `booking`, seeded. Each has owner-editable settings used only to help typing: commission % (seed Airbnb 15.5, Booking 15), payment fees % (seed Booking 1.4), VAT on the commission yes / no (seed yes). Effective rate `t = (commission + fees) × (1 + 0.20 if VAT)`. Activating or retiring a platform belongs to F013.
* **Prices are set by the owners** (ADR-0008, ADR-0018), in euros with cents, stored in cents: for each channel, a **listed** price (what the guest sees and pays on that channel) and a **net** price (what the owners keep). Both are required when a channel price is saved, and net ≤ listed. No calculation is ever applied to them.
* **Default price** per channel: covers every night without range. Seed: site 100 € listed / 100 € net; Airbnb and Booking have none until the owners type them.
* **Date ranges**: start (first night) and end (excluded), chosen with the shared `DateRangeField`; optional label ("Été", "Noël", at most 60 characters); minimum number of nights (1 to 30), the same on every channel; and the prices of the channels the owners want to change on that range.
* **Price of a night for a channel** = the price of that channel in the range covering the night, otherwise the channel's default price; if there is none, the channel is **incomplete** on that night: the admin shows a "prix incomplet" alert and that channel cannot be booked on those nights (never an invented price). Minimum nights of a night = the range's, otherwise the cottage default (seed 2).
* A new range **overwrites** the ranges it overlaps: they are cut or removed (cases: inside, overlapping on the left or the right, covering, adjacent), so ranges never overlap. Changing a range's dates applies the same rule; deleting a range gives its nights back to the default prices.
* **Cleaning fee**: one fixed amount per stay for the cottage (seed 0 €), the same on every channel, never multiplied by the nights.
* **Fields fill each other** (ADR-0018): typing the listed price fills the net price with `listed × (1 − t)`, typing the net price fills the listed price with `net / (1 − t)`, rounded to the euro (site: both equal). Both fields stay editable and the field typed last drives the other. What is saved is exactly what the fields show; a later change of a channel's rate never recomputes stored prices. A non-blocking warning appears when the implicit commission `(listed − net) / listed` differs from `t` by more than 5 points (seed, owner-editable).
* **Validation required**: the price table of the defaults and the table of each range have their own **"Valider"** button; nothing is saved before it (unsaved changes are marked), and it saves the whole table at once (every channel, one history line).
* **Typical stays**: for the default prices and each range, the total of a 2-night and a 7-night stay per channel (nights + cleaning), listed and net (net cleaning estimated with the channel's rate for platforms).
* **Tools**: copy the ranges of a year to the next one on the same weekdays (52 weeks later, overwriting what is there); copy every price of one channel to another once (editable afterwards).
* **History** of every change (who, when, what, before / after).
* Every change that alters prices visitors pay is confirmed with a dialog. Prices apply to the site at once; sending them to Airbnb and Booking needs a channel manager (F013), and the screen says so.
* All price endpoints are admin only (F001).

## Acceptance criteria

```gherkin
Feature: Prices per date range and per channel

  Background:
    Given the owner is logged in

  Scenario: F006-S01 Default price of the site
    When the owner sets the default site price to 80 € listed and 79 € net
    Then a night without range costs 80 € on the site

  Scenario: F006-S02 Range with prices per channel
    When the owner creates a range from 6 July to 31 August 2027 named "Été" with 2 nights minimum
    And sets site 110 € / 108 €, Airbnb 130 € / 110 €, Booking 130 € / 109 €
    Then the night of 10 July costs 110 € on the site, 130 € on Airbnb and on Booking
    And the night of 5 July uses the default prices

  Scenario: F006-S03 A new range cuts the ranges it overlaps
    Given a range from 1 to 31 July
    When the owner creates a range from 10 to 20 July
    Then the ranges are 1 to 10 July, 10 to 20 July and 20 to 31 July

  Scenario: F006-S04 Listed and net fill each other
    Given Booking has a 15 % commission, 1.4 % fees and VAT on the commission
    When the owner types 110 € net for Booking
    Then the listed price is filled with 137 €
    When the owner types 140 € listed for Booking
    Then the net price is filled with 112 €
    When the owner corrects the net price to 115 € and saves
    Then Booking is saved at 140 € listed and 115 € net

  Scenario: F006-S13 A new rate never changes stored prices
    Given Booking is saved at 140 € listed and 115 € net
    When the owner changes the Booking commission to 17 %
    Then Booking prices stay 140 € and 115 €

  Scenario: F006-S05 Net above listed is refused
    When the owner saves 100 € listed and 120 € net
    Then the price is refused with a clear message

  Scenario: F006-S06 Missing price makes a channel incomplete
    Given Booking has no default price
    Then the admin shows a "prix incomplet" alert for Booking
    And a night without Booking range price has no Booking price

  Scenario: F006-S07 Cleaning fee and typical stays
    Given the cleaning fee is 60 € and the site default price is 100 €
    Then a 7-night stay on the site is shown at 760 €

  Scenario: F006-S08 Minimum nights
    Given the range of 15 July requires 3 nights and the default is 2
    Then the night of 15 July shows 3 nights minimum and the night of 1 June 2

  Scenario: F006-S09 Copy to next year
    Given a range from 6 July to 31 August 2027
    When the owner copies 2027 to 2028
    Then a range from 4 July to 29 August 2028 exists with the same prices

  Scenario: F006-S10 Copy a channel
    When the owner copies the site prices to Airbnb
    Then Airbnb has the site prices on the default and on every range

  Scenario: F006-S11 History
    When the owner changes a price
    Then the history shows who changed it, when, and the old and new prices

  Scenario: F006-S12 Admin only
    Given the owner is not logged in
    When someone calls any price endpoint
    Then the response status is 401
```

## Out of scope

* Sending prices to the platforms and activating / retiring them (F013), quotes and the public calendar (F004), applying one price to several ranges at once, dynamic pricing, promotions and coupons, taxes (F011, F012).

## Tests

| Scenario | Test |
|---|---|
| F006-S01 → S13 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
* v1 frozen (2026-11-21): channel settings editable here, fallback of a range to the default price of a channel, seed values, bulk apply left out.
* v2 (2026-11-21): listed and net prices fill each other as they are typed, both editable (ADR-0018, at the owners' request), instead of a greyed suggestion; new scenario S13; mandatory "Valider" button for the default prices and for each range.
