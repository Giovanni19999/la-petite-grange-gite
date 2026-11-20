---
status: accepted
date: 2026-11-20
---

# Record architecture decisions as MADR files and freeze features before coding

## Context and Problem Statement

The project is built by one developer for non-technical owners, over several months, and the repository is public. Decisions and feature scope must stay traceable and must not drift silently between the specification and the code.

## Decision Drivers

* Traceability of "why" for future maintenance and for reviewers
* No silent scope change
* Lightweight, readable on GitHub

## Considered Options

* MADR 4 ADRs + feature sheets with Gherkin acceptance criteria
* Free-form wiki pages
* No written decisions (commit messages only)

## Decision Outcome

Chosen option: "MADR 4 ADRs + feature sheets with Gherkin acceptance criteria", because it is the most widespread lightweight standard, versioned with the code, and each Gherkin scenario maps to a test.

### Consequences

* Good, because every decision and feature has a reviewable, dated record
* Good, because acceptance criteria are testable (scenario ids appear in test names)
* Bad, because writing docs before code costs time

### Confirmation

* `docs/features/README.md` lists every feature sheet with its status (`draft`, `frozen`, `implemented`).
* A feature is coded only once `frozen`; an accepted ADR is never edited, only superseded.
* CI checks Markdown links and that scenario ids of `implemented` features appear in tests.

## More Information

* Architecture overview: [arc42 + C4](../architecture/README.md)
* Feature index: [features](../features/README.md)
