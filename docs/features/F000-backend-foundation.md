---
id: F000
title: "Backend foundation: configuration, health check, error format"
status: frozen
version: 1
adr: [ADR-0002, ADR-0003, ADR-0006, ADR-0007, ADR-0010]
---

# F000 - Backend foundation: configuration, health check, error format

## User story

As the developer and operator of the site,
I want the API to start only with a valid configuration and to expose a health check,
so that misconfigurations fail fast and monitoring can tell whether the API and its database are up.

## Business rules

* Technical configuration is read from environment variables (`.env` in development) and validated at startup; secrets are never logged.
* `APP_ENV` is one of `dev`, `test`, `staging`, `production`.
* The server refuses to start when a required variable is missing or invalid, with a clear message naming the variable.
* `GET /api/health` returns the API status, version and database status. It exposes no secret and no internal detail.
* Every API error is returned as JSON `{ "error": { "code": "...", "message": "..." } }` with the matching HTTP status; internal errors never leak details to the client.

## Acceptance criteria

```gherkin
Feature: Backend foundation

  Scenario: F000-S01 Health check when the database is reachable
    Given the API is running with a reachable database
    When a client calls GET /api/health
    Then the response status is 200
    And the body contains "status": "ok", the API version and "database": "ok"

  Scenario: F000-S02 Health check when the database is unreachable
    Given the API is running and the database becomes unreachable
    When a client calls GET /api/health
    Then the response status is 503
    And the body contains "status": "degraded" and "database": "unavailable"

  Scenario: F000-S03 Startup refused on missing configuration
    Given DATABASE_URL is not set
    When the API starts
    Then it exits with an error naming DATABASE_URL

  Scenario: F000-S04 Startup refused on invalid environment name
    Given APP_ENV is set to "prod"
    When the API starts
    Then it exits with an error listing the allowed values

  Scenario: F000-S05 Unknown route returns a JSON error
    Given the API is running
    When a client calls GET /api/does-not-exist
    Then the response status is 404
    And the body is {"error": {"code": "not_found", "message": "..."}}
```

## Out of scope

* Authentication, business endpoints, payment and channel manager configuration files (later features).

## Tests

| Scenario | Test |
|---|---|
| F000-S01 | `tests/health.rs::f000_s01_health_ok` |
| F000-S02 | `tests/health.rs::f000_s02_health_degraded` |
| F000-S03 | `src/config.rs::tests::f000_s03_missing_database_url` |
| F000-S04 | `src/config.rs::tests::f000_s04_invalid_app_env` |
| F000-S05 | `tests/health.rs::f000_s05_unknown_route_json_error` |

## Changelog

* v1 (2026-11-20): first version.
