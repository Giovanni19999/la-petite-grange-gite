---
id: F015
title: "Reviews: criteria /10, moderation, public owner replies"
status: draft
version: 1
adr: [ADR-0010, ADR-0007, ADR-0009]
---

# F015 - Reviews: criteria /10, moderation, public owner replies

## User story

As a guest who booked directly,
I want to rate my stay after leaving,
so that the owners and future visitors know what I thought.

As an owner,
I want to gather the reviews of every source in one place, moderate them, answer them and choose which ones appear on the site,
so that the site shows genuine, verified reviews.

## Business rules

* **Sources**: site (direct stays), Booking (channel manager or CSV import), Airbnb (assisted manual entry with a reminder after each Airbnb departure and a direct link), Google (Business Profile API, reading and replying). Scraping is excluded.
* **Site review form**: unique link in the after-stay email (F016, default D+1), real stays only; one score **/10 per criterion**; the criteria list is owner-editable (label FR / EN, order, active, "counts in the overall score"); criteria that do not count are **private** (admin only, never returned publicly). Overall score = average of the criteria that count, 1 decimal, never typed by hand. Criteria and scores are frozen on the review when it is written. Optional consent to publication on the site.
* **Everything shown out of 10**: the original score and scale are kept (`note`, `note_max`), display = `note × 10 / note_max`, 1 decimal, original reminded when it was not out of 10 ("4,6/5 sur Airbnb").
* **Moderation before any public display**: `en_attente` (default) → `valide` or `refuse` with a reason; email to the owners for each new review, reminder if waiting more than a setting (default 3 days). A review text is **never editable**. Refusing a site review emails its author with the reason. Moderation rules are editable and shown in the transparency mention.
* **Owners reply** (comment only, never a score on the guest), one per review, editable with history, public under the review on the site; Google replies are sent to Google; Airbnb / Booking replies are copied to paste on the platform. There is no rating of guests by the owners anywhere.
* **Site display**: "Avis" block of the page editor (number of reviews, sources, averages yes / no, list or carousel) shows only reviews `valide` **and** "afficher sur le site" (default off), site reviews only with consent; first name only, stay date, score /10, source, owner reply; a transparency mention (selected by the owners, verified or not, date of each review).
* Low score alert under a threshold (default 7/10). Platform overall ratings (score and number of reviews shown on the listings) can be typed and displayed.
* Demo reviews exist only in dev / staging, marked, and the API refuses to start in production if one exists (fake reviews are forbidden).

## Acceptance criteria

```gherkin
Feature: Reviews

  Scenario: F015-S01 Site review after a direct stay
    Given a direct stay ended yesterday
    When the guest opens the review link and scores cleanliness 9, comfort 8 and the private criterion "literie" 6
    Then the review is saved "en_attente" with an overall score of 8.5
    And the owners are emailed

  Scenario: F015-S02 Private criteria never public
    When the public reviews are requested
    Then no private criterion appears

  Scenario: F015-S03 Scores out of 10
    Given an Airbnb review of 4.6/5
    Then it is shown 9.2/10 with "4,6/5 sur Airbnb"

  Scenario: F015-S04 Moderation before display
    Given a review waiting for moderation
    When the owner tries to show it on the site
    Then it is refused until the review is validated

  Scenario: F015-S05 Refusal emails the author
    When the owner refuses a site review for "données personnelles"
    Then its author receives the reason by email

  Scenario: F015-S06 Public owner reply
    Given a validated review shown on the site
    When the owner replies
    Then the reply appears under the review for every visitor

  Scenario: F015-S07 Consent required for site reviews
    Given a validated site review without consent to publication
    Then it cannot be shown on the site

  Scenario: F015-S08 No demo review in production
    Given a demo review in the database
    When the API starts in production
    Then it refuses to start

  Scenario: F015-S09 Booking CSV import without duplicates
    When the owner imports the same Booking reviews CSV twice
    Then each review exists once
```

## Out of scope

* Rating guests (never), reviews of Airbnb by API (not available), statistics on reviews (F018).

## Tests

| Scenario | Test |
|---|---|
| F015-S01 → S09 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
