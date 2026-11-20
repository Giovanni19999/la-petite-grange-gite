---
id: F001
title: "Owner authentication (admin login)"
status: implemented
version: 1
adr: [ADR-0011, ADR-0002, ADR-0004]
---

# F001 - Owner authentication (admin login)

## User story

As an owner,
I want to log into the admin with my email and password, stay logged in on my phone, and log out,
so that only we can manage the site.

## Business rules

* Accounts are created by the operator with `gite-api create-admin --email <email>` (password typed twice, hidden); there is no sign-up page.
* Password: at least 12 characters (technical security rule).
* Email is case-insensitive and trimmed.
* Login errors never reveal whether the email exists: always "Email ou mot de passe incorrect".
* After 5 failed attempts in 15 minutes for the same email or the same IP, login is refused with HTTP 429 until the window passes.
* A session lasts at most 30 days and ends after 7 days without use (configurable technical settings).
* Logging out ends the session immediately, on the server.
* Every `/admin` page except the login page requires a valid session; otherwise the owner is sent to `/admin/connexion`, then back to the requested page after login.
* Every `/api/admin/*` endpoint requires a valid session; otherwise HTTP 401.

## Acceptance criteria

```gherkin
Feature: Owner authentication

  Background:
    Given an owner account "proprio@example.test" with password "correct horse battery"

  Scenario: F001-S01 Successful login
    When the owner logs in with "PROPRIO@example.test " and "correct horse battery"
    Then the response status is 200
    And an HttpOnly, Secure, SameSite=Strict session cookie is set
    And GET /api/auth/me returns the owner's email

  Scenario: F001-S02 Wrong password
    When the owner logs in with "proprio@example.test" and "wrong password!"
    Then the response status is 401
    And the error message is "Email ou mot de passe incorrect"
    And no session cookie is set

  Scenario: F001-S03 Unknown email gives the same answer
    When someone logs in with "nobody@example.test" and "whatever password"
    Then the response status is 401
    And the error message is "Email ou mot de passe incorrect"

  Scenario: F001-S04 Too many failed attempts
    Given 5 failed login attempts for "proprio@example.test" in the last 15 minutes
    When the owner logs in with the correct password
    Then the response status is 429

  Scenario: F001-S05 Logout ends the session
    Given the owner is logged in
    When the owner logs out
    Then the session cookie is cleared
    And GET /api/auth/me with the old cookie returns 401

  Scenario: F001-S06 Expired session
    Given a session that has not been used for more than 7 days
    When GET /api/auth/me is called with its cookie
    Then the response status is 401

  Scenario: F001-S07 Admin pages require a session
    Given the owner is not logged in
    When the owner opens "/admin"
    Then the login page "/admin/connexion" is displayed
    And after a successful login the owner is back on "/admin"

  Scenario: F001-S08 Account creation from the command line
    When the operator runs create-admin with a valid email and a password of 12 characters or more
    Then the account exists and the password is stored only as an Argon2id hash
    And running it with a shorter password is refused
```

## Out of scope

* Password reset by email, two-factor authentication, account management screens (later).

## Tests

| Scenario | Test |
|---|---|
| F001-S01 | `Back/tests/auth.rs::f001_s01_successful_login` |
| F001-S02 | `Back/tests/auth.rs::f001_s02_wrong_password` |
| F001-S03 | `Back/tests/auth.rs::f001_s03_unknown_email` |
| F001-S04 | `Back/tests/auth.rs::f001_s04_rate_limited` |
| F001-S05 | `Back/tests/auth.rs::f001_s05_logout` |
| F001-S06 | `Back/src/domain/auth.rs::tests::f001_s06_idle_session_expired` |
| F001-S07 | `Front/src/app/core/auth/auth.guard.spec.ts`, `Front/src/app/features/admin/login/login.page.spec.ts` |
| F001-S08 | `Back/src/application/admin_accounts.rs::tests::f001_s08_*`, `Back/src/cli.rs::tests` |

## Changelog

* v1 (2026-11-20): first version.
