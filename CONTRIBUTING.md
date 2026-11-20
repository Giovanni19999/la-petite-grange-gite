# Contributing

## Branches

```
main      ← stable, protected: changes only through a pull request from develop, CI green
  ↑ PR
develop   ← integration branch: every feature is merged here first
  ↑ PR
feat/F001-admin-auth, fix/…, docs/…   ← one short-lived branch per feature or fix, created from develop
```

1. Create the branch from `develop`: `git switch develop && git pull && git switch -c feat/F001-admin-auth`.
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
