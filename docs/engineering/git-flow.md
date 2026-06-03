# GitFlow

This repository uses a lightweight GitFlow policy.

## Branches

- `main`: production-ready history and release tags.
- `develop`: integration branch for completed work before release.
- `feature/<short-name>`: feature work branched from `develop`.
- `release/<version>`: release stabilization branched from `develop`, merged into `main` and back to `develop`.
- `hotfix/<short-name>`: urgent fixes branched from `main`, merged into `main` and back to `develop`.

## Rules

- Open pull requests into `develop` for normal feature work.
- Open pull requests into `main` only for release branches and hotfix branches.
- Use semantic version tags on `main`, for example `v0.1.0`.
- Keep CI green before merging.
- Do not commit generated dependency or build output.
- Do not push from a dirty worktree. Run `git status --short --branch` before
  every push and commit only the intended files.
- Push completed implementation to `develop` first, then wait for GitHub CI to
  pass before promoting to `main`.
- Promote `develop` to `main` only from a clean worktree. If the active worktree
  is dirty, use a temporary worktree rooted at `origin/main` for the promotion.
- Do not force-push `main`.

## Local Pre-Push Check

Use the local verification cadence B for normal feature work. The goal is to
avoid repeated workspace-wide Cargo builds while still proving each coherent
change before it is pushed.

During development:

- run only one Cargo command at a time, otherwise the shared `target` directory
  lock can make local feedback appear stuck;
- prefer `cargo check --locked -p <crate>` for the affected Rust crate;
- do not run workspace tests after every small edit.

Before an atomic commit, run the checks that match the feature group:

```bash
bash scripts/ci/check_repo.sh
cargo fmt --all -- --check
cargo check --locked -p <affected-crate>
cargo test --locked -p <affected-crate> <focused-test-filter>
```

For frontend changes, also run:

```bash
cd apps/web-console
cargo fmt -- --check
cargo check --locked --target wasm32-unknown-unknown
NO_COLOR=false trunk build --release --locked
node ../../scripts/demo/smoke_web_console.mjs
```

Run `cargo test --locked --workspace` locally only for release stabilization,
large cross-crate refactors, or when CI failures suggest a workspace-level
interaction. Normal pushes should rely on GitHub CI for the full Rust matrix
after focused local checks pass.

## Release Flow

```bash
git checkout develop
git pull origin develop
git checkout -b release/v0.1.0

# finalize release notes and fixes

git checkout main
git merge --no-ff release/v0.1.0
git tag v0.1.0
git push origin main v0.1.0

git checkout develop
git merge --no-ff release/v0.1.0
git push origin develop
```

## Hotfix Flow

```bash
git checkout main
git pull origin main
git checkout -b hotfix/<short-name>

# apply fix

git checkout main
git merge --no-ff hotfix/<short-name>
git tag v0.1.1
git push origin main v0.1.1

git checkout develop
git merge --no-ff hotfix/<short-name>
git push origin develop
```

## Branch Protection

Recommended GitHub branch protection for `main` and `develop`:

- require pull requests before merging
- require CI to pass before merging
- require branches to be up to date before merging
- block force pushes

`main` must be protected. Required status checks should include:

- `repository-health`
- `rust`
- `migrations`
- `demo-smoke`
- `python`
- `frontend`

`develop` should block force pushes and require CI when multiple contributors or
agents are pushing concurrently. If direct pushes to `develop` are allowed for
speed, only push after the local pre-push check above passes.

## GitHub Environments

Use the `staging` GitHub Environment for manual staging deployment packaging.
The workflow `.github/workflows/deploy-staging.yml` should be dispatched only
after CI is green for the selected commit. It builds the staging deployment
package and uploads it as an artifact; applying the package to a cluster remains
an environment-specific operator action with customer-approved secrets.
