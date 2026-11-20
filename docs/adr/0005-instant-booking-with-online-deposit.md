---
status: accepted
date: 2026-11-20
---

# Instant booking confirmed by an online deposit

## Context and Problem Statement

Guests should be able to book directly on the site at any time, without waiting for the owners. The same dates are also sold on Airbnb and Booking.com.

## Decision Drivers

* Guest experience (book in one go)
* No manual validation by the owners
* Avoid double bookings with the platforms

## Considered Options

* Instant booking: the booking is confirmed when the online deposit is confirmed by the payment provider
* Booking request validated by the owners

## Decision Outcome

Chosen option: "Instant booking".

* A booking is created in `pending_payment` with a short **hold** on the dates (default 20 minutes), inside a transaction that re-checks availability.
* The **only** transition to `confirmed` is `Booking::confirm(ConfirmedPayment)`, called on a **verified** provider webhook (or a bank transfer marked as received by an owner, for last-minute bookings).
* Availability is re-checked against the channel manager (or a freshly downloaded iCal) just before payment; if it cannot be checked, online booking is suspended for those dates.
* Late conflict detected → owners alerted, guest fully refunded.
* Deposit (default 50 % of the stay price, or a fixed amount), balance due at D-30, automatic cancellation if unpaid; last-minute bookings pay 100 % online or by bank transfer within 3 days. **All values are owner-editable settings** (see ADR-0010).

### Consequences

* Good, because guests book in one step, day or night
* Bad, because the iCal delay (30 min to hours) leaves a residual double-booking risk without a channel manager (see ADR-0007)

### Confirmation

Domain tests on the status machine; integration test: two concurrent bookings on the same dates → exactly one succeeds.
