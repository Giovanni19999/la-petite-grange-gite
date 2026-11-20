---
status: accepted
date: 2026-11-20
---

# Public URLs: French at the root, English under /en; SSR fetches the API directly

## Context and Problem Statement

The site is bilingual (French first, English second) and rendered on the server for search engines. Pages are built from blocks stored by the API. Which URLs do pages get in each language, and how does the SSR server reach the API?

## Decision Drivers

* French is the main audience: short French URLs, French at `/`
* One URL per language for search engines (`hreflang`)
* The SSR server runs in its own container (ADR-0012) and cannot use relative `/api` URLs

## Considered Options

* French at the root (`/`, `/<slug-fr>`), English under `/en` (`/en`, `/en/<slug-en>`)
* Both languages prefixed (`/fr/...`, `/en/...`)
* Language from the browser only, same URL

## Decision Outcome

Chosen option: "French at the root, English under `/en`".

* Each page has a French and an English slug; the home page is `/` and `/en`.
* The language switch links to the same page in the other language; pages declare `hreflang` alternates.
* English texts fall back to French when empty.
* The browser calls the API with relative URLs (`/api/...`, through Caddy or the dev proxy). The **SSR server** calls the API with the absolute base URL from `API_INTERNAL_URL` (`http://api:8080` in Docker Compose, `http://127.0.0.1:8080` in development); an HTTP interceptor active only on the server adds it.
* Public pages are rendered on the server at request time (they change when the owners publish), not pre-rendered at build time. An unknown page returns HTTP 404.

### Consequences

* Good, because French URLs stay short and every page has one URL per language
* Bad, because pages are rendered per request (acceptable at this traffic; caching can be added later)

### Confirmation

F002 acceptance scenarios (French and English URLs, fallback, 404, SSR HTML contains the page content).
