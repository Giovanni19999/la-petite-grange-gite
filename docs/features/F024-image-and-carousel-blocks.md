---
id: F024
title: "Fixed image or carousel: hero carousel, image and carousel blocks"
status: implemented
version: 2
adr: [ADR-0009, ADR-0017]
---

# F024 - Fixed image or carousel: hero carousel, image and carousel blocks

## User story

As an owner,
I want to show either one fixed photo or several photos that scroll, in the banner and anywhere in a page,
so that visitors see the cottage from several angles without a long page.

As a visitor,
I want to browse the photos by swiping on my phone or with the dots, and the scrolling to stop while I look,
so that I look at them at my own pace.

## Business rules

* **Hero**: a choice "Image fixe" (one photo) or "Carrousel" (2 to 10 photos), with title, subtitle and button over the photos as today (F002). Darkening applies to every photo.
* **New block "Image"**: one photo with its caption, width narrow / wide / full width; the common block settings apply (background, spacing, hide on mobile, visible).
* **New block "Carrousel"**: 2 to 20 photos, with their captions; height small / medium / large; common block settings apply.
* **Carousels**: position dots and swipe (the photos follow the finger); keyboard accessible (arrow keys when focused); no on-screen arrows and no pause button (owners' choice, v2); each photo keeps its focal point (F003).
* **Simulated loop**: past the last photo the carousel slides on to the first one in the same direction, and before the first photo it shows the last one (no rewind across all photos).
* **Automatic scrolling**: an option per carousel (on by default in the hero, off by default in the block), with a delay of 3 to 15 seconds (default 6). It stops while the pointer is over the carousel, while it has the keyboard focus and while a finger drags it, waits a full delay again after each manual move, and is off when the visitor's device asks for reduced motion.
* Only the first photo of a carousel is loaded at once; the next ones load when they are about to be shown.
* Photos are chosen and ordered (drag and drop) from the media library in the editor (F023), the same way as the gallery.
* The API validates the new options and the number of photos; existing pages keep working (a hero without the new options is a fixed image).

## Acceptance criteria

```gherkin
Feature: Fixed image or carousel

  Background:
    Given the owner is logged in
    And the media library contains at least four photos

  Scenario: F024-S01 Hero with a fixed image
    When the owner chooses "Image fixe" and one photo for the hero, and publishes
    Then visitors see that photo behind the title

  Scenario: F024-S02 Hero carousel
    When the owner chooses "Carrousel" and three photos for the hero, and publishes
    Then visitors see the first photo, then the next ones scroll automatically
    And the scrolling stops while the pointer is over the photos or they have the keyboard focus

  Scenario: F024-S03 Reduced motion
    Given the visitor's device asks for reduced motion
    When the visitor opens a page with a hero carousel
    Then the photos do not scroll by themselves
    And the dots, the swipe and the keyboard still work

  Scenario: F024-S04 Image block
    When the owner adds an "Image" block with a captioned photo in full width
    Then the preview and the published page show the photo across the page with its caption

  Scenario: F024-S05 Carousel block
    When the owner adds a "Carrousel" block with four photos
    Then visitors move between photos with the dots, a swipe or the keyboard
    And going past the last photo slides on to the first one without rewinding
    And the photos do not scroll by themselves unless the owner turned it on

  Scenario: F024-S06 Invalid carousels are refused
    When the owner saves a carousel block with one photo, or a hero carousel with eleven photos
    Then the draft is refused with a clear message

  Scenario: F024-S07 Lazy loading
    When a visitor opens a page with a carousel
    Then only the first photo is downloaded before the visitor moves to the next one
```

## Out of scope

* Full-screen photo viewer (lightbox), video, transitions other than sliding, captions over the hero photos.

## Tests

| Scenario | Test |
|---|---|
| F024-S01 | `Back/src/domain/page.rs::tests::f024_s01_*`, `Back/tests/media.rs::f024_s01_s02_s04_s05_*` |
| F024-S02 | `Back/tests/media.rs::f024_s01_s02_s04_s05_*`, `Front/src/app/shared/media/carousel.spec.ts` |
| F024-S03 | `Front/src/app/shared/media/carousel.spec.ts` |
| F024-S04 | `Back/src/domain/page.rs::tests::f024_s04_s05_*`, `Back/tests/media.rs::f024_s01_s02_s04_s05_*`, `Front/src/app/features/admin/pages/editor/page-editor.page.spec.ts` |
| F024-S05 | `Back/src/domain/page.rs::tests::f024_s04_s05_*`, `Front/src/app/shared/media/carousel.spec.ts` (dots, keys, swipe, simulated loop), `Front/src/app/features/admin/pages/editor/page-editor.page.spec.ts` |
| F024-S06 | `Back/src/domain/page.rs::tests::f024_s06_*`, `Back/tests/media.rs::f024_s06_*` |
| F024-S07 | `Front/src/app/shared/media/carousel.spec.ts` |

## Changelog

* v1 (2026-11-20): first version.
* v2 (2026-11-20): no on-screen arrows and no pause button (owners' choice); simulated loop at both ends; the delay restarts after a manual move. Accessibility note: WCAG 2.2.2 recommends a visible pause control for content that moves by itself; here the scrolling stops on hover, focus and touch and is off with reduced motion.
