# CI/CD

This repository uses GitHub Actions.

## CI

Workflow: `.github/workflows/ci.yml`

CI runs on:

- pushes to `main`, `develop`, `feature/**`, `release/**`, and `hotfix/**`
- pull requests into `main` or `develop`
- manual dispatch

Current required check:

- repository health check through `scripts/ci/check_repo.sh`

Language-specific build checks should be added when those project files exist:

- Rust: `cargo fmt --all -- --check` and `cargo test --all --locked`
- Node: `npm ci`, lint, and tests
- Python: install project dependencies and run tests

## CD

Workflow: `.github/workflows/release.yml`

The current CD target is GitHub Releases. A release is published when a semantic version tag matching `v*.*.*` is pushed.

Example:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Deployment to an external runtime is intentionally not configured yet because this repository does not have an application, environment, or deployment target.
