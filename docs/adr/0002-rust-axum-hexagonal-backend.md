---
status: accepted
date: 2026-11-20
---

# Rust + axum backend with a hexagonal architecture

## Context and Problem Statement

The backend handles bookings, payments confirmations, calendar synchronisation and owner-editable rules. Business rules (deposit, balance deadline, cancellation, availability) must be correct and testable without a database or network.

## Decision Drivers

* Correctness of money and date rules
* Testability of the domain in isolation
* Ability to swap external providers (payment, channel manager, SMS, mail) by configuration
* Low hosting cost (small VPS)

## Considered Options

* Rust (axum, tokio) with hexagonal architecture (ports and adapters)
* Node.js (NestJS) layered architecture
* Rust with a framework-centric layered design (handlers calling SQL directly)

## Decision Outcome

Chosen option: "Rust (axum, tokio) with hexagonal architecture", because the type system lets us make invalid states unrepresentable (`Money` in cents, `DateRange` with start < end, status enums), the binary is small and fast, and ports keep every provider replaceable.

Layers:

* `domain/`: pure rules, no axum, no sqlx, no I/O.
* `application/`: use cases and ports (traits) such as `PaymentGateway`, `RatePublisher`, `Mailer`.
* `infrastructure/`: adapters implementing the ports (MariaDB, Stripe, Beds24, iCal, SMTP…).
* `http/`: axum routes, DTOs, extractors; thin handlers only.

### Consequences

* Good, because the domain is unit-tested at ~100 % without containers
* Good, because changing a provider means adding an adapter, not touching the domain
* Bad, because more boilerplate (traits, DTO mapping) than a framework-centric design

### Confirmation

* `domain` has no dependency on axum, sqlx, reqwest or tokio I/O (checked in review and by module visibility).
* `cargo clippy -- -D warnings -W clippy::pedantic` in CI; `unwrap()`/`expect()` forbidden outside tests and startup.
