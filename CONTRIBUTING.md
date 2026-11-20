# Contributing

## Branches

```
main      ← stable, protected: changes only through a pull request (from develop, or from a hotfix branch), CI green
  ↑ PR                      ↑ PR (then merged back into develop)
develop                  hotfix/…   ← the ONLY branch type created from main: urgent bug fix in production
  ↑ PR
feat/F001-admin-auth, fix/…, docs/…   ← one short-lived branch per feature or fix, created from develop
```

Every branch is created from `develop`, **except** `hotfix/…` branches, which are created from `main` to fix a production bug. A hotfix is merged into `main` and then into `develop`, so the fix is never lost.

1. Create the branch from `develop`: `git switch develop && git pull && git switch -c feat/F001-admin-auth` (hotfix only: `git switch main && git pull && git switch -c hotfix/short-description`).
2. Freeze the feature sheet in `docs/features/` before coding.
3. Commit each step with [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `build:`, `ci:`…).
4. Open a pull request to `develop`; merge when CI is green (see the rules below).
5. Release: pull request from `develop` to `main`, then tag the version (`vX.Y.Z`) and update `CHANGELOG.md`.

## Pull request rules

| Rule | Detail | Checked by |
|---|---|---|
| Title | Conventional Commits, lowercase subject, no final period: `feat(back): add admin login` | CI (`PR title`) |
| Branch name | `feat/`, `fix/`, `docs/`, `test/`, `refactor/`, `perf/`, `build/`, `ci/`, `chore/`, `hotfix/` | CI (`Source and target branches`) |
| Target | `develop`; only `develop` (release) and `hotfix/…` may target `main` | CI (`Source and target branches`) |
| Size | one feature or one fix per pull request; aim for less than ~400 changed lines (lock files excluded) | review |
| Description | filled [template](.github/pull_request_template.md): what, linked feature sheet / ADR, how it was tested | review |
| Feature sheet | `frozen` before coding, set to `implemented` in the same pull request | review |
| Checks | all CI jobs green; conversations resolved | branch protection |
| Changelog | `CHANGELOG.md` updated under *Unreleased* (not needed for `ci`, `build`, `chore`, `test`) | review |
| Security | no secret, no personal data, no real guest data | gitleaks + review |

Merge methods:

| Pull request | Method | Why |
|---|---|---|
| `feat/…`, `fix/…`, … → `develop` | **Rebase and merge** | keeps one Conventional Commit per step, linear history |
| `develop` → `main` (release) | **Create a merge commit** | `main` and `develop` keep the same commits |
| `hotfix/…` → `main`, then `main` → `develop` | **Create a merge commit** | the fix reaches both branches without rewriting history |

After merging, the source branch is deleted (except `develop`).

## Local checks

```bash
cd Back
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo deny check
```

```bash
cd Front
npm run lint
npm run format:check
npm run test:ci
npm run build && npm run check:ssr
npm run licenses
```
