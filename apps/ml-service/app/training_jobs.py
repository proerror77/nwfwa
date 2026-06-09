from __future__ import annotations

import json
import sqlite3
from pathlib import Path
from threading import RLock
from typing import Any

from .training_job_records import (
    artifact_registry_summary,
    attach_artifact_registry,
    render_prometheus_metrics,
    row_to_record,
    submit_path,
    utc_after,
    utc_now,
    worker_row_to_record,
)


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
    retry_delay_seconds INTEGER NOT NULL DEFAULT 60,
    worker_id TEXT,
    lease_expires_at TEXT,
    next_attempt_at TEXT,
    dead_letter_at TEXT,
    queued_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    failed_at TEXT
);

CREATE TABLE IF NOT EXISTS training_workers (
    worker_id TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    last_heartbeat_at TEXT NOT NULL,
    current_job_id TEXT,
    processed_jobs INTEGER NOT NULL DEFAULT 0,
    idle_polls INTEGER NOT NULL DEFAULT 0,
    metadata_json TEXT NOT NULL DEFAULT '{}'
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
            "retry_delay_seconds": "ALTER TABLE training_jobs ADD COLUMN retry_delay_seconds INTEGER NOT NULL DEFAULT 60",
            "worker_id": "ALTER TABLE training_jobs ADD COLUMN worker_id TEXT",
            "lease_expires_at": "ALTER TABLE training_jobs ADD COLUMN lease_expires_at TEXT",
            "next_attempt_at": "ALTER TABLE training_jobs ADD COLUMN next_attempt_at TEXT",
            "dead_letter_at": "ALTER TABLE training_jobs ADD COLUMN dead_letter_at TEXT",
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
            "retry_delay_seconds": request.retry_delay_seconds,
            "worker_id": None,
            "lease_expires_at": None,
            "next_attempt_at": None,
            "dead_letter_at": None,
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
                    retry_delay_seconds, worker_id, lease_expires_at,
                    next_attempt_at, dead_letter_at, queued_at, created_at,
                    updated_at, started_at, completed_at, failed_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
                    record["retry_delay_seconds"],
                    record["worker_id"],
                    record["lease_expires_at"],
                    record["next_attempt_at"],
                    record["dead_letter_at"],
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
                    (status = 'queued' AND (next_attempt_at IS NULL OR next_attempt_at <= ?))
                    OR (status = 'running' AND lease_expires_at IS NOT NULL AND lease_expires_at <= ?)
                  )
                  AND attempt_count < max_attempts
                """,
                (worker_id, lease_expires_at, now, now, job_id, now, now),
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
                    (status = 'queued' AND (next_attempt_at IS NULL OR next_attempt_at <= ?))
                    OR (status = 'running' AND lease_expires_at IS NOT NULL AND lease_expires_at <= ?)
                )
                  AND attempt_count < max_attempts
                ORDER BY queued_at, created_at, job_id
                LIMIT 1
                """,
                (now, now),
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
                    next_attempt_at = NULL,
                    dead_letter_at = NULL,
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

    def renew_lease(
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
                SET lease_expires_at = ?,
                    updated_at = ?
                WHERE job_id = ?
                  AND status = 'running'
                  AND worker_id = ?
                  AND (lease_expires_at IS NULL OR lease_expires_at > ?)
                """,
                (lease_expires_at, now, job_id, worker_id, now),
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
            row = connection.execute(
                "SELECT attempt_count, max_attempts, retry_delay_seconds FROM training_jobs WHERE job_id = ?",
                (job_id,),
            ).fetchone()
            if row is None:
                return None
            will_retry = row["attempt_count"] < row["max_attempts"]
            next_attempt_at = (
                utc_after(seconds=row["retry_delay_seconds"]) if will_retry else None
            )
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
                    next_attempt_at = ?,
                    dead_letter_at = CASE
                        WHEN attempt_count < max_attempts THEN NULL
                        ELSE ?
                    END,
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
                (
                    json.dumps(error, sort_keys=True),
                    next_attempt_at,
                    now,
                    now,
                    now,
                    now,
                    job_id,
                    worker_id,
                ),
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
                    attempt_count = 0,
                    error_json = NULL,
                    worker_id = NULL,
                    lease_expires_at = NULL,
                    next_attempt_at = NULL,
                    dead_letter_at = NULL,
                    queued_at = ?,
                    updated_at = ?,
                    failed_at = NULL
                WHERE job_id = ?
                  AND status = 'failed'
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

    def list_artifact_registries(
        self,
        model_key: str | None = None,
        candidate_model_version: str | None = None,
        limit: int = 50,
    ) -> list[dict[str, Any]]:
        limit = max(1, min(limit, 200))
        conditions = ["artifact_registry_json IS NOT NULL"]
        values: list[Any] = []
        if model_key:
            conditions.append("model_key = ?")
            values.append(model_key)
        if candidate_model_version:
            conditions.append("candidate_model_version = ?")
            values.append(candidate_model_version)
        values.append(limit)
        with self._lock, self._connect() as connection:
            rows = connection.execute(
                f"""
                SELECT *
                FROM training_jobs
                WHERE {' AND '.join(conditions)}
                ORDER BY completed_at DESC, updated_at DESC, job_id
                LIMIT ?
                """,
                values,
            ).fetchall()
        return [artifact_registry_summary(row_to_record(row)) for row in rows]

    def get_artifact_registry(
        self,
        model_key: str,
        candidate_model_version: str,
    ) -> dict[str, Any] | None:
        registries = self.list_artifact_registries(
            model_key=model_key,
            candidate_model_version=candidate_model_version,
            limit=1,
        )
        return registries[0] if registries else None

    def record_heartbeat(
        self,
        worker_id: str,
        status: str,
        current_job_id: str | None = None,
        processed_jobs: int = 0,
        idle_polls: int = 0,
        metadata: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        now = utc_now()
        with self._lock, self._connect() as connection:
            connection.execute(
                """
                INSERT INTO training_workers (
                    worker_id, status, last_heartbeat_at, current_job_id,
                    processed_jobs, idle_polls, metadata_json
                )
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(worker_id) DO UPDATE SET
                    status = excluded.status,
                    last_heartbeat_at = excluded.last_heartbeat_at,
                    current_job_id = excluded.current_job_id,
                    processed_jobs = excluded.processed_jobs,
                    idle_polls = excluded.idle_polls,
                    metadata_json = excluded.metadata_json
                """,
                (
                    worker_id,
                    status,
                    now,
                    current_job_id,
                    processed_jobs,
                    idle_polls,
                    json.dumps(metadata or {}, sort_keys=True),
                ),
            )
        record = self.get_worker(worker_id)
        if record is None:
            raise RuntimeError(f"training worker heartbeat was not recorded: {worker_id}")
        return record

    def get_worker(self, worker_id: str) -> dict[str, Any] | None:
        with self._lock, self._connect() as connection:
            row = connection.execute(
                "SELECT * FROM training_workers WHERE worker_id = ?",
                (worker_id,),
            ).fetchone()
        return worker_row_to_record(row) if row else None

    def list_workers(self, limit: int = 50) -> list[dict[str, Any]]:
        limit = max(1, min(limit, 200))
        with self._lock, self._connect() as connection:
            rows = connection.execute(
                """
                SELECT * FROM training_workers
                ORDER BY last_heartbeat_at DESC, worker_id
                LIMIT ?
                """,
                (limit,),
            ).fetchall()
        return [worker_row_to_record(row) for row in rows]

    def queue_metrics(self) -> dict[str, Any]:
        now = utc_now()
        with self._lock, self._connect() as connection:
            status_rows = connection.execute(
                """
                SELECT status, COUNT(*) AS count
                FROM training_jobs
                GROUP BY status
                """
            ).fetchall()
            ready_jobs = connection.execute(
                """
                SELECT COUNT(*) AS count
                FROM training_jobs
                WHERE status = 'queued'
                  AND (next_attempt_at IS NULL OR next_attempt_at <= ?)
                  AND attempt_count < max_attempts
                """,
                (now,),
            ).fetchone()["count"]
            delayed_jobs = connection.execute(
                """
                SELECT COUNT(*) AS count
                FROM training_jobs
                WHERE status = 'queued'
                  AND next_attempt_at IS NOT NULL
                  AND next_attempt_at > ?
                """,
                (now,),
            ).fetchone()["count"]
            expired_leases = connection.execute(
                """
                SELECT COUNT(*) AS count
                FROM training_jobs
                WHERE status = 'running'
                  AND lease_expires_at IS NOT NULL
                  AND lease_expires_at <= ?
                """,
                (now,),
            ).fetchone()["count"]
            dead_letter_jobs = connection.execute(
                """
                SELECT COUNT(*) AS count
                FROM training_jobs
                WHERE status = 'failed'
                  AND dead_letter_at IS NOT NULL
                """
            ).fetchone()["count"]
            registered_workers = connection.execute(
                "SELECT COUNT(*) AS count FROM training_workers"
            ).fetchone()["count"]
        return {
            "metrics_kind": "training_queue_metrics",
            "generated_at": now,
            "jobs_by_status": {row["status"]: row["count"] for row in status_rows},
            "ready_jobs": ready_jobs,
            "delayed_jobs": delayed_jobs,
            "expired_leases": expired_leases,
            "dead_letter_jobs": dead_letter_jobs,
            "registered_workers": registered_workers,
        }

    def prometheus_metrics(self) -> str:
        return render_prometheus_metrics(
            self.queue_metrics(),
            self.list_workers(limit=200),
        )
