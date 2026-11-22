---
status: accepted
date: 2026-11-22
---

# Message templates written with the rich text editor and rendered by MiniJinja in a sandbox

## Context and Problem Statement

The owners write the emails and SMS sent by the site (F007, then F016), with variables such as the guest's first name or the arrival date. Emails must look good in every mail client and carry the cottage's layout; texts typed by the owners and values typed by guests must never inject HTML nor code. How are templates stored, checked and rendered?

## Decision Drivers

* The owners write with the editor they already know (ADR-0015)
* Unknown or misspelt variables refused when saving, not discovered when sending
* No injection: owner HTML sanitised, guest values escaped, no file or code access from a template
* A plain-text version for every email (spam filters, text-only clients)

## Considered Options

* MiniJinja (sandboxed Jinja-like engine) over HTML from Tiptap sanitised by `ammonia`, with an allow-list of variables per template
* Tera or Handlebars
* Home-made replacement of `{{ name }}` placeholders

## Decision Outcome

Chosen option: "MiniJinja over sanitised Tiptap HTML", because it is small, maintained, has no file loader unless one is given, escapes HTML automatically, and can list the variables a template uses.

* Templates are stored in the database (`message_templates`): trigger, recipient (guest / owners), channel, language, subject, body; default French and English texts are seeded and can be restored.
* The email body is HTML from the Tiptap editor, **sanitised by `ammonia`** with the allow-list of ADR-0015 before saving. Only plain variables `{{ a.b }}` are accepted: on save, the template is parsed, every variable used is compared with the allow-list of its trigger, and tags (`{% … %}`) are refused, so the owners cannot write loops, includes or filters.
* Rendering uses an environment with no loader, auto-escaping on for HTML, strict undefined variables, and only the values of the booking given as context. The rendered body goes into a fixed layout (cottage name, footer), and a plain-text version is derived from the HTML.
* SMS and email subjects are plain text, rendered without HTML escaping, then cut of control characters.
* Dates and amounts are formatted in the language of the message before rendering (the template receives strings).
* The rendered message is queued by F025 (ADR-0019); attachments are referenced by the queued message and read when sending.

### Consequences

* Good, because errors are found when saving, with the name of the wrong variable
* Good, because the same engine serves the other templates of F016 later
* Bad, because one more dependency (`minijinja`, Apache-2.0) to keep up to date
* Neutral, because the Tiptap toolbar for emails is a subset of the page editor's, kept aligned with `ammonia` by tests

### Confirmation

Unit tests: unknown variable refused, tag refused, guest value with `<script>` escaped, English falls back to French; `cargo deny check` for the licence.
