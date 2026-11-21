# Features

One sheet per feature, using the [template](000-template.md). A feature is coded only when its status is **frozen**; a frozen sheet changes only through a new version (see its changelog). Acceptance scenarios are written in Gherkin and their ids (`FNNN-SNN`) appear in the test names.

| Id | Feature | Status | ADR |
|---|---|---|---|
| [F000](F000-backend-foundation.md) | Backend foundation: configuration, health check, error format | implemented | 0002, 0003, 0010 |
| [F001](F001-admin-auth.md) | Owner authentication (admin login) | implemented | 0011 |
| [F002](F002-block-pages.md) | Pages built from blocks: draft, publication, versions and public rendering (FR/EN) | implemented | 0009, 0013 |
| [F003](F003-media-library.md) | Media library: upload, WebP sizes, focal point, alt text, images in blocks | implemented | 0009, 0017 |
| [F004](F004-availability-calendar.md) | Availability calendar with the nightly price under each free date | draft | 0005, 0008, 0009, 0010 |
| [F005](F005-blocked-periods.md) | Blocked periods: closure, family, private event | implemented | 0002, 0003, 0007, 0010, 0011 |
| [F006](F006-prices-per-period-and-channel.md) | Prices per date range and per channel (listed / net), default price | draft | 0007, 0008, 0010 |
| [F007](F007-instant-booking-with-deposit.md) | Instant booking with online deposit (percentage or fixed amount) | draft | 0005, 0006, 0007, 0010 |
| [F008](F008-balance-reminders-auto-cancellation.md) | Balance at D-30, reminders, automatic cancellation | draft | 0005, 0006, 0010 |
| [F009](F009-last-minute-booking.md) | Last-minute booking: 100 % online or bank transfer within 3 days | draft | 0005, 0006, 0010 |
| [F010](F010-cancellation-and-refunds.md) | Cancellation and refunds | draft | 0005, 0006, 0010 |
| [F011](F011-tourist-tax.md) | Tourist tax (3-star rate) paid on arrival | draft | 0010 |
| [F012](F012-vat-and-invoices.md) | VAT ready (disabled by default), invoices | draft | 0010 |
| [F013](F013-channel-manager-sync.md) | Channel manager synchronisation, connection monitoring, platform retirement | draft | 0007, 0008, 0010 |
| [F014](F014-ical-import-export.md) | iCal import / export | draft | 0007 |
| [F015](F015-reviews.md) | Reviews: criteria /10, moderation, public owner replies | draft | 0007, 0009, 0010 |
| [F016](F016-editable-messages.md) | Editable emails and SMS, scheduled and manual messages | draft | 0010, 0015 |
| [F017](F017-contact-form.md) | Contact form with owner notification (Reply-To) | draft | 0009, 0010 |
| [F018](F018-statistics.md) | Statistics | draft | 0007, 0010 |
| [F019](F019-listing-content-per-platform.md) | Listing content per platform (assisted mode) | draft | 0007, 0017 |
| [F020](F020-appearance-themes.md) | Appearance: predefined light themes (blue default) | draft | 0004, 0009, 0013 |
| [F021](F021-frontend-foundation.md) | Frontend foundation: Angular SSR, light blue theme, public and admin areas | implemented | 0004 |
| [F022](F022-containers.md) | Containers: API and front end in separate images, behind Caddy | implemented | 0012 |
| [F023](F023-page-editor.md) | Visual page editor in the admin (preview, drag and drop, FR/EN, publish dialog) | implemented | 0009, 0015 |
| [F024](F024-image-and-carousel-blocks.md) | Fixed image or carousel: hero carousel, image and carousel blocks | implemented | 0009, 0017 |
