---
status: accepted
date: 2026-11-20
---

# MariaDB with sqlx (compile-time checked queries, SQL migrations)

## Context and Problem Statement

Data is relational (accommodations, bookings, payments, prices per channel, reviews) and must stay consistent under concurrent booking attempts.

## Decision Drivers

* Transactions and foreign keys
* Familiar, free, easy to host and back up on a small VPS
* No ORM magic: explicit SQL, checked at compile time

## Considered Options

* MariaDB 11 LTS + sqlx (mysql driver)
* PostgreSQL + sqlx
* SQLite

## Decision Outcome

Chosen option: "MariaDB 11 LTS + sqlx", per project constraint and hosting familiarity.

Conventions: InnoDB, `utf8mb4`, money in cents (`BIGINT`), timestamps `DATETIME(3)` in UTC, enums as `VARCHAR` + `CHECK`, migrations with `sqlx migrate`, offline query metadata (`cargo sqlx prepare`) committed for CI.

### Consequences

* Good, because queries are type-checked against the schema
* Bad, because MariaDB has no exclusion constraint on ranges: double-booking prevention relies on a transaction with `SELECT … FOR UPDATE` on the accommodation row (enforced in the repository, covered by an integration test)

### Confirmation

Integration tests run against a disposable MariaDB container (`testcontainers`).
