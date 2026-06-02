import os
from functools import lru_cache
from typing import Any

import joblib
import pandas as pd

from .schemas import ModelExplanation, ScoreRequest, ScoreResponse


def score_claim(request: ScoreRequest) -> ScoreResponse:
    artifact_uri = os.getenv("FWA_MODEL_ARTIFACT_URI", "").strip()
    if artifact_uri:
        return score_with_artifact(request, artifact_uri)
    return score_with_heuristic(request)


def score_with_heuristic(request: ScoreRequest) -> ScoreResponse:
    ratio = float(request.features.get("claim_amount_to_limit_ratio", 0.0))
    provider_tier = str(request.features.get("provider_risk_tier", "LOW"))
    high_cost_item_ratio = float(request.features.get("high_cost_item_ratio", 0.0))
    tier_bonus = {"LOW": 0, "MEDIUM": 8, "HIGH": 18}.get(provider_tier, 0)
    score = max(0, min(100, round(ratio * 100 + tier_bonus)))
    label = "HIGH_RISK" if score >= 70 else "LOW_RISK"
    fraud_probability = round(score / 100, 4)
    abuse_probability = round(max(0.0, min(1.0, ratio * 0.7 + tier_bonus / 200)), 4)
    waste_probability = round(max(0.0, min(1.0, high_cost_item_ratio * 0.7 + ratio * 0.2)), 4)
    return ScoreResponse(
        model_key=request.model_key,
        model_version=request.model_version,
        score=score,
        label=label,
        explanations=[
            ModelExplanation(
                feature="claim_amount_to_limit_ratio",
                direction="increases_risk",
                contribution=ratio,
                reason="理赔金额占保障额度比例较高",
            )
        ],
        metadata={
            "runtime_kind": "python_fastapi",
            "execution_provider": "cpu",
            "calibration": "baseline_v0",
            "fraud_probability": fraud_probability,
            "abuse_probability": abuse_probability,
            "waste_probability": waste_probability,
        },
    )


def score_with_artifact(request: ScoreRequest, artifact_uri: str) -> ScoreResponse:
    bundle = load_model_artifact(artifact_uri)
    feature_columns = list(bundle["feature_columns"])
    threshold = float(bundle.get("threshold", 0.5))
    model = bundle["pipeline"]
    frame = pd.DataFrame(
        [
            {
                feature: float(request.features.get(feature, 0.0))
                for feature in feature_columns
            }
        ]
    )
    probability = float(model.predict_proba(frame)[0][1])
    score = max(0, min(100, round(probability * 100)))
    return ScoreResponse(
        model_key=str(bundle["model_key"]),
        model_version=str(bundle["model_version"]),
        score=score,
        label="HIGH_RISK" if probability >= threshold else "LOW_RISK",
        explanations=[
            ModelExplanation(
                feature=feature,
                direction="model_input",
                contribution=float(frame.iloc[0][feature]),
                reason="模型 artifact 使用的输入特征",
            )
            for feature in feature_columns[:5]
        ],
        metadata={
            "runtime_kind": bundle.get("runtime_kind", "sklearn"),
            "execution_provider": bundle.get("execution_provider", "cpu"),
            "calibration": "artifact_threshold",
            "fraud_probability": round(probability, 4),
            "threshold": threshold,
            "feature_count": len(feature_columns),
        },
    )


@lru_cache(maxsize=4)
def load_model_artifact(artifact_uri: str) -> dict[str, Any]:
    bundle = joblib.load(artifact_uri)
    required_keys = {
        "model_key",
        "model_version",
        "feature_columns",
        "pipeline",
    }
    missing_keys = sorted(required_keys - set(bundle))
    if missing_keys:
        raise ValueError(f"model artifact missing keys: {', '.join(missing_keys)}")
    return bundle


def reset_model_artifact_cache() -> None:
    load_model_artifact.cache_clear()
