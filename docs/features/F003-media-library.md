---
id: F003
title: "Media library: upload, WebP sizes, focal point, alt text, images in blocks"
status: frozen
version: 1
adr: [ADR-0017, ADR-0009, ADR-0010, ADR-0011, ADR-0016]
---

# F003 - Media library: upload, WebP sizes, focal point, alt text, images in blocks

## User story

As an owner,
I want to send the photos of the cottage from my computer or my phone, describe them and use them in the pages,
so that the site shows nice, fast-loading photos without any technical work.

As a visitor,
I want the photos to load quickly on my phone and to be described for screen readers,
so that I can discover the cottage even on a slow connection.

## Business rules

* **Upload**: one or several files at once, by drag and drop or file picker (phone gallery or camera included). Accepted formats: JPEG, PNG, WebP. Other formats (HEIC from a computer, GIF, SVG…) are refused with a clear message; phones convert their photos to JPEG when sending from a browser.
* Limits are technical settings with defaults: maximum file size (20 MB), files per upload (20), maximum image size (50 megapixels). A file over a limit is refused with the reason; the other files of the same upload are still saved.
* **Processing** (ADR-0017): the photo is turned the right way up (EXIF orientation), all metadata is removed (GPS position included), and WebP versions are produced at the configured widths (defaults 480, 960, 1440, 1920 px), never wider than the original. The original is kept privately and never published.
* **Media details**: alternative text FR / EN, caption FR / EN (optional), **focal point** (the important part of the photo, chosen by clicking on it; default the centre). The focal point keeps that part visible whatever the shape of the frame (banner, square, phone).
* A missing French alternative text is flagged in the library ("texte alternatif manquant"); the English one falls back to French. Without any alternative text the image is treated as decorative (`alt=""`).
* **Library** (`/admin/medias`): grid of thumbnails, newest first, with a search on the texts; usable on a phone. Each media shows the pages where it is used.
* **Delete**: refused while the media is used in the draft or the published content of a page (the message names the pages). Images of old versions are not checked: a restored version shows a placeholder where a deleted image was.
* **Images in blocks** (F002, F023): in the editor the owner chooses images from the library (or uploads one on the spot) for the hero, text + image and gallery blocks; the gallery order is changed by drag and drop. Ids that do not exist are refused when the draft is saved.
* **Public rendering**: images use `srcset` / `sizes` with the WebP widths, explicit width and height (no layout shift), lazy loading except the first image of the page, the focal point as `object-position`, and the alternative text and caption in the page language.
* Media files are served under `/uploads/` by the proxy with a one-year immutable cache; an upload never replaces an existing file.
* Every media endpoint is admin only (session, F001) except the files themselves.

## Acceptance criteria

```gherkin
Feature: Media library

  Background:
    Given the owner is logged in

  Scenario: F003-S01 Upload several photos
    When the owner uploads two JPEG photos at once
    Then both appear in the library with their thumbnails
    And WebP versions exist at each configured width not larger than the photo

  Scenario: F003-S02 Orientation and metadata
    Given a phone photo taken upright with an EXIF orientation flag and a GPS position
    When the owner uploads it
    Then the published versions are upright
    And they contain no EXIF data

  Scenario: F003-S03 Refused files
    When the owner uploads a GIF, a file over the size limit and a valid JPEG together
    Then the GIF and the large file are refused with their reasons
    And the JPEG is saved

  Scenario: F003-S04 Image too large in pixels
    When the owner uploads an image above the pixel limit
    Then it is refused without being decoded entirely

  Scenario: F003-S05 Describe a photo
    When the owner sets the French alternative text, an English caption and the focal point at 30 % / 70 %
    Then the library no longer flags a missing alternative text
    And the public pages use these texts and "object-position: 30% 70%"

  Scenario: F003-S06 Delete an unused photo
    Given a photo used in no page
    When the owner deletes it and confirms
    Then it disappears from the library and its files are removed

  Scenario: F003-S07 Delete a used photo
    Given a photo used in the published home page
    When the owner tries to delete it
    Then the deletion is refused and the home page is named

  Scenario: F003-S08 Choose an image in a block
    When the owner chooses a photo for the hero block in the editor
    Then the preview shows it
    And saving the draft with an unknown media id is refused

  Scenario: F003-S09 Public rendering
    Given the published home page uses a photo
    When a visitor opens "/"
    Then the image has a srcset of the WebP widths, width and height attributes and its alternative text
    And the file is served under "/uploads/" with a long immutable cache

  Scenario: F003-S10 Admin only
    Given the owner is not logged in
    When someone calls any /api/admin/media endpoint
    Then the response status is 401
```

## Out of scope

* Cropping (focal point only), AVIF, video, image editing (filters, rotation by hand), automatic alternative text, image carousels and the single image block (F024), photos of the listings on Airbnb / Booking (F019).

## Tests

| Scenario | Test |
|---|---|
| F003-S01 → S10 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version.
