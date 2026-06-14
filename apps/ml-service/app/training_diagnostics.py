from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import numpy as np
import pandas as pd
from sklearn.inspection import permutation_importance
from sklearn.pipeline import Pipeline


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
        split for split in required_splits if time_split_field not in splits[split].columns
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
