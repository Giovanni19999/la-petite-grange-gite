---
id: F020
title: "Appearance: predefined light themes (blue default)"
status: draft
version: 1
adr: [ADR-0004, ADR-0009, ADR-0013]
---

# F020 - Appearance: predefined light themes (blue default)

## User story

As an owner,
I want to choose the colours of the site among ready-made themes and see the result before publishing,
so that the site looks like the cottage without breaking readability.

## Business rules

* An "Apparence" admin page lists the **light** themes of spartan/ui (no dark mode, no free colour), each a set of CSS variables (`--primary`, `--background`, `--accent`, `--radius`…); default **blue** (seed), with a "revenir au thème d'origine" button.
* **Live preview** of the home page with the chosen theme before publishing; publishing goes through the shared confirmation dialog (it changes what visitors see).
* The published theme is stored in the settings and applied by the server rendering (`theme-<name>` class on `<html>`), without rebuilding nor redeploying; it is part of the public settings.
* Blocks store colour **roles** (background, muted, primary…), never colours (F002), so changing theme recolours the whole site and stays readable.
* Every theme offered passes WCAG AA contrast for text on its backgrounds (checked by a test on the theme variables).

## Acceptance criteria

```gherkin
Feature: Appearance themes

  Scenario: F020-S01 Blue by default
    When a visitor opens the site for the first time
    Then the blue theme is applied

  Scenario: F020-S02 Preview before publishing
    When the owner selects the green theme
    Then the preview shows the home page in green
    And visitors still see the blue theme

  Scenario: F020-S03 Publish with confirmation
    When the owner publishes the green theme and confirms
    Then the server-rendered pages carry the green theme class

  Scenario: F020-S04 Back to the original theme
    When the owner clicks "revenir au thème d'origine" and confirms
    Then the blue theme is applied again

  Scenario: F020-S05 Contrast
    Then every offered theme passes WCAG AA for text on its background colours

  Scenario: F020-S06 No dark mode
    Given a visitor's device prefers a dark scheme
    Then the site stays in the published light theme
```

## Out of scope

* Free colours, fonts upload, dark mode, a theme per page.

## Tests

| Scenario | Test |
|---|---|
| F020-S01 → S06 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
