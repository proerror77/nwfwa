#!/usr/bin/env python3
"""Validate a generated customer-gated production deployment package."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path


REQUIRED_PACKAGE_PATHS = {
    "apply.sh",
    "rollback.md",
    "tools/validate_production_secret_file.py",
    "k8s/production/kustomization.yaml",
    "k8s/production/namespace.yaml",
    "k8s/production/api-server.yaml",
    "k8s/production/web-console.yaml",
    "k8s/production/ml-service.yaml",
    "k8s/production/worker-cronjobs.yaml",
    "k8s/production/worker-serviceaccount.yaml",
    "k8s/production/ingress.yaml",
    "k8s/production/hpa.yaml",
    "k8s/production/pdb.yaml",
    "k8s/production/networkpolicy.yaml",
}

REQUIRED_RENDERED_SNIPPETS = [
    "kind: Ingress",
    "kind: HorizontalPodAutoscaler",
    "kind: PodDisruptionBudget",
    "kind: NetworkPolicy",
    "name: default-deny-ingress",
    "name: allow-same-namespace",
    "name: allow-ingress-controller-to-web-and-api",
    "prometheus.io/path: /metrics",
]

FORBIDDEN_RENDERED_SNIPPETS = [
    "nwfwa-staging",
    "staging-",
    "environment: staging",
    "replace-with-staging",
    "dev-secret",
    "demo-customer",
    "demo-seed",
    "seed_demo.sql",
    "public-mvp",
    "local://demo-artifacts",
    "replace-me",
    "example.invalid",
    ":latest",
]


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
        manifest.get("artifact_kind") == "customer_gated_production_deployment_package",
        "wrong production deployment package artifact kind",
    )
    require(manifest.get("environment") == "production", "production package must target production")
    require(manifest.get("namespace") == "nwfwa-production", "default production namespace must be nwfwa-production")
    require(manifest.get("commit_sha"), "deployment manifest missing commit_sha")
    require(manifest.get("host") and "example.invalid" not in manifest["host"], "production host must be explicit")
    images = manifest.get("images")
    require(isinstance(images, dict) and images, "deployment manifest missing images")
    for image in images.values():
        require("replace-me" not in image, f"production image is still a placeholder: {image}")
        require(not image.endswith(":latest"), f"production image must not use latest tag: {image}")
    require(
        "never applies automatically" in manifest.get("deployment_boundary", ""),
        "deployment boundary must remain customer-gated",
    )
    required_environment = manifest.get("required_environment", {})
    require(
        required_environment.get("required_secret_file") == "NWFWA_PRODUCTION_SECRET_FILE",
        "manifest must require production secret file",
    )
    require(
        "customer-approved production cluster" in required_environment.get("required_kube_context", ""),
        "manifest must require customer-approved production kube context",
    )
    require(manifest.get("tls_secret"), "manifest missing tls_secret")
    package_files = manifest.get("package_files")
    require(isinstance(package_files, list) and package_files, "package_files must be non-empty")
    package_by_path = {item.get("path"): item for item in package_files if isinstance(item, dict)}
    require(REQUIRED_PACKAGE_PATHS.issubset(set(package_by_path)), "manifest missing required package files")
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
        "NWFWA_PRODUCTION_SECRET_FILE:?",
        "NWFWA_PRODUCTION_KUBE_CONTEXT:?",
        "kubectl config current-context",
        "Refusing to apply production package on Kubernetes context",
        "tools/validate_production_secret_file.py",
        "kubectl apply -f k8s/production/namespace.yaml --dry-run=server",
        "kubectl apply -k k8s/production --dry-run=server",
        "rollout status statefulset/postgres",
        "rollout status deployment/api-server",
        "get ingress nwfwa",
        "get hpa api-server web-console",
        "get networkpolicy default-deny-ingress",
    ]:
        require(snippet in text, f"apply.sh missing snippet: {snippet}")


def validate_rendered_manifests(package_dir: Path, namespace: str) -> None:
    production_dir = package_dir / "k8s" / "production"
    if shutil.which("kubectl"):
        rendered_text = subprocess.check_output(
            ["kubectl", "kustomize", str(production_dir)],
            cwd=package_dir,
            text=True,
        )
    else:
        rendered_text = "\n".join(path.read_text(encoding="utf-8") for path in sorted(production_dir.glob("*.yaml")))
    require(namespace in rendered_text, "production namespace missing from manifests")
    for snippet in REQUIRED_RENDERED_SNIPPETS:
        require(snippet in rendered_text, f"production manifests missing snippet: {snippet}")
    for snippet in FORBIDDEN_RENDERED_SNIPPETS:
        require(snippet not in rendered_text, f"production manifests contain forbidden snippet: {snippet}")


def validate_index(package_dir: Path) -> None:
    index = load_json(package_dir / "index.json")
    require(index.get("artifact_kind") == "production_deployment_package_index", "wrong index artifact kind")
    require(index.get("customer_data_required") is False, "deployment package must not contain customer data")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-dir", default="artifacts/production-deployment")
    args = parser.parse_args()

    package_dir = Path(args.package_dir)
    manifest = load_json(package_dir / "deployment_manifest.json")
    validate_manifest(package_dir, manifest)
    validate_apply_script(package_dir)
    validate_rendered_manifests(package_dir, manifest["namespace"])
    validate_index(package_dir)
    print("production deployment package validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"production deployment package validation failed: {exc}")
        raise SystemExit(1)
