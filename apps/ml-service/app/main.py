from fastapi import FastAPI, HTTPException

from .schemas import ScoreRequest, ScoreResponse
from .scorer import ModelServingError, score_claim

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
