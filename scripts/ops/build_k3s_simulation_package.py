#!/usr/bin/env python3
"""Build a local K3s simulation package from the staging manifests."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
STAGING_DIR = ROOT / "infra" / "k8s" / "staging"
DEFAULT_OUTPUT_DIR = Path("artifacts/k3s-simulation")


IMAGE_REPLACEMENTS = {
    "ghcr.io/replace-me/nwfwa-api-server:staging": "nwfwa-api-server:local",
    "ghcr.io/replace-me/nwfwa-web-console:staging": "nwfwa-web-console:local",
    "ghcr.io/replace-me/nwfwa-ml-service:staging": "nwfwa-ml-service:local",
    "ghcr.io/replace-me/nwfwa-worker:staging": "nwfwa-worker:local",
    "ghcr.io/replace-me/nwfwa-ops:staging": "nwfwa-ops:local",
}


SIMULATION_SECRET = """apiVersion: v1
kind: Secret
metadata:
  name: {secret_name}
  labels:
    app.kubernetes.io/name: nwfwa
    environment: k3s-simulation
type: Opaque
stringData:
  FWA_API_KEY: k3s-simulation-api-key
  FWA_API_KEY_PRINCIPALS: k3s-simulation-api-key|k3s-tpa-system|tpa_system|k3s-tpa|k3s-simulation-customer|tpa:*
  FWA_MODEL_SIGNATURE_KEY: k3s-simulation-model-signing-key
  FWA_ALERT_RECEIVER_TOKEN: k3s-simulation-alert-token
  FWA_ALERT_RECEIVER_SIGNING_SECRET: k3s-simulation-alert-signing-secret
  DATABASE_URL: postgres://fwa:k3s-simulation-postgres-password@postgres:5432/fwa
  POSTGRES_USER: fwa
  POSTGRES_PASSWORD: k3s-simulation-postgres-password
  POSTGRES_DB: fwa
  MINIO_ROOT_USER: k3sminio
  MINIO_ROOT_PASSWORD: k3s-simulation-minio-password
"""


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


def patch_text(text: str, namespace: str, image_replacements: dict[str, str]) -> str:
    patched = text.replace("nwfwa-staging", namespace)
    patched = patched.replace("staging-customer", "k3s-simulation-customer")
    patched = patched.replace("staging-tpa", "k3s-tpa")
    patched = patched.replace("http://otel-collector:4318", f"http://otel-collector.{namespace}:4318")
    for source, target in image_replacements.items():
        patched = patched.replace(source, target)
    return patched


def copy_and_patch_manifests(
    output_dir: Path,
    namespace: str,
    image_replacements: dict[str, str],
) -> list[dict]:
    target_dir = output_dir / "k8s" / "simulation"
    if target_dir.exists():
        shutil.rmtree(target_dir)
    shutil.copytree(STAGING_DIR, target_dir)

    secret_name = f"{namespace}-secrets"
    for path in sorted(target_dir.glob("*.yaml")) + sorted(target_dir.glob("*.md")):
        if path.name == "secrets.example.yaml":
            continue
        text = path.read_text(encoding="utf-8")
        path.write_text(patch_text(text, namespace, image_replacements), encoding="utf-8")
    (target_dir / "secrets.example.yaml").unlink(missing_ok=True)

    kustomization = target_dir / "kustomization.yaml"
    text = kustomization.read_text(encoding="utf-8")
    if "secrets.k3s-simulation.yaml" not in text:
        text = text.replace(
            "  - configmap.yaml\n",
            "  - configmap.yaml\n  - secrets.k3s-simulation.yaml\n",
        )
    kustomization.write_text(text, encoding="utf-8")

    write_text(
        target_dir / "secrets.k3s-simulation.yaml",
        SIMULATION_SECRET.format(secret_name=secret_name),
    )

    manifests = []
    for path in sorted(target_dir.glob("*.yaml")) + sorted(target_dir.glob("*.md")):
        manifests.append(
            {
                "path": str(path.relative_to(output_dir)),
                "sha256": sha256_file(path),
            }
        )
    return manifests


def build_package(
    output_dir: Path,
    namespace: str,
    image_replacements: dict[str, str],
) -> dict:
    generated_at = datetime.now(timezone.utc).isoformat()
    output_dir.mkdir(parents=True, exist_ok=True)
    manifests = copy_and_patch_manifests(output_dir, namespace, image_replacements)

    apply_script = f"""#!/usr/bin/env bash
set -euo pipefail

namespace="${{NWFWA_K3S_NAMESPACE:-{namespace}}}"
context="$(kubectl config current-context)"
if [[ "${{NWFWA_K3S_ALLOW_NON_K3S:-}}" != "1" && "$context" != *k3s* && "$context" != *k3d* ]]; then
  echo "Refusing to apply to non-K3s context: $context" >&2
  echo "Set NWFWA_K3S_ALLOW_NON_K3S=1 only for an intentional local simulator context." >&2
  exit 1
fi

kubectl apply -f k8s/simulation/namespace.yaml
kubectl apply -k k8s/simulation --dry-run=server
kubectl apply -k k8s/simulation
kubectl -n "$namespace" rollout status statefulset/postgres --timeout=180s
kubectl -n "$namespace" rollout status deployment/object-storage --timeout=180s
kubectl -n "$namespace" rollout status deployment/ml-service --timeout=180s
kubectl -n "$namespace" rollout status deployment/api-server --timeout=180s
kubectl -n "$namespace" rollout status deployment/web-console --timeout=180s
kubectl -n "$namespace" get cronjob governance-ops-plan ai-evidence-execution-plan analytics-export-plan pilot-readiness-proof mlops-monitoring-runtime
"""
    write_text(output_dir / "apply.sh", apply_script)
    (output_dir / "apply.sh").chmod(0o755)

    smoke_script = f"""#!/usr/bin/env bash
set -euo pipefail

namespace="${{NWFWA_K3S_NAMESPACE:-{namespace}}}"
kubectl -n "$namespace" get pods
kubectl -n "$namespace" get cronjob
kubectl -n "$namespace" port-forward svc/ml-service 8001:8001 >/tmp/nwfwa-k3s-ml-service-port-forward.log 2>&1 &
pf_pid="$!"
trap 'kill "$pf_pid" >/dev/null 2>&1 || true' EXIT
sleep 3
curl -fsS http://127.0.0.1:8001/health
curl -fsS http://127.0.0.1:8001/metrics | grep 'fwa_ml_training_jobs'
curl -fsS http://127.0.0.1:8001/artifact-registries
"""
    write_text(output_dir / "smoke.sh", smoke_script)
    (output_dir / "smoke.sh").chmod(0o755)

    manifest = {
        "artifact_kind": "k3s_staging_simulation_package",
        "generated_at": generated_at,
        "namespace": namespace,
        "simulation_boundary": "local K3s simulator only; not a customer production deployment",
        "images": image_replacements,
        "apply_command": "./apply.sh",
        "smoke_command": "./smoke.sh",
        "package_files": [
            {"path": "apply.sh", "sha256": sha256_file(output_dir / "apply.sh")},
            {"path": "smoke.sh", "sha256": sha256_file(output_dir / "smoke.sh")},
            *manifests,
        ],
    }
    write_json(output_dir / "simulation_manifest.json", manifest)
    write_json(
        output_dir / "index.json",
        {
            "artifact_kind": "k3s_simulation_package_index",
            "generated_at": generated_at,
            "artifacts": [
                "simulation_manifest.json",
                "apply.sh",
                "smoke.sh",
                "k8s/simulation",
            ],
            "customer_data_required": False,
        },
    )
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", default=str(DEFAULT_OUTPUT_DIR))
    parser.add_argument("--namespace", default="nwfwa-k3s-sim")
    parser.add_argument("--api-image", default=IMAGE_REPLACEMENTS["ghcr.io/replace-me/nwfwa-api-server:staging"])
    parser.add_argument("--web-console-image", default=IMAGE_REPLACEMENTS["ghcr.io/replace-me/nwfwa-web-console:staging"])
    parser.add_argument("--ml-service-image", default=IMAGE_REPLACEMENTS["ghcr.io/replace-me/nwfwa-ml-service:staging"])
    parser.add_argument("--worker-image", default=IMAGE_REPLACEMENTS["ghcr.io/replace-me/nwfwa-worker:staging"])
    parser.add_argument("--ops-image", default=IMAGE_REPLACEMENTS["ghcr.io/replace-me/nwfwa-ops:staging"])
    args = parser.parse_args()

    replacements = {
        "ghcr.io/replace-me/nwfwa-api-server:staging": args.api_image,
        "ghcr.io/replace-me/nwfwa-web-console:staging": args.web_console_image,
        "ghcr.io/replace-me/nwfwa-ml-service:staging": args.ml_service_image,
        "ghcr.io/replace-me/nwfwa-worker:staging": args.worker_image,
        "ghcr.io/replace-me/nwfwa-ops:staging": args.ops_image,
    }
    manifest = build_package(Path(args.output_dir), args.namespace, replacements)
    print(json.dumps(manifest, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
