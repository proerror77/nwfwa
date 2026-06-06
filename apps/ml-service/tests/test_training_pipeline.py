import json
import math
from pathlib import Path

import pandas as pd
from fastapi.testclient import TestClient

from app.main import app
from app.schemas import TrainRequest
from app.training import train_from_manifest
from app.training_jobs import TrainingJobStore


client = TestClient(app)


def write_split(path: Path, rows: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    pd.DataFrame(rows).to_parquet(path, index=False)


def write_training_manifest(tmp_path: Path) -> Path:
    dataset_root = tmp_path / "dataset"
    common_rows = [
        {
            "claim_id": "CLM-1",
            "member_id": "MBR-1",
            "policy_id": "POL-1",
            "provider_id": "PRV-1",
            "service_date_ord": 1,
            "claim_amount_to_limit_ratio": 0.12,
            "provider_profile_score": 10.0,
            "high_cost_item_ratio": 0.0,
            "confirmed_fwa": 0,
        },
        {
            "claim_id": "CLM-2",
            "member_id": "MBR-2",
            "policy_id": "POL-2",
            "provider_id": "PRV-2",
            "service_date_ord": 2,
            "claim_amount_to_limit_ratio": 0.88,
            "provider_profile_score": 80.0,
            "high_cost_item_ratio": 1.0,
            "confirmed_fwa": 1,
        },
        {
            "claim_id": "CLM-3",
            "member_id": "MBR-3",
            "policy_id": "POL-3",
            "provider_id": "PRV-3",
            "service_date_ord": 3,
            "claim_amount_to_limit_ratio": 0.2,
            "provider_profile_score": 15.0,
            "high_cost_item_ratio": 0.0,
            "confirmed_fwa": 0,
        },
        {
            "claim_id": "CLM-4",
            "member_id": "MBR-4",
            "policy_id": "POL-4",
            "provider_id": "PRV-4",
            "service_date_ord": 4,
            "claim_amount_to_limit_ratio": 0.95,
            "provider_profile_score": 85.0,
            "high_cost_item_ratio": 1.0,
            "confirmed_fwa": 1,
        },
    ]
    write_split(dataset_root / "train.parquet", common_rows)
    write_split(
        dataset_root / "validation.parquet",
        [
            {**common_rows[0], "claim_id": "CLM-5", "member_id": "MBR-5", "policy_id": "POL-5", "provider_id": "PRV-5"},
            {**common_rows[1], "claim_id": "CLM-6", "member_id": "MBR-6", "policy_id": "POL-6", "provider_id": "PRV-6"},
        ],
    )
    write_split(
        dataset_root / "out_of_time.parquet",
        [
            {**common_rows[2], "claim_id": "CLM-7", "member_id": "MBR-7", "policy_id": "POL-7", "provider_id": "PRV-7"},
            {**common_rows[3], "claim_id": "CLM-8", "member_id": "MBR-8", "policy_id": "POL-8", "provider_id": "PRV-8"},
        ],
    )
    manifest = {
        "dataset_key": "claims_model",
        "dataset_version": "2026-06-02",
        "dataset_usage_scope": "pilot_validated",
        "pilot_validation_status": "passed",
        "label_column": "confirmed_fwa",
        "entity_keys": ["claim_id", "member_id", "policy_id", "provider_id"],
        "time_split_field": "service_date_ord",
        "group_split_fields": ["member_id", "policy_id", "provider_id"],
        "splits": [
            {"split_name": "train", "data_uri": "train.parquet"},
            {"split_name": "validation", "data_uri": "validation.parquet"},
            {"split_name": "out_of_time", "data_uri": "out_of_time.parquet"},
        ],
    }
    manifest_path = dataset_root / "manifest.json"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    return manifest_path


def test_training_pipeline_writes_artifacts_and_validation_payload(tmp_path: Path):
    manifest_path = write_training_manifest(tmp_path)

    payload = train_from_manifest(
        manifest_path=manifest_path,
        artifact_base_uri=tmp_path / "artifacts",
        model_key="baseline_fwa",
        base_model_version="0.1.0",
        job_id="model_retraining_job_1",
        actor="trainer-worker",
    )

    assert payload["candidate_model_version"] == "0.1.0-candidate-model_retraining_job_1"
    assert payload["artifact_uri"].endswith("/rust_serving_artifact.json")
    assert payload["training_artifact_uri"].endswith("/model.joblib")
    assert payload["validation_report_uri"].endswith("/validation.json")
    assert payload["feature_importance_uri"].endswith("/feature_importance.parquet")
    assert payload["permutation_importance_uri"].endswith("/permutation_importance.parquet")
    assert payload["serving_manifest_uri"].endswith("/serving_manifest.json")
    assert payload["model_artifact_evaluation_report_uri"].endswith(
        "/artifact-evaluation/model_artifact_evaluation_report.json"
    )
    assert payload["feature_store_manifest_uri"].endswith("/feature_store_manifest.json")
    assert payload["shadow_report_uri"].endswith("/shadow_report.json")
    assert payload["drift_report_uri"].endswith("/drift_report.json")
    assert payload["fairness_report_uri"].endswith("/fairness_report.json")
    assert Path(payload["artifact_uri"]).exists()
    assert Path(payload["training_artifact_uri"]).exists()
    assert Path(payload["validation_report_uri"]).exists()
    assert Path(payload["feature_importance_uri"]).exists()
    assert Path(payload["permutation_importance_uri"]).exists()
    assert Path(payload["serving_manifest_uri"]).exists()
    assert Path(payload["model_artifact_evaluation_report_uri"]).exists()
    assert Path(payload["feature_store_manifest_uri"]).exists()
    assert Path(payload["shadow_report_uri"]).exists()
    assert Path(payload["drift_report_uri"]).exists()
    assert Path(payload["fairness_report_uri"]).exists()
    assert payload["artifact_sha256"].startswith("sha256:")
    assert payload["artifact_signature"].startswith("hmac-sha256:")
    assert payload["metrics_json"]["runtime_kind"] == "rust_logistic_regression"
    assert payload["metrics_json"]["algorithm"] == "logistic_regression"
    assert payload["metrics_json"]["algorithm_family"] == "linear_baseline"
    assert payload["metrics_json"]["training_artifact_uri"].endswith("/model.joblib")
    assert payload["metrics_json"]["time_group_split_status"] == "passed"
    assert payload["metrics_json"]["leakage_check_status"] == "passed"
    assert payload["metrics_json"]["shadow_comparison_status"] == "passed"
    assert payload["metrics_json"]["serving_version_lock_status"] == "passed"
    assert payload["metrics_json"]["artifact_integrity_status"] == "passed"
    assert payload["metrics_json"]["feature_store_materialization_status"] == "passed"
    assert payload["metrics_json"]["rust_feature_set_status"] == "passed"
    assert payload["metrics_json"]["rust_feature_set_manifest_uri"].endswith(
        "/rust_feature_set/feature_set_manifest.json"
    )
    assert payload["metrics_json"]["model_artifact_evaluation_status"] == "passed"
    assert payload["metrics_json"]["model_artifact_evaluation_report_uri"].endswith(
        "/artifact-evaluation/model_artifact_evaluation_report.json"
    )
    assert payload["metrics_json"]["rust_serving_status"] == "passed"
    assert payload["metrics_json"]["rust_serving_latency_status"] == "passed"
    assert payload["metrics_json"]["rust_serving_p95_latency_ms"] == 18
    assert payload["metrics_json"]["segment_fairness_status"] == "passed"
    assert payload["metrics_json"]["score_psi"] is not None
    assert payload["metrics_json"]["label_provenance_status"] == "passed"
    assert payload["metrics_json"]["data_quality_score"] == 1.0
    assert payload["metrics_json"]["source_data_quality_score"] == 1.0
    assert payload["metrics_json"]["permutation_importance_status"] == "passed"
    assert payload["metrics_json"]["permutation_importance_uri"].endswith(
        "/permutation_importance.parquet"
    )
    assert payload["metrics_json"]["dataset_usage_scope"] == "pilot_validated"
    assert payload["metrics_json"]["pilot_validation_status"] == "passed"
    assert payload["auc"] is not None
    assert payload["precision"] is not None
    assert payload["recall"] is not None
    assert payload["threshold"] == "0.5000"

    serving_manifest = json.loads(
        Path(payload["serving_manifest_uri"]).read_text(encoding="utf-8")
    )
    rust_artifact = json.loads(Path(payload["artifact_uri"]).read_text(encoding="utf-8"))
    assert rust_artifact["runtime_kind"] == "rust_logistic_regression"
    assert rust_artifact["model_key"] == "baseline_fwa"
    assert rust_artifact["model_version"] == payload["candidate_model_version"]
    assert rust_artifact["feature_columns"] == [
        "service_date_ord",
        "claim_amount_to_limit_ratio",
        "provider_profile_score",
        "high_cost_item_ratio",
    ]
    assert isinstance(rust_artifact["intercept"], float)
    assert set(rust_artifact["coefficients"]) == set(rust_artifact["feature_columns"])
    assert serving_manifest["model_version"] == payload["candidate_model_version"]
    assert serving_manifest["artifact_sha256"] == payload["artifact_sha256"]
    assert serving_manifest["artifact_signature"] == payload["artifact_signature"]
    assert serving_manifest["version_lock"] == payload["candidate_model_version"]
    assert serving_manifest["runtime_kind"] == "rust_logistic_regression"
    assert serving_manifest["artifact_uri"] == payload["artifact_uri"]
    assert serving_manifest["training_artifact_uri"] == payload["training_artifact_uri"]

    feature_store_manifest = json.loads(
        Path(payload["feature_store_manifest_uri"]).read_text(encoding="utf-8")
    )
    assert feature_store_manifest["materialization_status"] == "materialized"
    assert feature_store_manifest["feature_columns"] == [
        "service_date_ord",
        "claim_amount_to_limit_ratio",
        "provider_profile_score",
        "high_cost_item_ratio",
    ]
    assert feature_store_manifest["split_row_counts"]["train"] == 4
    rust_feature_set_manifest = json.loads(
        Path(payload["metrics_json"]["rust_feature_set_manifest_uri"]).read_text(
            encoding="utf-8"
        )
    )
    assert rust_feature_set_manifest["feature_columns"] == feature_store_manifest[
        "feature_columns"
    ]
    assert rust_feature_set_manifest["split_row_counts"]["train"] == 4

    artifact_evaluation = json.loads(
        Path(payload["model_artifact_evaluation_report_uri"]).read_text(encoding="utf-8")
    )
    assert artifact_evaluation["report_kind"] == "model_artifact_evaluation"
    assert artifact_evaluation["gate_status"] == "passed"
    assert artifact_evaluation["runtime_kind"] == "rust_logistic_regression"
    assert artifact_evaluation["rust_serving_p95_latency_ms"] == 18

    permutation_importance = pd.read_parquet(payload["permutation_importance_uri"])
    assert set(permutation_importance["feature"]) == set(rust_artifact["feature_columns"])
    assert set(permutation_importance["importance_kind"]) == {"permutation_auc_drop"}
    assert (permutation_importance["importance"] >= 0.0).all()
    assert any(
        ref == f"model_permutation_importance:{payload['permutation_importance_uri']}"
        for ref in payload["evidence_refs"]
    )

    assert payload["mined_rule_owner"] == "external-training-platform"
    mined_rules = payload["mined_rule_candidates"]
    assert len(mined_rules) >= 1
    tree_rule = next(
        rule
        for rule in mined_rules
        if rule.get("metadata", {}).get("mining_algorithm") == "shallow_decision_tree"
    )
    assert tree_rule["rule_id"].startswith("candidate_tree_")
    assert len(tree_rule["conditions"]) >= 1
    assert tree_rule["conditions"][0]["operator"] in {"<=", ">="}
    assert tree_rule["action"]["recommended_action"] == "ManualReview"
    assert "shallow decision-tree path" in tree_rule["action"]["reason"]
    amount_rule = next(
        rule
        for rule in mined_rules
        if rule["conditions"][0]["field"] == "claim_amount_to_limit_ratio"
    )
    assert amount_rule["scheme_family"] == "high_risk_claim"
    assert amount_rule["action"]["recommended_action"] == "ManualReview"
    assert "negative-class mean + 1.5 standard deviations" in amount_rule["action"]["reason"]
    assert math.isclose(amount_rule["conditions"][0]["value"], 0.244853, rel_tol=1e-5)


def test_training_pipeline_writes_xgboost_candidate_payload(tmp_path: Path):
    manifest_path = write_training_manifest(tmp_path)

    payload = train_from_manifest(
        manifest_path=manifest_path,
        artifact_base_uri=tmp_path / "artifacts",
        model_key="baseline_fwa",
        base_model_version="0.1.0",
        job_id="model_retraining_job_1",
        actor="trainer-worker",
        algorithm="xgboost",
    )

    assert payload["candidate_model_version"] == "0.1.0-xgboost-candidate-model_retraining_job_1"
    assert payload["artifact_uri"].endswith("/model.onnx")
    assert payload["training_artifact_uri"].endswith("/model.joblib")
    assert not (Path(payload["artifact_uri"]).parent / "rust_serving_artifact.json").exists()
    assert payload["metrics_json"]["algorithm"] == "xgboost"
    assert payload["metrics_json"]["algorithm_family"] == "gradient_boosted_tree"
    assert payload["metrics_json"]["runtime_kind"] == "xgboost_onnx"
    assert payload["metrics_json"]["python_runtime_kind"] == "xgboost_classifier"
    assert payload["metrics_json"]["model_artifact_evaluation_status"] == "passed"
    assert payload["metrics_json"]["rust_serving_status"] == "passed"
    assert payload["metrics_json"]["rust_serving_latency_status"] == "passed"
    assert payload["metrics_json"]["rust_serving_p95_latency_ms"] == 24
    assert payload["metrics_json"]["onnx_export_status"] == "exported"
    assert payload["metrics_json"]["onnx_parity_status"] == "passed"
    assert payload["metrics_json"]["permutation_importance_status"] == "passed"
    assert Path(payload["permutation_importance_uri"]).exists()
    assert (
        payload["metrics_json"]["rust_serving_gate_status"]
        == "onnx_export_parity_and_rust_runtime_ready"
    )
    assert Path(payload["feature_importance_uri"]).exists()
    assert Path(payload["artifact_uri"]).exists()
    assert Path(payload["training_artifact_uri"]).exists()
    assert Path(payload["model_artifact_evaluation_report_uri"]).exists()
    assert Path(payload["onnx_parity_report_uri"]).exists()

    serving_manifest = json.loads(
        Path(payload["serving_manifest_uri"]).read_text(encoding="utf-8")
    )
    assert serving_manifest["runtime_kind"] == "xgboost_onnx"
    assert serving_manifest["artifact_uri"] == payload["artifact_uri"]
    assert serving_manifest["training_artifact_uri"] == payload["training_artifact_uri"]

    parity_report = json.loads(
        Path(payload["onnx_parity_report_uri"]).read_text(encoding="utf-8")
    )
    assert parity_report["status"] == "passed"
    assert parity_report["serving_runtime_kind"] == "xgboost_onnx"
    assert parity_report["max_abs_probability_delta"] <= parity_report["tolerance"]

    feature_importance = pd.read_parquet(payload["feature_importance_uri"])
    assert set(feature_importance["feature"]) == {
        "service_date_ord",
        "claim_amount_to_limit_ratio",
        "provider_profile_score",
        "high_cost_item_ratio",
    }
    assert set(feature_importance["importance_kind"]) == {"feature_importance"}


def test_training_pipeline_writes_lightgbm_candidate_payload(tmp_path: Path):
    manifest_path = write_training_manifest(tmp_path)

    payload = train_from_manifest(
        manifest_path=manifest_path,
        artifact_base_uri=tmp_path / "artifacts",
        model_key="baseline_fwa",
        base_model_version="0.1.0",
        job_id="model_retraining_job_1",
        actor="trainer-worker",
        algorithm="lightgbm",
    )

    assert payload["candidate_model_version"] == "0.1.0-lightgbm-candidate-model_retraining_job_1"
    assert payload["artifact_uri"].endswith("/model.onnx")
    assert payload["training_artifact_uri"].endswith("/model.joblib")
    assert not (Path(payload["artifact_uri"]).parent / "rust_serving_artifact.json").exists()
    assert payload["metrics_json"]["algorithm"] == "lightgbm"
    assert payload["metrics_json"]["algorithm_family"] == "gradient_boosted_tree"
    assert payload["metrics_json"]["runtime_kind"] == "lightgbm_onnx"
    assert payload["metrics_json"]["python_runtime_kind"] == "lightgbm_classifier"
    assert payload["metrics_json"]["model_artifact_evaluation_status"] == "passed"
    assert payload["metrics_json"]["rust_serving_status"] == "passed"
    assert payload["metrics_json"]["rust_serving_latency_status"] == "passed"
    assert payload["metrics_json"]["rust_serving_p95_latency_ms"] == 24
    assert payload["metrics_json"]["onnx_export_status"] == "exported"
    assert payload["metrics_json"]["onnx_parity_status"] == "passed"
    assert payload["metrics_json"]["permutation_importance_status"] == "passed"
    assert Path(payload["permutation_importance_uri"]).exists()
    assert (
        payload["metrics_json"]["rust_serving_gate_status"]
        == "onnx_export_parity_and_rust_runtime_ready"
    )
    assert Path(payload["feature_importance_uri"]).exists()
    assert Path(payload["artifact_uri"]).exists()
    assert Path(payload["training_artifact_uri"]).exists()
    assert Path(payload["model_artifact_evaluation_report_uri"]).exists()
    assert Path(payload["onnx_parity_report_uri"]).exists()

    serving_manifest = json.loads(
        Path(payload["serving_manifest_uri"]).read_text(encoding="utf-8")
    )
    assert serving_manifest["runtime_kind"] == "lightgbm_onnx"
    assert serving_manifest["artifact_uri"] == payload["artifact_uri"]
    assert serving_manifest["training_artifact_uri"] == payload["training_artifact_uri"]

    parity_report = json.loads(
        Path(payload["onnx_parity_report_uri"]).read_text(encoding="utf-8")
    )
    assert parity_report["status"] == "passed"
    assert parity_report["serving_runtime_kind"] == "lightgbm_onnx"
    assert parity_report["max_abs_probability_delta"] <= parity_report["tolerance"]

    feature_importance = pd.read_parquet(payload["feature_importance_uri"])
    assert set(feature_importance["feature"]) == {
        "service_date_ord",
        "claim_amount_to_limit_ratio",
        "provider_profile_score",
        "high_cost_item_ratio",
    }
    assert set(feature_importance["importance_kind"]) == {"feature_importance"}


def test_training_api_returns_completed_training_package(tmp_path: Path):
    manifest_path = write_training_manifest(tmp_path)

    response = client.post(
        "/train",
        json={
            "manifest_path": str(manifest_path),
            "artifact_base_uri": str(tmp_path / "artifacts"),
            "model_key": "baseline_fwa",
            "base_model_version": "0.1.0",
            "job_id": "model_retraining_job_1",
            "actor": "trainer-worker",
            "algorithm": "logistic_regression",
        },
    )

    assert response.status_code == 200
    payload = response.json()
    assert payload["artifact_uri"].endswith("/rust_serving_artifact.json")
    assert payload["serving_manifest_uri"].endswith("/serving_manifest.json")
    assert payload["metrics_json"]["rule_mining_status"] == "passed"
    assert payload["mined_rule_owner"] == "external-training-platform"
    assert payload["mined_rule_candidates"][0]["scheme_family"] == "high_risk_claim"


def test_training_job_api_stores_completed_provider_output(tmp_path: Path):
    manifest_path = write_training_manifest(tmp_path)
    db_path = tmp_path / "training_jobs.sqlite3"
    app.state.training_job_store = TrainingJobStore(db_path)

    response = client.post(
        "/training-jobs",
        json={
            "manifest_path": str(manifest_path),
            "artifact_base_uri": str(tmp_path / "artifacts"),
            "model_key": "baseline_fwa",
            "base_model_version": "0.1.0",
            "job_id": "model_retraining_job_1",
            "actor": "external-training-platform",
            "algorithm": "logistic_regression",
        },
    )

    assert response.status_code == 202
    queued = response.json()
    assert queued["job_id"] == "model_retraining_job_1"
    assert queued["status"] == "queued"
    assert queued["handoff_kind"] == "external_training_platform_job"
    assert queued["provider_output"] is None
    assert queued["submit_path"] == "/api/v1/ops/model-retraining-jobs/model_retraining_job_1/output"

    status_response = client.get("/training-jobs/model_retraining_job_1")

    assert status_response.status_code == 200
    stored = status_response.json()
    assert stored["status"] == "completed"
    assert stored["provider_output"]["candidate_model_version"] == (
        "0.1.0-candidate-model_retraining_job_1"
    )
    assert stored["provider_output"]["mined_rule_owner"] == "external-training-platform"
    assert stored["provider_output"]["artifact_registry_uri"].endswith(
        "/artifact_registry.json"
    )
    assert any(
        ref == f"model_artifact_registries:{stored['provider_output']['artifact_registry_uri']}"
        for ref in stored["provider_output"]["evidence_refs"]
    )
    assert stored["artifact_registry"]["registry_kind"] == "training_artifact_registry"
    assert stored["artifact_registry"]["model_key"] == "baseline_fwa"
    assert stored["artifact_registry"]["base_model_version"] == "0.1.0"
    assert stored["artifact_registry"]["candidate_model_version"] == (
        "0.1.0-candidate-model_retraining_job_1"
    )
    assert {
        artifact["artifact_kind"] for artifact in stored["artifact_registry"]["artifacts"]
    } >= {
        "serving_model",
        "training_model",
        "serving_manifest",
        "validation_report",
        "model_artifact_evaluation",
        "permutation_importance",
        "shadow_report",
        "drift_report",
        "fairness_report",
        "mined_rule_candidates",
    }
    assert Path(stored["provider_output"]["artifact_registry_uri"]).exists()
    assert stored["governance_boundary"].startswith("training platform owns training execution")

    app.state.training_job_store = TrainingJobStore(db_path)
    durable_response = client.get("/training-jobs/model_retraining_job_1")

    assert durable_response.status_code == 200
    assert durable_response.json() == stored


def test_training_job_api_persists_failed_training_job(tmp_path: Path):
    db_path = tmp_path / "training_jobs.sqlite3"
    app.state.training_job_store = TrainingJobStore(db_path)

    response = client.post(
        "/training-jobs",
        json={
            "manifest_path": str(tmp_path / "missing_manifest.json"),
            "artifact_base_uri": str(tmp_path / "artifacts"),
            "model_key": "baseline_fwa",
            "base_model_version": "0.1.0",
            "job_id": "failed_training_job",
            "actor": "external-training-platform",
            "algorithm": "logistic_regression",
            "max_attempts": 1,
        },
    )

    assert response.status_code == 202
    status_response = client.get("/training-jobs/failed_training_job")

    assert status_response.status_code == 200
    failed = status_response.json()
    assert failed["status"] == "failed"
    assert failed["provider_output"] is None
    assert failed["error"]["code"] == "TRAINING_FAILED"
    assert "missing_manifest.json" in failed["error"]["message"]


def test_training_job_retry_and_claim_lease_are_durable(tmp_path: Path):
    db_path = tmp_path / "training_jobs.sqlite3"
    store = TrainingJobStore(db_path)
    request = TrainRequest(
        manifest_path=str(tmp_path / "missing_manifest.json"),
        artifact_base_uri=str(tmp_path / "artifacts"),
        model_key="baseline_fwa",
        base_model_version="0.1.0",
        job_id="retry_training_job",
        actor="external-training-platform",
        algorithm="logistic_regression",
        max_attempts=2,
    )
    store.create_queued(request)

    first_claim = store.claim_next(worker_id="worker-a", lease_seconds=-1)
    assert first_claim["status"] == "running"
    assert first_claim["worker_id"] == "worker-a"
    assert first_claim["attempt_count"] == 1

    reclaimed = store.claim_next(worker_id="worker-b", lease_seconds=900)
    assert reclaimed["job_id"] == "retry_training_job"
    assert reclaimed["worker_id"] == "worker-b"
    assert reclaimed["attempt_count"] == 2

    stale_completion = store.mark_completed(
        "retry_training_job",
        "worker-a",
        {
            "candidate_model_version": "0.1.0-candidate-retry_training_job",
            "metrics_json": {"algorithm": "logistic_regression"},
        },
        {"registry_kind": "training_artifact_registry"},
    )
    assert stale_completion is None

    exhausted = store.mark_failed(
        "retry_training_job",
        "worker-b",
        {"code": "TRAINING_FAILED", "message": "boom"},
    )
    assert exhausted["status"] == "failed"
    assert exhausted["error"]["message"] == "boom"

    store = TrainingJobStore(db_path)
    assert store.claim_next(worker_id="worker-c", lease_seconds=900) is None
    assert store.get("retry_training_job")["status"] == "failed"


def test_training_job_claim_run_and_artifact_registry_endpoint(tmp_path: Path):
    manifest_path = write_training_manifest(tmp_path)
    db_path = tmp_path / "training_jobs.sqlite3"
    store = TrainingJobStore(db_path)
    app.state.training_job_store = store
    store.create_queued(
        TrainRequest(
            manifest_path=str(manifest_path),
            artifact_base_uri=str(tmp_path / "artifacts"),
            model_key="baseline_fwa",
            base_model_version="0.1.0",
            job_id="claimed_training_job",
            actor="external-training-platform",
            algorithm="logistic_regression",
        )
    )

    claim_response = client.post(
        "/training-jobs/claim-next",
        json={"worker_id": "trainer-worker", "lease_seconds": 900},
    )
    assert claim_response.status_code == 200
    claimed = claim_response.json()
    assert claimed["job_id"] == "claimed_training_job"
    assert claimed["status"] == "running"
    assert claimed["worker_id"] == "trainer-worker"

    run_response = client.post(
        "/training-jobs/claimed_training_job/run",
        json={"worker_id": "trainer-worker", "lease_seconds": 900},
    )
    assert run_response.status_code == 200
    completed = run_response.json()
    assert completed["status"] == "completed"
    assert completed["provider_output"]["candidate_model_version"] == (
        "0.1.0-candidate-claimed_training_job"
    )

    artifacts_response = client.get("/training-jobs/claimed_training_job/artifacts")
    assert artifacts_response.status_code == 200
    artifact_registry = artifacts_response.json()["artifact_registry"]
    assert artifact_registry["job_id"] == "claimed_training_job"
    assert artifact_registry["artifact_registry_uri"].endswith("/artifact_registry.json")
