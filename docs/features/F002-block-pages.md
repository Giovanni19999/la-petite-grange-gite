---
id: F002
title: "Pages built from blocks: draft, publication, versions and public rendering (FR/EN)"
status: implemented
version: 2
adr: [ADR-0009, ADR-0013, ADR-0002, ADR-0011]
---

# F002 - Pages built from blocks: draft, publication, versions and public rendering (FR/EN)

## User story

As an owner,
I want the pages of the site to be made of blocks that I change in a draft and publish when I am ready,
so that visitors only see finished pages, in French and in English, and I can go back to an earlier version.

As a visitor,
I want to read the pages of the site in French or in English, with a menu,
so that I discover the cottage in my language.

## Business rules

* A page has, per language, a slug, a title and a meta description, plus "shown in the menu" and a menu position. The home page has the reserved slug `accueil` / `home`, is always in the menu first and cannot be deleted.
* Content is an ordered list of blocks. Block types in this feature: `hero` (image, title, subtitle, button text and link; text alignment, height, image darkening 0–80 %), `text` (rich text; width narrow/wide; alignment), `text_image` (rich text, image; image left/right; ratio 50-50 or 33-67), `columns` (2 to 4 items with title and rich text; text alignment left / center / right, left by default, also for blocks saved before this option), `gallery` (images; 2/3/4 columns). Common settings: visible, background (`background` / `muted` / `primary`), spacing (`small` / `medium` / `large`), hide on mobile.
* Texts exist in French and in English; an empty English text falls back to French.
* Rich text is sanitised on save: only paragraphs, line breaks, bold, italic, lists, headings h2–h4 and links (http, https, mailto, tel); anything else is removed.
* Unknown block types or options, missing French title, duplicated or invalid slugs are rejected with a clear message.
* Editing changes the **draft** only. Publishing copies the draft to the public page and keeps the previous published content as a **version**. Publishing requires the draft version the owner confirmed: if the draft changed meanwhile, publishing is refused (HTTP 409).
* The admin can get a **summary of changes** between draft and published page (blocks added, removed, changed, moved; texts changed per language; page settings changed) to show before publishing.
* Restoring a version puts its content back into the draft (it is published only when the owner publishes).
* Images reference the media library (F003) by id; until F003, blocks are shown without their images.
* Visitors only ever see published pages; a page never published is unknown to them (404).

## Acceptance criteria

```gherkin
Feature: Pages built from blocks

  Background:
    Given the default pages exist and are published
    And the owner is logged in

  Scenario: F002-S01 Visitor reads the home page in French and in English
    When a visitor opens "/"
    Then the page title and blocks are shown in French, rendered on the server
    When the visitor opens "/en"
    Then the same page is shown in English, with French where the English text is empty

  Scenario: F002-S02 Menu
    When a visitor opens any page
    Then the menu lists the published pages shown in the menu, in menu order, in the page language

  Scenario: F002-S03 Unknown page
    When a visitor opens "/page-inconnue"
    Then the response status is 404

  Scenario: F002-S04 Draft is not public
    When the owner saves a draft with a new title
    Then visitors still see the published title

  Scenario: F002-S05 Publish
    Given the owner saved a draft with a new title
    When the owner publishes with the current draft version
    Then visitors see the new title
    And the previous content is kept as a version

  Scenario: F002-S06 Stale draft version
    Given the draft was saved again after the owner opened it
    When the owner publishes with the older draft version
    Then the response status is 409
    And nothing is published

  Scenario: F002-S07 Summary of changes
    Given the owner changed a text, added a block and removed another one in the draft
    When the owner asks for the summary of changes
    Then the summary lists one block added, one removed and one changed, with the languages concerned

  Scenario: F002-S08 Restore a version
    When the owner restores a previous version
    Then the draft contains that version's content
    And visitors still see the current published page until the owner publishes

  Scenario: F002-S09 Invalid content is rejected
    When the owner saves a block of an unknown type, or a text containing a script
    Then an unknown type is rejected with a clear message
    And the script is removed from the saved text

  Scenario: F002-S10 Admin endpoints require a session
    Given the owner is not logged in
    When someone calls any /api/admin/pages endpoint
    Then the response status is 401

  Scenario: F002-S11 Text alignment of a columns block
    Given a page with a "columns" block of 2 columns
    When the owner chooses "Centré" (or "À droite") for the block's text
    Then the title and text of every column are centered (or aligned right), in the preview and on the site
    And a columns block saved before this option stays aligned left
```

## Out of scope

* The visual editor in the admin (F023), the media library (F003), data blocks (prices, availability, reviews, contact form, booking buttons: their own features), automatic translation.

## Tests

| Scenario | Test |
|---|---|
| F002-S01 | `Back/tests/pages.rs::f002_s01_*`, `Front/src/app/app.routes.spec.ts`, `deploy/smoke-test.sh` |
| F002-S02 | `Back/tests/pages.rs::f002_s02_menu` |
| F002-S03 | `Back/tests/pages.rs::f002_s03_unknown_page`, `Front/src/app/app.routes.spec.ts`, `deploy/smoke-test.sh` (404 status) |
| F002-S04 | `Back/tests/pages.rs::f002_s04_draft_not_public` |
| F002-S05 | `Back/tests/pages.rs::f002_s05_publish` |
| F002-S06 | `Back/tests/pages.rs::f002_s06_stale_draft` |
| F002-S07 | `Back/src/domain/page.rs::tests::f002_s07_*`, `Back/tests/pages.rs::f002_s07_changes_endpoint` |
| F002-S08 | `Back/tests/pages.rs::f002_s08_restore` |
| F002-S09 | `Back/src/domain/page.rs::tests::f002_s09_*`, `Back/tests/pages.rs::f002_s09_*` |
| F002-S10 | `Back/tests/pages.rs::f002_s10_admin_requires_session` |
| F002-S11 | `Back/src/domain/page.rs::tests::f002_s11_*`, `Front/src/app/features/public/pages/ui/blocks/blocks.spec.ts` |

## Changelog

* v1 (2026-11-20): first version.
* v2 (2026-11-21): text alignment of the columns block (left / center / right), at the owners' request; scenario S11.
