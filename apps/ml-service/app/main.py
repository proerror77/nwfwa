from pathlib import Path

from fastapi import FastAPI, HTTPException

from .schemas import ScoreRequest, ScoreResponse, TrainRequest
from .scorer import ModelServingError, score_claim
from .training import train_from_manifest

app = FastAPI(title="FWA ML Service")


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
