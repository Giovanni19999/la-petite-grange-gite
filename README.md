# La Petite Grange — holiday cottage website

[![CI](https://github.com/Giovanni19999/la-petite-grange-gite/actions/workflows/ci.yml/badge.svg)](https://github.com/Giovanni19999/la-petite-grange-gite/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Website of a 3-star holiday cottage in France: bilingual showcase (FR/EN), **instant direct booking** confirmed by an online deposit, and a mobile-first admin that lets non-technical owners manage pages, photos, prices per channel, calendar, bookings, reviews and automatic messages. Airbnb and Booking.com stay in place, synchronised through a channel manager.

> Work in progress (v0.1): the backend and frontend foundations are in place; features are delivered one by one, each frozen in `docs/features/` before being coded.

## Stack

| Part | Technologies |
|---|---|
| Back | Rust, axum, tokio, sqlx, MariaDB 11, tracing — hexagonal architecture (ports and adapters) |
| Front | Angular 22 (SSR, standalone, signals), spartan/ui, Tailwind CSS v4, Vitest, ESLint, Prettier |
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

## Run the front end locally

Requirements: Node.js 26 LTS (see `Front/.nvmrc`).

```bash
cd Front
npm ci
npm start                   # http://localhost:4200, /api proxied to the API on :8080
```

Checks: `npm run lint`, `npm run format:check`, `npm run test:ci`, `npm run build && npm run check:ssr`, `npm run licenses`.

## Repository layout

```
Back/   Rust API (domain, application, infrastructure, http)
Front/  Angular SSR application (public site and admin)
docs/   architecture, ADRs, feature sheets
```

## License

Code under the [MIT License](LICENSE). Photos, texts and logo of La Petite Grange are **not** covered by this license.
