---
status: accepted
date: 2026-11-20
---

# Allow MPL-2.0 only for the dependencies of the HTML sanitiser

## Context and Problem Statement

Rich text written by the owners (F002) is sanitised by `ammonia`, the reference HTML sanitiser in Rust. Since version 4.1, `ammonia` depends on `cssparser` and `dtoa-short`, licensed under MPL-2.0, which is not in the dependency license allow-list (MIT, Apache-2.0, ISC, BSD, Zlib, Unicode…). `cargo deny` therefore fails.

## Decision Drivers

* Security: a well-maintained, widely used sanitiser rather than a home-made one
* License compatibility with the MIT-licensed code of this repository
* Keep the allow-list strict for every other dependency

## Considered Options

* Scoped exception: allow MPL-2.0 only for `cssparser` and `dtoa-short`
* Older `ammonia` without those dependencies (4.0, unmaintained line)
* Own sanitiser built on `html5ever`
* Allow MPL-2.0 globally

## Decision Outcome

Chosen option: "Scoped exception for `cssparser` and `dtoa-short`".

* MPL-2.0 is a weak, **file-level** copyleft: unmodified MPL files can be combined with MIT code without changing the license of our code; their sources stay available on crates.io. We do not modify them.
* The exception is declared per crate in `Back/deny.toml` (`[[licenses.exceptions]]`); any other MPL-2.0 dependency is still rejected.
* If these crates are ever modified or vendored, the modified files must stay under MPL-2.0 and be published.

### Consequences

* Good, because the site keeps a robust sanitiser and the allow-list stays strict
* Bad, because one more rule to keep in mind when updating `ammonia` (an update that adds another MPL crate fails CI and needs a new decision)

### Confirmation

`cargo deny check` in CI.
