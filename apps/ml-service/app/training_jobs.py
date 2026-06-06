from __future__ import annotations

import json
import sqlite3
from datetime import datetime, timedelta, timezone
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
    attempt_count INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 2,
    worker_id TEXT,
    lease_expires_at TEXT,
    queued_at TEXT NOT NULL,
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
            self._migrate_columns(connection)

    def _migrate_columns(self, connection: sqlite3.Connection) -> None:
        columns = {
            row["name"]
            for row in connection.execute("PRAGMA table_info(training_jobs)").fetchall()
        }
        migrations = {
            "attempt_count": "ALTER TABLE training_jobs ADD COLUMN attempt_count INTEGER NOT NULL DEFAULT 0",
            "max_attempts": "ALTER TABLE training_jobs ADD COLUMN max_attempts INTEGER NOT NULL DEFAULT 2",
            "worker_id": "ALTER TABLE training_jobs ADD COLUMN worker_id TEXT",
            "lease_expires_at": "ALTER TABLE training_jobs ADD COLUMN lease_expires_at TEXT",
            "queued_at": "ALTER TABLE training_jobs ADD COLUMN queued_at TEXT",
        }
        for column, statement in migrations.items():
            if column not in columns:
                connection.execute(statement)
        connection.execute(
            "UPDATE training_jobs SET queued_at = created_at WHERE queued_at IS NULL"
        )

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
            "attempt_count": 0,
            "max_attempts": request.max_attempts,
            "worker_id": None,
            "lease_expires_at": None,
            "submit_path": submit_path(request.job_id),
            "governance_boundary": GOVERNANCE_BOUNDARY,
            "queued_at": now,
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
                    submit_path, governance_boundary, attempt_count, max_attempts,
                    worker_id, lease_expires_at, queued_at, created_at, updated_at,
                    started_at, completed_at, failed_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
                    record["attempt_count"],
                    record["max_attempts"],
                    record["worker_id"],
                    record["lease_expires_at"],
                    record["queued_at"],
                    record["created_at"],
                    record["updated_at"],
                    record["started_at"],
                    record["completed_at"],
                    record["failed_at"],
                ),
            )
        return record

    def claim_job(
        self,
        job_id: str,
        worker_id: str,
        lease_seconds: int,
    ) -> dict[str, Any] | None:
        now = utc_now()
        lease_expires_at = utc_after(seconds=lease_seconds)
        with self._lock, self._connect() as connection:
            updated = connection.execute(
                """
                UPDATE training_jobs
                SET status = 'running',
                    worker_id = ?,
                    lease_expires_at = ?,
                    attempt_count = attempt_count + 1,
                    updated_at = ?,
                    started_at = COALESCE(started_at, ?)
                WHERE job_id = ?
                  AND (
                    status = 'queued'
                    OR (status = 'running' AND lease_expires_at IS NOT NULL AND lease_expires_at <= ?)
                  )
                  AND attempt_count < max_attempts
                """,
                (worker_id, lease_expires_at, now, now, job_id, now),
            )
            if updated.rowcount == 0:
                return None
        return self.get(job_id)

    def claim_next(self, worker_id: str, lease_seconds: int) -> dict[str, Any] | None:
        now = utc_now()
        lease_expires_at = utc_after(seconds=lease_seconds)
        with self._lock, self._connect() as connection:
            connection.execute("BEGIN IMMEDIATE")
            row = connection.execute(
                """
                SELECT job_id
                FROM training_jobs
                WHERE (
                    status = 'queued'
                    OR (status = 'running' AND lease_expires_at IS NOT NULL AND lease_expires_at <= ?)
                )
                  AND attempt_count < max_attempts
                ORDER BY queued_at, created_at, job_id
                LIMIT 1
                """,
                (now,),
            ).fetchone()
            if row is None:
                connection.commit()
                return None
            connection.execute(
                """
                UPDATE training_jobs
                SET status = 'running',
                    worker_id = ?,
                    lease_expires_at = ?,
                    attempt_count = attempt_count + 1,
                    updated_at = ?,
                    started_at = COALESCE(started_at, ?)
                WHERE job_id = ?
                """,
                (worker_id, lease_expires_at, now, now, row["job_id"]),
            )
            connection.commit()
            job_id = row["job_id"]
        return self.get(job_id)

    def mark_completed(
        self,
        job_id: str,
        worker_id: str,
        provider_output: dict[str, Any],
        artifact_registry: dict[str, Any],
    ) -> dict[str, Any] | None:
        now = utc_now()
        with self._lock, self._connect() as connection:
            updated = connection.execute(
                """
                UPDATE training_jobs
                SET status = 'completed',
                    candidate_model_version = ?,
                    algorithm = ?,
                    provider_output_json = ?,
                    artifact_registry_json = ?,
                    error_json = NULL,
                    worker_id = NULL,
                    lease_expires_at = NULL,
                    updated_at = ?,
                    completed_at = ?
                WHERE job_id = ?
                  AND status = 'running'
                  AND worker_id = ?
                  AND (lease_expires_at IS NULL OR lease_expires_at > ?)
                """,
                (
                    provider_output.get("candidate_model_version"),
                    provider_output.get("metrics_json", {}).get("algorithm", "unknown"),
                    json.dumps(provider_output, sort_keys=True),
                    json.dumps(artifact_registry, sort_keys=True),
                    now,
                    now,
                    job_id,
                    worker_id,
                    now,
                ),
            )
            if updated.rowcount == 0:
                return None
        return self.get(job_id)

    def mark_failed(
        self,
        job_id: str,
        worker_id: str,
        error: Any,
    ) -> dict[str, Any] | None:
        now = utc_now()
        with self._lock, self._connect() as connection:
            updated = connection.execute(
                """
                UPDATE training_jobs
                SET status = CASE
                        WHEN attempt_count < max_attempts THEN 'queued'
                        ELSE 'failed'
                    END,
                    error_json = ?,
                    worker_id = NULL,
                    lease_expires_at = NULL,
                    queued_at = CASE
                        WHEN attempt_count < max_attempts THEN ?
                        ELSE queued_at
                    END,
                    updated_at = ?,
                    failed_at = CASE
                        WHEN attempt_count < max_attempts THEN NULL
                        ELSE ?
                    END
                WHERE job_id = ?
                  AND status = 'running'
                  AND worker_id = ?
                """,
                (json.dumps(error, sort_keys=True), now, now, now, job_id, worker_id),
            )
            if updated.rowcount == 0:
                return None
        return self.get(job_id)

    def retry_failed(self, job_id: str) -> dict[str, Any] | None:
        now = utc_now()
        with self._lock, self._connect() as connection:
            updated = connection.execute(
                """
                UPDATE training_jobs
                SET status = 'queued',
                    error_json = NULL,
                    worker_id = NULL,
                    lease_expires_at = NULL,
                    queued_at = ?,
                    updated_at = ?,
                    failed_at = NULL
                WHERE job_id = ?
                  AND status = 'failed'
                  AND attempt_count < max_attempts
                """,
                (now, now, job_id),
            )
            if updated.rowcount == 0:
                return None
        return self.get(job_id)

    def get(self, job_id: str) -> dict[str, Any] | None:
        with self._lock, self._connect() as connection:
            row = connection.execute(
                "SELECT * FROM training_jobs WHERE job_id = ?",
                (job_id,),
            ).fetchone()
        return row_to_record(row) if row else None

    def list(self, status: str | None = None, limit: int = 50) -> list[dict[str, Any]]:
        limit = max(1, min(limit, 200))
        with self._lock, self._connect() as connection:
            if status:
                rows = connection.execute(
                    """
                    SELECT * FROM training_jobs
                    WHERE status = ?
                    ORDER BY queued_at DESC, created_at DESC
                    LIMIT ?
                    """,
                    (status, limit),
                ).fetchall()
            else:
                rows = connection.execute(
                    """
                    SELECT * FROM training_jobs
                    ORDER BY queued_at DESC, created_at DESC
                    LIMIT ?
                    """,
                    (limit,),
                ).fetchall()
        return [row_to_record(row) for row in rows]


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
        "attempt_count": row["attempt_count"],
        "max_attempts": row["max_attempts"],
        "worker_id": row["worker_id"],
        "lease_expires_at": row["lease_expires_at"],
        "submit_path": row["submit_path"],
        "governance_boundary": row["governance_boundary"],
        "queued_at": row["queued_at"],
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


def utc_after(seconds: int) -> str:
    return (datetime.now(timezone.utc) + timedelta(seconds=seconds)).isoformat()
