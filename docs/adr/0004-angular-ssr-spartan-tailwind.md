---
status: accepted
date: 2026-11-20
---

# Angular SSR front end with spartan/ui and Tailwind CSS v4, light theme only

## Context and Problem Statement

The public site must be indexed by search engines and fast on mobile; the admin must be usable on a phone by non-technical owners. The developer is not a web designer.

## Decision Drivers

* SEO (server-side rendering)
* Ready-made, accessible components with a permissive license
* Almost no hand-written CSS

## Considered Options

* Angular SSR + spartan/ui (MIT) + Tailwind v4
* Angular + PrimeNG
* Angular + Angular Material

## Decision Outcome

Chosen option: "Angular SSR + spartan/ui + Tailwind v4", because components are copied into the project (no license risk), styled with Tailwind, and themable with CSS variables.

* One light theme chosen by the owners from predefined themes (blue by default). **No dark mode**: `color-scheme: light`, Tailwind `dark:` variants not used.
* Mobile first everywhere.

### Consequences

* Good, because all dependencies are MIT/ISC and checked in CI
* Bad, because spartan/ui is younger than Material; a missing component may need Angular CDK

### Confirmation

License allow-list in CI; lint rule rejects hard-coded colors (theme tokens only).

## Pros and Cons of the Options

### Angular + PrimeNG

* Bad, because its license became proprietary from v22 (annual key)
