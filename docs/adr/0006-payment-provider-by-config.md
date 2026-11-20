---
status: accepted
date: 2026-11-20
---

# One online payment provider, selected by configuration; no card data on our server

## Context and Problem Statement

The deposit must be paid online. Several providers are acceptable (Stripe, Mollie, PayPal, SumUp, a bank gateway) and the choice may change.

## Decision Drivers

* PCI DSS scope as small as possible (SAQ A)
* Change provider without touching the domain or the screens
* Amount always computed server side

## Considered Options

* `PaymentGateway` port + one adapter per provider, one active provider in `config/payment.toml`
* Hard-code a single provider

## Decision Outcome

Chosen option: "`PaymentGateway` port + adapters, one active provider in config".

* Port: `create_checkout`, `confirm`, `refund`, `parse_webhook`, `kind` (embedded or redirect checkout).
* Card / account data are entered only on the provider's hosted fields or pages; the server receives a signed, idempotent webhook.
* `config/payment.toml` selects the provider and mode (`sandbox` / `live`); secrets only in `.env`. Startup fails on an unknown provider, a missing secret or live/test key mismatch.

### Consequences

* Good, because adding a provider = one adapter + one enum variant + one TOML block
* Bad, because only the common subset of provider features is used

### Confirmation

Contract tests per adapter against the provider sandbox; webhook signature and idempotency tests.
