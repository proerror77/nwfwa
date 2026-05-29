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
- Rust: `cargo fetch --locked`, `cargo fmt --all -- --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, and `cargo test --locked --workspace`
- PostgreSQL migration idempotency
- demo seed idempotency and minimum demo-data presence through `scripts/demo/seed_demo.sh`
- Python ML service install and `pytest`
- Web console `npm ci`, TypeScript lint, Vitest, and production build

## Rust Compilation

Rust CI is optimized for fast, reproducible validation rather than release artifact generation.

- Dependency resolution is locked with `Cargo.lock` through `--locked`.
- `Swatinem/rust-cache@v2` caches the Cargo registry, git dependencies, and build artifacts.
- CI disables incremental compilation because GitHub-hosted runners are cold and cache restores are a better fit for repeated builds.
- CI disables dev/test debug info with `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0` to reduce compile output size and linking work.
- Local dev/test profiles keep line-table debug info through `debug = 1`, which is lighter than full debug info while preserving useful failure locations.
- Release-mode Rust builds are intentionally not part of default CI yet. Add `cargo build --release --locked --workspace` only when release artifact validation becomes a required signal.

## GitHub Actions Runtime

GitHub announced Node.js 20 deprecation for JavaScript actions and migration toward Node.js 24. Workflows use Node 24-compatible action majors:

- `actions/checkout@v6`
- `actions/setup-python@v6`
- `actions/setup-node@v6`
- `Swatinem/rust-cache@v2`

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
