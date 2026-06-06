from __future__ import annotations

from pathlib import Path
from typing import Any

from fastapi import HTTPException

from .schemas import TrainRequest
from .training import train_from_manifest
from .training_jobs import TrainingJobStore, attach_artifact_registry


def execute_next_training_job(
    store: TrainingJobStore,
    worker_id: str,
    lease_seconds: int,
) -> dict[str, Any] | None:
    record = store.claim_next(worker_id=worker_id, lease_seconds=lease_seconds)
    if record is None:
        return None
    return execute_training_job(store, record, worker_id)


def execute_training_job(
    store: TrainingJobStore,
    record: dict[str, Any],
    worker_id: str,
) -> dict[str, Any] | None:
    request = TrainRequest(**record["request"])
    try:
        provider_output = run_training_request(request)
        artifact_registry = attach_artifact_registry(
            provider_output,
            request.job_id,
            request.actor,
            request.model_key,
            request.base_model_version,
        )
    except HTTPException as error:
        return store.mark_failed(request.job_id, worker_id, error.detail)
    return store.mark_completed(
        request.job_id,
        worker_id,
        provider_output,
        artifact_registry,
    )


def run_training_request(request: TrainRequest) -> dict[str, Any]:
    try:
        return train_from_manifest(
            manifest_path=Path(request.manifest_path),
            artifact_base_uri=Path(request.artifact_base_uri),
            model_key=request.model_key,
            base_model_version=request.base_model_version,
            job_id=request.job_id,
            actor=request.actor,
            algorithm=request.algorithm,
        )
    except (OSError, ValueError, TypeError) as error:
        raise HTTPException(
            status_code=400,
            detail={"code": "TRAINING_FAILED", "message": str(error)},
        ) from error
