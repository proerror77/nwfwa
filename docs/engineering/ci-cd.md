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
- local runtime launcher presence and shell syntax for
  `scripts/dev/start_local_runtime.sh` and `scripts/dev/stop_local_runtime.sh`
- `staging-proof`: Kubernetes manifest validation, container packaging checks,
  staging deployment package validation, staging evidence artifacts,
  operational drill proof validation, and MLOps monitoring-plan simulation
- Rust: `cargo fetch --locked`, `cargo fmt --all -- --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, and `cargo test --locked --workspace`
- PostgreSQL migration idempotency
- demo seed idempotency, minimum demo-data presence, and API/ML demo smoke through `scripts/demo/seed_demo.sh` and `scripts/demo/smoke_demo.py`
- Python ML service install and `pytest`
- Web console Rust/WASM check, Trunk production build, and Node-based build smoke

Every job has a timeout so a stuck service, dependency install, or demo smoke
cannot hold the branch gate indefinitely. Python jobs use pip dependency caching
keyed by `apps/ml-service/pyproject.toml`; Rust jobs keep the Cargo cache through
`Swatinem/rust-cache@v2`.

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
- `dtolnay/rust-toolchain@stable`
- `Swatinem/rust-cache@v2`

Keep hosted runners on GitHub-managed `ubuntu-latest`. If self-hosted runners are introduced, they must be at least runner `v2.327.1` before using the v6 setup actions.

## CD

Workflow: `.github/workflows/release.yml`

The current CD target is GitHub Releases. A release is published when a semantic version tag matching `v*.*.*` is pushed.
The release workflow rejects manual dispatch tags that do not match
`vMAJOR.MINOR.PATCH`, and it verifies that the tag commit is reachable from
`origin/main` before publishing. This keeps releases tied to the protected
production-ready branch.

Workflow: `.github/workflows/deploy-staging.yml`

`Deploy Staging` is the GitHub Environment gated staging deployment workflow.
It runs only through manual dispatch and uses the `staging` GitHub Environment.
This is the repository's GitHub Environment based deployment boundary for
staging.
The job verifies a successful CI run for the selected commit by default,
validates Kubernetes staging manifests and container packaging, then builds a
deployment package with `scripts/ops/build_staging_deployment_package.py`.
The generated package is validated by
`scripts/ops/validate_staging_deployment_package.py` before upload.

The package includes:

- copied `infra/k8s/staging` manifests;
- `deployment_manifest.json` with commit, image tag, package checksums, and
  environment boundary;
- `apply.sh`, which requires `NWFWA_STAGING_SECRET_FILE` and applies the
  package to a customer-approved staging cluster;
- `rollback.md`, which keeps rollback tied to the previous approved package or
  a reverted commit and preserves the human approval gate for destruction.

The workflow uploads the package as a GitHub Actions artifact. It does not run
`kubectl apply` by itself because cluster credentials, secrets, and customer
environment ownership are still environment-specific.

Example:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Deployment to an external runtime is intentionally package-only for now.
Kubernetes staging manifests now exist under `infra/k8s/staging`, and the GitHub
Environment workflow packages them for a customer-approved staging cluster.
Production deployment remains intentionally unconfigured until image registry,
managed secrets, network controls, observability, and customer environment
ownership are selected.

Local Docker Desktop development is separate from deployment packaging. Use
`scripts/dev/start_local_runtime.sh` for the supported hybrid local runtime;
use full Docker Compose or K3d simulation when validating packaging or
Kubernetes-style scheduling.
