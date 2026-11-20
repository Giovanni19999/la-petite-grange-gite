# Features

One sheet per feature, using the [template](000-template.md). A feature is coded only when its status is **frozen**; a frozen sheet changes only through a new version (see its changelog). Acceptance scenarios are written in Gherkin and their ids (`FNNN-SNN`) appear in the test names.

| Id | Feature | Status | ADR |
|---|---|---|---|
| [F000](F000-backend-foundation.md) | Backend foundation: configuration, health check, error format | frozen | 0002, 0003, 0010 |
| F001 | Owner authentication (admin login) | draft | 0002 |
| F002 | Public pages built from blocks (FR/EN), draft/publish with confirmation | draft | 0009 |
| F003 | Media library (upload, resize, crop, focal point, alt text) | draft | 0009 |
| F004 | Availability calendar with the nightly price under each free date | draft | 0005, 0008 |
| F005 | Blocked periods: closure, family, private event | draft | 0010 |
| F006 | Prices per date range and per channel (listed / net), default price | draft | 0008 |
| F007 | Instant booking with online deposit (percentage or fixed amount) | draft | 0005, 0006 |
| F008 | Balance at D-30, reminders, automatic cancellation | draft | 0005, 0010 |
| F009 | Last-minute booking: 100 % online or bank transfer within 3 days | draft | 0005 |
| F010 | Cancellation and refunds | draft | 0005, 0006 |
| F011 | Tourist tax (3-star rate) paid on arrival | draft | 0010 |
| F012 | VAT ready (disabled by default), invoices | draft | 0010 |
| F013 | Channel manager synchronisation, connection monitoring, platform retirement | draft | 0007 |
| F014 | iCal import / export | draft | 0007 |
| F015 | Reviews: criteria /10, moderation, public owner replies | draft | 0010 |
| F016 | Editable emails and SMS, scheduled and manual messages | draft | 0010 |
| F017 | Contact form with owner notification (Reply-To) | draft | — |
| F018 | Statistics | draft | 0007 |
| F019 | Listing content per platform (assisted mode) | draft | 0007 |
| F020 | Appearance: predefined light themes (blue default) | draft | 0004 |
