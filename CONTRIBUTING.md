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
4. Open a pull request to `develop`; merge when CI is green (squash or rebase, no merge commits on feature branches).
5. Release: pull request from `develop` to `main`, then tag the version (`vX.Y.Z`) and update `CHANGELOG.md`.

## Local checks

```bash
cd Back
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo deny check
```
