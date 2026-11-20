---
status: accepted
date: 2026-11-20
---

# Block-based page editor instead of free positioning

## Context and Problem Statement

Non-technical owners must edit texts, images and layout of the public pages without breaking the mobile rendering.

## Considered Options

* Pages as an ordered list of typed blocks with layout options (image left/right, columns, background role)
* Free positioning (pixel-based, Wix-like)

## Decision Outcome

Chosen option: "Typed blocks", because free positioning almost always breaks on mobile, where most guests browse.

* Draft / publish, version history, confirmation dialog summarising the changes before publishing (409 if the draft changed meanwhile).
* Block JSON validated server side; rich text sanitised (`ammonia`).
* New block type = new component registered in a block registry (open/closed principle).

### Consequences

* Good, because the site always stays responsive and on-theme
* Bad, because less layout freedom than a free editor
