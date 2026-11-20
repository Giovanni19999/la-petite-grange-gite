---
status: accepted
date: 2026-11-20
---

# Tiptap for rich text editing in the admin

## Context and Problem Statement

The page editor (F023) lets the owners write rich text in French and English inside blocks (`text`, `text_image`, `columns`). The API only keeps paragraphs, line breaks, bold, italic, lists, headings h2–h4 and links (F002, sanitised by `ammonia`). Which editor do the owners type into?

## Decision Drivers

* Non-technical owners, often on a phone: what they type must look like what visitors see
* Produce only the HTML the API accepts, so nothing is silently removed on save
* Permissive license (MIT-compatible allow-list, `npm run licenses`), maintained, works with Angular SSR

## Considered Options

* Tiptap (`@tiptap/core` and a few extensions, on top of ProseMirror), behind a thin Angular component of our own
* Home-made `contenteditable` with a small toolbar
* Quill

## Decision Outcome

Chosen option: "Tiptap behind our own Angular component", because its schema is restricted to exactly the allowed elements (unknown markup is dropped while typing or pasting), it is MIT licensed and maintained, and it handles mobile input and paste correctly.

* Only the extensions matching the F002 allow-list are loaded: paragraph, hard break, bold, italic, bullet and ordered lists, headings levels 2 to 4, link (http, https, mailto, tel).
* The editor is created in the browser only (never during server-side rendering) and lives in the admin area, which is not pre-rendered.
* No third-party Angular wrapper: a small component of ours owns the editor's lifecycle, so Angular upgrades do not wait for a wrapper.
* The API stays the authority: the HTML is still sanitised server side.

### Consequences

* Good, because the owners cannot produce markup that would be removed or break the layout
* Good, because the dependency is limited to the admin bundle (lazy-loaded route)
* Bad, because one more front-end dependency to keep up to date
* Neutral, because the editor's HTML and `ammonia`'s allow-list must stay aligned (covered by tests)

### Confirmation

`npm run licenses` in CI; a front-end test checks that the toolbar only offers the allowed formats and that pasted forbidden markup is dropped.
