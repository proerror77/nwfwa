import os
from functools import lru_cache
from typing import Any

import joblib
import pandas as pd

from .mlops import artifact_signature, file_sha256, heuristic_score_from_features
from .schemas import ModelExplanation, ScoreRequest, ScoreResponse


class ModelServingError(Exception):
    def __init__(self, code: str, message: str, status_code: int = 409):
        super().__init__(message)
        self.code = code
        self.message = message
        self.status_code = status_code


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
    artifact_sha256 = verify_artifact_checksum(artifact_uri)
    bundle = load_model_artifact(artifact_uri, artifact_sha256)
    verify_model_version_lock(str(bundle["model_version"]))
    verify_artifact_signature(
        str(bundle["model_key"]),
        str(bundle["model_version"]),
        artifact_sha256,
    )
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
    metadata = {
        "runtime_kind": bundle.get("runtime_kind", "sklearn"),
        "execution_provider": bundle.get("execution_provider", "cpu"),
        "calibration": "artifact_threshold",
        "fraud_probability": round(probability, 4),
        "threshold": threshold,
        "feature_count": len(feature_columns),
        "artifact_sha256": artifact_sha256,
        "artifact_integrity_status": "passed",
        "artifact_signature_status": "passed",
        "serving_version_lock": os.getenv("FWA_MODEL_VERSION_LOCK", "").strip()
        or str(bundle["model_version"]),
        "serving_version_lock_status": "passed",
    }
    if shadow_heuristic_enabled():
        shadow_score = heuristic_score_from_features(request.features)
        shadow_delta = score - shadow_score
        metadata.update(
            {
                "shadow_mode": "heuristic_baseline",
                "shadow_score": shadow_score,
                "shadow_delta": shadow_delta,
                "shadow_status": shadow_status(shadow_delta),
            }
        )
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
        metadata=metadata,
    )


def verify_artifact_checksum(artifact_uri: str) -> str:
    actual = file_sha256(artifact_uri)
    expected = (
        os.getenv("FWA_MODEL_ARTIFACT_SHA256", "").strip()
        or os.getenv("FWA_MODEL_ARTIFACT_CHECKSUM", "").strip()
    )
    if expected and expected != actual:
        raise ModelServingError(
            "MODEL_ARTIFACT_CHECKSUM_MISMATCH",
            "model artifact checksum does not match configured value",
        )
    return actual


def verify_model_version_lock(model_version: str) -> None:
    locked_version = os.getenv("FWA_MODEL_VERSION_LOCK", "").strip()
    if locked_version and locked_version != model_version:
        raise ModelServingError(
            "MODEL_VERSION_LOCK_MISMATCH",
            "loaded model version does not match configured serving version lock",
        )


def verify_artifact_signature(
    model_key: str,
    model_version: str,
    artifact_sha256: str,
) -> None:
    expected = os.getenv("FWA_MODEL_ARTIFACT_SIGNATURE", "").strip()
    if not expected:
        return
    signing_key = os.getenv("FWA_MODEL_SIGNATURE_KEY", "").strip()
    if not signing_key:
        raise ModelServingError(
            "MODEL_ARTIFACT_SIGNATURE_KEY_MISSING",
            "model artifact signature is configured but signing key is missing",
        )
    actual = artifact_signature(model_key, model_version, artifact_sha256, signing_key)
    if actual != expected:
        raise ModelServingError(
            "MODEL_ARTIFACT_SIGNATURE_MISMATCH",
            "model artifact signature does not match configured value",
        )


def shadow_heuristic_enabled() -> bool:
    return os.getenv("FWA_MODEL_SHADOW_HEURISTIC", "").strip().lower() in {
        "1",
        "true",
        "yes",
    }


def shadow_status(delta: int) -> str:
    absolute_delta = abs(delta)
    if absolute_delta <= 10:
        return "passed"
    if absolute_delta <= 25:
        return "watch"
    return "drift"


@lru_cache(maxsize=4)
def load_model_artifact(artifact_uri: str, artifact_sha256: str) -> dict[str, Any]:
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
