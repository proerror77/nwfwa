from fastapi import FastAPI

from .schemas import ScoreRequest, ScoreResponse
from .scorer import score_claim

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
    return score_claim(request)
