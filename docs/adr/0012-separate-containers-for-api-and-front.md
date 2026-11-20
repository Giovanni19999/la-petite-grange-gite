---
status: accepted
date: 2026-11-20
---

# Separate containers for the Rust API and the Angular SSR server, behind Caddy

## Context and Problem Statement

The site runs on one small VPS with Docker Compose (see the architecture overview). The Rust API and the Angular SSR server have different toolchains, release rhythms and runtime needs. How are they packaged and exposed?

## Decision Drivers

* Independent build, deployment and restart of the API and the front end
* Small, non-root images with a minimal attack surface
* One public entry point with HTTPS
* Same images in local tests, CI and production

## Considered Options

* Two images (API, front) + MariaDB + Caddy, assembled by Docker Compose
* One image running both processes
* Front served as static files by Caddy (no SSR)

## Decision Outcome

Chosen option: "Two images + MariaDB + Caddy with Docker Compose".

* **API image** (`Back/Dockerfile`): multi-stage build on the official Rust image (`SQLX_OFFLINE=true`), runtime on `gcr.io/distroless/cc-debian12:nonroot`: no shell, non-root user, binary only (migrations are embedded). Entrypoint `gite-api`, default command `serve`, so `create-admin` runs in the same image.
* **Front image** (`Front/Dockerfile`): multi-stage build on the official Node image, runtime on `node:<lts>-alpine` as the non-root `node` user, running the bundled SSR server only (no `node_modules`).
* **Caddy** is the only public entry point: `/api/*` → API, everything else → front; HTTPS with Let's Encrypt in production. The API runs with `TRUST_PROXY_HEADERS=true` and is not exposed directly.
* **MariaDB** is on the internal network only, with a persistent volume.
* Compose files and the Caddyfile live in `deploy/`; secrets in `deploy/.env` (never committed).

### Consequences

* Good, because each part can be rebuilt and restarted alone, and images are small and non-root
* Bad, because two images to build and keep up to date (automated in CI)

### Confirmation

CI builds both images on every push; F022 acceptance scenarios check the running stack.
