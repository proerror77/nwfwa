#!/usr/bin/env python3
"""Build a GitHub Environment gated staging deployment package."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
STAGING_DIR = ROOT / "infra" / "k8s" / "staging"
DEFAULT_OUTPUT_DIR = Path("artifacts/staging-deployment")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return "sha256:" + digest.hexdigest()


def write_text(path: Path, value: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(value, encoding="utf-8")


def write_json(path: Path, payload: dict) -> None:
    write_text(path, json.dumps(payload, indent=2, sort_keys=True) + "\n")


def copy_staging_manifests(output_dir: Path) -> list[dict]:
    manifests_dir = output_dir / "k8s" / "staging"
    if manifests_dir.exists():
        shutil.rmtree(manifests_dir)
    shutil.copytree(STAGING_DIR, manifests_dir)

    manifests = []
    for path in sorted(manifests_dir.glob("*.yaml")) + sorted(manifests_dir.glob("*.md")):
        manifests.append(
            {
                "path": str(path.relative_to(output_dir)),
                "sha256": sha256_file(path),
            }
        )
    return manifests


def build_package(output_dir: Path, image_tag: str, commit_sha: str, environment: str) -> dict:
    generated_at = datetime.now(timezone.utc).isoformat()
    output_dir.mkdir(parents=True, exist_ok=True)
    manifests = copy_staging_manifests(output_dir)

    apply_script = """#!/usr/bin/env bash
set -euo pipefail

namespace="${NWFWA_STAGING_NAMESPACE:-nwfwa-staging}"
secret_file="${NWFWA_STAGING_SECRET_FILE:?set NWFWA_STAGING_SECRET_FILE to the customer-approved Secret YAML}"

kubectl apply -f k8s/staging/namespace.yaml
kubectl apply -n "$namespace" -f "$secret_file"
kubectl apply -k k8s/staging
kubectl -n "$namespace" rollout status deployment/api-server
kubectl -n "$namespace" rollout status deployment/web-console
kubectl -n "$namespace" rollout status deployment/ml-service
"""
    write_text(output_dir / "apply.sh", apply_script)
    (output_dir / "apply.sh").chmod(0o755)

    rollback = """# Staging Rollback

This package does not perform rollback automatically.

Rollback options:

1. Re-run the previous approved GitHub Environment deployment package.
2. Revert the Git commit and regenerate a new staging deployment package.
3. Restore PostgreSQL from the customer-approved backup manifest only after a
   restore-drill report confirms the target and scope.

Never delete retained evidence or object-storage artifacts without the
`human_approval_required_before_destroy` gate from the governance ops plan.
"""
    write_text(output_dir / "rollback.md", rollback)

    manifest = {
        "artifact_kind": "github_environment_staging_deployment_package",
        "generated_at": generated_at,
        "environment": environment,
        "commit_sha": commit_sha,
        "image_tag": image_tag,
        "deployment_boundary": "GitHub Environment gated package only; does not apply to a cluster automatically",
        "required_environment": {
            "github_environment": environment,
            "required_secret_file": "NWFWA_STAGING_SECRET_FILE",
            "required_kube_context": "customer-approved staging cluster context",
        },
        "validation_commands": [
            "python3 scripts/ops/validate_k8s_staging.py",
            "python3 scripts/ops/validate_container_packaging.py",
            "python3 scripts/ops/validate_staging_deployment_package.py --package-dir artifacts/staging-deployment",
            "bash scripts/ci/check_repo.sh",
        ],
        "package_files": [
            {"path": "apply.sh", "sha256": sha256_file(output_dir / "apply.sh")},
            {"path": "rollback.md", "sha256": sha256_file(output_dir / "rollback.md")},
            *manifests,
        ],
        "apply_command": "NWFWA_STAGING_SECRET_FILE=/path/to/secret.yaml ./apply.sh",
        "rollback_ref": "rollback.md",
    }
    write_json(output_dir / "deployment_manifest.json", manifest)
    write_json(
        output_dir / "index.json",
        {
            "artifact_kind": "staging_deployment_package_index",
            "generated_at": generated_at,
            "artifacts": [
                "deployment_manifest.json",
                "apply.sh",
                "rollback.md",
                "k8s/staging",
            ],
            "customer_data_required": False,
        },
    )
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", default=str(DEFAULT_OUTPUT_DIR))
    parser.add_argument("--image-tag", default="staging")
    parser.add_argument("--commit-sha", default="local")
    parser.add_argument("--environment", default="staging")
    args = parser.parse_args()

    manifest = build_package(
        Path(args.output_dir),
        image_tag=args.image_tag,
        commit_sha=args.commit_sha,
        environment=args.environment,
    )
    print(json.dumps(manifest, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
