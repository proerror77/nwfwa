from pathlib import Path

from fastapi import FastAPI, HTTPException

from .schemas import ScoreRequest, ScoreResponse, TrainRequest
from .scorer import ModelServingError, score_claim
from .training import train_from_manifest

app = FastAPI(title="FWA ML Service")
TRAINING_JOBS: dict[str, dict[str, object]] = {}


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
def create_training_job(request: TrainRequest) -> dict[str, object]:
    try:
        provider_output = run_training(request)
    except HTTPException as error:
        TRAINING_JOBS[request.job_id] = {
            "job_id": request.job_id,
            "handoff_kind": "external_training_platform_job",
            "status": "failed",
            "model_key": request.model_key,
            "base_model_version": request.base_model_version,
            "algorithm": request.algorithm or "manifest_or_logistic_regression",
            "actor": request.actor,
            "error": error.detail,
            "submit_path": f"/api/v1/ops/model-retraining-jobs/{request.job_id}/output",
            "governance_boundary": (
                "training platform owns training execution; FWA consumes only "
                "completed provider output through the retraining output API"
            ),
        }
        raise

    record = {
        "job_id": request.job_id,
        "handoff_kind": "external_training_platform_job",
        "status": "completed",
        "model_key": request.model_key,
        "base_model_version": request.base_model_version,
        "candidate_model_version": provider_output["candidate_model_version"],
        "algorithm": provider_output.get("metrics_json", {}).get(
            "algorithm",
            request.algorithm or "manifest_or_logistic_regression",
        ),
        "actor": request.actor,
        "submit_path": f"/api/v1/ops/model-retraining-jobs/{request.job_id}/output",
        "provider_output": provider_output,
        "governance_boundary": (
            "training platform owns training execution; FWA consumes only "
            "completed provider output through the retraining output API"
        ),
    }
    TRAINING_JOBS[request.job_id] = record
    return record


@app.get("/training-jobs/{job_id}")
def get_training_job(job_id: str) -> dict[str, object]:
    record = TRAINING_JOBS.get(job_id)
    if record is None:
        raise HTTPException(
            status_code=404,
            detail={
                "code": "TRAINING_JOB_NOT_FOUND",
                "message": f"training job not found: {job_id}",
            },
        )
    return record


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
