---
status: accepted
date: 2026-11-20
---

# No business value hard-coded: every rule is an owner-editable setting

## Context and Problem Statement

Owners must be able to change every business value themselves: delays, percentages, amounts, hours, capacity, message timings, review criteria, thresholds.

## Considered Options

* All business values stored in the database (seeded defaults), passed to the domain as typed settings
* Constants in code, changed by a developer

## Decision Outcome

Chosen option: "Database settings with seeded defaults".

* The domain receives settings as inputs (`BookingRules`, `DepositRule`, `MessageSchedule`, `ReviewSettings`…); defaults come from seed data, never from code.
* Rules that bind a guest (cancellation, payment schedule, hours) are **copied onto each booking** when it is made; later changes never alter existing bookings.
* Every settings change is versioned (who, when, old → new) and can be reverted.
* Only **technical** settings live in files (`config/*.toml`, `.env`).

### Consequences

* Good, because owners are autonomous
* Bad, because more validation code and tests with several value sets

### Confirmation

Review rejects business numbers in code; clippy `disallowed_*` rules; domain tests run with several rule sets.
