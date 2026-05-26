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

Conditional checks are enabled when the relevant project files exist:

- Rust: runs when `Cargo.toml` exists
- Node: runs when `package.json` exists
- Python: runs when `pyproject.toml` or `requirements.txt` exists

## CD

Workflow: `.github/workflows/release.yml`

The current CD target is GitHub Releases. A release is published when a semantic version tag matching `v*.*.*` is pushed.

Example:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Deployment to an external runtime is intentionally not configured yet because this repository does not have an application, environment, or deployment target.
