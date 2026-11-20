---
status: accepted
date: 2026-11-20
---

# Media: processed by the API into WebP sizes, stored on a volume, served by Caddy

## Context and Problem Statement

The owners upload photos from a camera or a phone (F003): large JPEG or PNG files, often several megabytes, with EXIF metadata (including the GPS position of the house) and an orientation flag. Visitors mostly browse on phones over slow rural connections. Where are the images processed, stored and served?

## Decision Drivers

* Fast pages on mobile: right-sized, compressed images (`srcset`), long browser cache
* Privacy: no EXIF / GPS data published
* Low maintenance on a single VPS (ADR-0012), no paid service, simple backups
* Static API image with no shared library (ADR-0016)

## Considered Options

* Processing in the API (Rust `image` for decoding and resizing, `webp` / libwebp for lossy WebP), files on a Docker volume served by Caddy
* External image service or CDN with on-the-fly resizing
* Processing in the browser before upload

## Decision Outcome

Chosen option: "Processing in the API, volume served by Caddy".

* At upload the API decodes the image (JPEG, PNG, WebP), applies the EXIF orientation, rejects images above a pixel limit, and writes **WebP** versions at a few widths (never larger than the original) with all metadata removed. The original is kept in a private directory (not served) so that sizes can be regenerated later.
* Files are named after the content hash (`<hash>-<width>.webp`): a new upload never overwrites a published file, so Caddy serves `/uploads/*` with `Cache-Control: public, max-age=31536000, immutable`.
* The `uploads` volume is written by the API (non-root) and mounted **read-only** in Caddy; the original directory is not mounted in Caddy. Backups copy the volume with the database dump (plan, backups).
* Limits (maximum file size, number of files per upload, pixel limit, widths, WebP quality) are technical settings of the API configuration with defaults.
* `webp` builds libwebp (BSD-3-Clause) from source into the static binary: no shared library, ADR-0016 still holds.
* Images are referenced by id in page content (F002); the API resolves ids to URLs and sizes for the public pages.

### Consequences

* Good, because pages get small WebP files at the right width, cached for a year, with no metadata
* Good, because no external service, and everything stays in the existing backup
* Bad, because upload requests take some CPU time on the VPS (a few hundred milliseconds per photo), done off the async runtime
* Bad, because a C library (libwebp) is compiled into the API; it is widely used and maintained, and updated with the crate

### Confirmation

Back tests check orientation, metadata removal, widths and the pixel limit; the smoke test fetches an uploaded image through Caddy; `cargo deny check` for licenses.
