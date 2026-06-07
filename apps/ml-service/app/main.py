import os

from fastapi import BackgroundTasks, FastAPI, HTTPException, Query
from fastapi.responses import JSONResponse, PlainTextResponse

from .schemas import (
    ClaimTrainingJobRequest,
    ScoreRequest,
    ScoreResponse,
    TrainingWorkerHeartbeatRequest,
    TrainRequest,
)
from .scorer import ModelServingError, score_claim
from .training_runner import execute_next_training_job, execute_training_job, run_training_request
from .training_jobs import TrainingJobStore

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
        background_tasks.add_task(
            execute_next_training_job,
            training_job_store(),
            "ml-service-background",
            900,
        )
    status_code = 202 if record["status"] in {"queued", "running"} else 200
    return JSONResponse(record, status_code=status_code)


@app.get("/training-jobs")
def list_training_jobs(
    status: str | None = None,
    limit: int = Query(default=50, ge=1, le=200),
) -> dict[str, object]:
    return {"jobs": training_job_store().list(status=status, limit=limit)}


@app.get("/training-jobs/metrics")
def get_training_job_metrics() -> dict[str, object]:
    return training_job_store().queue_metrics()


@app.get("/metrics", response_class=PlainTextResponse)
def get_prometheus_metrics() -> PlainTextResponse:
    return PlainTextResponse(
        training_job_store().prometheus_metrics(),
        media_type="text/plain; version=0.0.4; charset=utf-8",
    )


@app.get("/artifact-registries")
def list_artifact_registries(
    model_key: str | None = None,
    candidate_model_version: str | None = None,
    limit: int = Query(default=50, ge=1, le=200),
) -> dict[str, object]:
    return {
        "registries": training_job_store().list_artifact_registries(
            model_key=model_key,
            candidate_model_version=candidate_model_version,
            limit=limit,
        )
    }


@app.get("/artifact-registries/{model_key}/{candidate_model_version}")
def get_artifact_registry(
    model_key: str,
    candidate_model_version: str,
) -> dict[str, object]:
    registry = training_job_store().get_artifact_registry(
        model_key,
        candidate_model_version,
    )
    if registry is None:
        raise HTTPException(
            status_code=404,
            detail={
                "code": "TRAINING_ARTIFACT_REGISTRY_NOT_FOUND",
                "message": (
                    "training artifact registry not found for model version: "
                    f"{model_key}/{candidate_model_version}"
                ),
            },
        )
    return registry


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


@app.get("/training-jobs/{job_id}/artifacts")
def get_training_job_artifacts(job_id: str) -> dict[str, object]:
    record = training_job_store().get(job_id)
    if record is None:
        raise HTTPException(
            status_code=404,
            detail={
                "code": "TRAINING_JOB_NOT_FOUND",
                "message": f"training job not found: {job_id}",
            },
        )
    if record["artifact_registry"] is None:
        raise HTTPException(
            status_code=404,
            detail={
                "code": "TRAINING_ARTIFACT_REGISTRY_NOT_READY",
                "message": f"training artifact registry is not ready for job: {job_id}",
            },
        )
    return {
        "job_id": job_id,
        "status": record["status"],
        "artifact_registry": record["artifact_registry"],
    }


@app.post("/training-jobs/claim-next")
def claim_next_training_job(request: ClaimTrainingJobRequest) -> JSONResponse:
    record = training_job_store().claim_next(
        worker_id=request.worker_id,
        lease_seconds=request.lease_seconds,
    )
    if record is None:
        return JSONResponse(
            {
                "status": "empty",
                "message": "no queued or expired training job is available",
            },
            status_code=404,
        )
    return JSONResponse(record, status_code=200)


@app.post("/training-jobs/{job_id}/run")
def run_claimed_training_job(
    job_id: str,
    request: ClaimTrainingJobRequest,
) -> dict[str, object]:
    record = training_job_store().claim_job(
        job_id,
        worker_id=request.worker_id,
        lease_seconds=request.lease_seconds,
    )
    if record is None:
        record = training_job_store().get(job_id)
        if record is None:
            raise HTTPException(
                status_code=404,
                detail={
                    "code": "TRAINING_JOB_NOT_FOUND",
                    "message": f"training job not found: {job_id}",
                },
            )
        if record["status"] != "running" or record["worker_id"] != request.worker_id:
            raise HTTPException(
                status_code=409,
                detail={
                    "code": "TRAINING_JOB_NOT_CLAIMED",
                    "message": f"training job is not claimed by worker: {request.worker_id}",
                },
            )
    completed = execute_training_job(training_job_store(), record, request.worker_id)
    if completed is None:
        raise HTTPException(
            status_code=409,
            detail={
                "code": "TRAINING_JOB_LEASE_LOST",
                "message": f"training job lease is no longer owned by worker: {request.worker_id}",
            },
        )
    return completed


@app.post("/training-jobs/{job_id}/renew-lease")
def renew_training_job_lease(
    job_id: str,
    request: ClaimTrainingJobRequest,
) -> dict[str, object]:
    record = training_job_store().renew_lease(
        job_id,
        worker_id=request.worker_id,
        lease_seconds=request.lease_seconds,
    )
    if record is None:
        raise HTTPException(
            status_code=409,
            detail={
                "code": "TRAINING_JOB_LEASE_NOT_RENEWED",
                "message": f"training job lease could not be renewed by worker: {request.worker_id}",
            },
        )
    return record


@app.post("/training-jobs/{job_id}/retry")
def retry_training_job(job_id: str) -> dict[str, object]:
    record = training_job_store().retry_failed(job_id)
    if record is None:
        raise HTTPException(
            status_code=409,
            detail={
                "code": "TRAINING_JOB_RETRY_NOT_AVAILABLE",
                "message": f"training job cannot be requeued: {job_id}",
            },
        )
    return record


@app.post("/training-workers/heartbeat")
def record_training_worker_heartbeat(
    request: TrainingWorkerHeartbeatRequest,
) -> dict[str, object]:
    return training_job_store().record_heartbeat(
        worker_id=request.worker_id,
        status=request.status,
        current_job_id=request.current_job_id,
        processed_jobs=request.processed_jobs,
        idle_polls=request.idle_polls,
        metadata=request.metadata,
    )


@app.get("/training-workers")
def list_training_workers(
    limit: int = Query(default=50, ge=1, le=200),
) -> dict[str, object]:
    return {"workers": training_job_store().list_workers(limit=limit)}


def run_training(request: TrainRequest) -> dict[str, object]:
    return run_training_request(request)
