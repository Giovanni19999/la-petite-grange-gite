---
id: F012
title: "VAT ready (disabled by default), invoices"
status: draft
version: 1
adr: [ADR-0010]
---

# F012 - VAT ready (disabled by default), invoices

## User story

As an owner,
I want invoices and receipts numbered without gaps, with VAT shown correctly if I ever become liable,
so that my accounts are right without changing the software.

## Business rules

* **VAT settings, dated**: liable yes / no (default **no**), rate (default 10 %), start date, VAT number, legal mention (default "TVA non applicable, art. 293 B du CGI", to be confirmed by the accountant). History kept.
* Prices typed by the owners stay **all taxes included** (= listed). If liable, `VatBreakdown::from_ttc(amount, rate)` computes excluded-VAT and VAT per line (nights, cleaning), rounded to the cent, with excluded + VAT = included guaranteed. Never on the tourist tax nor the deposit (caution).
* The VAT rate and amounts are **frozen on the booking**: a change of regime never alters existing bookings.
* **Invoices**: continuous numbering without gaps (one sequence per year, e.g. `F-2027-0001`), lines excluded / VAT / included, or the legal mention when not liable; PDF generated and kept; issued when a booking is fully paid, or on request; never deleted (a credit note cancels an invoice).
* Quotes, contract, invoices and accounting exports (CSV) show excluded / VAT / included when liable, otherwise the mention.
* Thresholds (VAT franchise, para-hotel services) are **information only**: a non-blocking alert when the year's turnover approaches the threshold typed in the settings; never applied automatically.
* Becoming liable proposes (to confirm) to set "VAT on commission" off for the platforms (F006).

## Acceptance criteria

```gherkin
Feature: VAT and invoices

  Scenario: F012-S01 Not liable by default
    When a booking of 520 € is fully paid
    Then its invoice shows 520 € and "TVA non applicable, art. 293 B du CGI"

  Scenario: F012-S02 Liable at 10 %
    Given the owners are liable at 10 % from 1 January
    When a booking with 460 € of nights and 60 € of cleaning is invoiced
    Then each line shows its excluded-VAT amount, VAT and included amount
    And the sums of excluded VAT and VAT equal 520 €
    And the tourist tax line has no VAT

  Scenario: F012-S03 Change of regime does not touch past bookings
    Given a booking made before becoming liable
    Then its invoice keeps the "TVA non applicable" mention

  Scenario: F012-S04 Numbering without gaps
    When three invoices are issued in 2027
    Then they are numbered F-2027-0001, F-2027-0002 and F-2027-0003
    And cancelling the second one creates a credit note instead of deleting it

  Scenario: F012-S05 Accounting export
    When the owners export the invoices of a month
    Then a CSV lists number, date, booking, amounts and VAT

  Scenario: F012-S06 Threshold alert
    Given a threshold of 85 000 € typed in the settings
    When the year's turnover reaches 80 000 €
    Then the admin shows a non-blocking alert
```

## Out of scope

* Filing VAT returns, bookkeeping software integration, invoices for platform stays (issued by the platforms).

## Tests

| Scenario | Test |
|---|---|
| F012-S01 → S06 | to be filled in when implemented |

## Changelog

* v1 (2026-11-20): first version (draft).
