---
id: F019
title: "Listing content per platform (assisted mode)"
status: draft
version: 1
adr: [ADR-0007, ADR-0017]
---

# F019 - Listing content per platform (assisted mode)

## User story

As an owner,
I want to write and organise the content of my Airbnb and Booking listings from the admin, starting from the site,
so that every listing is consistent and I only copy what changed.

## Business rules

* The site is the **reference**: each listing field exists in a common version (taken from the site) and can be **customised per platform**: title, short / long description FR / EN, photos (selection, order, captions, from the media library), amenities (one internal list mapped to each platform's list), capacity, rooms, beds, bathrooms, house rules (no pets…), arrival / departure times, deposit (caution), cancellation policy (choice among each platform's policies). Prices, cleaning and minimum nights stay in F006.
* "Listings" screen: one tab per platform, each field marked "comme le site" or "personnalisé" with a button to go back to common; **preview** of the listing; **character counters** and alerts with each platform's limits (owner-editable); photo count and format checks before export.
* **Assisted mode** (no affordable channel manager publishes content by API): for each field a **"Copier"** button and a **direct link** to the right page of Airbnb / the Booking extranet; one-click export of the selected photos, in order and in the right size; a "fait sur la plateforme" tick with its date, so the owners see what is up to date.
* When the service allows it later, the `ListingPublisher` port publishes automatically without changing the screen; until then the screen never pretends a field is synchronised.
* Content of a retired platform is kept and found back on reactivation (F013).
* Initial import of the current listings if the service allows it, otherwise typed once.

## Acceptance criteria

```gherkin
Feature: Listing content per platform

  Scenario: F019-S01 Common content by default
    Given the long description of the site
    When the owner opens the Airbnb tab
    Then the description shows "comme le site" with the site text

  Scenario: F019-S02 Customise then go back to common
    When the owner customises the Booking title
    Then the Booking title shows "personnalisé"
    When the owner clicks "revenir au commun"
    Then the Booking title follows the site again

  Scenario: F019-S03 Character limit
    Given the Airbnb title limit is 50 characters
    When the owner types a 55-character title
    Then the counter shows the excess and an alert

  Scenario: F019-S04 Copy and mark done
    When the owner copies the Airbnb description and ticks "fait sur la plateforme"
    Then the field shows the date it was done
    When the common description changes afterwards
    Then the field shows "à mettre à jour sur Airbnb"

  Scenario: F019-S05 Photo export in order
    When the owner exports the Booking photos
    Then a single download contains the selected photos in order and in the right size
```

## Out of scope

* Automatic publication of listing content (not available today), platform account settings, promotions, replies to platform reviews (F015).

## Tests

| Scenario | Test |
|---|---|
| F019-S01 → S05 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
