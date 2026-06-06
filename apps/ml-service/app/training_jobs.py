from __future__ import annotations

import json
import sqlite3
from datetime import datetime, timezone
from pathlib import Path
from threading import RLock
from typing import Any


JOB_STORE_SCHEMA = """
CREATE TABLE IF NOT EXISTS training_jobs (
    job_id TEXT PRIMARY KEY,
    handoff_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    model_key TEXT NOT NULL,
    base_model_version TEXT NOT NULL,
    candidate_model_version TEXT,
    algorithm TEXT NOT NULL,
    actor TEXT NOT NULL,
    request_json TEXT NOT NULL,
    provider_output_json TEXT,
    artifact_registry_json TEXT,
    error_json TEXT,
    submit_path TEXT NOT NULL,
    governance_boundary TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    failed_at TEXT
);
"""


GOVERNANCE_BOUNDARY = (
    "training platform owns training execution; FWA consumes only "
    "completed provider output through the retraining output API"
)


class TrainingJobStore:
    def __init__(self, db_path: str | Path):
        self.db_path = Path(db_path)
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        self._lock = RLock()
        self._initialize()

    def _initialize(self) -> None:
        with self._connect() as connection:
            connection.executescript(JOB_STORE_SCHEMA)

    def _connect(self) -> sqlite3.Connection:
        connection = sqlite3.connect(self.db_path)
        connection.row_factory = sqlite3.Row
        return connection

    def create_queued(self, request: Any) -> dict[str, Any]:
        now = utc_now()
        record = {
            "job_id": request.job_id,
            "handoff_kind": "external_training_platform_job",
            "status": "queued",
            "model_key": request.model_key,
            "base_model_version": request.base_model_version,
            "candidate_model_version": None,
            "algorithm": request.algorithm or "manifest_or_logistic_regression",
            "actor": request.actor,
            "request": request.model_dump(),
            "provider_output": None,
            "artifact_registry": None,
            "error": None,
            "submit_path": submit_path(request.job_id),
            "governance_boundary": GOVERNANCE_BOUNDARY,
            "created_at": now,
            "updated_at": now,
            "started_at": None,
            "completed_at": None,
            "failed_at": None,
        }
        with self._lock, self._connect() as connection:
            existing = self.get(request.job_id)
            if existing is not None:
                existing["_existing"] = True
                return existing
            connection.execute(
                """
                INSERT INTO training_jobs (
                    job_id, handoff_kind, status, model_key, base_model_version,
                    candidate_model_version, algorithm, actor, request_json,
                    provider_output_json, artifact_registry_json, error_json,
                    submit_path, governance_boundary, created_at, updated_at,
                    started_at, completed_at, failed_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    record["job_id"],
                    record["handoff_kind"],
                    record["status"],
                    record["model_key"],
                    record["base_model_version"],
                    record["candidate_model_version"],
                    record["algorithm"],
                    record["actor"],
                    json.dumps(record["request"], sort_keys=True),
                    None,
                    None,
                    None,
                    record["submit_path"],
                    record["governance_boundary"],
                    record["created_at"],
                    record["updated_at"],
                    record["started_at"],
                    record["completed_at"],
                    record["failed_at"],
                ),
            )
        return record

    def mark_running(self, job_id: str) -> dict[str, Any] | None:
        now = utc_now()
        with self._lock, self._connect() as connection:
            connection.execute(
                """
                UPDATE training_jobs
                SET status = 'running', updated_at = ?, started_at = COALESCE(started_at, ?)
                WHERE job_id = ? AND status IN ('queued', 'running')
                """,
                (now, now, job_id),
            )
        return self.get(job_id)

    def mark_completed(
        self,
        job_id: str,
        provider_output: dict[str, Any],
        artifact_registry: dict[str, Any],
    ) -> dict[str, Any] | None:
        now = utc_now()
        with self._lock, self._connect() as connection:
            connection.execute(
                """
                UPDATE training_jobs
                SET status = 'completed',
                    candidate_model_version = ?,
                    algorithm = ?,
                    provider_output_json = ?,
                    artifact_registry_json = ?,
                    error_json = NULL,
                    updated_at = ?,
                    completed_at = ?
                WHERE job_id = ?
                """,
                (
                    provider_output.get("candidate_model_version"),
                    provider_output.get("metrics_json", {}).get("algorithm", "unknown"),
                    json.dumps(provider_output, sort_keys=True),
                    json.dumps(artifact_registry, sort_keys=True),
                    now,
                    now,
                    job_id,
                ),
            )
        return self.get(job_id)

    def mark_failed(self, job_id: str, error: Any) -> dict[str, Any] | None:
        now = utc_now()
        with self._lock, self._connect() as connection:
            connection.execute(
                """
                UPDATE training_jobs
                SET status = 'failed',
                    error_json = ?,
                    updated_at = ?,
                    failed_at = ?
                WHERE job_id = ?
                """,
                (json.dumps(error, sort_keys=True), now, now, job_id),
            )
        return self.get(job_id)

    def get(self, job_id: str) -> dict[str, Any] | None:
        with self._lock, self._connect() as connection:
            row = connection.execute(
                "SELECT * FROM training_jobs WHERE job_id = ?",
                (job_id,),
            ).fetchone()
        return row_to_record(row) if row else None


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
            }
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
        "submit_path": row["submit_path"],
        "governance_boundary": row["governance_boundary"],
        "created_at": row["created_at"],
        "updated_at": row["updated_at"],
        "started_at": row["started_at"],
        "completed_at": row["completed_at"],
        "failed_at": row["failed_at"],
    }


def parse_json(value: str | None) -> Any:
    return json.loads(value) if value else None


def submit_path(job_id: str) -> str:
    return f"/api/v1/ops/model-retraining-jobs/{job_id}/output"


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()
