---
status: accepted
date: 2026-11-20
---

# Admin authentication with server-side sessions in an HttpOnly cookie

## Context and Problem Statement

Only the owners (a few accounts) log into the admin, mostly from their phones. Logging out, or revoking a stolen device, must take effect immediately. The public site and the admin are served from the same domain.

## Decision Drivers

* Immediate revocation (logout, password change)
* No token readable by JavaScript (XSS cannot steal it)
* Simple for a single server with a database
* Comfortable on a phone: stay logged in for weeks

## Considered Options

* Opaque random session token in an `HttpOnly; Secure; SameSite=Strict` cookie, only its SHA-256 hash stored in the database
* Stateless JWT in an HttpOnly cookie
* JWT in `localStorage` with an `Authorization` header

## Decision Outcome

Chosen option: "Opaque session token in an HttpOnly cookie, hashed in the database", because a database lookup per admin request is negligible here and revocation is immediate.

* Token: 32 random bytes (OS CSPRNG), base64url; the database stores only `SHA-256(token)`, so a database leak does not leak usable sessions.
* Passwords hashed with **Argon2id** (`argon2` crate defaults); a wrong email and a wrong password return the same error and take the same time (a dummy hash is verified when the email is unknown).
* Session lifetime: absolute expiry (default 30 days) and idle expiry (default 7 days, refreshed on use) — technical security settings in configuration.
* Login attempts are rate-limited per client IP and per email (default 5 failures per 15 minutes) → HTTP 429.
* CSRF: `SameSite=Strict` cookie, state-changing endpoints accept JSON only.
* Accounts are created with a command-line tool on the server (`gite-api create-admin`); there is no public sign-up.

This supersedes the "JWT in cookie" line of the original technical plan.

### Consequences

* Good, because logout and revocation are immediate; no secret signing key to rotate
* Bad, because every admin request hits the database (acceptable at this scale)

### Confirmation

Acceptance tests of F001 (login, wrong password, unknown email, rate limit, logout, expired session, protected route without cookie).
