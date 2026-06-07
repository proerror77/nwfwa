#!/usr/bin/env python3
"""Validate a generated local K3s staging simulation package."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path


REQUIRED_PACKAGE_PATHS = {
    "apply.sh",
    "smoke.sh",
    "k8s/simulation/kustomization.yaml",
    "k8s/simulation/namespace.yaml",
    "k8s/simulation/secrets.k3s-simulation.yaml",
    "k8s/simulation/api-server.yaml",
    "k8s/simulation/web-console.yaml",
    "k8s/simulation/ml-service.yaml",
    "k8s/simulation/worker-cronjobs.yaml",
    "k8s/simulation/worker-serviceaccount.yaml",
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
        manifest.get("artifact_kind") == "k3s_staging_simulation_package",
        "wrong simulation package artifact kind",
    )
    namespace = manifest.get("namespace")
    require(namespace and namespace != "nwfwa-staging", "simulation namespace must be isolated")
    require(
        "not a customer production deployment" in manifest.get("simulation_boundary", ""),
        "simulation boundary must be explicit",
    )
    images = manifest.get("images", {})
    require(images, "simulation manifest missing images")
    for source, target in images.items():
        require("ghcr.io/replace-me" in source, "image source must be the staging placeholder")
        require("ghcr.io/replace-me" not in target, f"image target not replaced: {source}")
    package_files = manifest.get("package_files")
    require(isinstance(package_files, list) and package_files, "package_files must be non-empty")
    package_by_path = {item.get("path"): item for item in package_files if isinstance(item, dict)}
    require(REQUIRED_PACKAGE_PATHS.issubset(set(package_by_path)), "package missing required files")
    for relative_path, item in package_by_path.items():
        path = package_dir / relative_path
        require(path.is_file(), f"package file missing: {relative_path}")
        require(item.get("sha256") == sha256_file(path), f"checksum mismatch for {relative_path}")


def validate_apply_script(package_dir: Path) -> None:
    path = package_dir / "apply.sh"
    require(path.is_file(), "missing apply.sh")
    require(os.access(path, os.X_OK), "apply.sh must be executable")
    text = path.read_text(encoding="utf-8")
    for snippet in [
        "kubectl config current-context",
        "Refusing to apply to non-K3s context",
        "kubectl apply -k k8s/simulation --dry-run=server",
        "rollout status deployment/ml-service",
        "get cronjob governance-ops-plan",
    ]:
        require(snippet in text, f"apply.sh missing snippet: {snippet}")


def validate_smoke_script(package_dir: Path) -> None:
    path = package_dir / "smoke.sh"
    require(path.is_file(), "missing smoke.sh")
    require(os.access(path, os.X_OK), "smoke.sh must be executable")
    text = path.read_text(encoding="utf-8")
    for snippet in [
        "port-forward svc/ml-service 8001:8001",
        "/health",
        "/metrics",
        "fwa_ml_training_jobs",
        "/artifact-registries",
    ]:
        require(snippet in text, f"smoke.sh missing snippet: {snippet}")


def validate_manifests(package_dir: Path, namespace: str) -> None:
    simulation_dir = package_dir / "k8s" / "simulation"
    rendered_text = "\n".join(
        path.read_text(encoding="utf-8")
        for path in sorted(simulation_dir.glob("*.yaml"))
    )
    require(namespace in rendered_text, "simulation namespace missing from manifests")
    require("nwfwa-staging" not in rendered_text, "staging namespace leaked into simulation")
    require("ghcr.io/replace-me" not in rendered_text, "placeholder image leaked into simulation")
    require("kind: Secret" in rendered_text, "simulation Secret missing")
    require("kind: ServiceAccount" in rendered_text, "worker ServiceAccount missing")
    require("prometheus.io/path: /metrics" in rendered_text, "Prometheus scrape annotation missing")


def validate_index(package_dir: Path) -> None:
    index = load_json(package_dir / "index.json")
    require(index.get("artifact_kind") == "k3s_simulation_package_index", "wrong index artifact kind")
    require(index.get("customer_data_required") is False, "simulation package must not require customer data")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-dir", default="artifacts/k3s-simulation")
    args = parser.parse_args()

    package_dir = Path(args.package_dir)
    manifest = load_json(package_dir / "simulation_manifest.json")
    validate_manifest(package_dir, manifest)
    validate_apply_script(package_dir)
    validate_smoke_script(package_dir)
    validate_manifests(package_dir, manifest["namespace"])
    validate_index(package_dir)
    print("k3s simulation package validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"k3s simulation package validation failed: {exc}")
        raise SystemExit(1)
