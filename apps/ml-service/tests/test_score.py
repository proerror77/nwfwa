from fastapi.testclient import TestClient

from app.main import app


client = TestClient(app)


def score_payload(features: dict[str, object]) -> dict[str, object]:
    return {
        "run_id": "run_test",
        "claim_id": "CLM-1",
        "model_key": "baseline_fwa",
        "model_version": "0.1.0",
        "features": features,
    }


def test_health_returns_ok():
    response = client.get("/health")

    assert response.status_code == 200
    assert response.json() == {
        "status": "ok",
        "service": "ml-service",
        "version": "0.1.0",
        "checks": [
            {"name": "http_router", "status": "ok"},
            {"name": "baseline_scorer", "status": "ok"},
        ],
    }


def test_score_returns_high_risk_for_large_amount_ratio():
    response = client.post(
        "/score",
        json=score_payload(
            {
                "claim_amount_to_limit_ratio": 0.82,
                "provider_risk_tier": "MEDIUM",
                "high_cost_item_ratio": 0.6,
            },
        ),
    )

    assert response.status_code == 200
    payload = response.json()
    assert payload["score"] == 90
    assert payload["label"] == "HIGH_RISK"
    assert payload["model_version"] == "0.1.0"
    assert payload["metadata"]["runtime_kind"] == "python_fastapi"
    assert payload["metadata"]["fraud_probability"] == 0.9
    assert payload["metadata"]["abuse_probability"] == 0.614
    assert payload["metadata"]["waste_probability"] == 0.584


def test_score_returns_low_risk_for_low_amount_ratio():
    response = client.post(
        "/score",
        json=score_payload(
            {
                "claim_amount_to_limit_ratio": 0.12,
                "provider_risk_tier": "LOW",
            },
        ),
    )

    assert response.status_code == 200
    payload = response.json()
    assert payload["score"] == 12
    assert payload["label"] == "LOW_RISK"
    assert payload["metadata"]["execution_provider"] == "cpu"
    assert payload["explanations"][0]["feature"] == "claim_amount_to_limit_ratio"


def test_score_echoes_requested_model_version():
    payload = score_payload({"claim_amount_to_limit_ratio": 0.72})
    payload["model_version"] = "0.2.0-active"

    response = client.post("/score", json=payload)

    assert response.status_code == 200
    assert response.json()["model_version"] == "0.2.0-active"


def test_score_clamps_score_to_response_contract_range():
    response = client.post(
        "/score",
        json=score_payload(
            {
                "claim_amount_to_limit_ratio": 1.5,
                "provider_risk_tier": "HIGH",
            },
        ),
    )

    assert response.status_code == 200
    payload = response.json()
    assert payload["score"] == 100
    assert payload["label"] == "HIGH_RISK"


def test_score_rejects_invalid_payload():
    response = client.post(
        "/score",
        json={
            "claim_id": "CLM-1",
            "features": {"claim_amount_to_limit_ratio": 0.5},
        },
    )

    assert response.status_code == 422
