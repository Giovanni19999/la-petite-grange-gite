---
id: F021
title: "Frontend foundation: Angular SSR, light blue theme, public and admin areas"
status: frozen
version: 1
adr: [ADR-0004]
---

# F021 - Frontend foundation: Angular SSR, light blue theme, public and admin areas

## User story

As the developer of the site,
I want an Angular application rendered on the server, styled with spartan/ui and Tailwind, with separate public and admin areas,
so that every later page is fast, indexable, mobile first and consistent with the chosen theme.

## Business rules

* Angular (latest major) with server-side rendering, standalone components, signals, `OnPush`, TypeScript `strict`.
* Styling: Tailwind CSS v4 + spartan/ui components (helm copied into the project). **One light theme**, blue by default; no dark mode (`color-scheme: light`, no `dark:` variant).
* Two lazily loaded areas: public site (`/`) and admin (`/admin`).
* In development, `/api` is proxied to the backend (`http://127.0.0.1:8080`).
* The home page shows the site name and a "Book" call to action placeholder; content comes later (F002).
* Unknown URLs show a "page not found" page with a link to the home page.
* Only permissive licenses (MIT, ISC, Apache-2.0, BSD, 0BSD) are allowed in dependencies.

## Acceptance criteria

```gherkin
Feature: Frontend foundation

  Scenario: F021-S01 Home page rendered on the server
    Given the front end is built with SSR
    When a client requests "/" without running JavaScript
    Then the HTML response already contains the site name

  Scenario: F021-S02 Light theme only
    Given the visitor's device prefers a dark color scheme
    When the home page is displayed
    Then the page uses the light theme

  Scenario: F021-S03 Admin area is a separate lazy area
    Given the front end is running
    When a client opens "/admin"
    Then the admin placeholder page is displayed

  Scenario: F021-S04 Unknown page
    Given the front end is running
    When a client opens "/does-not-exist"
    Then a "page not found" page is displayed with a link to the home page
```

## Out of scope

* Real page content, i18n (FR/EN), authentication, API data (later features).

## Tests

| Scenario | Test |
|---|---|
| F021-S01 | `Front` SSR render test (`app.config.server` render of `/`) |
| F021-S02 | `src/styles.css` declares `color-scheme: light`; theme test |
| F021-S03 | `app.routes.spec.ts` |
| F021-S04 | `app.routes.spec.ts` |

## Changelog

* v1 (2026-11-20): first version.
