---
status: accepted
date: 2026-11-20
---

# Static API binary on distroless "static" instead of "cc"

## Context and Problem Statement

ADR-0012 runs the API on `gcr.io/distroless/cc-debian12:nonroot`. That image ships Debian 12 shared libraries (glibc, OpenSSL `libssl3`, `libstdc++6`, `libgcc-s1`, `libgomp1`) and image scanners report their CVEs against the API image (1 critical, 8 high, 12 medium, 16 low on 2026-11-20). The API uses none of OpenSSL, libstdc++ or libgomp: TLS is `rustls` with `ring`, and the binary only needs a C library. How do we keep the runtime image free of these libraries?

## Decision Drivers

* Smallest attack surface and a clean vulnerability report for the image put on the internet
* Keep the guarantees of ADR-0012: no shell, non-root user, single binary with embedded migrations
* No change to the application code

## Considered Options

* Static binary built for the musl target, on `gcr.io/distroless/static-debian12:nonroot`
* Keep `cc-debian12` and wait for Debian updates
* `cc-debian13` (newer Debian, same set of shared libraries)
* glibc binary linked statically (`crt-static`)

## Decision Outcome

Chosen option: "Static musl binary on distroless static", because the runtime image then contains no shared library at all (only CA certificates, time zones and the `nonroot` user), so the reported CVEs disappear instead of waiting for fixes.

* The build stage installs `musl-tools` and builds `--target <arch>-unknown-linux-musl` (the architecture of the build host, amd64 or arm64).
* A static glibc build was rejected: name resolution (`mariadb` service name) relies on NSS modules that static glibc cannot load reliably; musl resolves names itself.
* Everything else in ADR-0012 stays (entrypoint `gite-api`, command `serve`, `create-admin` in the same image).

### Consequences

* Good, because no OpenSSL / glibc / C++ runtime CVE can affect the API image
* Good, because the image is smaller
* Bad, because musl's memory allocator is slower than glibc's; negligible for the load of one holiday cottage, to be revisited (alternative allocator) if profiling ever shows it
* Neutral, because a future dependency needing OpenSSL or a C++ library would require a new decision

### Confirmation

The smoke test (`deploy/smoke-test.sh`) runs the stack built from this image; the image scan in Docker Desktop / Docker Scout shows no vulnerable package for `gite-api`.
