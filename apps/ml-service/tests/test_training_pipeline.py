import json
import math
from pathlib import Path

import pandas as pd

from app.training import train_from_manifest
from training_fixtures import (
    BASE_FEATURES,
    write_imbalanced_training_manifest,
    write_training_manifest,
)


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
    assert payload["automl_feature_search_report_uri"].endswith(
        "/automl_feature_search_report.json"
    )
    assert payload["automl_factor_ranking_report_uri"].endswith(
        "/automl_factor_ranking_report.json"
    )
    assert payload["overfitting_diagnostics_report_uri"].endswith(
        "/overfitting_diagnostics_report.json"
    )
    assert payload["shadow_report_uri"].endswith("/shadow_report.json")
    assert payload["drift_report_uri"].endswith("/drift_report.json")
    assert payload["fairness_report_uri"].endswith("/fairness_report.json")
    assert payload["metrics_json"]["rule_candidate_backtest_report_uri"].endswith(
        "/rule-candidates/backtest/rule_candidate_backtest_report.json"
    )
    assert payload["metrics_json"]["rule_candidate_review_tasks_uri"].endswith(
        "/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json"
    )
    assert Path(payload["artifact_uri"]).exists()
    assert Path(payload["training_artifact_uri"]).exists()
    assert Path(payload["validation_report_uri"]).exists()
    assert Path(payload["feature_importance_uri"]).exists()
    assert Path(payload["permutation_importance_uri"]).exists()
    assert Path(payload["serving_manifest_uri"]).exists()
    assert Path(payload["model_artifact_evaluation_report_uri"]).exists()
    assert Path(payload["feature_store_manifest_uri"]).exists()
    assert Path(payload["automl_feature_search_report_uri"]).exists()
    assert Path(payload["automl_factor_ranking_report_uri"]).exists()
    assert Path(payload["overfitting_diagnostics_report_uri"]).exists()
    assert Path(payload["shadow_report_uri"]).exists()
    assert Path(payload["drift_report_uri"]).exists()
    assert Path(payload["fairness_report_uri"]).exists()
    assert Path(payload["metrics_json"]["rule_candidate_backtest_report_uri"]).exists()
    assert Path(payload["metrics_json"]["rule_candidate_review_tasks_uri"]).exists()
    assert payload["artifact_sha256"].startswith("sha256:")
    assert payload["artifact_signature"].startswith("hmac-sha256:")
    assert payload["metrics_json"]["runtime_kind"] == "rust_logistic_regression"
    assert payload["metrics_json"]["algorithm"] == "logistic_regression"
    assert payload["metrics_json"]["algorithm_family"] == "linear_baseline"
    assert payload["metrics_json"]["training_artifact_uri"].endswith("/model.joblib")
    assert payload["metrics_json"]["time_group_split_status"] == "passed"
    assert payload["metrics_json"]["leakage_check_status"] == "passed"
    assert payload["metrics_json"]["out_of_time_validation_status"] == "passed"
    assert payload["metrics_json"]["overfitting_diagnostics_status"] == "passed"
    assert payload["metrics_json"]["overfitting_diagnostics_report_uri"].endswith(
        "/overfitting_diagnostics_report.json"
    )
    assert payload["metrics_json"]["shadow_comparison_status"] == "passed"
    assert payload["metrics_json"]["serving_version_lock_status"] == "passed"
    assert payload["metrics_json"]["artifact_integrity_status"] == "passed"
    assert payload["metrics_json"]["feature_store_materialization_status"] == "passed"
    assert payload["metrics_json"]["automl_feature_search_status"] == "passed"
    assert payload["metrics_json"]["automl_generated_feature_count"] >= 1
    assert payload["metrics_json"]["automl_selected_feature_count"] > len(BASE_FEATURES)
    assert payload["metrics_json"]["automl_factor_ranking_status"] == "passed"
    assert payload["metrics_json"]["automl_ranked_factor_count"] == payload["metrics_json"][
        "automl_selected_feature_count"
    ]
    assert payload["metrics_json"]["automl_factor_ranking_report_uri"].endswith(
        "/automl_factor_ranking_report.json"
    )
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
    assert payload["metrics_json"]["rust_serving_latency_measurement_kind"] == "simulated_fixture"
    assert payload["metrics_json"]["rust_serving_latency_sample_count"] == 0
    assert payload["metrics_json"]["segment_fairness_status"] == "passed"
    assert payload["metrics_json"]["score_psi"] is not None
    assert payload["metrics_json"]["max_feature_psi"] is not None
    assert payload["metrics_json"]["score_stability_status"] == "passed"
    assert payload["metrics_json"]["feature_stability_status"] == "passed"
    assert payload["metrics_json"]["label_provenance_status"] == "passed"
    assert payload["metrics_json"]["data_quality_score"] == 1.0
    assert payload["metrics_json"]["source_data_quality_score"] == 1.0
    assert payload["metrics_json"]["permutation_importance_status"] == "passed"
    assert payload["metrics_json"]["permutation_importance_uri"].endswith(
        "/permutation_importance.parquet"
    )
    assert payload["metrics_json"]["rule_candidate_backtest_status"] == "passed"
    assert (
        payload["metrics_json"]["rule_library_writeback_status"]
        == "blocked_pending_human_review_and_policy_governance_approval"
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
    assert BASE_FEATURES.issubset(set(rust_artifact["feature_columns"]))
    assert any(feature.startswith("automl__") for feature in rust_artifact["feature_columns"])
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
    assert BASE_FEATURES.issubset(set(feature_store_manifest["feature_columns"]))
    assert any(
        definition["feature"].startswith("automl__")
        for definition in feature_store_manifest["feature_definitions"]
    )
    feature_search_report = json.loads(
        Path(payload["automl_feature_search_report_uri"]).read_text(encoding="utf-8")
    )
    assert feature_search_report["report_kind"] == "automl_feature_search"
    assert feature_search_report["feature_engineering_policy"] == (
        "bounded_pairwise_difference_and_safe_ratio_generation"
    )
    assert feature_search_report["generated_feature_count"] >= 1
    assert feature_search_report["selected_feature_count"] == payload["metrics_json"][
        "automl_selected_feature_count"
    ]
    assert feature_search_report["ranked_features"]
    assert any(
        row["feature_kind"] == "generated"
        for row in feature_search_report["ranked_features"]
    )
    assert any(
        ref == f"automl_feature_search_reports:{payload['automl_feature_search_report_uri']}"
        for ref in payload["evidence_refs"]
    )
    factor_ranking_report = json.loads(
        Path(payload["automl_factor_ranking_report_uri"]).read_text(encoding="utf-8")
    )
    assert factor_ranking_report["report_kind"] == "automl_factor_ranking"
    assert factor_ranking_report["status"] == "passed"
    assert factor_ranking_report["ranked_factor_count"] == payload["metrics_json"][
        "automl_selected_feature_count"
    ]
    assert [row["rank"] for row in factor_ranking_report["ranked_factors"]] == list(
        range(1, factor_ranking_report["ranked_factor_count"] + 1)
    )
    assert any(
        ref == f"automl_factor_rankings:{payload['automl_factor_ranking_report_uri']}"
        for ref in payload["evidence_refs"]
    )
    overfitting_diagnostics = json.loads(
        Path(payload["overfitting_diagnostics_report_uri"]).read_text(encoding="utf-8")
    )
    assert overfitting_diagnostics["report_kind"] == "overfitting_diagnostics"
    assert overfitting_diagnostics["status"] == "passed"
    assert overfitting_diagnostics["time_group_split_status"] == "passed"
    assert overfitting_diagnostics["leakage_check_status"] == "passed"
    assert overfitting_diagnostics["permutation_importance_status"] == "passed"
    assert any(
        ref
        == f"model_overfitting_diagnostics:{payload['overfitting_diagnostics_report_uri']}"
        for ref in payload["evidence_refs"]
    )
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
    assert artifact_evaluation["rust_serving_latency_measurement_kind"] == "simulated_fixture"
    assert artifact_evaluation["rust_serving_latency_sample_count"] == 0

    permutation_importance = pd.read_parquet(payload["permutation_importance_uri"])
    assert set(permutation_importance["feature"]) == set(rust_artifact["feature_columns"])
    assert set(permutation_importance["importance_kind"]) == {"permutation_auc_drop"}
    assert (permutation_importance["importance"] >= 0.0).all()
    assert any(
        ref == f"model_feature_importance:{payload['feature_importance_uri']}"
        for ref in payload["evidence_refs"]
    )
    assert any(
        ref == f"model_permutation_importance:{payload['permutation_importance_uri']}"
        for ref in payload["evidence_refs"]
    )
    assert any(
        ref
        == f"rule_candidate_backtests:{payload['metrics_json']['rule_candidate_backtest_report_uri']}"
        for ref in payload["evidence_refs"]
    )
    assert any(
        ref
        == f"rule_candidate_review_tasks:{payload['metrics_json']['rule_candidate_review_tasks_uri']}"
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


def test_training_pipeline_reports_metrics_for_imbalanced_labels(tmp_path: Path):
    manifest_path = write_imbalanced_training_manifest(tmp_path)

    payload = train_from_manifest(
        manifest_path=manifest_path,
        artifact_base_uri=tmp_path / "artifacts",
        model_key="baseline_fwa",
        base_model_version="0.1.0",
        job_id="imbalanced_training_job",
        actor="trainer-worker",
    )

    confusion_matrix = payload["confusion_matrix_json"]
    assert confusion_matrix["tp"] + confusion_matrix["fn"] == 2
    assert confusion_matrix["tn"] + confusion_matrix["fp"] == 18
    assert float(payload["auc"]) >= 0.8
    assert payload["precision"] is not None
    assert payload["recall"] is not None
    assert payload["threshold"] == "0.5000"
    assert payload["metrics_json"]["out_of_time_validation_status"] == "passed"
    assert payload["metrics_json"]["overfitting_diagnostics_status"] == "passed"


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
    assert payload["metrics_json"]["rust_serving_latency_measurement_kind"] == "simulated_fixture"
    assert payload["metrics_json"]["rust_serving_latency_sample_count"] == 0
    assert payload["metrics_json"]["onnx_export_status"] == "exported"
    assert payload["metrics_json"]["onnx_parity_status"] == "passed"
    assert payload["metrics_json"]["automl_factor_ranking_status"] == "passed"
    assert payload["metrics_json"]["overfitting_diagnostics_status"] == "passed"
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
    assert BASE_FEATURES.issubset(set(feature_importance["feature"]))
    assert any(feature.startswith("automl__") for feature in feature_importance["feature"])
    assert set(feature_importance["importance_kind"]) == {"feature_importance"}


def test_training_pipeline_marks_group_overlap_as_leakage(tmp_path: Path):
    manifest_path = write_training_manifest(tmp_path)
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    validation_path = manifest_path.parent / manifest["splits"][1]["data_uri"]
    validation = pd.read_parquet(validation_path)
    validation.loc[0, "member_id"] = "MBR-1"
    validation.to_parquet(validation_path, index=False)

    payload = train_from_manifest(
        manifest_path=manifest_path,
        artifact_base_uri=tmp_path / "artifacts",
        model_key="baseline_fwa",
        base_model_version="0.1.0",
        job_id="model_retraining_job_1",
        actor="trainer-worker",
    )

    assert payload["metrics_json"]["overfitting_diagnostics_status"] == "failed"
    assert payload["metrics_json"]["leakage_check_status"] == "failed"
    overfitting_diagnostics = json.loads(
        Path(payload["overfitting_diagnostics_report_uri"]).read_text(encoding="utf-8")
    )
    assert overfitting_diagnostics["group_overlap_counts"]["member_id"]["validation"] == 1
    assert "leakage_check_status:failed" in overfitting_diagnostics["blocking_reasons"]


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
    assert payload["metrics_json"]["rust_serving_latency_measurement_kind"] == "simulated_fixture"
    assert payload["metrics_json"]["rust_serving_latency_sample_count"] == 0
    assert payload["metrics_json"]["onnx_export_status"] == "exported"
    assert payload["metrics_json"]["onnx_parity_status"] == "passed"
    assert payload["metrics_json"]["automl_factor_ranking_status"] == "passed"
    assert payload["metrics_json"]["overfitting_diagnostics_status"] == "passed"
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
    assert BASE_FEATURES.issubset(set(feature_importance["feature"]))
    assert any(feature.startswith("automl__") for feature in feature_importance["feature"])
    assert set(feature_importance["importance_kind"]) == {"feature_importance"}


def test_training_pipeline_writes_deep_learning_candidate_payload(tmp_path: Path):
    manifest_path = write_training_manifest(tmp_path)

    payload = train_from_manifest(
        manifest_path=manifest_path,
        artifact_base_uri=tmp_path / "artifacts",
        model_key="baseline_fwa",
        base_model_version="0.1.0",
        job_id="model_retraining_job_1",
        actor="trainer-worker",
        algorithm="deep_learning",
    )

    assert (
        payload["candidate_model_version"]
        == "0.1.0-deep_learning-candidate-model_retraining_job_1"
    )
    assert payload["artifact_uri"].endswith("/model.joblib")
    assert payload["training_artifact_uri"].endswith("/model.joblib")
    assert payload["onnx_parity_report_uri"] is None
    assert payload["metrics_json"]["algorithm"] == "deep_learning"
    assert payload["metrics_json"]["algorithm_family"] == "deep_learning"
    assert payload["metrics_json"]["runtime_kind"] == "deep_learning_sklearn_mlp"
    assert payload["metrics_json"]["python_runtime_kind"] == "sklearn_mlp_classifier"
    assert payload["metrics_json"]["onnx_export_status"] == "not_required"
    assert payload["metrics_json"]["onnx_parity_status"] == "not_required"
    assert payload["metrics_json"]["rust_serving_gate_status"] == "rust_native_artifact_ready"
    assert payload["metrics_json"]["model_artifact_evaluation_status"] == "passed"
    assert payload["metrics_json"]["automl_feature_search_status"] == "passed"
    assert payload["metrics_json"]["automl_factor_ranking_status"] == "passed"
    assert payload["metrics_json"]["overfitting_diagnostics_status"] == "passed"
    assert payload["metrics_json"]["permutation_importance_status"] == "passed"
    assert Path(payload["artifact_uri"]).exists()
    assert Path(payload["feature_importance_uri"]).exists()
    assert Path(payload["permutation_importance_uri"]).exists()
    assert Path(payload["automl_factor_ranking_report_uri"]).exists()
    assert Path(payload["overfitting_diagnostics_report_uri"]).exists()

    serving_manifest = json.loads(
        Path(payload["serving_manifest_uri"]).read_text(encoding="utf-8")
    )
    assert serving_manifest["runtime_kind"] == "deep_learning_sklearn_mlp"
    assert serving_manifest["artifact_uri"] == payload["artifact_uri"]

    feature_importance = pd.read_parquet(payload["feature_importance_uri"])
    assert BASE_FEATURES.issubset(set(feature_importance["feature"]))
    assert any(feature.startswith("automl__") for feature in feature_importance["feature"])
    assert set(feature_importance["importance_kind"]) == {"first_layer_weight_abs_mean"}

    factor_ranking = json.loads(
        Path(payload["automl_factor_ranking_report_uri"]).read_text(encoding="utf-8")
    )
    assert factor_ranking["status"] == "passed"
    assert factor_ranking["ranked_factor_count"] > len(BASE_FEATURES)
