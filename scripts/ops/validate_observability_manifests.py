#!/usr/bin/env python3
"""Validate the local Kubernetes observability manifests."""

from __future__ import annotations

from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[2]
OBSERVABILITY_DIR = ROOT / "infra" / "k8s" / "observability"

REQUIRED_FILES = [
    "kustomization.yaml",
    "namespace.yaml",
    "prometheus-rbac.yaml",
    "prometheus.yaml",
    "alertmanager.yaml",
]

REQUIRED_TEXT = {
    "kustomization.yaml": [
        "namespace: nwfwa-observability",
        "prometheus-rbac.yaml",
        "prometheus.yaml",
        "alertmanager.yaml",
    ],
    "namespace.yaml": [
        "pod-security.kubernetes.io/enforce: restricted",
    ],
    "prometheus-rbac.yaml": [
        "kind: ClusterRole",
        "resources: [\"pods\"]",
        "kind: ClusterRoleBinding",
    ],
    "prometheus.yaml": [
        "prom/prometheus:v3.7.3",
        "kubernetes_sd_configs:",
        "__meta_kubernetes_pod_annotation_prometheus_io_scrape",
        "fwa_ml_training_jobs_dead_letter",
        "fwa_ml_training_workers_registered",
        "fwa_ml_training_jobs_ready",
        "storage.tsdb.retention.time=30d",
        "allowPrivilegeEscalation: false",
        "fsGroup: 65534",
    ],
    "alertmanager.yaml": [
        "prom/alertmanager:v0.29.0",
        "nwfwa-governance-review",
        "mlops-alert-router.nwfwa-production",
        "/alertmanager/webhook",
        "send_resolved: true",
        "credentials_file: /etc/alertmanager-webhook-token/token",
        "secretName: mlops-alert-router-webhook-token",
        "allowPrivilegeEscalation: false",
        "fsGroup: 65534",
    ],
}


FORBIDDEN_TEXT = [
    "replace-with-",
    "dev-secret",
    "changeme",
    "mlops-alert-delivery",
]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError as exc:
        raise AssertionError(f"missing required file: {path}") from exc


def main() -> int:
    for file_name in REQUIRED_FILES:
        require((OBSERVABILITY_DIR / file_name).is_file(), f"missing {file_name}")

    joined = ""
    for file_name, snippets in REQUIRED_TEXT.items():
        text = read(OBSERVABILITY_DIR / file_name)
        joined += text
        for snippet in snippets:
            require(snippet in text, f"{file_name} missing snippet: {snippet}")

    for forbidden in FORBIDDEN_TEXT:
        require(forbidden not in joined, f"observability manifests contain forbidden text: {forbidden}")

    print("observability manifest validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"observability manifest validation failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
