from pydantic import BaseModel, Field


class ScoreRequest(BaseModel):
    run_id: str
    claim_id: str
    model_key: str = "baseline_fwa"
    model_version: str = "0.1.0"
    features: dict[str, object]


class TrainRequest(BaseModel):
    manifest_path: str
    artifact_base_uri: str
    model_key: str
    base_model_version: str
    job_id: str
    actor: str
    algorithm: str | None = None
    max_attempts: int = Field(default=2, ge=1, le=5)
    retry_delay_seconds: int = Field(default=60, ge=0, le=3600)


class ClaimTrainingJobRequest(BaseModel):
    worker_id: str
    lease_seconds: int = Field(default=900, ge=30, le=86400)


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
