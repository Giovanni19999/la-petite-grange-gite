---
id: FNNN
title: "{Feature title}"
status: draft            # draft | frozen | implemented
version: 1
adr: []                  # e.g. [ADR-0005]
---

# FNNN - {Feature title}

## User story

As a {guest | owner | visitor},
I want {capability},
so that {benefit}.

## Business rules

* {rule 1 — values are owner-editable settings unless stated otherwise}

## Acceptance criteria

```gherkin
Feature: {Feature title}

  Scenario: FNNN-S01 {short name}
    Given {context}
    When {action}
    Then {expected outcome}
```

## Out of scope

* {what this feature does not do}

## Tests

| Scenario | Test |
|---|---|
| FNNN-S01 | {path::test_name} |

## Changelog

* v1 ({YYYY-MM-DD}): first version.
