---
id: F022
title: "Containers: API and front end in separate images, behind Caddy"
status: implemented
version: 1
adr: [ADR-0012]
---

# F022 - Containers: API and front end in separate images, behind Caddy

## User story

As the operator of the site,
I want the API and the front end packaged in two separate container images and started together with the database and a reverse proxy,
so that the site runs the same way on my computer, in CI and on the server, and each part can be updated alone.

## Business rules

* Two images: `gite-api` (Rust) and `gite-front` (Angular SSR); both run as a non-root user.
* `docker compose -f deploy/compose.yaml up` starts MariaDB, the API, the front end and Caddy.
* Only Caddy is published on the host; the API and the database are reachable only inside the Compose network.
* `/api/*` goes to the API, every other path to the front end.
* Secrets come from `deploy/.env` (from `deploy/.env.example`); none is baked into an image.

## Acceptance criteria

```gherkin
Feature: Containers

  Scenario: F022-S01 The stack starts and is healthy
    Given deploy/.env is filled in
    When the operator runs docker compose -f deploy/compose.yaml up -d --build
    Then GET http://localhost:8000/api/health returns 200 with "database": "ok"
    And GET http://localhost:8000/ returns the pre-rendered home page

  Scenario: F022-S02 Unknown pages are 404 through the proxy
    When a client opens http://localhost:8000/does-not-exist
    Then the response status is 404

  Scenario: F022-S03 Images run as non-root users
    When the operator inspects the running api and front containers
    Then neither process runs as root

  Scenario: F022-S04 The API and the database are not published
    When the operator lists the published ports
    Then only the proxy port is published on the host

  Scenario: F022-S05 Admin accounts can be created with the API image
    When the operator runs docker compose -f deploy/compose.yaml run --rm api create-admin --email owner@example.test
    Then the command uses the same image as the API
```

## Out of scope

* Production HTTPS certificates and the real domain (set at deployment), image registry, automatic deployment.

## Tests

| Scenario | Test |
|---|---|
| F022-S01 | `deploy/smoke-test.sh` |
| F022-S02 | `deploy/smoke-test.sh` |
| F022-S03 | `deploy/smoke-test.sh` |
| F022-S04 | `deploy/smoke-test.sh` |
| F022-S05 | manual check (command documented in the README) |

## Changelog

* v1 (2026-11-20): first version.
