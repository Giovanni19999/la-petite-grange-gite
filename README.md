# La Petite Grange — holiday cottage website

[![CI](https://github.com/Giovanni19999/la-petite-grange-gite/actions/workflows/ci.yml/badge.svg)](https://github.com/Giovanni19999/la-petite-grange-gite/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Website of a 3-star holiday cottage in France: bilingual showcase (FR/EN), **instant direct booking** confirmed by an online deposit, and a mobile-first admin that lets non-technical owners manage pages, photos, prices per channel, calendar, bookings, reviews and automatic messages. Airbnb and Booking.com stay in place, synchronised through a channel manager.

> Work in progress: the backend foundation is in place; features are delivered one by one, each frozen in `docs/features/` before being coded.

## Stack

| Part | Technologies |
|---|---|
| Back | Rust, axum, tokio, sqlx, MariaDB 11, tracing — hexagonal architecture (ports and adapters) |
| Front (planned) | Angular SSR, spartan/ui, Tailwind CSS v4 |
| External services | payment provider (Stripe, Mollie…) and channel manager (Beds24…) selected by configuration, iCal fallback |
| Quality | clippy pedantic, rustfmt, unit and acceptance tests, cargo-deny (licenses, advisories), gitleaks, GitHub Actions |

## Documentation

* [Architecture overview](docs/architecture/README.md) (arc42 + C4 diagrams)
* [Architecture decision records](docs/adr/) (MADR)
* [Features](docs/features/README.md) (user stories and Gherkin acceptance criteria)
* [Changelog](CHANGELOG.md)

## Run the API locally

Requirements: Rust (stable), Docker.

```bash
cd Back
cp .env.example .env        # then set your own passwords
docker compose up -d        # local MariaDB
cargo run
curl http://127.0.0.1:8080/api/health
```

Tests and checks:

```bash
cd Back
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Repository layout

```
Back/   Rust API (domain, application, infrastructure, http)
docs/   architecture, ADRs, feature sheets
```

## License

Code under the [MIT License](LICENSE). Photos, texts and logo of La Petite Grange are **not** covered by this license.
