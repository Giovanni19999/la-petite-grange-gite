# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Documentation structure: ADRs (MADR 4), architecture overview (arc42 + C4), feature sheets with Gherkin acceptance criteria.
- F000 backend foundation: typed configuration validated at startup, `GET /api/health` (API version and database status), JSON error format, local MariaDB with Docker Compose.
- F021 frontend foundation: Angular 22 with SSR and pre-rendering, Tailwind CSS v4 and spartan/ui with a single light blue theme, lazy public and admin areas, 404 page, API proxy, ESLint, Prettier, Vitest and license checks.
- CI job for the front end (lint, format, tests, build, SSR checks, licenses).
- F001 owner authentication: server-side sessions in an HttpOnly, Secure, SameSite=Strict cookie (ADR-0011), Argon2id passwords, throttled login, `POST /api/auth/login`, `POST /api/auth/logout`, `GET /api/auth/me`, `gite-api create-admin` command, admin login page, route guard and logout.
- First database migration (users, sessions), applied at startup; acceptance tests on a disposable MariaDB container.
- F022 containers: API image (distroless, non-root, ~31 MB) and front-end image (Alpine, non-root), Docker Compose stack in `deploy/` with MariaDB on an internal network and Caddy as the only entry point, smoke test and CI job running the stack.
- F002 pages built from blocks: hero, text, text and image, columns and gallery blocks validated and sanitised by the API, draft / publish with version check (409 when stale), summary of changes, versions and restore, admin endpoints, seed pages; public pages rendered on the server in French (`/`) and English (`/en`, ADR-0013) with menu, language switch, hreflang and real 404 status.
