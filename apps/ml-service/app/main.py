from fastapi import FastAPI

from .schemas import ScoreRequest, ScoreResponse
from .scorer import score_claim

app = FastAPI(title="FWA ML Service")


@app.get("/health")
def health() -> dict[str, str]:
    return {"status": "ok"}


@app.post("/score", response_model=ScoreResponse)
def score(request: ScoreRequest) -> ScoreResponse:
    return score_claim(request)
