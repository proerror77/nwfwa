from fastapi.testclient import TestClient
import hashlib
import joblib

from app.main import app
from app.mlops import artifact_signature
from app.scorer import reset_model_artifact_cache


client = TestClient(app)


class FixedProbabilityModel:
    def predict_proba(self, _features):
        return [[0.18, 0.82]]


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


def test_score_uses_configured_model_artifact(monkeypatch, tmp_path):
    artifact_path = tmp_path / "model.joblib"
    joblib.dump(
        {
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-candidate",
            "runtime_kind": "sklearn_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": [
                "claim_amount_to_limit_ratio",
                "provider_profile_score",
            ],
            "pipeline": FixedProbabilityModel(),
        },
        artifact_path,
    )
    monkeypatch.setenv("FWA_MODEL_ARTIFACT_URI", str(artifact_path))
    reset_model_artifact_cache()

    response = client.post(
        "/score",
        json=score_payload(
            {
                "claim_amount_to_limit_ratio": 0.92,
                "provider_profile_score": 80.0,
            },
        ),
    )

    reset_model_artifact_cache()
    assert response.status_code == 200
    payload = response.json()
    assert payload["model_key"] == "baseline_fwa"
    assert payload["model_version"] == "0.2.0-candidate"
    assert payload["score"] == 82
    assert payload["label"] == "HIGH_RISK"
    assert payload["metadata"]["runtime_kind"] == "sklearn_logistic_regression"
    assert payload["metadata"]["calibration"] == "artifact_threshold"


def test_score_enforces_artifact_checksum_and_version_lock(monkeypatch, tmp_path):
    artifact_path = tmp_path / "model.joblib"
    joblib.dump(
        {
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-candidate",
            "runtime_kind": "sklearn_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "pipeline": FixedProbabilityModel(),
        },
        artifact_path,
    )
    checksum = "sha256:" + hashlib.sha256(artifact_path.read_bytes()).hexdigest()
    monkeypatch.setenv("FWA_MODEL_ARTIFACT_URI", str(artifact_path))
    monkeypatch.setenv("FWA_MODEL_VERSION_LOCK", "0.2.0-candidate")
    monkeypatch.setenv("FWA_MODEL_ARTIFACT_SHA256", checksum)
    monkeypatch.setenv("FWA_MODEL_SIGNATURE_KEY", "test-signing-key")
    monkeypatch.setenv(
        "FWA_MODEL_ARTIFACT_SIGNATURE",
        artifact_signature(
            "baseline_fwa",
            "0.2.0-candidate",
            checksum,
            "test-signing-key",
        ),
    )
    reset_model_artifact_cache()

    response = client.post(
        "/score",
        json=score_payload({"claim_amount_to_limit_ratio": 0.92}),
    )

    reset_model_artifact_cache()
    assert response.status_code == 200
    payload = response.json()
    assert payload["metadata"]["artifact_sha256"] == checksum
    assert payload["metadata"]["artifact_signature_status"] == "passed"
    assert payload["metadata"]["serving_version_lock"] == "0.2.0-candidate"
    assert payload["metadata"]["serving_version_lock_status"] == "passed"


def test_score_rejects_locked_model_version_mismatch(monkeypatch, tmp_path):
    artifact_path = tmp_path / "model.joblib"
    joblib.dump(
        {
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-candidate",
            "runtime_kind": "sklearn_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "pipeline": FixedProbabilityModel(),
        },
        artifact_path,
    )
    monkeypatch.setenv("FWA_MODEL_ARTIFACT_URI", str(artifact_path))
    monkeypatch.setenv("FWA_MODEL_VERSION_LOCK", "0.3.0-active")
    reset_model_artifact_cache()

    response = client.post(
        "/score",
        json=score_payload({"claim_amount_to_limit_ratio": 0.92}),
    )

    reset_model_artifact_cache()
    assert response.status_code == 409
    assert response.json()["detail"]["code"] == "MODEL_VERSION_LOCK_MISMATCH"


def test_score_records_shadow_heuristic_comparison(monkeypatch, tmp_path):
    artifact_path = tmp_path / "model.joblib"
    joblib.dump(
        {
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-candidate",
            "runtime_kind": "sklearn_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "pipeline": FixedProbabilityModel(),
        },
        artifact_path,
    )
    monkeypatch.setenv("FWA_MODEL_ARTIFACT_URI", str(artifact_path))
    monkeypatch.setenv("FWA_MODEL_SHADOW_HEURISTIC", "true")
    reset_model_artifact_cache()

    response = client.post(
        "/score",
        json=score_payload(
            {
                "claim_amount_to_limit_ratio": 0.92,
                "provider_risk_tier": "MEDIUM",
            },
        ),
    )

    reset_model_artifact_cache()
    assert response.status_code == 200
    metadata = response.json()["metadata"]
    assert metadata["shadow_mode"] == "heuristic_baseline"
    assert metadata["shadow_score"] == 100
    assert metadata["shadow_delta"] == -18
    assert metadata["shadow_status"] == "watch"
