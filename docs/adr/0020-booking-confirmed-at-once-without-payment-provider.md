---
status: accepted
date: 2026-11-22
---

# Without a payment provider, a direct booking is confirmed at once and paid offline

## Context and Problem Statement

ADR-0005 confirms a direct booking only on a verified event of the online payment provider, and ADR-0006 makes online booking impossible while the provider is `none`. The owners have not chosen a provider yet and want direct bookings now; guests already pay deposits by bank transfer or cheque for this cottage. How are direct bookings handled until a provider is plugged in?

## Decision Drivers

* Direct bookings available now, without a payment provider nor its fees
* No double booking: the dates must be blocked as soon as the booking exists
* Plugging a provider in later must not change the domain nor the screens much
* No manual validation of each booking by the owners

## Considered Options

* Booking confirmed at once, deposit and balance paid offline and recorded by the owners
* Booking request validated by the owners after the deposit arrives
* Keep the contact form only until a provider is chosen

## Decision Outcome

Chosen option: "Booking confirmed at once, paid offline", because it gives instant booking now with what the owners already do for payments. It supersedes, **while `PAYMENT_PROVIDER=none`**, the bullet of ADR-0005 "the only transition to `confirmed` is on a verified provider event" and the rule of ADR-0006 that online booking is impossible with the provider `none`; the rest of both ADRs stays, and applies again for a configured provider.

* `PAYMENT_PROVIDER` (environment, default `none`) replaces `config/payment.toml` of ADR-0006; secrets stay in the environment. An unknown value stops the API at startup.
* With `none`, `POST /api/bookings` creates the booking directly in `confirmee` (deposit expected), in the transaction that locks the cottage and re-checks availability. With a provider (later), the same use case creates `en_paiement` with a hold and ADR-0005 applies.
* The `PaymentGateway` port of ADR-0006 is declared in the application layer with the `none` adapter only. The booking keeps a payment mode (`offline` / `online`) so that both kinds can coexist.
* Payments are **recorded by the owners** (amount, date, method): the domain moves the booking to `acompte_recu` then `soldee`. The method list is a house rule (ADR-0010).
* A daily task alerts the owners **once** when the deposit is late; nothing is cancelled automatically (F008 decides that later).
* The bank details document attached to the confirmation email is stored in the private directory of the API (not served by Caddy, ADR-0017) and backed up with the volume; it never goes into the repository.

### Consequences

* Good, because the owners can take direct bookings now, without commission nor provider fees
* Good, because the dates are blocked at once, with the same locking as with online payment
* Bad, because a fake or unpaid booking blocks the dates until the owners cancel it (mitigated by the deposit alert, the rate limit and a one-click cancellation)
* Bad, because the owners must record each payment by hand

### Confirmation

Domain tests of the status machine (offline mode); integration test: two concurrent bookings on the same dates → exactly one stored; test that the API refuses to start with an unknown `PAYMENT_PROVIDER`.

## More Information

Revisit when a payment provider is chosen: then write its adapter and contract tests (ADR-0006) and decide whether offline payment stays offered.
