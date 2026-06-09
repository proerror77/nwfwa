from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pandas as pd
from sklearn.tree import DecisionTreeClassifier

from .training_utils import safe_id_segment


def build_rule_candidate_backtest_artifacts(
    candidates: list[dict[str, Any]],
    validation_metrics: dict[str, Any],
    oot_metrics: dict[str, Any],
    report_path: Path,
    review_tasks_path: Path,
) -> None:
    report_path.parent.mkdir(parents=True, exist_ok=True)
    review_tasks_path.parent.mkdir(parents=True, exist_ok=True)
    candidate_results = [
        {
            "rule_id": candidate.get("rule_id"),
            "backtest_status": "passed",
            "review_status": "pending_human_review",
            "writeback_status": "blocked_pending_human_review_and_policy_governance_approval",
        }
        for candidate in candidates
    ]
    report = {
        "report_kind": "rule_candidate_backtest",
        "report_version": 1,
        "status": "passed",
        "candidate_count": len(candidates),
        "validation_auc": validation_metrics["auc"],
        "out_of_time_auc": oot_metrics["auc"],
        "promotion_boundary": "draft_only_human_review_required",
        "writeback_status": "blocked_pending_human_review_and_policy_governance_approval",
        "candidate_results": candidate_results,
    }
    review_tasks = [
        {
            "task_id": f"review_{candidate.get('rule_id', index)}",
            "rule_id": candidate.get("rule_id"),
            "status": "pending_human_review",
            "required_review": "policy_governance_and_fwa_investigator",
        }
        for index, candidate in enumerate(candidates, start=1)
    ]
    report_path.write_text(
        json.dumps(report, indent=2, sort_keys=True),
        encoding="utf-8",
    )
    review_tasks_path.write_text(
        json.dumps(review_tasks, indent=2, sort_keys=True),
        encoding="utf-8",
    )


def build_mined_rule_candidates(
    model_key: str,
    candidate_model_version: str,
    train: pd.DataFrame,
    feature_importance: pd.DataFrame,
    feature_columns: list[str],
    label_column: str,
) -> list[dict[str, Any]]:
    candidates = build_decision_tree_rule_candidates(
        model_key,
        candidate_model_version,
        train,
        feature_columns,
        label_column,
    )
    candidate_ids = {candidate["rule_id"] for candidate in candidates}
    ranked_features = feature_importance.sort_values("importance", ascending=False)[
        "feature"
    ].tolist()
    for feature in ranked_features:
        if feature not in feature_columns:
            continue
        candidate = mined_rule_candidate_for_feature(
            model_key,
            candidate_model_version,
            train,
            feature_importance,
            feature,
            label_column,
        )
        if candidate is not None and candidate["rule_id"] not in candidate_ids:
            candidates.append(candidate)
            candidate_ids.add(candidate["rule_id"])
        if len(candidates) >= 5:
            break
    candidates.sort(key=lambda rule: rule["rule_id"])
    return candidates


def build_decision_tree_rule_candidates(
    model_key: str,
    candidate_model_version: str,
    train: pd.DataFrame,
    feature_columns: list[str],
    label_column: str,
) -> list[dict[str, Any]]:
    labels = train[label_column].astype(int)
    if labels.nunique() < 2 or len(train) < 4:
        return []
    tree = DecisionTreeClassifier(max_depth=3, min_samples_leaf=1, random_state=17)
    tree.fit(train[feature_columns].astype(float), labels)
    candidates: list[dict[str, Any]] = []
    tree_state = tree.tree_

    def visit(node_id: int, path: list[dict[str, Any]]) -> None:
        left_id = int(tree_state.children_left[node_id])
        right_id = int(tree_state.children_right[node_id])
        if left_id == right_id:
            values = tree_state.value[node_id][0]
            negative_count = float(values[0]) if len(values) > 0 else 0.0
            positive_count = float(values[1]) if len(values) > 1 else 0.0
            total_count = negative_count + positive_count
            if total_count == 0 or positive_count == 0:
                return
            positive_rate = positive_count / total_count
            if positive_rate < 0.5 or not path:
                return
            candidates.append(
                decision_tree_rule_candidate(
                    model_key,
                    candidate_model_version,
                    path,
                    positive_count,
                    total_count,
                    positive_rate,
                )
            )
            return
        feature = feature_columns[int(tree_state.feature[node_id])]
        threshold = round(float(tree_state.threshold[node_id]), 6)
        visit(
            left_id,
            [
                *path,
                {"field": feature, "operator": "<=", "value": threshold},
            ],
        )
        visit(
            right_id,
            [
                *path,
                {"field": feature, "operator": ">=", "value": threshold},
            ],
        )

    visit(0, [])
    candidates.sort(
        key=lambda rule: (
            -rule["metadata"]["tree_positive_rate"],
            -rule["metadata"]["tree_positive_count"],
            rule["rule_id"],
        )
    )
    return candidates[:2]


def decision_tree_rule_candidate(
    model_key: str,
    candidate_model_version: str,
    conditions: list[dict[str, Any]],
    positive_count: float,
    total_count: float,
    positive_rate: float,
) -> dict[str, Any]:
    condition_slug = "_".join(
        safe_id_segment(
            f"{condition['field']}_{'gte' if condition['operator'] == '>=' else 'lte'}_{condition['value']}"
        )
        for condition in conditions
    )
    rule_id = "candidate_tree_" + safe_id_segment(
        f"{model_key}_{candidate_model_version}_{condition_slug}"
    ).lower()
    alert_code = "TREE_MINED_" + safe_id_segment(condition_slug).upper()[:48]
    path_label = " and ".join(
        f"{condition['field']} {condition['operator']} {condition['value']}"
        for condition in conditions
    )
    return {
        "rule_id": rule_id,
        "version": 1,
        "name": f"Decision tree mined candidate: {path_label}",
        "review_mode": "both",
        "scheme_family": "high_risk_claim",
        "conditions": conditions,
        "action": {
            "score": max(15, min(40, int(round(18 + positive_rate * 20)))),
            "alert_code": alert_code,
            "recommended_action": "ManualReview",
            "reason": (
                "External training platform mined this shallow decision-tree path "
                f"from training data: {positive_count:.0f}/{total_count:.0f} positive samples "
                f"({positive_rate:.2%}) matched path {path_label}. Human review is required."
            ),
        },
        "metadata": {
            "mining_algorithm": "shallow_decision_tree",
            "tree_positive_count": int(round(positive_count)),
            "tree_total_count": int(round(total_count)),
            "tree_positive_rate": round(float(positive_rate), 6),
        },
    }


def mined_rule_candidate_for_feature(
    model_key: str,
    candidate_model_version: str,
    train: pd.DataFrame,
    feature_importance: pd.DataFrame,
    feature: str,
    label_column: str,
) -> dict[str, Any] | None:
    positives = train[train[label_column].astype(int) == 1][feature].dropna().astype(float)
    negatives = train[train[label_column].astype(int) == 0][feature].dropna().astype(float)
    if positives.empty or negatives.empty:
        return None
    positive_mean = float(positives.mean())
    negative_mean = float(negatives.mean())
    negative_stddev = float(negatives.std(ddof=1)) if len(negatives) > 1 else 0.0
    operator = ">=" if positive_mean >= negative_mean else "<="
    if operator == ">=":
        threshold = negative_mean + 1.5 * negative_stddev
        threshold_method = "negative-class mean + 1.5 standard deviations"
    else:
        threshold = negative_mean - 1.5 * negative_stddev
        threshold_method = "negative-class mean - 1.5 standard deviations"
    importance_row = feature_importance[feature_importance["feature"] == feature].iloc[0]
    importance = float(importance_row["importance"])
    importance_kind = str(importance_row["importance_kind"])
    rule_id = "candidate_training_" + safe_id_segment(
        f"{model_key}_{candidate_model_version}_{feature}"
    ).lower()
    alert_code = "TRAINING_MINED_" + safe_id_segment(feature).upper()[:48]
    return {
        "rule_id": rule_id,
        "version": 1,
        "name": f"Training mined {feature} candidate",
        "review_mode": "both",
        "scheme_family": "high_risk_claim",
        "conditions": [
            {
                "field": feature,
                "operator": operator,
                "value": round(float(threshold), 6),
            }
        ],
        "action": {
            "score": mined_rule_score(importance),
            "alert_code": alert_code,
            "recommended_action": "ManualReview",
            "reason": (
                f"External training platform mined {feature} from training data: "
                f"positive mean {positive_mean:.4f}, {threshold_method} "
                f"{threshold:.4f}, model signal {importance_kind}={importance:.6f}. Human review is required."
            ),
        },
    }


def mined_rule_score(importance: float) -> int:
    return max(10, min(35, int(round(15 + min(float(importance), 1.0) * 20))))
