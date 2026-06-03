#!/usr/bin/env python3
"""Static checks for the nwfwa Kubernetes staging manifests."""

from __future__ import annotations

from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[2]
STAGING_DIR = ROOT / "infra" / "k8s" / "staging"

REQUIRED_FILES = [
    "kustomization.yaml",
    "namespace.yaml",
    "configmap.yaml",
    "secrets.example.yaml",
    "postgres.yaml",
    "object-storage.yaml",
    "clickhouse.yaml",
    "database-jobs.yaml",
    "ml-service.yaml",
    "api-server.yaml",
    "worker-cronjobs.yaml",
    "web-console.yaml",
    "README.md",
]

REQUIRED_TEXT = {
    "kustomization.yaml": [
        "namespace: nwfwa-staging",
        "api-server.yaml",
        "object-storage.yaml",
        "clickhouse.yaml",
        "worker-cronjobs.yaml",
    ],
    "configmap.yaml": [
        "FWA_OBJECT_STORAGE_URI: s3://nwfwa-staging-artifacts",
        "FWA_ANALYTICS_CLICKHOUSE_URL: http://clickhouse:8123",
        "FWA_RETENTION_POLICY_ID: staging-retention-v1",
        "FWA_BACKUP_RESTORE_PLAN_ID: staging-backup-restore-v1",
        "FWA_OBSERVABILITY_EXPORTER_ENDPOINT: http://otel-collector:4318",
    ],
    "secrets.example.yaml": [
        "replace-with-staging-api-key",
        "FWA_API_KEY_PRINCIPALS:",
        "DATABASE_URL:",
    ],
    "api-server.yaml": [
        "kind: Deployment",
        "name: api-server",
        "path: /api/v1/health",
        "configMapRef:",
        "secretRef:",
    ],
    "ml-service.yaml": [
        "kind: Deployment",
        "name: ml-service",
        "path: /health",
    ],
    "object-storage.yaml": [
        "quay.io/minio/minio",
        "/minio/health/ready",
        "PersistentVolumeClaim",
    ],
    "clickhouse.yaml": [
        "kind: StatefulSet",
        "name: clickhouse",
        "clickhouse/clickhouse-server:24.8",
        "path: /ping",
        "CLICKHOUSE_DB",
        "volumeClaimTemplates",
    ],
    "database-jobs.yaml": [
        "kind: Job",
        "name: database-migrate",
        "name: demo-seed",
        "nwfwa-ops:staging",
        "/app/migrations/0001_initial.sql",
        "/app/scripts/demo/seed_demo.sql",
    ],
    "worker-cronjobs.yaml": [
        "kind: CronJob",
        "check-pilot-readiness",
        "--require-ready",
        "build-mlops-monitoring-plan",
        "build-analytics-export-plan",
        "http://clickhouse:8123",
        "s3://nwfwa-staging-artifacts",
    ],
    "web-console.yaml": [
        "kind: Deployment",
        "name: web-console",
        "path: /",
    ],
}

FORBIDDEN_TEXT = [
    "dev-secret",
    "demo-customer",
    "local://demo-artifacts",
    "postgres://postgres:postgres@localhost",
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
        require((STAGING_DIR / file_name).is_file(), f"missing {file_name}")

    joined = ""
    for file_name, snippets in REQUIRED_TEXT.items():
        text = read(STAGING_DIR / file_name)
        joined += text
        for snippet in snippets:
            require(snippet in text, f"{file_name} missing snippet: {snippet}")

    for forbidden in FORBIDDEN_TEXT:
        require(forbidden not in joined, f"staging manifests contain forbidden demo value: {forbidden}")

    print("k8s staging manifest validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"k8s staging validation failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
