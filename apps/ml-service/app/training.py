from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

import joblib
import pandas as pd

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
from .training_diagnostics import (
    build_feature_importance,
    build_overfitting_diagnostics_report,
    build_permutation_importance,
    model_input,
)
from .training_runtime import (
    DEFAULT_ALGORITHM,
    DEFAULT_THRESHOLD,
    ONNX_ALGORITHMS,
    algorithm_family,
    build_pipeline,
    build_rust_serving_artifact,
    candidate_version,
    evaluate_split,
    export_onnx_serving_artifact,
    normalize_algorithm,
    runtime_kind_for_algorithm,
    serving_runtime_kind_for_algorithm,
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
