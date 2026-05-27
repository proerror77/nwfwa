from .schemas import ModelExplanation, ScoreRequest, ScoreResponse


def score_claim(request: ScoreRequest) -> ScoreResponse:
    ratio = float(request.features.get("claim_amount_to_limit_ratio", 0.0))
    provider_tier = str(request.features.get("provider_risk_tier", "LOW"))
    tier_bonus = {"LOW": 0, "MEDIUM": 8, "HIGH": 18}.get(provider_tier, 0)
    score = max(0, min(100, round(ratio * 100 + tier_bonus)))
    label = "HIGH_RISK" if score >= 70 else "LOW_RISK"
    return ScoreResponse(
        model_key=request.model_key,
        model_version="0.1.0",
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
        },
    )
