from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import numpy as np
import pandas as pd

from .mlops import population_stability_index
from .training_utils import safe_path_segment


MAX_AUTOML_FEATURE_BASE_COLUMNS = 8
MAX_AUTOML_GENERATED_FEATURES = 32


def materialize_automl_candidate_features(
    splits: dict[str, pd.DataFrame],
    base_features: list[str],
) -> tuple[dict[str, pd.DataFrame], list[str], list[dict[str, Any]]]:
    materialized = {split_name: frame.copy() for split_name, frame in splits.items()}
    generated_definitions: list[dict[str, Any]] = []
    generated_names: list[str] = []
    bounded_features = base_features[:MAX_AUTOML_FEATURE_BASE_COLUMNS]

    def add_generated_feature(
        name: str,
        transformation: str,
        source_features: list[str],
        values_by_split: dict[str, pd.Series],
    ) -> None:
        if len(generated_names) >= MAX_AUTOML_GENERATED_FEATURES:
            return
        if name in base_features or name in generated_names:
            return
        for split_name, values in values_by_split.items():
            materialized[split_name][name] = values.astype(float).replace(
                [np.inf, -np.inf],
                0.0,
            ).fillna(0.0)
        generated_names.append(name)
        generated_definitions.append(
            {
                "feature": name,
                "feature_kind": "generated",
                "transformation": transformation,
                "source_features": source_features,
            }
        )

    for left_index, left in enumerate(bounded_features):
        for right in bounded_features[left_index + 1 :]:
            if len(generated_names) >= MAX_AUTOML_GENERATED_FEATURES:
                break
            left_slug = safe_path_segment(left).lower()
            right_slug = safe_path_segment(right).lower()
            add_generated_feature(
                f"automl__diff__{left_slug}__minus__{right_slug}",
                "pairwise_difference",
                [left, right],
                {
                    split_name: pd.to_numeric(frame[left], errors="coerce")
                    - pd.to_numeric(frame[right], errors="coerce")
                    for split_name, frame in materialized.items()
                },
            )
            add_generated_feature(
                f"automl__ratio__{left_slug}__over__{right_slug}",
                "safe_pairwise_ratio",
                [left, right],
                {
                    split_name: safe_ratio_series(frame[left], frame[right])
                    for split_name, frame in materialized.items()
                },
            )

    return materialized, [*base_features, *generated_names], generated_definitions


def safe_ratio_series(numerator: pd.Series, denominator: pd.Series) -> pd.Series:
    numerator_values = pd.to_numeric(numerator, errors="coerce").astype(float)
    denominator_values = pd.to_numeric(denominator, errors="coerce").astype(float)
    safe_denominator = denominator_values.where(denominator_values.abs() > 1e-9)
    return (numerator_values / safe_denominator).replace([np.inf, -np.inf], 0.0).fillna(0.0)


def build_feature_search_report(
    train: pd.DataFrame,
    out_of_time: pd.DataFrame,
    candidate_features: list[str],
    generated_feature_definitions: list[dict[str, Any]],
    label_column: str,
    output_path: Path,
) -> dict[str, Any]:
    labels = train[label_column].astype(float)
    definitions_by_feature = {
        definition["feature"]: definition for definition in generated_feature_definitions
    }
    feature_rows: list[dict[str, Any]] = []
    selected_features: list[str] = []
    for feature in candidate_features:
        series = train[feature].astype(float)
        variance = float(series.var(ddof=0)) if len(series) else 0.0
        missing_rate = float(series.isna().mean())
        correlation = label_correlation(series, labels)
        feature_psi = population_stability_index(series, out_of_time[feature].astype(float))
        rejection_reasons = []
        if variance <= 0.0:
            rejection_reasons.append("zero_variance")
        if missing_rate > 0.30:
            rejection_reasons.append("high_missing_rate")
        stability_status = "drift_watch" if feature_psi >= 0.25 else "stable"
        status = "rejected" if rejection_reasons else "selected"
        if status == "selected":
            selected_features.append(feature)
        feature_rows.append(
            {
                "feature": feature,
                "feature_kind": "generated"
                if feature in definitions_by_feature
                else "source",
                "transformation": definitions_by_feature.get(feature, {}).get(
                    "transformation"
                ),
                "source_features": definitions_by_feature.get(feature, {}).get(
                    "source_features",
                    [feature],
                ),
                "status": status,
                "variance": round(variance, 6),
                "missing_rate": round(missing_rate, 6),
                "abs_label_correlation": round(abs(correlation), 6),
                "feature_psi": round(feature_psi, 6),
                "stability_status": stability_status,
                "rejection_reasons": rejection_reasons,
            }
        )
    ranked_features = sorted(
        feature_rows,
        key=lambda row: (
            row["status"] != "selected",
            -row["abs_label_correlation"],
            row["feature"],
        ),
    )
    report = {
        "report_kind": "automl_feature_search",
        "report_version": 1,
        "status": "passed" if selected_features else "blocked",
        "feature_engineering_policy": "bounded_pairwise_difference_and_safe_ratio_generation",
        "selection_policy": "source_and_generated_numeric_features_with_variance_missingness_label_correlation_and_oot_psi_watch",
        "label_column": label_column,
        "candidate_feature_count": len(candidate_features),
        "generated_feature_count": len(generated_feature_definitions),
        "selected_feature_count": len(selected_features),
        "rejected_feature_count": len(candidate_features) - len(selected_features),
        "selected_features": selected_features,
        "generated_feature_definitions": generated_feature_definitions,
        "ranked_features": ranked_features,
    }
    output_path.write_text(
        json.dumps(report, indent=2, sort_keys=True),
        encoding="utf-8",
    )
    return report


def selected_feature_definitions(feature_search_report: dict[str, Any]) -> list[dict[str, Any]]:
    selected = set(feature_search_report.get("selected_features", []))
    return [
        {
            "feature": row["feature"],
            "feature_kind": row.get("feature_kind", "source"),
            "transformation": row.get("transformation"),
            "source_features": row.get("source_features", [row["feature"]]),
        }
        for row in feature_search_report.get("ranked_features", [])
        if row.get("feature") in selected
    ]


def build_factor_ranking_report(
    feature_search_report: dict[str, Any],
    feature_importance: pd.DataFrame,
    permutation_importance: pd.DataFrame,
    drift_report: dict[str, Any],
    output_path: Path,
) -> dict[str, Any]:
    search_rows = {
        row["feature"]: row for row in feature_search_report.get("ranked_features", [])
    }
    model_importance = {
        str(row["feature"]): float(row["importance"])
        for row in feature_importance.to_dict(orient="records")
    }
    permutation_scores = {
        str(row["feature"]): float(row["importance"])
        for row in permutation_importance.to_dict(orient="records")
    }
    feature_psi = drift_report.get("feature_psi", {})
    max_model_importance = max(model_importance.values(), default=0.0)
    max_permutation = max(permutation_scores.values(), default=0.0)
    factor_rows = []
    for feature in feature_search_report.get("selected_features", []):
        search_row = search_rows.get(feature, {})
        normalized_model_importance = normalize_positive_score(
            model_importance.get(feature, 0.0),
            max_model_importance,
        )
        normalized_permutation = normalize_positive_score(
            permutation_scores.get(feature, 0.0),
            max_permutation,
        )
        stability_penalty = min(float(feature_psi.get(feature, 0.0)), 1.0)
        label_correlation_value = float(search_row.get("abs_label_correlation", 0.0))
        ranking_score = (
            normalized_model_importance * 0.45
            + normalized_permutation * 0.35
            + label_correlation_value * 0.20
            - stability_penalty * 0.25
        )
        factor_rows.append(
            {
                "feature": feature,
                "model_importance": round(model_importance.get(feature, 0.0), 6),
                "permutation_importance": round(permutation_scores.get(feature, 0.0), 6),
                "abs_label_correlation": round(label_correlation_value, 6),
                "feature_psi": round(float(feature_psi.get(feature, 0.0)), 6),
                "stability_status": search_row.get("stability_status", "unknown"),
                "ranking_score": round(float(ranking_score), 6),
                "recommended_use": "model_factor_and_rule_candidate_review",
            }
        )
    ranked_factors = sorted(
        factor_rows,
        key=lambda row: (-row["ranking_score"], row["feature"]),
    )
    for index, row in enumerate(ranked_factors, start=1):
        row["rank"] = index
    report = {
        "report_kind": "automl_factor_ranking",
        "report_version": 1,
        "status": "passed" if ranked_factors else "blocked",
        "ranking_policy": "model_importance_permutation_label_correlation_minus_feature_psi",
        "ranked_factor_count": len(ranked_factors),
        "ranked_factors": ranked_factors,
    }
    output_path.write_text(
        json.dumps(report, indent=2, sort_keys=True),
        encoding="utf-8",
    )
    return report


def normalize_positive_score(value: float, max_value: float) -> float:
    if max_value <= 0.0:
        return 0.0
    return max(0.0, float(value)) / max_value


def label_correlation(feature: pd.Series, labels: pd.Series) -> float:
    if feature.nunique(dropna=True) <= 1 or labels.nunique(dropna=True) <= 1:
        return 0.0
    correlation = feature.corr(labels)
    if pd.isna(correlation):
        return 0.0
    return float(correlation)
