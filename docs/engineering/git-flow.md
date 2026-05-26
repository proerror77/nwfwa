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

This is not enforced yet. Enabling branch protection changes repository write policy and should be done deliberately.
