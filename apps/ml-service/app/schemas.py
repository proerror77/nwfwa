from pydantic import BaseModel, Field


class ScoreRequest(BaseModel):
    run_id: str
    claim_id: str
    model_key: str = "baseline_fwa"
    model_version: str = "0.1.0"
    features: dict[str, object]


class ModelExplanation(BaseModel):
    feature: str
    direction: str
    contribution: float
    reason: str


class ScoreResponse(BaseModel):
    model_key: str
    model_version: str
    score: int = Field(ge=0, le=100)
    label: str
    explanations: list[ModelExplanation]
    metadata: dict[str, object]
