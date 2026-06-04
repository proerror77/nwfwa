import json
from pathlib import Path

import pandas as pd

from app.training import train_from_manifest


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
    assert payload["serving_manifest_uri"].endswith("/serving_manifest.json")
    assert payload["feature_store_manifest_uri"].endswith("/feature_store_manifest.json")
    assert payload["shadow_report_uri"].endswith("/shadow_report.json")
    assert payload["drift_report_uri"].endswith("/drift_report.json")
    assert payload["fairness_report_uri"].endswith("/fairness_report.json")
    assert Path(payload["artifact_uri"]).exists()
    assert Path(payload["training_artifact_uri"]).exists()
    assert Path(payload["validation_report_uri"]).exists()
    assert Path(payload["feature_importance_uri"]).exists()
    assert Path(payload["serving_manifest_uri"]).exists()
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
    assert payload["metrics_json"]["segment_fairness_status"] == "passed"
    assert payload["metrics_json"]["score_psi"] is not None
    assert payload["metrics_json"]["label_provenance_status"] == "passed"
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
    assert payload["artifact_uri"].endswith("/model.joblib")
    assert payload["training_artifact_uri"] == payload["artifact_uri"]
    assert not (Path(payload["artifact_uri"]).parent / "rust_serving_artifact.json").exists()
    assert payload["metrics_json"]["algorithm"] == "xgboost"
    assert payload["metrics_json"]["algorithm_family"] == "gradient_boosted_tree"
    assert payload["metrics_json"]["runtime_kind"] == "xgboost_classifier"
    assert payload["metrics_json"]["python_runtime_kind"] == "xgboost_classifier"
    assert Path(payload["feature_importance_uri"]).exists()

    serving_manifest = json.loads(
        Path(payload["serving_manifest_uri"]).read_text(encoding="utf-8")
    )
    assert serving_manifest["runtime_kind"] == "xgboost_classifier"
    assert serving_manifest["artifact_uri"] == payload["artifact_uri"]
    assert serving_manifest["training_artifact_uri"] == payload["training_artifact_uri"]

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
    assert payload["artifact_uri"].endswith("/model.joblib")
    assert payload["training_artifact_uri"] == payload["artifact_uri"]
    assert not (Path(payload["artifact_uri"]).parent / "rust_serving_artifact.json").exists()
    assert payload["metrics_json"]["algorithm"] == "lightgbm"
    assert payload["metrics_json"]["algorithm_family"] == "gradient_boosted_tree"
    assert payload["metrics_json"]["runtime_kind"] == "lightgbm_classifier"
    assert payload["metrics_json"]["python_runtime_kind"] == "lightgbm_classifier"
    assert Path(payload["feature_importance_uri"]).exists()

    serving_manifest = json.loads(
        Path(payload["serving_manifest_uri"]).read_text(encoding="utf-8")
    )
    assert serving_manifest["runtime_kind"] == "lightgbm_classifier"
    assert serving_manifest["artifact_uri"] == payload["artifact_uri"]
    assert serving_manifest["training_artifact_uri"] == payload["training_artifact_uri"]

    feature_importance = pd.read_parquet(payload["feature_importance_uri"])
    assert set(feature_importance["feature"]) == {
        "service_date_ord",
        "claim_amount_to_limit_ratio",
        "provider_profile_score",
        "high_cost_item_ratio",
    }
    assert set(feature_importance["importance_kind"]) == {"feature_importance"}
