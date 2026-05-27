from fastapi.testclient import TestClient

from app.main import app


client = TestClient(app)


def test_score_returns_high_risk_for_large_amount_ratio():
    response = client.post(
        "/score",
        json={
            "run_id": "run_test",
            "claim_id": "CLM-1",
            "model_key": "baseline_fwa",
            "features": {
                "claim_amount_to_limit_ratio": 0.82,
                "provider_risk_tier": "MEDIUM",
            },
        },
    )

    assert response.status_code == 200
    payload = response.json()
    assert payload["score"] == 90
    assert payload["label"] == "HIGH_RISK"
    assert payload["model_version"] == "0.1.0"
