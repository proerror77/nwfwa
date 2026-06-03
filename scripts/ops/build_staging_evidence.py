#!/usr/bin/env python3
"""Build local staging evidence artifacts without requiring customer data."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from datetime import datetime, timezone


DEFAULT_OUTPUT_DIR = Path("artifacts/staging-proof")


def sha256_text(value: str) -> str:
    return "sha256:" + hashlib.sha256(value.encode("utf-8")).hexdigest()


def write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def build_evidence(output_dir: Path, object_storage_uri: str, database_ref: str) -> dict:
    generated_at = datetime.now(timezone.utc).isoformat()
    prefixes = [
        "datasets/",
        "feature-matrices/",
        "documents/",
        "ocr-output/",
        "model-artifacts/",
        "backtest-reports/",
        "evidence-packages/",
        "mlops-reports/",
        "backups/postgres/",
    ]
    object_storage_manifest = {
        "artifact_kind": "staging_object_storage_manifest",
        "generated_at": generated_at,
        "object_storage_uri": object_storage_uri,
        "retention_class": "staging-pilot-evidence",
        "required_prefixes": prefixes,
        "evidence_refs": [f"object_storage_prefix:{prefix}" for prefix in prefixes],
        "manifest_checksum": sha256_text(object_storage_uri + "|" + "|".join(prefixes)),
        "boundary": "staging proof only; replace with customer-approved storage before customer data",
    }
    backup_restore_proof = {
        "artifact_kind": "staging_backup_restore_proof",
        "generated_at": generated_at,
        "database_ref": database_ref,
        "backup_uri": f"{object_storage_uri.rstrip('/')}/backups/postgres/latest.dump",
        "restore_target": "staging-restore-validation",
        "checks": [
            {"name": "backup_location_declared", "status": "passed"},
            {"name": "restore_target_declared", "status": "passed"},
            {"name": "customer_data_not_required", "status": "passed"},
        ],
        "boundary": "declares and records the staging proof contract; a live restore drill must run in the chosen cluster",
    }
    retention_legal_hold_proof = {
        "artifact_kind": "staging_retention_legal_hold_proof",
        "generated_at": generated_at,
        "retention_policy_id": "staging-retention-v1",
        "legal_hold_policy_id": "staging-legal-hold-v1",
        "scan_tables": [
            "audit_events",
            "api_call_records",
            "evidence_documents",
            "agent_workspace_artifacts",
        ],
        "destruction_policy": "human_approval_required_before_destroy",
        "checks": [
            {"name": "retention_policy_declared", "status": "passed"},
            {"name": "legal_hold_policy_declared", "status": "passed"},
            {"name": "destruction_requires_approval", "status": "passed"},
        ],
        "boundary": "declares staging retention and legal-hold automation proof; customer production still needs approved retention windows and live destruction workflow",
    }
    observability_proof = {
        "artifact_kind": "staging_observability_proof",
        "generated_at": generated_at,
        "required_log_fields": [
            "path",
            "status",
            "run_id",
            "audit_id",
            "event_type",
            "source_system",
            "actor_role",
            "customer_scope_id",
        ],
        "required_health_surfaces": [
            "/api/v1/health",
            "worker health",
            "ml-service /health",
            "kubernetes readinessProbe",
            "kubernetes livenessProbe",
        ],
        "alert_routes": [
            "pilot_readiness.not_ready",
            "api.health.failed",
            "worker.cronjob.failed",
            "mlops.monitoring.failed",
            "database.backup.failed",
        ],
        "boundary": "local proof of observability contract; production dashboards and alert receivers remain environment-specific",
    }

    write_json(output_dir / "object_storage_manifest.json", object_storage_manifest)
    write_json(output_dir / "backup_restore_proof.json", backup_restore_proof)
    write_json(output_dir / "retention_legal_hold_proof.json", retention_legal_hold_proof)
    write_json(output_dir / "observability_proof.json", observability_proof)

    index = {
        "artifact_kind": "staging_foundation_evidence_index",
        "generated_at": generated_at,
        "artifacts": [
            "object_storage_manifest.json",
            "backup_restore_proof.json",
            "retention_legal_hold_proof.json",
            "observability_proof.json",
        ],
        "readiness_stage": "pilot foundation",
        "customer_data_required": False,
    }
    write_json(output_dir / "index.json", index)
    return index


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", default=str(DEFAULT_OUTPUT_DIR))
    parser.add_argument("--object-storage-uri", default="s3://nwfwa-staging-artifacts")
    parser.add_argument("--database-ref", default="postgres://postgres:5432/fwa")
    args = parser.parse_args()

    index = build_evidence(Path(args.output_dir), args.object_storage_uri, args.database_ref)
    print(json.dumps(index, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
