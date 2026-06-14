#!/usr/bin/env python3
"""Validate a generated GitHub Environment staging deployment package."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path


REQUIRED_PACKAGE_PATHS = {
    "apply.sh",
    "rollback.md",
    "k8s/staging/kustomization.yaml",
    "k8s/staging/api-server.yaml",
    "k8s/staging/web-console.yaml",
    "k8s/staging/ml-service.yaml",
    "k8s/staging/worker-cronjobs.yaml",
    "k8s/staging/worker-serviceaccount.yaml",
    "k8s/staging/secrets.example.yaml",
}

REQUIRED_VALIDATION_COMMANDS = {
    "python3 scripts/ops/validate_k8s_staging.py",
    "python3 scripts/ops/validate_container_packaging.py",
    "python3 scripts/ops/validate_staging_secret_file.py --secret-file infra/k8s/staging/secrets.example.yaml --allow-placeholders",
    "python3 scripts/ops/validate_staging_deployment_package.py --package-dir artifacts/staging-deployment",
    "bash scripts/ci/check_repo.sh",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return "sha256:" + digest.hexdigest()


def load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise AssertionError(f"missing JSON artifact: {path}") from exc


def validate_manifest(package_dir: Path, manifest: dict) -> None:
    require(
        manifest.get("artifact_kind") == "github_environment_staging_deployment_package",
        "wrong deployment package artifact kind",
    )
    require(manifest.get("environment") == "staging", "deployment package must target staging")
    require(manifest.get("commit_sha"), "deployment manifest missing commit_sha")
    require(manifest.get("image_tag"), "deployment manifest missing image_tag")
    require("does not apply to a cluster automatically" in manifest.get("deployment_boundary", ""), "deployment boundary must remain package-only")

    required_environment = manifest.get("required_environment", {})
    require(required_environment.get("github_environment") == "staging", "manifest must require staging GitHub Environment")
    require(required_environment.get("required_secret_file") == "NWFWA_STAGING_SECRET_FILE", "manifest must require secret file")
    require("customer-approved staging cluster" in required_environment.get("required_kube_context", ""), "manifest must require customer-approved kube context")

    commands = set(manifest.get("validation_commands", []))
    require(REQUIRED_VALIDATION_COMMANDS.issubset(commands), "manifest missing required validation commands")

    package_files = manifest.get("package_files")
    require(isinstance(package_files, list) and package_files, "manifest package_files must be non-empty")
    package_by_path = {item.get("path"): item for item in package_files if isinstance(item, dict)}
    require(REQUIRED_PACKAGE_PATHS.issubset(set(package_by_path)), "manifest missing required package files")

    for relative_path, item in package_by_path.items():
        path = package_dir / relative_path
        require(path.is_file(), f"package file missing: {relative_path}")
        require(item.get("sha256") == sha256_file(path), f"checksum mismatch for {relative_path}")

    require(manifest.get("apply_command") == "NWFWA_STAGING_SECRET_FILE=/path/to/secret.yaml ./apply.sh", "unexpected apply command")
    require(manifest.get("rollback_ref") == "rollback.md", "rollback_ref must point to rollback.md")


def validate_apply_script(package_dir: Path) -> None:
    path = package_dir / "apply.sh"
    require(path.is_file(), "missing apply.sh")
    require(os.access(path, os.X_OK), "apply.sh must be executable")
    text = path.read_text(encoding="utf-8")
    for snippet in [
        "NWFWA_STAGING_SECRET_FILE:?",
        "secret file must define Secret nwfwa-staging-secrets",
        "secret file still contains placeholder marker",
        "kubectl apply -f k8s/staging/namespace.yaml",
        "kubectl apply -n \"$namespace\" -f \"$secret_file\" --dry-run=server",
        "kubectl apply -k k8s/staging --dry-run=server",
        "kubectl apply -n \"$namespace\" -f \"$secret_file\"",
        "kubectl apply -k k8s/staging",
        "rollout status deployment/api-server",
        "rollout status deployment/web-console",
        "rollout status deployment/ml-service",
        "get cronjob governance-ops-plan",
    ]:
        require(snippet in text, f"apply.sh missing snippet: {snippet}")


def validate_rollback(package_dir: Path) -> None:
    path = package_dir / "rollback.md"
    require(path.is_file(), "missing rollback.md")
    text = path.read_text(encoding="utf-8")
    for snippet in [
        "does not perform rollback automatically",
        "previous approved GitHub Environment deployment package",
        "Revert the Git commit",
        "human_approval_required_before_destroy",
    ]:
        require(snippet in text, f"rollback.md missing snippet: {snippet}")


def validate_index(package_dir: Path) -> None:
    index = load_json(package_dir / "index.json")
    require(index.get("artifact_kind") == "staging_deployment_package_index", "wrong index artifact kind")
    require(index.get("customer_data_required") is False, "deployment package must not require customer data")
    artifacts = set(index.get("artifacts", []))
    for artifact in ["deployment_manifest.json", "apply.sh", "rollback.md", "k8s/staging"]:
        require(artifact in artifacts, f"index missing artifact {artifact}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-dir", default="artifacts/staging-deployment")
    args = parser.parse_args()

    package_dir = Path(args.package_dir)
    validate_manifest(package_dir, load_json(package_dir / "deployment_manifest.json"))
    validate_apply_script(package_dir)
    validate_rollback(package_dir)
    validate_index(package_dir)
    print("staging deployment package validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"staging deployment package validation failed: {exc}")
        raise SystemExit(1)
