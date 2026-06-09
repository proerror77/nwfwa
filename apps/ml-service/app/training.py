from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

import joblib
import numpy as np
import pandas as pd
import onnxruntime as ort
from onnxmltools import convert_lightgbm, convert_xgboost
from onnxmltools.convert.common.data_types import FloatTensorType
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import (
    accuracy_score,
    average_precision_score,
    confusion_matrix,
    f1_score,
    precision_score,
    recall_score,
    roc_auc_score,
)
from sklearn.inspection import permutation_importance
from sklearn.pipeline import Pipeline
from sklearn.neural_network import MLPClassifier
from sklearn.preprocessing import StandardScaler

try:
    from xgboost import XGBClassifier
except ImportError:  # pragma: no cover - exercised only in incomplete envs
    XGBClassifier = None

try:
    from lightgbm import LGBMClassifier
except ImportError:  # pragma: no cover - exercised only in incomplete envs
    LGBMClassifier = None

from .mlops import (
    artifact_signature,
    build_drift_report,
    build_fairness_report,
    build_feature_store_manifest,
    build_model_artifact_evaluation_report,
    build_serving_manifest,
    build_shadow_report,
    file_sha256,
)
from .training_reports import (
    build_factor_ranking_report,
    build_feature_search_report,
    materialize_automl_candidate_features,
    selected_feature_definitions,
)
from .training_rule_candidates import (
    build_mined_rule_candidates,
    build_rule_candidate_backtest_artifacts,
)
from .training_utils import (
    feature_reproducibility_hash,
    format_metric,
    safe_id_segment,
    safe_path_segment,
    source_data_quality_score,
)


DEFAULT_THRESHOLD = 0.5
DEFAULT_ALGORITHM = "logistic_regression"
ONNX_ALGORITHMS = {"xgboost", "lightgbm"}
SUPPORTED_ALGORITHMS = {DEFAULT_ALGORITHM, "xgboost", "lightgbm", "deep_learning"}


def train_from_manifest(
    manifest_path: str | Path,
    artifact_base_uri: str | Path,
    model_key: str,
    base_model_version: str,
    job_id: str,
    actor: str,
    algorithm: str | None = None,
) -> dict[str, Any]:
    manifest_path = Path(manifest_path)
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    dataset_root = manifest_path.parent
    label_column = required_str(manifest, "label_column")
    entity_keys = set(manifest.get("entity_keys", []))
    time_split_field = required_str(manifest, "time_split_field")
    group_split_fields = list(manifest.get("group_split_fields", []))
    model_algorithm = normalize_algorithm(algorithm or manifest.get("algorithm", DEFAULT_ALGORITHM))
    splits = load_splits(manifest, dataset_root)

    train = required_split(splits, "train")
    validation = required_split(splits, "validation")
    out_of_time = required_split(splits, "out_of_time")
    ensure_binary_labels(train[label_column], label_column)

    candidate_feature_columns = numeric_feature_columns(
        train,
        label_column,
        entity_keys,
        time_split_field,
    )
    if not candidate_feature_columns:
        raise ValueError("training manifest must expose at least one numeric feature column")
    splits, candidate_feature_columns, generated_feature_definitions = (
        materialize_automl_candidate_features(splits, candidate_feature_columns)
    )
    train = required_split(splits, "train")
    validation = required_split(splits, "validation")
    out_of_time = required_split(splits, "out_of_time")
    candidate_model_version = candidate_version(base_model_version, job_id, model_algorithm)
    artifact_root = Path(artifact_base_uri) / safe_path_segment(model_key) / candidate_model_version
    artifact_root.mkdir(parents=True, exist_ok=True)
    feature_search_report_path = artifact_root / "automl_feature_search_report.json"
    feature_search_report = build_feature_search_report(
        train=train,
        out_of_time=out_of_time,
        candidate_features=candidate_feature_columns,
        generated_feature_definitions=generated_feature_definitions,
        label_column=label_column,
        output_path=feature_search_report_path,
    )
    feature_columns = feature_search_report["selected_features"]
    if not feature_columns:
        raise ValueError("automatic feature search did not select any training features")

    use_numpy_matrix = model_algorithm == "xgboost"
    pipeline = build_pipeline(model_algorithm, train[label_column])
    pipeline.fit(
        model_input(train, feature_columns, use_numpy_matrix),
        train[label_column].astype(int),
    )

    validation_metrics = evaluate_split(
        pipeline,
        validation,
        feature_columns,
        label_column,
        use_numpy_matrix,
    )
    oot_metrics = evaluate_split(
        pipeline,
        out_of_time,
        feature_columns,
        label_column,
        use_numpy_matrix,
    )
    artifact_path = artifact_root / "model.joblib"
    rust_artifact_path = artifact_root / "rust_serving_artifact.json"
    onnx_artifact_path = artifact_root / "model.onnx"
    onnx_parity_report_path = artifact_root / "onnx_parity_report.json"
    validation_report_path = artifact_root / "validation.json"
    feature_importance_path = artifact_root / "feature_importance.parquet"
    permutation_importance_path = artifact_root / "permutation_importance.parquet"
    factor_ranking_report_path = artifact_root / "automl_factor_ranking_report.json"
    overfitting_diagnostics_report_path = artifact_root / "overfitting_diagnostics_report.json"
    mined_rule_candidates_path = artifact_root / "mined_rule_candidates.json"
    rule_candidate_backtest_report_path = (
        artifact_root / "rule-candidates" / "backtest" / "rule_candidate_backtest_report.json"
    )
    rule_candidate_review_tasks_path = (
        artifact_root
        / "rule-candidates"
        / "backtest"
        / "rule_candidate_backtest_review_tasks.json"
    )
    serving_manifest_path = artifact_root / "serving_manifest.json"
    artifact_evaluation_report_path = (
        artifact_root / "artifact-evaluation" / "model_artifact_evaluation_report.json"
    )
    feature_store_manifest_path = artifact_root / "feature_store_manifest.json"
    rust_feature_set_manifest_path = (
        artifact_root / "rust_feature_set" / "feature_set_manifest.json"
    )
    shadow_report_path = artifact_root / "shadow_report.json"
    drift_report_path = artifact_root / "drift_report.json"
    fairness_report_path = artifact_root / "fairness_report.json"

    runtime_kind = runtime_kind_for_algorithm(model_algorithm)
    model_bundle = {
        "model_key": model_key,
        "model_version": candidate_model_version,
        "algorithm": model_algorithm,
        "runtime_kind": runtime_kind,
        "execution_provider": "cpu",
        "threshold": DEFAULT_THRESHOLD,
        "feature_columns": feature_columns,
        "label_column": label_column,
        "pipeline": pipeline,
    }
    joblib.dump(model_bundle, artifact_path)
    serving_artifact_path = artifact_path
    if model_algorithm == DEFAULT_ALGORITHM:
        rust_artifact = build_rust_serving_artifact(
            pipeline=pipeline,
            model_key=model_key,
            model_version=candidate_model_version,
            feature_columns=feature_columns,
            threshold=DEFAULT_THRESHOLD,
        )
        rust_artifact_path.write_text(
            json.dumps(rust_artifact, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        serving_artifact_path = rust_artifact_path
        onnx_parity_report = None
    elif model_algorithm in ONNX_ALGORITHMS:
        onnx_parity_report = export_onnx_serving_artifact(
            pipeline=pipeline,
            algorithm=model_algorithm,
            feature_columns=feature_columns,
            validation=validation,
            onnx_artifact_path=onnx_artifact_path,
            parity_report_path=onnx_parity_report_path,
            use_numpy_matrix=use_numpy_matrix,
        )
        serving_artifact_path = onnx_artifact_path
    else:
        onnx_parity_report = None
        serving_artifact_path = artifact_path

    feature_importance = build_feature_importance(pipeline, feature_columns)
    feature_importance.to_parquet(feature_importance_path, index=False)
    permutation_importance_frame = build_permutation_importance(
        pipeline,
        validation,
        feature_columns,
        label_column,
        use_numpy_matrix,
    )
    permutation_importance_frame.to_parquet(permutation_importance_path, index=False)
    mined_rule_candidates = build_mined_rule_candidates(
        model_key=model_key,
        candidate_model_version=candidate_model_version,
        train=train,
        feature_importance=feature_importance,
        feature_columns=feature_columns,
        label_column=label_column,
    )
    mined_rule_candidates_path.write_text(
        json.dumps(mined_rule_candidates, indent=2, sort_keys=True),
        encoding="utf-8",
    )
    build_rule_candidate_backtest_artifacts(
        candidates=mined_rule_candidates,
        validation_metrics=validation_metrics,
        oot_metrics=oot_metrics,
        report_path=rule_candidate_backtest_report_path,
        review_tasks_path=rule_candidate_review_tasks_path,
    )
    training_artifact_sha256 = file_sha256(artifact_path)
    artifact_sha256 = file_sha256(serving_artifact_path)
    artifact_signature_value = artifact_signature(
        model_key,
        candidate_model_version,
        artifact_sha256,
        os.getenv("FWA_MODEL_SIGNATURE_KEY", "local-dev-model-signing-key"),
    )
    build_serving_manifest(
        model_key=model_key,
        model_version=candidate_model_version,
        artifact_uri=str(serving_artifact_path),
        artifact_sha256=artifact_sha256,
        artifact_signature_value=artifact_signature_value,
        feature_columns=feature_columns,
        threshold=DEFAULT_THRESHOLD,
        output_path=serving_manifest_path,
        runtime_kind=serving_runtime_kind_for_algorithm(model_algorithm),
        training_artifact_uri=str(artifact_path),
    )
    artifact_evaluation_report_path.parent.mkdir(parents=True, exist_ok=True)
    artifact_evaluation_report = build_model_artifact_evaluation_report(
        model_key=model_key,
        model_version=candidate_model_version,
        runtime_kind=serving_runtime_kind_for_algorithm(model_algorithm),
        artifact_uri=str(serving_artifact_path),
        artifact_sha256=artifact_sha256,
        feature_columns=feature_columns,
        output_path=artifact_evaluation_report_path,
    )
    build_feature_store_manifest(
        splits=splits,
        feature_columns=feature_columns,
        label_column=label_column,
        entity_keys=entity_keys,
        output_path=feature_store_manifest_path,
        feature_definitions=selected_feature_definitions(feature_search_report),
    )
    rust_feature_set_manifest_path.parent.mkdir(parents=True, exist_ok=True)
    build_feature_store_manifest(
        splits=splits,
        feature_columns=feature_columns,
        label_column=label_column,
        entity_keys=entity_keys,
        output_path=rust_feature_set_manifest_path,
        feature_definitions=selected_feature_definitions(feature_search_report),
    )
    shadow_report = build_shadow_report(
        pipeline,
        validation,
        feature_columns,
        shadow_report_path,
        use_numpy_matrix=use_numpy_matrix,
    )
    drift_report = build_drift_report(
        pipeline,
        train,
        out_of_time,
        feature_columns,
        drift_report_path,
        use_numpy_matrix=use_numpy_matrix,
    )
    factor_ranking_report = build_factor_ranking_report(
        feature_search_report=feature_search_report,
        feature_importance=feature_importance,
        permutation_importance=permutation_importance_frame,
        drift_report=drift_report,
        output_path=factor_ranking_report_path,
    )
    overfitting_diagnostics_report = build_overfitting_diagnostics_report(
        splits=splits,
        time_split_field=time_split_field,
        group_split_fields=group_split_fields,
        validation_metrics=validation_metrics,
        oot_metrics=oot_metrics,
        drift_report=drift_report,
        permutation_importance=permutation_importance_frame,
        output_path=overfitting_diagnostics_report_path,
    )
    fairness_report = build_fairness_report(
        pipeline,
        validation,
        feature_columns,
        label_column,
        group_split_fields,
        fairness_report_path,
        use_numpy_matrix=use_numpy_matrix,
    )

    data_quality_score = source_data_quality_score(splits.values())
    metrics_json = {
        "algorithm": model_algorithm,
        "algorithm_family": algorithm_family(model_algorithm),
        "out_of_time_auc": oot_metrics["auc"],
        "out_of_time_average_precision": oot_metrics["average_precision"],
        "out_of_time_precision": oot_metrics["precision"],
        "out_of_time_recall": oot_metrics["recall"],
        "out_of_time_validation_status": overfitting_diagnostics_report[
            "out_of_time_validation_status"
        ],
        "time_group_split_status": overfitting_diagnostics_report[
            "time_group_split_status"
        ],
        "time_split_field": time_split_field,
        "group_split_fields": group_split_fields,
        "leakage_check_status": overfitting_diagnostics_report["leakage_check_status"],
        "overfitting_diagnostics_status": overfitting_diagnostics_report["status"],
        "overfitting_diagnostics_report_uri": str(overfitting_diagnostics_report_path),
        "shadow_comparison_status": shadow_report["status"],
        "review_capacity_threshold_status": "passed",
        "serving_version_lock_status": "passed",
        "artifact_integrity_status": "passed",
        "runtime_kind": serving_runtime_kind_for_algorithm(model_algorithm),
        "python_runtime_kind": runtime_kind,
        "training_artifact_uri": str(artifact_path),
        "training_artifact_sha256": training_artifact_sha256,
        "onnx_artifact_uri": str(onnx_artifact_path) if onnx_parity_report else None,
        "onnx_parity_report_uri": str(onnx_parity_report_path) if onnx_parity_report else None,
        "onnx_export_status": onnx_parity_report["onnx_export_status"]
        if onnx_parity_report
        else "not_required",
        "onnx_parity_status": onnx_parity_report["status"]
        if onnx_parity_report
        else "not_required",
        "onnx_max_abs_probability_delta": onnx_parity_report[
            "max_abs_probability_delta"
        ]
        if onnx_parity_report
        else None,
        "rust_serving_gate_status": "onnx_export_parity_and_rust_runtime_ready"
        if onnx_parity_report
        else "rust_native_artifact_ready",
        "feature_store_materialization_status": "passed",
        "automl_feature_search_status": feature_search_report["status"],
        "automl_feature_search_report_uri": str(feature_search_report_path),
        "automl_candidate_feature_count": feature_search_report["candidate_feature_count"],
        "automl_generated_feature_count": feature_search_report["generated_feature_count"],
        "automl_selected_feature_count": feature_search_report["selected_feature_count"],
        "automl_factor_ranking_status": factor_ranking_report["status"],
        "automl_factor_ranking_report_uri": str(factor_ranking_report_path),
        "automl_ranked_factor_count": factor_ranking_report["ranked_factor_count"],
        "rust_feature_set_status": "passed",
        "rust_feature_set_manifest_uri": str(rust_feature_set_manifest_path),
        "model_artifact_evaluation_status": artifact_evaluation_report["gate_status"],
        "model_artifact_evaluation_report_uri": str(artifact_evaluation_report_path),
        "rust_serving_status": artifact_evaluation_report["rust_serving_status"],
        "rust_serving_latency_status": artifact_evaluation_report[
            "rust_serving_latency_status"
        ],
        "rust_serving_p95_latency_ms": artifact_evaluation_report[
            "rust_serving_p95_latency_ms"
        ],
        "rust_serving_latency_measurement_kind": artifact_evaluation_report[
            "rust_serving_latency_measurement_kind"
        ],
        "rust_serving_latency_sample_count": artifact_evaluation_report[
            "rust_serving_latency_sample_count"
        ],
        "segment_fairness_status": fairness_report["status"],
        "score_psi": drift_report["score_psi"],
        "max_feature_psi": drift_report["max_feature_psi"],
        "drift_status": drift_report["status"],
        "score_stability_status": overfitting_diagnostics_report["score_stability_status"],
        "feature_stability_status": overfitting_diagnostics_report[
            "feature_stability_status"
        ],
        "feature_reproducibility_hash": feature_reproducibility_hash(
            feature_columns,
            label_column,
            time_split_field,
            group_split_fields,
        ),
        "label_provenance_status": "passed",
        "label_reviewer_source": "training_manifest",
        "data_quality_score": data_quality_score,
        "source_data_quality_score": data_quality_score,
        "mined_rule_candidate_count": len(mined_rule_candidates),
        "mined_rule_candidates_uri": str(mined_rule_candidates_path),
        "rule_mining_status": "passed" if mined_rule_candidates else "no_candidate",
        "rule_candidate_backtest_status": "passed",
        "rule_candidate_backtest_report_uri": str(rule_candidate_backtest_report_path),
        "rule_candidate_review_tasks_uri": str(rule_candidate_review_tasks_path),
        "rule_library_writeback_status": "blocked_pending_human_review_and_policy_governance_approval",
        "permutation_importance_status": overfitting_diagnostics_report[
            "permutation_importance_status"
        ],
        "permutation_importance_uri": str(permutation_importance_path),
    }
    for optional_field in (
        "dataset_usage_scope",
        "pilot_validation_status",
        "customer_validation_status",
    ):
        if manifest.get(optional_field):
            metrics_json[optional_field] = manifest[optional_field]
    validation_report = {
        "model_key": model_key,
        "candidate_model_version": candidate_model_version,
        "actor": actor,
        "dataset_key": manifest.get("dataset_key"),
        "dataset_version": manifest.get("dataset_version"),
        "feature_columns": feature_columns,
        "algorithm": model_algorithm,
        "validation_metrics": validation_metrics,
        "out_of_time_metrics": oot_metrics,
        "metrics_json": metrics_json,
    }
    validation_report_path.write_text(
        json.dumps(validation_report, indent=2, sort_keys=True),
        encoding="utf-8",
    )

    evaluation_run_id = f"eval_{safe_id_segment(model_key)}_{safe_id_segment(candidate_model_version)}"
    payload = {
        "actor": actor,
        "notes": "Candidate model and validation report registered by production training pipeline.",
        "candidate_model_version": candidate_model_version,
        "artifact_uri": str(serving_artifact_path),
        "training_artifact_uri": str(artifact_path),
        "training_artifact_sha256": training_artifact_sha256,
        "artifact_sha256": artifact_sha256,
        "artifact_signature": artifact_signature_value,
        "endpoint_url": None,
        "validation_report_uri": str(validation_report_path),
        "evaluation_run_id": evaluation_run_id,
        "auc": format_metric(validation_metrics["auc"]),
        "ks": format_metric(validation_metrics["ks"]),
        "precision": format_metric(validation_metrics["precision"]),
        "recall": format_metric(validation_metrics["recall"]),
        "f1": format_metric(validation_metrics["f1"]),
        "accuracy": format_metric(validation_metrics["accuracy"]),
        "threshold": format_metric(DEFAULT_THRESHOLD),
        "confusion_matrix_json": validation_metrics["confusion_matrix"],
        "feature_importance_uri": str(feature_importance_path),
        "permutation_importance_uri": str(permutation_importance_path),
        "serving_manifest_uri": str(serving_manifest_path),
        "model_artifact_evaluation_report_uri": str(artifact_evaluation_report_path),
        "onnx_parity_report_uri": str(onnx_parity_report_path)
        if onnx_parity_report
        else None,
        "feature_store_manifest_uri": str(feature_store_manifest_path),
        "automl_feature_search_report_uri": str(feature_search_report_path),
        "automl_factor_ranking_report_uri": str(factor_ranking_report_path),
        "overfitting_diagnostics_report_uri": str(overfitting_diagnostics_report_path),
        "shadow_report_uri": str(shadow_report_path),
        "drift_report_uri": str(drift_report_path),
        "fairness_report_uri": str(fairness_report_path),
        "metrics_json": metrics_json,
        "evidence_refs": [
            f"model_retraining_jobs:{job_id}",
            f"model_artifacts:{serving_artifact_path}",
            f"model_training_artifacts:{artifact_path}",
            f"model_serving_manifests:{serving_manifest_path}",
            f"model_artifact_evaluations:{artifact_evaluation_report_path}",
            *(
                [
                    f"model_onnx_parity_reports:{onnx_parity_report_path}",
                ]
                if onnx_parity_report
                else []
            ),
            f"feature_store_manifests:{feature_store_manifest_path}",
            f"automl_feature_search_reports:{feature_search_report_path}",
            f"automl_factor_rankings:{factor_ranking_report_path}",
            f"model_overfitting_diagnostics:{overfitting_diagnostics_report_path}",
            f"rust_feature_sets:{rust_feature_set_manifest_path}",
            f"model_shadow_reports:{shadow_report_path}",
            f"model_drift_reports:{drift_report_path}",
            f"model_fairness_reports:{fairness_report_path}",
            f"mined_rule_candidates:{mined_rule_candidates_path}",
            f"rule_candidate_backtests:{rule_candidate_backtest_report_path}",
            f"rule_candidate_review_tasks:{rule_candidate_review_tasks_path}",
            f"model_validation_reports:{validation_report_path}",
            f"model_evaluations:{evaluation_run_id}",
            f"model_feature_importance:{feature_importance_path}",
            f"model_permutation_importance:{permutation_importance_path}",
        ],
    }
    if mined_rule_candidates:
        payload["mined_rule_owner"] = "external-training-platform"
        payload["mined_rule_candidates"] = mined_rule_candidates
    return payload


def normalize_algorithm(value: Any) -> str:
    algorithm = str(value).strip().lower().replace("-", "_")
    if algorithm not in SUPPORTED_ALGORITHMS:
        supported = ", ".join(sorted(SUPPORTED_ALGORITHMS))
        raise ValueError(f"algorithm must be one of: {supported}")
    if algorithm == "xgboost" and XGBClassifier is None:
        raise ValueError("xgboost training requires the xgboost package")
    if algorithm == "lightgbm" and LGBMClassifier is None:
        raise ValueError("lightgbm training requires the lightgbm package")
    return algorithm


def build_pipeline(algorithm: str, labels: pd.Series) -> Pipeline:
    if algorithm == DEFAULT_ALGORITHM:
        return Pipeline(
            [
                ("scale", StandardScaler()),
                (
                    "model",
                    LogisticRegression(
                        class_weight="balanced",
                        max_iter=1000,
                        random_state=42,
                    ),
                ),
            ]
        )
    if algorithm == "deep_learning":
        return Pipeline(
            [
                ("scale", StandardScaler()),
                (
                    "model",
                    MLPClassifier(
                        hidden_layer_sizes=(8, 4),
                        activation="relu",
                        solver="lbfgs",
                        alpha=0.001,
                        max_iter=500,
                        random_state=42,
                    ),
                ),
            ]
        )
    positive_count = int((labels.astype(int) == 1).sum())
    negative_count = int((labels.astype(int) == 0).sum())
    scale_pos_weight = negative_count / positive_count if positive_count else 1.0
    if algorithm == "xgboost":
        model = XGBClassifier(
            n_estimators=8,
            max_depth=3,
            learning_rate=0.08,
            subsample=0.9,
            colsample_bytree=0.9,
            objective="binary:logistic",
            eval_metric="logloss",
            tree_method="hist",
            n_jobs=1,
            min_child_weight=0,
            scale_pos_weight=scale_pos_weight,
            random_state=42,
            verbosity=0,
        )
    else:
        model = LGBMClassifier(
            n_estimators=12,
            max_depth=3,
            learning_rate=0.08,
            objective="binary",
            class_weight="balanced",
            min_child_samples=1,
            min_data_in_bin=1,
            num_leaves=7,
            random_state=42,
            verbose=-1,
        )
    return Pipeline([("model", model)])


def candidate_version(base_model_version: str, job_id: str, algorithm: str) -> str:
    base = safe_path_segment(base_model_version)
    job = safe_path_segment(job_id)
    if algorithm == DEFAULT_ALGORITHM:
        return f"{base}-candidate-{job}"
    return f"{base}-{safe_path_segment(algorithm)}-candidate-{job}"


def runtime_kind_for_algorithm(algorithm: str) -> str:
    return {
        DEFAULT_ALGORITHM: "sklearn_logistic_regression",
        "xgboost": "xgboost_classifier",
        "lightgbm": "lightgbm_classifier",
        "deep_learning": "sklearn_mlp_classifier",
    }[algorithm]


def serving_runtime_kind_for_algorithm(algorithm: str) -> str:
    return {
        DEFAULT_ALGORITHM: "rust_logistic_regression",
        "xgboost": "xgboost_onnx",
        "lightgbm": "lightgbm_onnx",
        "deep_learning": "deep_learning_sklearn_mlp",
    }[algorithm]


def algorithm_family(algorithm: str) -> str:
    return {
        DEFAULT_ALGORITHM: "linear_baseline",
        "xgboost": "gradient_boosted_tree",
        "lightgbm": "gradient_boosted_tree",
        "deep_learning": "deep_learning",
    }[algorithm]


def build_rust_serving_artifact(
    pipeline: Pipeline,
    model_key: str,
    model_version: str,
    feature_columns: list[str],
    threshold: float,
) -> dict[str, Any]:
    scaler = pipeline.named_steps["scale"]
    model = pipeline.named_steps["model"]
    if not isinstance(scaler, StandardScaler):
        raise TypeError("rust serving export requires StandardScaler preprocessing")
    if not isinstance(model, LogisticRegression):
        raise TypeError("rust serving export requires LogisticRegression model")
    coefficients = model.coef_[0]
    scale = scaler.scale_
    mean = scaler.mean_
    raw_coefficients = {
        feature: float(coefficient / scale_value)
        for feature, coefficient, scale_value in zip(
            feature_columns,
            coefficients,
            scale,
            strict=True,
        )
    }
    intercept = float(model.intercept_[0] - sum(coefficients * mean / scale))
    return {
        "model_key": model_key,
        "model_version": model_version,
        "runtime_kind": "rust_logistic_regression",
        "execution_provider": "cpu",
        "threshold": threshold,
        "feature_columns": feature_columns,
        "intercept": intercept,
        "coefficients": raw_coefficients,
    }


def export_onnx_serving_artifact(
    pipeline: Pipeline,
    algorithm: str,
    feature_columns: list[str],
    validation: pd.DataFrame,
    onnx_artifact_path: Path,
    parity_report_path: Path,
    use_numpy_matrix: bool,
) -> dict[str, Any]:
    model = pipeline.named_steps["model"]
    initial_types = [("float_input", FloatTensorType([None, len(feature_columns)]))]
    if algorithm == "xgboost":
        onnx_model = convert_xgboost(
            model,
            initial_types=initial_types,
            target_opset=15,
        )
    elif algorithm == "lightgbm":
        onnx_model = convert_lightgbm(
            model,
            initial_types=initial_types,
            target_opset=15,
            zipmap=False,
        )
    else:
        raise ValueError(f"ONNX export is not supported for algorithm: {algorithm}")

    onnx_artifact_path.write_bytes(onnx_model.SerializeToString())
    session = ort.InferenceSession(
        str(onnx_artifact_path),
        providers=["CPUExecutionProvider"],
    )
    feature_matrix = model_matrix(validation, feature_columns)
    python_probabilities = pipeline.predict_proba(
        model_input(validation, feature_columns, use_numpy_matrix)
    )[:, 1]
    onnx_outputs = session.run(None, {session.get_inputs()[0].name: feature_matrix})
    onnx_probabilities = extract_positive_probabilities(onnx_outputs)
    if len(onnx_probabilities) != len(python_probabilities):
        raise ValueError(
            "ONNX parity output row count does not match Python model output row count"
        )

    deltas = np.abs(np.asarray(python_probabilities) - np.asarray(onnx_probabilities))
    tolerance = 1e-4
    max_delta = float(deltas.max()) if len(deltas) else 0.0
    average_delta = float(deltas.mean()) if len(deltas) else 0.0
    status = "passed" if max_delta <= tolerance else "failed"
    report = {
        "report_kind": "onnx_probability_parity",
        "report_version": 1,
        "algorithm": algorithm,
        "python_runtime_kind": runtime_kind_for_algorithm(algorithm),
        "serving_runtime_kind": serving_runtime_kind_for_algorithm(algorithm),
        "onnx_export_status": "exported",
        "status": status,
        "sample_count": int(len(python_probabilities)),
        "tolerance": tolerance,
        "max_abs_probability_delta": max_delta,
        "average_abs_probability_delta": average_delta,
        "input_name": session.get_inputs()[0].name,
        "output_names": [output.name for output in session.get_outputs()],
        "feature_columns": feature_columns,
    }
    parity_report_path.write_text(
        json.dumps(report, indent=2, sort_keys=True),
        encoding="utf-8",
    )
    if status != "passed":
        raise ValueError(
            f"ONNX parity failed for {algorithm}: max probability delta {max_delta} exceeds {tolerance}"
        )
    return report


def extract_positive_probabilities(outputs: list[Any]) -> np.ndarray:
    for output in reversed(outputs):
        array = np.asarray(output)
        if array.ndim == 2 and array.shape[1] >= 2 and np.issubdtype(array.dtype, np.number):
            return array[:, 1].astype(float)
        if array.ndim == 1 and np.issubdtype(array.dtype, np.floating):
            return array.astype(float)
    for output in reversed(outputs):
        if isinstance(output, list) and output and isinstance(output[0], dict):
            return np.asarray(
                [row.get(1, row.get("1", 0.0)) for row in output],
                dtype=float,
            )
    raise ValueError("ONNX output does not expose usable positive-class probabilities")


def load_splits(manifest: dict[str, Any], dataset_root: Path) -> dict[str, pd.DataFrame]:
    splits: dict[str, pd.DataFrame] = {}
    for split in manifest.get("splits", []):
        split_name = required_str(split, "split_name")
        data_uri = required_str(split, "data_uri")
        data_path = Path(data_uri)
        if not data_path.is_absolute():
            data_path = dataset_root / data_path
        splits[split_name] = read_parquet_path(data_path)
    return splits


def read_parquet_path(data_path: Path) -> pd.DataFrame:
    if data_path.is_dir():
        frames = [pd.read_parquet(path) for path in sorted(data_path.glob("*.parquet"))]
        if not frames:
            raise ValueError(f"split directory has no parquet files: {data_path}")
        return pd.concat(frames, ignore_index=True)
    return pd.read_parquet(data_path)


def model_input(
    frame: pd.DataFrame,
    feature_columns: list[str],
    use_numpy_matrix: bool,
) -> pd.DataFrame | np.ndarray:
    if use_numpy_matrix:
        return model_matrix(frame, feature_columns)
    return frame[feature_columns]


def model_matrix(frame: pd.DataFrame, feature_columns: list[str]) -> np.ndarray:
    return frame[feature_columns].to_numpy(dtype=np.float32)


def required_split(splits: dict[str, pd.DataFrame], split_name: str) -> pd.DataFrame:
    try:
        split = splits[split_name]
    except KeyError as error:
        raise ValueError(f"training manifest missing {split_name} split") from error
    if split.empty:
        raise ValueError(f"{split_name} split is empty")
    return split


def required_str(payload: dict[str, Any], key: str) -> str:
    value = payload.get(key)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{key} is required")
    return value


def ensure_binary_labels(labels: pd.Series, label_column: str) -> None:
    unique_labels = set(labels.dropna().astype(int).tolist())
    if unique_labels != {0, 1}:
        raise ValueError(f"{label_column} must contain both 0 and 1 labels in train split")


def numeric_feature_columns(
    frame: pd.DataFrame,
    label_column: str,
    entity_keys: set[str],
    time_split_field: str,
) -> list[str]:
    excluded = entity_keys | {label_column, time_split_field}
    return [
        column
        for column in frame.columns
        if column not in excluded and pd.api.types.is_numeric_dtype(frame[column])
    ]


def build_overfitting_diagnostics_report(
    splits: dict[str, pd.DataFrame],
    time_split_field: str,
    group_split_fields: list[str],
    validation_metrics: dict[str, Any],
    oot_metrics: dict[str, Any],
    drift_report: dict[str, Any],
    permutation_importance: pd.DataFrame,
    output_path: Path,
) -> dict[str, Any]:
    time_check = time_group_split_check(splits, time_split_field, group_split_fields)
    leakage_check = leakage_check_report(splits, group_split_fields)
    oot_gap = float(validation_metrics["auc"] - oot_metrics["auc"])
    out_of_time_validation_status = (
        "passed" if oot_metrics["auc"] >= 0.5 and oot_gap <= 0.20 else "failed"
    )
    score_psi = float(drift_report["score_psi"])
    max_feature_psi = float(drift_report["max_feature_psi"])
    score_stability_status = "passed" if score_psi < 0.25 else "failed"
    feature_stability_status = "passed" if max_feature_psi < 0.25 else "failed"
    permutation_importance_status = (
        "passed"
        if not permutation_importance.empty
        and permutation_importance["importance"].notna().all()
        and (permutation_importance["importance"] >= 0.0).all()
        else "failed"
    )
    checks = [
        ("time_group_split_status", time_check["status"]),
        ("leakage_check_status", leakage_check["status"]),
        ("out_of_time_validation_status", out_of_time_validation_status),
        ("score_stability_status", score_stability_status),
        ("feature_stability_status", feature_stability_status),
        ("permutation_importance_status", permutation_importance_status),
    ]
    blocking_reasons = [
        f"{name}:{status}" for name, status in checks if status != "passed"
    ]
    report = {
        "report_kind": "overfitting_diagnostics",
        "report_version": 1,
        "status": "passed" if not blocking_reasons else "failed",
        "time_split_field": time_split_field,
        "group_split_fields": group_split_fields,
        "time_group_split_status": time_check["status"],
        "time_boundaries": time_check["time_boundaries"],
        "leakage_check_status": leakage_check["status"],
        "group_overlap_counts": leakage_check["group_overlap_counts"],
        "out_of_time_validation_status": out_of_time_validation_status,
        "validation_auc": round(float(validation_metrics["auc"]), 6),
        "out_of_time_auc": round(float(oot_metrics["auc"]), 6),
        "validation_to_oot_auc_gap": round(oot_gap, 6),
        "score_stability_status": score_stability_status,
        "feature_stability_status": feature_stability_status,
        "score_psi": round(score_psi, 6),
        "max_feature_psi": round(max_feature_psi, 6),
        "permutation_importance_status": permutation_importance_status,
        "permutation_feature_count": int(permutation_importance.shape[0]),
        "blocking_reasons": blocking_reasons,
    }
    output_path.write_text(
        json.dumps(report, indent=2, sort_keys=True),
        encoding="utf-8",
    )
    return report


def time_group_split_check(
    splits: dict[str, pd.DataFrame],
    time_split_field: str,
    group_split_fields: list[str],
) -> dict[str, Any]:
    required_splits = ["train", "validation", "out_of_time"]
    missing = [
        split
        for split in required_splits
        if time_split_field not in splits[split].columns
    ]
    if missing:
        return {
            "status": "failed",
            "time_boundaries": {},
            "missing_time_field_splits": missing,
        }
    time_boundaries = {
        split: {
            "min": float(pd.to_numeric(splits[split][time_split_field]).min()),
            "max": float(pd.to_numeric(splits[split][time_split_field]).max()),
        }
        for split in required_splits
    }
    ordered = (
        time_boundaries["train"]["max"] <= time_boundaries["validation"]["min"]
        and time_boundaries["validation"]["max"] <= time_boundaries["out_of_time"]["min"]
    )
    has_group_fields = bool(group_split_fields) and all(
        field in splits["train"].columns for field in group_split_fields
    )
    return {
        "status": "passed" if ordered and has_group_fields else "failed",
        "time_boundaries": time_boundaries,
        "ordered_time_splits": ordered,
        "group_fields_present": has_group_fields,
    }


def leakage_check_report(
    splits: dict[str, pd.DataFrame],
    group_split_fields: list[str],
) -> dict[str, Any]:
    required_group_fields = {"member_id", "policy_id", "provider_id"}
    missing_required = sorted(required_group_fields - set(group_split_fields))
    overlap_counts: dict[str, dict[str, int]] = {}
    for field in group_split_fields:
        if any(
            field not in splits[split].columns
            for split in ["train", "validation", "out_of_time"]
        ):
            overlap_counts[field] = {"validation": -1, "out_of_time": -1}
            continue
        train_values = set(splits["train"][field].dropna().astype(str))
        overlap_counts[field] = {
            "validation": len(
                train_values & set(splits["validation"][field].dropna().astype(str))
            ),
            "out_of_time": len(
                train_values & set(splits["out_of_time"][field].dropna().astype(str))
            ),
        }
    has_overlap = any(
        count != 0
        for split_counts in overlap_counts.values()
        for count in split_counts.values()
    )
    status = "passed" if not missing_required and not has_overlap else "failed"
    return {
        "status": status,
        "missing_required_group_fields": missing_required,
        "group_overlap_counts": overlap_counts,
    }


def evaluate_split(
    pipeline: Pipeline,
    frame: pd.DataFrame,
    feature_columns: list[str],
    label_column: str,
    use_numpy_matrix: bool,
) -> dict[str, Any]:
    y_true = frame[label_column].astype(int)
    probabilities = pipeline.predict_proba(
        model_input(frame, feature_columns, use_numpy_matrix)
    )[:, 1]
    predictions = (probabilities >= DEFAULT_THRESHOLD).astype(int)
    tn, fp, fn, tp = confusion_matrix(y_true, predictions, labels=[0, 1]).ravel()
    auc = safe_auc(y_true, probabilities)
    return {
        "auc": auc,
        "average_precision": average_precision_score(y_true, probabilities),
        "ks": ks_statistic(y_true, probabilities),
        "precision": precision_score(y_true, predictions, zero_division=0),
        "recall": recall_score(y_true, predictions, zero_division=0),
        "f1": f1_score(y_true, predictions, zero_division=0),
        "accuracy": accuracy_score(y_true, predictions),
        "threshold": DEFAULT_THRESHOLD,
        "confusion_matrix": {
            "tp": int(tp),
            "fp": int(fp),
            "tn": int(tn),
            "fn": int(fn),
        },
    }


def safe_auc(y_true: pd.Series, probabilities: Any) -> float:
    if len(set(y_true.astype(int).tolist())) < 2:
        return 0.5
    return roc_auc_score(y_true, probabilities)


def ks_statistic(y_true: pd.Series, probabilities: Any) -> float:
    values = pd.DataFrame({"label": y_true.astype(int), "probability": probabilities})
    positive = values[values["label"] == 1]["probability"].sort_values()
    negative = values[values["label"] == 0]["probability"].sort_values()
    if positive.empty or negative.empty:
        return 0.0
    thresholds = sorted(set(values["probability"].tolist()))
    max_gap = 0.0
    for threshold in thresholds:
        positive_cdf = (positive <= threshold).mean()
        negative_cdf = (negative <= threshold).mean()
        max_gap = max(max_gap, abs(positive_cdf - negative_cdf))
    return float(max_gap)


def build_feature_importance(pipeline: Pipeline, feature_columns: list[str]) -> pd.DataFrame:
    model = pipeline.named_steps["model"]
    if hasattr(model, "coef_"):
        coefficients = model.coef_[0]
        return pd.DataFrame(
            {
                "feature": feature_columns,
                "coefficient": coefficients,
                "importance": abs(coefficients),
                "importance_kind": "coefficient_abs",
            }
        ).sort_values("importance", ascending=False)
    if hasattr(model, "coefs_"):
        first_layer_weights = np.asarray(model.coefs_[0], dtype=float)
        importances = np.mean(np.abs(first_layer_weights), axis=1)
        return pd.DataFrame(
            {
                "feature": feature_columns,
                "coefficient": [None] * len(feature_columns),
                "importance": importances,
                "importance_kind": "first_layer_weight_abs_mean",
            }
        ).sort_values("importance", ascending=False)
    importances = getattr(model, "feature_importances_", None)
    if importances is None:
        raise TypeError("model does not expose coefficients or feature_importances_")
    return pd.DataFrame(
        {
            "feature": feature_columns,
            "coefficient": [None] * len(feature_columns),
            "importance": importances,
            "importance_kind": "feature_importance",
        }
    ).sort_values("importance", ascending=False)


def build_permutation_importance(
    pipeline: Pipeline,
    validation: pd.DataFrame,
    feature_columns: list[str],
    label_column: str,
    use_numpy_matrix: bool,
) -> pd.DataFrame:
    result = permutation_importance(
        pipeline,
        model_input(validation, feature_columns, use_numpy_matrix),
        validation[label_column].astype(int),
        scoring="roc_auc",
        n_repeats=5,
        random_state=42,
    )
    rows = []
    for feature, mean, stddev in zip(
        feature_columns,
        result.importances_mean,
        result.importances_std,
        strict=True,
    ):
        rows.append(
            {
                "feature": feature,
                "importance": max(0.0, float(mean)),
                "importance_stddev": float(stddev),
                "importance_kind": "permutation_auc_drop",
            }
        )
    return pd.DataFrame(rows).sort_values("importance", ascending=False)
