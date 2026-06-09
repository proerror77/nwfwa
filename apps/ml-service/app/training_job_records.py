from __future__ import annotations

import json
import sqlite3
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any
from urllib.parse import urlparse


def build_artifact_registry(
    provider_output: dict[str, Any],
    job_id: str,
    actor: str,
    model_key: str,
    base_model_version: str,
) -> dict[str, Any]:
    candidate_model_version = str(provider_output["candidate_model_version"])
    artifact_entries = []
    for field, artifact_kind, digest_field in [
        ("artifact_uri", "serving_model", "artifact_sha256"),
        ("training_artifact_uri", "training_model", "training_artifact_sha256"),
        ("validation_report_uri", "validation_report", None),
        ("feature_importance_uri", "feature_importance", None),
        ("permutation_importance_uri", "permutation_importance", None),
        ("serving_manifest_uri", "serving_manifest", None),
        ("model_artifact_evaluation_report_uri", "model_artifact_evaluation", None),
        ("onnx_parity_report_uri", "onnx_parity_report", None),
        ("feature_store_manifest_uri", "feature_store_manifest", None),
        ("shadow_report_uri", "shadow_report", None),
        ("drift_report_uri", "drift_report", None),
        ("fairness_report_uri", "fairness_report", None),
    ]:
        uri = provider_output.get(field)
        if uri:
            entry = {
                "artifact_kind": artifact_kind,
                "field": field,
                "uri": uri,
                "storage_uri": uri,
                "publish_status": artifact_publish_status(uri),
                "immutable": True,
            }
            if is_local_artifact_uri(uri):
                entry["local_staging_uri"] = uri
            if digest_field and provider_output.get(digest_field):
                entry["sha256"] = provider_output[digest_field]
            artifact_entries.append(entry)

    mined_rules_uri = provider_output.get("metrics_json", {}).get("mined_rule_candidates_uri")
    if mined_rules_uri:
        artifact_entries.append(
            {
                "artifact_kind": "mined_rule_candidates",
                "field": "metrics_json.mined_rule_candidates_uri",
                "uri": mined_rules_uri,
                "storage_uri": mined_rules_uri,
                "publish_status": artifact_publish_status(mined_rules_uri),
                "immutable": True,
            }
        )

    return {
        "registry_kind": "training_artifact_registry",
        "schema_version": "1.0",
        "job_id": job_id,
        "actor": actor,
        "model_key": model_key,
        "base_model_version": base_model_version,
        "candidate_model_version": candidate_model_version,
        "algorithm": provider_output.get("metrics_json", {}).get("algorithm"),
        "created_at": utc_now(),
        "artifacts": artifact_entries,
    }


def attach_artifact_registry(
    provider_output: dict[str, Any],
    job_id: str,
    actor: str,
    model_key: str,
    base_model_version: str,
) -> dict[str, Any]:
    registry = build_artifact_registry(
        provider_output,
        job_id,
        actor,
        model_key,
        base_model_version,
    )
    registry_uri = artifact_registry_uri(provider_output["artifact_uri"])
    Path(registry_uri).write_text(
        json.dumps(registry, indent=2, sort_keys=True),
        encoding="utf-8",
    )
    provider_output["artifact_registry_uri"] = registry_uri
    provider_output.setdefault("metrics_json", {})["artifact_registry_uri"] = registry_uri
    provider_output.setdefault("evidence_refs", []).append(
        f"model_artifact_registries:{registry_uri}"
    )
    registry["artifact_registry_uri"] = registry_uri
    return registry


def artifact_registry_uri(artifact_uri: str) -> str:
    return str(Path(artifact_uri).parent / "artifact_registry.json")


def artifact_registry_summary(record: dict[str, Any]) -> dict[str, Any]:
    registry = record["artifact_registry"]
    return {
        "job_id": record["job_id"],
        "status": record["status"],
        "model_key": record["model_key"],
        "base_model_version": record["base_model_version"],
        "candidate_model_version": record["candidate_model_version"],
        "algorithm": record["algorithm"],
        "actor": record["actor"],
        "completed_at": record["completed_at"],
        "artifact_registry_uri": registry.get("artifact_registry_uri"),
        "artifact_count": len(registry.get("artifacts", [])),
        "artifact_registry": registry,
    }


def artifact_publish_status(uri: str) -> str:
    if is_local_artifact_uri(uri):
        return "local_staging_available" if Path(uri).exists() else "local_staging_missing"
    scheme = urlparse(uri).scheme
    if scheme == "s3":
        return "external_object_storage_registered"
    return "external_uri_registered"


def is_local_artifact_uri(uri: str) -> bool:
    return urlparse(uri).scheme in {"", "file"}


def render_prometheus_metrics(
    queue_metrics: dict[str, Any],
    workers: list[dict[str, Any]],
) -> str:
    lines = [
        "# HELP fwa_ml_training_jobs Number of ML training jobs by status.",
        "# TYPE fwa_ml_training_jobs gauge",
    ]
    for status, count in sorted(queue_metrics["jobs_by_status"].items()):
        lines.append(f'fwa_ml_training_jobs{{status="{prom_label(status)}"}} {int(count)}')
    gauges = {
        "fwa_ml_training_jobs_ready": queue_metrics["ready_jobs"],
        "fwa_ml_training_jobs_delayed": queue_metrics["delayed_jobs"],
        "fwa_ml_training_jobs_expired_leases": queue_metrics["expired_leases"],
        "fwa_ml_training_jobs_dead_letter": queue_metrics["dead_letter_jobs"],
        "fwa_ml_training_workers_registered": queue_metrics["registered_workers"],
    }
    for metric_name, value in gauges.items():
        lines.extend(
            [
                f"# HELP {metric_name} ML training queue metric.",
                f"# TYPE {metric_name} gauge",
                f"{metric_name} {int(value)}",
            ]
        )
    worker_status_counts: dict[str, int] = {}
    for worker in workers:
        status = str(worker.get("status") or "unknown")
        worker_status_counts[status] = worker_status_counts.get(status, 0) + 1
    lines.extend(
        [
            "# HELP fwa_ml_training_workers Number of ML training workers by heartbeat status.",
            "# TYPE fwa_ml_training_workers gauge",
        ]
    )
    for status, count in sorted(worker_status_counts.items()):
        lines.append(f'fwa_ml_training_workers{{status="{prom_label(status)}"}} {count}')
    return "\n".join(lines) + "\n"


def prom_label(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")


def row_to_record(row: sqlite3.Row) -> dict[str, Any]:
    return {
        "job_id": row["job_id"],
        "handoff_kind": row["handoff_kind"],
        "status": row["status"],
        "model_key": row["model_key"],
        "base_model_version": row["base_model_version"],
        "candidate_model_version": row["candidate_model_version"],
        "algorithm": row["algorithm"],
        "actor": row["actor"],
        "request": parse_json(row["request_json"]),
        "provider_output": parse_json(row["provider_output_json"]),
        "artifact_registry": parse_json(row["artifact_registry_json"]),
        "error": parse_json(row["error_json"]),
        "attempt_count": row["attempt_count"],
        "max_attempts": row["max_attempts"],
        "retry_delay_seconds": row["retry_delay_seconds"],
        "worker_id": row["worker_id"],
        "lease_expires_at": row["lease_expires_at"],
        "next_attempt_at": row["next_attempt_at"],
        "dead_letter_at": row["dead_letter_at"],
        "submit_path": row["submit_path"],
        "governance_boundary": row["governance_boundary"],
        "queued_at": row["queued_at"],
        "created_at": row["created_at"],
        "updated_at": row["updated_at"],
        "started_at": row["started_at"],
        "completed_at": row["completed_at"],
        "failed_at": row["failed_at"],
    }


def worker_row_to_record(row: sqlite3.Row) -> dict[str, Any]:
    return {
        "worker_id": row["worker_id"],
        "status": row["status"],
        "last_heartbeat_at": row["last_heartbeat_at"],
        "current_job_id": row["current_job_id"],
        "processed_jobs": row["processed_jobs"],
        "idle_polls": row["idle_polls"],
        "metadata": parse_json(row["metadata_json"]) or {},
    }


def parse_json(value: str | None) -> Any:
    return json.loads(value) if value else None


def submit_path(job_id: str) -> str:
    return f"/api/v1/ops/model-retraining-jobs/{job_id}/output"


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def utc_after(seconds: int) -> str:
    return (datetime.now(timezone.utc) + timedelta(seconds=seconds)).isoformat()
