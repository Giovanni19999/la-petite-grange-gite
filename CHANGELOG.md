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
