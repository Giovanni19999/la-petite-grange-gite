---
id: F026
title: "Promotions and options offered by stay length (optional minimum and maximum nights)"
status: implemented
version: 2
adr: [ADR-0008, ADR-0010]
---

# F026 - Promotions and options offered by stay length (optional minimum and maximum nights)

## User story

As an owner,
I want to offer a discount or an option depending on the number of nights booked,
so that longer stays are more attractive (for example −10 % from 7 nights, or the bed linen offered from 3 nights).

As a visitor,
I want to see in the quote the promotion I get and the options I can add,
so that I know the exact price before booking.

## Business rules

* **Admin screen "Promotions et options"** (its own screen, button on the admin home next to "Tarifs").
* **Promotion**: name (FR / EN, English falls back to French), discount as a **percentage** (of the nights) or a **fixed amount in euros, per stay or per night**, **minimum nights (optional)** and **maximum nights (optional)**: it applies when `min ≤ nights ≤ max`, a missing bound being open (min ≤ max when both are set). Optional validity dates on the **arrival date** (from, until: both included), active / inactive.
* Only the **best** promotion applies (the largest discount); promotions are never added up. The discount applies to the nights only (never to the cleaning fee or the tourist tax) and never makes the stay negative.
* **Option**: name and description (FR / EN), price in euros **per stay, per night, per guest, or per guest and per night** (0 € for an offered option, shown "offert"), **minimum and maximum nights (both optional)** for it to be offered, display order, active / inactive. The visitor ticks an option or not (no quantity). Examples: bed linen, cleaning at the end of the stay, baby cot, late departure.
* The quote (F004, `POST /api/quote` with the ids of the ticked options) shows the promotion that applies (name and discount) and the options offered for the chosen stay with their price; the visitor ticks options and the quote is updated. The API computes everything: stay total = nights − discount + cleaning + ticked options; the deposit and balance are computed on that total; the tourist tax is unchanged. An option that is not offered for the stay is refused with a clear reason.
* The booking (F007) keeps the promotion and options **as they were** at booking time (name and amounts copied), so that a later change never modifies an existing booking.
* Only the direct booking on the site gets these promotions and options; prices on the platforms are managed there (F013, F019).
* Every change (created, changed, deleted) is kept in the price history of F006 with its author.

## Acceptance criteria

```gherkin
Feature: Promotions and options by stay length

  Scenario: F026-S01 Promotion from 7 nights
    Given a promotion of 10 % with minimum 7 nights and no maximum
    When a visitor asks a quote for 7 nights at 100 €
    Then the quote shows 700 € of nights, a discount of 70 € and the cleaning fee unchanged

  Scenario: F026-S02 Bounds are optional and inclusive
    Given a promotion with maximum 3 nights and no minimum
    Then it applies to 1, 2 and 3 nights and not to 4

  Scenario: F026-S03 Best promotion only
    Given a 10 % promotion from 7 nights and a 15 % promotion from 14 nights
    When a visitor asks a quote for 14 nights
    Then only the 15 % discount applies

  Scenario: F026-S04 Option offered by stay length
    Given the option "Linge de maison" at 15 € per guest, offered from 3 nights
    When a visitor asks a quote for 2 nights
    Then the option is not offered
    When the visitor asks a quote for 3 nights for 2 guests and ticks it
    Then the quote adds 30 € and the deposit is updated

  Scenario: F026-S05 Booking keeps its conditions
    Given a booking made with a 10 % promotion
    When the owner deletes the promotion
    Then the booking still shows its 10 % discount
```

## Out of scope

* Coupon codes, last-minute and early-booking discounts by date (possible later versions), promotions on the platforms.

## Tests

| Scenario | Test |
|---|---|
| F026-S01 | `Back/src/domain/offers.rs::tests::f026_s01_*`, `Back/src/domain/quote.rs::tests::f026_s01_s04_*`, `Back/tests/offers.rs::f026_s01_s03_*`, `Front/src/app/features/admin/offers/offers.page.spec.ts`, `Front/src/app/features/public/pages/ui/blocks/availability-block.spec.ts` |
| F026-S02 | `Back/src/domain/offers.rs::tests::f026_s02_*` |
| F026-S03 | `Back/src/domain/offers.rs::tests::f026_s03_*`, `Back/tests/offers.rs::f026_s01_s03_*` |
| F026-S04 | `Back/src/domain/offers.rs::tests::f026_s04_*`, `Back/src/domain/quote.rs::tests::f026_s01_s04_*`, `Back/tests/offers.rs::f026_s04_*`, `Front/src/app/features/admin/offers/offers.page.spec.ts`, `Front/src/app/features/public/pages/ui/blocks/availability-block.spec.ts` |
| Management, history, admin only | `Back/src/domain/offers.rs::tests::f026_offers_are_validated`, `Back/tests/offers.rs::f026_owners_manage_offers_with_history`, `Back/tests/offers.rs::f026_offers_are_admin_only`, `Front/src/app/features/admin/offers/offers.page.spec.ts` |
| F026-S05 | with the booking (F007) |

## Changelog

* v1 (2026-11-21): first version (draft), at the owners' request: promotions or options offered depending on the number of nights, with optional minimum and maximum.
* v2 (2026-11-21): frozen: best promotion only, option prices per stay / night / guest / guest and night, ticked without quantity, own admin screen "Promotions et options"; S05 is tested with the booking (F007).
