# CI/CD

This repository uses GitHub Actions.

## CI

Workflow: `.github/workflows/ci.yml`

CI runs on:

- pushes to `main`, `develop`, `feature/**`, `release/**`, and `hotfix/**`
- pull requests into `main` or `develop`
- manual dispatch

Current checks:

- repository health check through `scripts/ci/check_repo.sh`
- Rust: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`
- PostgreSQL migration idempotency
- Python ML service install and `pytest`
- Web console `npm ci`, TypeScript lint, Vitest, and production build

## GitHub Actions Runtime

GitHub announced Node.js 20 deprecation for JavaScript actions and migration toward Node.js 24. Workflows use Node 24-compatible action majors:

- `actions/checkout@v6`
- `actions/setup-python@v6`
- `actions/setup-node@v6`

Keep hosted runners on GitHub-managed `ubuntu-latest`. If self-hosted runners are introduced, they must be at least runner `v2.327.1` before using the v6 setup actions.

## CD

Workflow: `.github/workflows/release.yml`

The current CD target is GitHub Releases. A release is published when a semantic version tag matching `v*.*.*` is pushed.

Example:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Deployment to an external runtime is intentionally not configured yet because this repository does not have an application, environment, or deployment target.
