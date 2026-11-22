---
id: F007
title: "Direct booking confirmed at once, deposit paid offline, online payment prepared"
status: implemented
version: 2
adr: [ADR-0005, ADR-0006, ADR-0010, ADR-0015, ADR-0019, ADR-0020, ADR-0021]
---

# F007 - Direct booking confirmed at once, deposit paid offline, online payment prepared

## User story

As a visitor,
I want to book the cottage directly on the site, see clearly the house rules and how to pay the deposit,
so that my stay is confirmed at once, without commission and without waiting for an answer.

As an owner,
I want every direct booking to block the dates at once, never overlap another stay, and reach me by email and SMS,
so that I have nothing to validate, and I only check that the deposit arrives.

## Business rules

* **No online payment for now** (ADR-0020): while `PAYMENT_PROVIDER=none` (the default), a booking is **confirmed at once** and the deposit and balance are paid **offline** (bank transfer, cheque, holiday vouchers…). The `PaymentGateway` port of ADR-0006 and the `en_paiement` hold are prepared in the domain, but no provider adapter is written; plugging one in later means one adapter, one value of `PAYMENT_PROVIDER`, and the flow of ADR-0005 for that provider.
* **House rules** ("Règles du gîte", `booking_rules`, history with author; ADR-0010) gain, with these seed values:
  * arrival from 16:00, departure before 10:00;
  * pets allowed: no;
  * **payment methods** accepted (list, seed: virement, chèque, chèques-vacances, espèces), used when the owners record a payment;
  * **payment instructions**: rich text (ADR-0015, sanitised) telling how to pay (who to pay, cheque order, accepted vouchers…), seed empty;
  * **bank details document** ("RIB"): an optional PDF or image file uploaded on that screen, kept in a **private** directory (never served by Caddy, ADR-0017), attached to the guest's confirmation email;
  * **deposit alert delay**: 7 days after the booking without the deposit marked received, the owners are alerted;
  * **rental terms**: a page of the site (F002) chosen in a list, seed none; booking is impossible until a page is chosen.
  The code holds none of these values.
* **Booking page** (`/reserver` and `/en/book`, reached by "Réserver" under a valid quote of F004, with the dates, guests and options of F026 in the URL):
  1. summary of the quote recomputed by the API (nights, promotion, options, cleaning, stay total, deposit and balance with their due dates, tourist tax apart);
  2. the **house rules shown clearly**: arrival and departure times, capacity, pets, how to pay the deposit and the balance (payment instructions), "le RIB est joint à l'e-mail de confirmation" when a document is set;
  3. the guest's details: first name, last name, email, mobile phone (E.164, prefilled `+33`), postal address (street, postcode, town, country), expected arrival time (30-minute slots from the arrival time, until 23:30), message to the owners (optional, 2,000 characters at most);
  4. a mandatory checkbox "J'ai lu et j'accepte les conditions de location" linking to the rental terms page; the page id, its version and the time of acceptance are recorded;
  5. "Confirmer la réservation" → `POST /api/bookings`.
* `POST /api/bookings` **recomputes the quote** (prices are never taken from the browser), locks the cottage row (`SELECT … FOR UPDATE`) and re-checks availability **in the same transaction** as the insert: two visitors on the same nights → exactly one booking. A refused quote (F004 reasons) or taken dates refuse the booking with a French reason, and nothing is stored.
* The booking keeps a **copy** of the rules and amounts in force: nightly prices, promotion and options (F026-S05), cleaning, deposit mode and amount, balance and due dates, tourist tax, times, payment instructions, rental terms version. Later changes of rules, prices, promotions or options never alter it.
* Each booking has a public **reference** `LPG-<year of arrival>-<number on 4 digits>` (numbers counted per year of arrival) and a random **tracking token** (at least 128 bits, never the id). Its language is the language of the booking page (FR or EN).
* **Statuses**: `confirmee` (deposit expected) → `acompte_recu` → `soldee`, or `annulee`. `confirmee`, `acompte_recu` and `soldee` block the dates. `en_paiement` and `paiement_abandonne` exist only with an online provider (later). Whether the stay is past, in progress or to come is shown from the dates, not stored.
* **Calendar** (F004): nights of a booking that blocks the dates are unavailable, without any detail, like the blocked periods.
* **Messages** (F025 queue, in the same transaction as the booking): to the **guest**, the "Réservation confirmée" email (with the bank details document attached when one is set) and SMS; to the **owners** (addresses and phones of "Envoi des messages"), the "Nouvelle réservation" email and SMS, which remind them to block the dates on the other platforms (no channel manager nor iCal yet, F013 / F014). Each message has a unique key (booking, template, recipient): never queued twice.
* **Deposit alert**: once a day, a booking still `confirmee` beyond the deposit alert delay sends **once** an email and an SMS to the owners ("Acompte non reçu"). Nothing is cancelled automatically: the owners decide (F008 later adds reminders and cancellation rules).
* **Editable templates** (subset of F016, ADR-0021), on a new admin screen "Modèles de messages": the four templates "Réservation confirmée" (guest) and "Nouvelle réservation" (owners), each as an email and an SMS, plus "Acompte non reçu" (owners, email and SMS):
  * email: subject (plain text) and body written with the rich text editor (ADR-0015: paragraphs, bold, italic, lists, headings, links), sanitised by the API, inside a fixed layout (cottage name, footer with the site address); a plain-text version is derived from it;
  * SMS: plain text with a character and segment counter (GSM-7 160 / 153, Unicode 70 / 67) and a warning above 2 SMS;
  * French and English versions; the guest receives their language, French when the English text is empty; owners always receive French;
  * **variables** inserted with buttons, from an allow-list per template (`{{ voyageur.prenom }}`, `{{ voyageur.nom }}`, `{{ sejour.reference }}`, `{{ sejour.arrivee }}`, `{{ sejour.depart }}`, `{{ sejour.nuits }}`, `{{ sejour.voyageurs }}`, `{{ sejour.heure_arrivee }}`, `{{ montants.total }}`, `{{ montants.acompte }}`, `{{ montants.acompte_echeance }}`, `{{ montants.solde }}`, `{{ montants.solde_echeance }}`, `{{ montants.taxe_sejour }}`, `{{ regles.heure_arrivee }}`, `{{ regles.heure_depart }}`, `{{ regles.paiement }}`, `{{ lien_suivi }}`, and for the owners `{{ voyageur.email }}`, `{{ voyageur.telephone }}`, `{{ voyageur.message }}`, `{{ lien_admin }}`); dates and amounts formatted in the language of the message; an unknown variable is refused on save with its name;
  * rendering in a sandbox (only the given variables, no file nor code access), values escaped in emails;
  * **preview** with a sample booking, **restore the default text**; default French and English texts are seeded.
* **Tracking page** (`/reservation/{token}` and `/en/booking/{token}`, not indexed): reference, dates, guests, amounts and due dates, payments received, house rules and payment instructions, status. Until the day before arrival, the guest can change the expected arrival time (the owners get an email).
* **Admin "Réservations"**: list of the bookings (to come first, filters by status and by dates), with reference, guest, dates, amounts, deposit and balance state (expected, received, late); detail with the copy of the conditions, the history and the messages sent (F025 log). The owners can:
  * record a payment received (amount, date, method among the payment methods typed in the rules screen, note): the status becomes `acompte_recu` when the deposit is covered, `soldee` when the stay total is covered;
  * cancel the booking with a reason (confirmation asked): the dates are freed at once; refunds are done offline (F010 later adds the cancellation policy);
  * change the guest's details and the expected arrival time.
* Every event of a booking (creation, payment recorded, message queued, change, cancellation) is kept in its history with author and time.
* **Abuse**: at most 5 booking attempts per hour and per IP address (HTTP 429), a hidden field filled by robots refuses the request silently; the owners can cancel a fake booking in one click.
* Personal data of the guests are only shown to signed-in owners, never written in the logs.

## Acceptance criteria

```gherkin
Feature: Direct booking without online payment

  Background:
    Given no payment provider is configured
    And the house rules ask for a 50 % deposit and the balance 30 days before arrival
    And a rental terms page is chosen

  Scenario: F007-S01 Booking confirmed at once
    Given a free stay from 10 to 14 July costing 520 € with cleaning
    When a visitor fills in the form, accepts the terms and confirms
    Then the booking is "confirmee" with a reference "LPG-2027-0001" and a tracking link
    And the nights from 10 to 13 July are unavailable in the calendar
    And the deposit of 260 € is expected and the balance of 260 € is due on 10 June

  Scenario: F007-S02 Guest and owners are told
    When the booking of F007-S01 is made
    Then the guest gets the "Réservation confirmée" email with the bank details attached, and the SMS
    And each owner gets the "Nouvelle réservation" email and SMS
    And each message is queued once

  Scenario: F007-S03 Fixed deposit
    Given the house rules ask for a fixed deposit of 150 €
    When a visitor books a stay costing 520 €
    Then the deposit is 150 € and the balance 370 €

  Scenario: F007-S04 Tourist tax outside the deposit
    When a visitor books a stay costing 520 € with 12 € of tourist tax
    Then the deposit and balance are computed on 520 €
    And the tourist tax is shown apart, to pay on arrival

  Scenario: F007-S05 Two visitors on the same dates
    Given visitors A and B confirm 10 to 14 July and 12 to 15 July at the same time
    Then exactly one booking is stored
    And the other visitor is told the dates are no longer available

  Scenario: F007-S06 Prices come from the server
    When a visitor sends a booking with a stay total of 1 €
    Then the booking is stored with the stay total computed by the API

  Scenario: F007-S07 Deposit not received
    Given a booking made 8 days ago with a 7-day deposit alert delay
    When the daily check runs twice
    Then the owners get the "Acompte non reçu" email and SMS once
    And the booking is still "confirmee"

  Scenario: F007-S08 Payments recorded by the owners
    When the owner records 260 € received by bank transfer
    Then the booking is "acompte_recu" and the history shows the payment
    When the owner records 260 € more
    Then the booking is "soldee"

  Scenario: F007-S09 Rules are frozen on the booking
    Given a booking made with a 50 % deposit
    When the owners change the deposit to 30 %
    Then the existing booking keeps 50 %

  Scenario: F007-S10 Terms must be accepted
    When a visitor submits the form without accepting the rental terms
    Then the booking is refused and nothing is stored

  Scenario: F007-S11 Cancellation frees the dates
    When the owner cancels the booking with a reason
    Then the booking is "annulee" and the nights are free again in the calendar

  Scenario: F007-S12 Template edited with an unknown variable
    When the owner saves "Réservation confirmée" containing "{{ voyageur.prenon }}"
    Then the template is refused with the name of the unknown variable
    When the owner saves it with "{{ voyageur.prenom }}" in bold
    Then the next confirmation email uses the new text with the guest's first name

  Scenario: F007-S13 Language of the guest
    Given a visitor booking on the English page
    Then they receive the English version
    Given the English text of the SMS is empty
    Then the French SMS is sent

  Scenario: F007-S14 Tracking page
    When the guest opens their tracking link
    Then they see their stay, the amounts and the payment instructions
    And they can change the expected arrival time until the day before arrival
    When someone opens a tracking link with an unknown token
    Then the page is not found

  Scenario: F007-S15 Too many attempts
    Given 5 booking attempts from the same IP address in the last hour
    When a 6th attempt is sent
    Then it is refused with HTTP 429
```

## Out of scope

* Online payment and its provider adapters, the payment hold and its expiry, the late conflict refund (ADR-0005, ADR-0006: when a provider is chosen).
* Balance reminders and automatic cancellation (F008), last minute (F009), cancellation policy and refunds (F010), tourist tax rates (F011), invoices and VAT (F012), channel manager and iCal (F013, F014), the other templates and scheduled messages (F016), cancellation insurance.

## Tests

| Scenario | Test |
|---|---|
| F007-S01 | `Back/src/domain/booking.rs::tests::f007_s01_*`, `Back/tests/bookings.rs::f007_s01_s02_s04_*`, `Front/src/app/features/public/booking/booking.page.spec.ts`, `Front/src/app/features/public/pages/ui/blocks/availability-block.spec.ts` |
| F007-S02 | `Back/tests/bookings.rs::f007_s01_s02_s04_*`, `Back/tests/bookings.rs::f007_attachments_are_kept_in_the_queue` |
| F007-S03 | `Back/tests/bookings.rs::f007_s03_fixed_deposit` |
| F007-S04 | `Back/tests/bookings.rs::f007_s01_s02_s04_*` |
| F007-S05 | `Back/tests/bookings.rs::f007_s05_*` (two concurrent bookings) |
| F007-S06 | `Back/tests/bookings.rs::f007_s06_*` |
| F007-S07 | `Back/tests/bookings.rs::f007_s07_*` |
| F007-S08 | `Back/src/domain/booking.rs::tests::f007_s08_*`, `Back/tests/bookings.rs::f007_s08_*`, `Front/src/app/features/admin/bookings/bookings.page.spec.ts` |
| F007-S09 | `Back/tests/bookings.rs::f007_s09_f026_s05_*` (also F026-S05), `Front/src/app/features/admin/bookings/bookings.page.spec.ts` |
| F007-S10 | `Back/tests/bookings.rs::f007_s10_*`, `Front/src/app/features/public/booking/booking.page.spec.ts`, `Front/src/app/features/admin/house-rules/house-rules.page.spec.ts` |
| F007-S11 | `Back/tests/bookings.rs::f007_s11_*`, `Front/src/app/features/admin/bookings/bookings.page.spec.ts` |
| F007-S12 | `Back/src/domain/templates.rs::tests`, `Back/src/application/templates.rs::tests`, `Back/tests/bookings.rs::f007_s12_*`, `Front/src/app/features/admin/templates/templates.page.spec.ts` |
| F007-S13 | `Back/src/application/templates.rs::tests::f007_s13_*`, `Back/tests/bookings.rs::f007_s13_*`, `Front/src/app/features/admin/templates/templates.page.spec.ts` |
| F007-S14 | `Back/tests/bookings.rs::f007_s14_*`, `Front/src/app/features/public/booking/booking.page.spec.ts` |
| F007-S15 | `Back/tests/bookings.rs::f007_s15_*` |
| Robots, admin only, payment instructions | `Back/tests/bookings.rs::f007_robots_and_strangers_get_nothing`, `Back/tests/bookings.rs::f007_payment_instructions_and_document`, `Back/src/infrastructure/documents.rs::tests`, `Back/src/config.rs::tests::f007_*` |

## Changelog

* v1 (2026-11-20): first version (draft).
* v2 (2026-11-22): frozen at the owners' request: no online payment for now (ADR-0020), booking confirmed at once, deposit and balance paid offline and recorded by the owners, house rules and payment instructions shown clearly, bank details document attached to the confirmation email, alert to the owners when the deposit is late (no automatic cancellation), emails and SMS to the guest and the owners with templates edited with the rich text editor (ADR-0021); F026-S05 is tested here.
