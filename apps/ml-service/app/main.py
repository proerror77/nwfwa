import os
from pathlib import Path

from fastapi import BackgroundTasks, FastAPI, HTTPException
from fastapi.responses import JSONResponse

from .schemas import ScoreRequest, ScoreResponse, TrainRequest
from .scorer import ModelServingError, score_claim
from .training import train_from_manifest
from .training_jobs import TrainingJobStore, attach_artifact_registry

app = FastAPI(title="FWA ML Service")


def training_job_store() -> TrainingJobStore:
    store = getattr(app.state, "training_job_store", None)
    if store is None:
        db_path = os.getenv("FWA_TRAINING_JOB_DB", "data/ml-service/training_jobs.sqlite3")
        store = TrainingJobStore(db_path)
        app.state.training_job_store = store
    return store


@app.get("/health")
def health() -> dict[str, object]:
    return {
        "status": "ok",
        "service": "ml-service",
        "version": "0.1.0",
        "checks": [
            {"name": "http_router", "status": "ok"},
            {"name": "baseline_scorer", "status": "ok"},
        ],
    }


@app.post("/score", response_model=ScoreResponse)
def score(request: ScoreRequest) -> ScoreResponse:
    try:
        return score_claim(request)
    except ModelServingError as error:
        raise HTTPException(
            status_code=error.status_code,
            detail={"code": error.code, "message": error.message},
        ) from error


@app.post("/train")
def train(request: TrainRequest) -> dict[str, object]:
    return run_training(request)


@app.post("/training-jobs")
def create_training_job(
    request: TrainRequest,
    background_tasks: BackgroundTasks,
) -> JSONResponse:
    record = training_job_store().create_queued(request)
    existing = bool(record.pop("_existing", False))
    if record["request"] != request.model_dump():
        raise HTTPException(
            status_code=409,
            detail={
                "code": "TRAINING_JOB_CONFLICT",
                "message": f"training job already exists with different request: {request.job_id}",
            },
        )
    if record["status"] == "queued" and not existing:
        background_tasks.add_task(execute_training_job, request)
    status_code = 202 if record["status"] in {"queued", "running"} else 200
    return JSONResponse(record, status_code=status_code)


@app.get("/training-jobs/{job_id}")
def get_training_job(job_id: str) -> dict[str, object]:
    record = training_job_store().get(job_id)
    if record is None:
        raise HTTPException(
            status_code=404,
            detail={
                "code": "TRAINING_JOB_NOT_FOUND",
                "message": f"training job not found: {job_id}",
            },
        )
    return record


def execute_training_job(request: TrainRequest) -> None:
    store = training_job_store()
    store.mark_running(request.job_id)
    try:
        provider_output = run_training(request)
        artifact_registry = attach_artifact_registry(
            provider_output,
            request.job_id,
            request.actor,
            request.model_key,
            request.base_model_version,
        )
    except HTTPException as error:
        store.mark_failed(request.job_id, error.detail)
        return
    store.mark_completed(request.job_id, provider_output, artifact_registry)


def run_training(request: TrainRequest) -> dict[str, object]:
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
