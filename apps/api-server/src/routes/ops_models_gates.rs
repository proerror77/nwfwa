use super::{
    ops_datasets::build_dataset_health_record,
    ops_models::{
        ModelArtifactEvidenceSummary, ModelPromotionGate, ModelPromotionGatesResponse,
        ModelRetrainingReadinessResponse,
    },
};
use crate::repository::{
    canonical_feedback_target, DatasetRecord, ModelEvaluationRecord, ModelPerformanceRecord,
    ModelPromotionReviewRecord, ModelVersionRecord, QaFeedbackItemRecord,
};
use serde_json::Value;

struct SourceDataQualityGate {
    dataset_id: String,
    score: Option<f64>,
    status: String,
    passed: bool,
    blocker: &'static str,
    evidence_source: &'static str,
}

pub(super) fn activation_blockers(gates: &ModelPromotionGatesResponse) -> Vec<String> {
    gates
        .gates
        .iter()
        .filter(|gate| gate.label != "Active version" && !gate.passed)
        .map(|gate| gate.blocker.clone())
        .collect()
}

pub(super) fn build_model_promotion_gates(
    model: &ModelVersionRecord,
    performance: &ModelPerformanceRecord,
    evaluations: &[ModelEvaluationRecord],
    outcome_labels: &[crate::repository::OutcomeLabelRecord],
    feedback_items: &[QaFeedbackItemRecord],
    latest_review: Option<&ModelPromotionReviewRecord>,
    source_dataset: Option<&DatasetRecord>,
) -> ModelPromotionGatesResponse {
    let latest_evaluation = evaluations.iter().find(|evaluation| {
        evaluation.model_key == model.model_key && evaluation.model_version == model.version
    });
    let metrics = latest_evaluation
        .map(|evaluation| &evaluation.metrics_json)
        .unwrap_or(&serde_json::Value::Null);
    let has_out_of_time_metric = metrics.get("out_of_time_auc").is_some()
        || metrics.get("out_of_time_precision").is_some()
        || metrics.get("out_of_time_recall").is_some();
    let time_group_split_strategy = time_group_split_strategy_gate(metrics);
    let immutable_dataset = latest_evaluation
        .map(|evaluation| !evaluation.model_dataset_id.is_empty())
        .unwrap_or(false);
    let holdout_metrics = latest_evaluation
        .map(|evaluation| {
            evaluation.auc.is_some()
                && evaluation.precision.is_some()
                && evaluation.recall.is_some()
        })
        .unwrap_or(false);
    let review_capacity_threshold = latest_evaluation
        .map(|evaluation| {
            evaluation.threshold.is_some()
                && metrics
                    .get("review_capacity_threshold_status")
                    .and_then(|value| value.as_str())
                    == Some("passed")
        })
        .unwrap_or(false);
    let explanation_artifact = latest_evaluation
        .map(|evaluation| {
            evaluation.feature_importance_uri.is_some()
                || evaluation.permutation_importance_uri.is_some()
        })
        .unwrap_or(false);
    let leakage_check = metrics
        .get("leakage_check_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let shadow_comparison = metrics
        .get("shadow_comparison_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let serving_version_lock = metrics
        .get("serving_version_lock_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let artifact_integrity = metrics
        .get("artifact_integrity_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let feature_store_materialization = feature_materialization_gate(metrics);
    let segment_fairness = metrics
        .get("segment_fairness_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let rust_serving_evaluation = rust_serving_evaluation_gate(metrics);
    let source_data_quality = source_data_quality_gate(metrics, source_dataset);
    let feature_reproducibility = metrics
        .get("feature_reproducibility_hash")
        .and_then(|value| value.as_str())
        .map(|hash| hash.starts_with("sha256:") && hash.len() > "sha256:".len())
        .unwrap_or(false);
    let label_provenance = metrics
        .get("label_provenance_status")
        .and_then(|value| value.as_str())
        == Some("passed")
        && metrics
            .get("label_reviewer_source")
            .and_then(|value| value.as_str())
            .map(|source| !source.trim().is_empty())
            .unwrap_or(false);
    let pilot_customer_validation = pilot_customer_validation_gate(metrics);
    let approval = latest_review
        .map(|review| review.decision == "approved")
        .unwrap_or_else(|| {
            metrics
                .get("approval_status")
                .and_then(|value| value.as_str())
                == Some("approved")
        });
    let drift_status =
        evaluation_drift_status(metrics).unwrap_or_else(|| performance.drift_status.clone());
    let drift_gate_passed = drift_status == "stable";
    let active_version = model.status == "active";
    let open_model_feedback_count = feedback_items
        .iter()
        .filter(|item| {
            canonical_feedback_target(&item.feedback_target) == "model"
                && item.status == "open"
                && evidence_refs_apply_to_model_version(&item.evidence_refs, model)
        })
        .count();
    let unresolved_model_feedback_count = feedback_items
        .iter()
        .filter(|item| {
            canonical_feedback_target(&item.feedback_target) == "model"
                && is_unresolved_feedback_status(&item.status)
                && evidence_refs_apply_to_model_version(&item.evidence_refs, model)
        })
        .count();
    let model_labels = outcome_labels
        .iter()
        .filter(|label| {
            canonical_feedback_target(&label.feedback_target) == "model"
                && evidence_refs_apply_to_model_version(&label.evidence_refs, model)
        })
        .collect::<Vec<_>>();
    let approved_model_labels = model_labels
        .iter()
        .filter(|label| label.governance_status == "approved_for_training")
        .count();
    let needs_review_model_labels = model_labels
        .iter()
        .filter(|label| label.governance_status == "needs_review")
        .count();
    let label_governance = approved_model_labels > 0 && needs_review_model_labels == 0;
    let artifact_evidence = model_artifact_evidence_summary(metrics);

    let gates = vec![
        gate(
            "Immutable dataset",
            immutable_dataset,
            "dataset version missing",
            evidence_source(immutable_dataset, "evaluation"),
        ),
        gate(
            "Holdout metrics",
            holdout_metrics,
            "holdout metrics missing",
            evidence_source(holdout_metrics, "evaluation"),
        ),
        gate(
            "Out-of-time evidence",
            has_out_of_time_metric,
            "out-of-time metrics missing",
            evidence_source(has_out_of_time_metric, "evaluation"),
        ),
        gate(
            "Time/group split strategy",
            time_group_split_strategy,
            "time/group split strategy missing",
            evidence_source(time_group_split_strategy, "evaluation"),
        ),
        gate(
            "Review-capacity threshold",
            review_capacity_threshold,
            "review-capacity threshold missing",
            evidence_source(review_capacity_threshold, "evaluation"),
        ),
        gate(
            "Explanation artifact",
            explanation_artifact,
            "feature importance missing",
            evidence_source(explanation_artifact, "evaluation"),
        ),
        gate(
            "Leakage check",
            leakage_check,
            "leakage check missing",
            evidence_source(leakage_check, "evaluation"),
        ),
        gate(
            "Shadow comparison",
            shadow_comparison,
            "shadow comparison missing",
            evidence_source(shadow_comparison, "evaluation"),
        ),
        gate(
            "Serving version lock",
            serving_version_lock,
            "serving version lock missing",
            evidence_source(serving_version_lock, "evaluation"),
        ),
        gate(
            "Artifact integrity",
            artifact_integrity,
            "artifact integrity missing",
            evidence_source(artifact_integrity, "evaluation"),
        ),
        gate(
            "Feature materialization",
            feature_store_materialization,
            "rust feature-set materialization missing",
            evidence_source(feature_store_materialization, "evaluation"),
        ),
        gate(
            "Segment fairness",
            segment_fairness,
            "segment fairness review missing",
            evidence_source(segment_fairness, "evaluation"),
        ),
        gate(
            "Rust serving evaluation",
            rust_serving_evaluation,
            "rust serving artifact evaluation missing",
            evidence_source(rust_serving_evaluation, "evaluation"),
        ),
        gate(
            "Source data quality",
            source_data_quality.passed,
            source_data_quality.blocker,
            source_data_quality.evidence_source,
        ),
        gate(
            "Feature reproducibility",
            feature_reproducibility,
            "feature reproducibility hash missing",
            evidence_source(feature_reproducibility, "evaluation"),
        ),
        gate(
            "Label provenance",
            label_provenance,
            label_provenance_blocker(metrics),
            evidence_source(label_provenance, "evaluation"),
        ),
        gate(
            "Pilot/customer validation",
            pilot_customer_validation,
            "pilot/customer validation missing",
            pilot_customer_validation_evidence_source(metrics, pilot_customer_validation),
        ),
        gate(
            "Drift status",
            drift_gate_passed,
            drift_blocker(&drift_status),
            drift_evidence_source(&drift_status),
        ),
        gate(
            "Model QA feedback closure",
            unresolved_model_feedback_count == 0,
            "unresolved model QA feedback",
            "qa_feedback",
        ),
        gate(
            "Label governance",
            label_governance,
            label_governance_blocker(approved_model_labels, needs_review_model_labels),
            if model_labels.is_empty() {
                "missing"
            } else {
                "labels"
            },
        ),
        gate(
            "Approval",
            approval,
            "approval missing",
            evidence_source(approval, "approval"),
        ),
        gate(
            "Active version",
            active_version,
            "model is not active",
            evidence_source(active_version, "metadata"),
        ),
    ];
    let blockers = gates
        .iter()
        .filter(|gate| !gate.passed)
        .map(|gate| gate.blocker.clone())
        .collect::<Vec<_>>();

    ModelPromotionGatesResponse {
        model_key: model.model_key.clone(),
        model_version: model.version.clone(),
        review_mode: model.review_mode.clone(),
        decision: if blockers.is_empty() {
            "routing_allowed".into()
        } else {
            "routing_blocked".into()
        },
        passed_count: gates.len() - blockers.len(),
        total_count: gates.len(),
        latest_evaluation_id: latest_evaluation
            .map(|evaluation| evaluation.evaluation_run_id.clone())
            .unwrap_or_else(|| "none".into()),
        source_dataset_id: source_data_quality.dataset_id,
        source_data_quality_score: source_data_quality.score,
        source_data_quality_status: source_data_quality.status,
        data_status: performance.data_status.clone(),
        scored_runs: performance.scored_runs,
        open_model_feedback_count,
        unresolved_model_feedback_count,
        approved_label_count: approved_model_labels,
        needs_review_label_count: needs_review_model_labels,
        artifact_evidence,
        gates,
        blockers,
    }
}

pub(super) fn build_model_retraining_readiness(
    model: &ModelVersionRecord,
    performance: &ModelPerformanceRecord,
    latest_evaluation: Option<&ModelEvaluationRecord>,
    outcome_labels: &[crate::repository::OutcomeLabelRecord],
    feedback_items: &[QaFeedbackItemRecord],
    source_dataset: Option<&DatasetRecord>,
) -> ModelRetrainingReadinessResponse {
    let metrics = latest_evaluation
        .map(|evaluation| &evaluation.metrics_json)
        .unwrap_or(&serde_json::Value::Null);
    let source_data_quality = source_data_quality_gate(metrics, source_dataset);
    let open_model_feedback_count = feedback_items
        .iter()
        .filter(|item| {
            canonical_feedback_target(&item.feedback_target) == "model"
                && item.status == "open"
                && evidence_refs_apply_to_model_version(&item.evidence_refs, model)
        })
        .count();
    let model_labels = outcome_labels
        .iter()
        .filter(|label| {
            canonical_feedback_target(&label.feedback_target) == "model"
                && evidence_refs_apply_to_model_version(&label.evidence_refs, model)
        })
        .collect::<Vec<_>>();
    let approved_label_count = model_labels
        .iter()
        .filter(|label| label.governance_status == "approved_for_training")
        .count();
    let needs_review_label_count = model_labels
        .iter()
        .filter(|label| label.governance_status == "needs_review")
        .count();

    let mut retraining_triggers = Vec::new();
    if matches!(performance.drift_status.as_str(), "watch" | "drift") {
        retraining_triggers.push(format!("score drift status: {}", performance.drift_status));
    }
    if open_model_feedback_count > 0 {
        retraining_triggers.push("open model QA feedback".into());
    }
    if approved_label_count > 0 {
        retraining_triggers.push("approved model labels available".into());
    }

    let mut blockers = Vec::new();
    if latest_evaluation.is_none() {
        blockers.push("latest model evaluation missing".into());
    }
    if !source_data_quality.passed {
        blockers.push(source_data_quality.blocker.into());
    }
    if approved_label_count == 0 {
        blockers.push("approved model outcome labels missing".into());
    }
    if needs_review_label_count > 0 {
        blockers.push("model outcome labels need review".into());
    }

    let recommendation = if !blockers.is_empty() {
        "blocked"
    } else if retraining_triggers.is_empty() {
        "monitor"
    } else {
        "prepare_retraining"
    };

    ModelRetrainingReadinessResponse {
        model_key: model.model_key.clone(),
        model_version: model.version.clone(),
        recommendation: recommendation.into(),
        latest_evaluation_id: latest_evaluation
            .map(|evaluation| evaluation.evaluation_run_id.clone())
            .unwrap_or_else(|| "none".into()),
        drift_status: performance.drift_status.clone(),
        source_dataset_id: source_data_quality.dataset_id,
        source_data_quality_score: source_data_quality.score,
        source_data_quality_status: source_data_quality.status,
        open_model_feedback_count,
        approved_label_count,
        needs_review_label_count,
        retraining_triggers,
        blockers,
    }
}

fn is_unresolved_feedback_status(status: &str) -> bool {
    matches!(status, "open" | "in_progress")
}

fn evidence_refs_apply_to_model_version(
    evidence_refs: &[String],
    model: &ModelVersionRecord,
) -> bool {
    let mut has_model_version_ref = false;
    let expected = format!("{}:{}", model.model_key, model.version);
    for evidence_ref in evidence_refs {
        let Some(model_version_ref) = evidence_ref.trim().strip_prefix("model_versions:") else {
            continue;
        };
        has_model_version_ref = true;
        if model_version_ref == expected {
            return true;
        }
    }
    !has_model_version_ref
}

fn evidence_source(passed: bool, source: &'static str) -> &'static str {
    if passed {
        source
    } else {
        "missing"
    }
}

fn drift_blocker(status: &str) -> &'static str {
    match status {
        "not_available" => "model drift status unavailable",
        _ => "model drift detected",
    }
}

fn drift_evidence_source(status: &str) -> &'static str {
    match status {
        "not_available" => "missing",
        _ => "evaluation",
    }
}

fn evaluation_drift_status(metrics: &Value) -> Option<String> {
    metrics
        .get("score_psi")
        .or_else(|| metrics.get("psi"))
        .and_then(Value::as_f64)
        .map(|score_psi| {
            if score_psi < 0.10 {
                "stable"
            } else if score_psi < 0.25 {
                "watch"
            } else {
                "drift"
            }
            .to_string()
        })
}

fn label_governance_blocker(approved_count: usize, needs_review_count: usize) -> &'static str {
    if approved_count == 0 {
        "approved model outcome labels missing"
    } else if needs_review_count > 0 {
        "model outcome labels need review"
    } else {
        "none"
    }
}

fn time_group_split_strategy_gate(metrics: &serde_json::Value) -> bool {
    let status_passed = metrics
        .get("time_group_split_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let has_time_field = metrics
        .get("time_split_field")
        .and_then(|value| value.as_str())
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let has_group_field = metrics
        .get("group_split_fields")
        .and_then(|value| value.as_array())
        .map(|fields| {
            fields
                .iter()
                .any(|field| field.as_str().is_some_and(|value| !value.trim().is_empty()))
        })
        .unwrap_or(false);
    status_passed && has_time_field && has_group_field
}

fn feature_materialization_gate(metrics: &serde_json::Value) -> bool {
    let feature_store_status = metrics
        .get("feature_store_materialization_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let rust_feature_set_status = metrics
        .get("rust_feature_set_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let has_rust_feature_set_manifest = metrics
        .get("rust_feature_set_manifest_uri")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty());
    feature_store_status && rust_feature_set_status && has_rust_feature_set_manifest
}

fn pilot_customer_validation_gate(metrics: &serde_json::Value) -> bool {
    let validation_status_passed = ["pilot_validation_status", "customer_validation_status"]
        .into_iter()
        .any(|field| metrics.get(field).and_then(|value| value.as_str()) == Some("passed"));
    let usage_scope_validated = metrics
        .get("dataset_usage_scope")
        .and_then(|value| value.as_str())
        .is_some_and(|scope| {
            matches!(
                scope,
                "customer_pilot_validated"
                    | "customer_production_validated"
                    | "customer_validated"
                    | "pilot_validated"
            )
        });
    validation_status_passed || usage_scope_validated
}

fn pilot_customer_validation_evidence_source(
    metrics: &serde_json::Value,
    passed: bool,
) -> &'static str {
    if passed
        || metrics.get("dataset_usage_scope").is_some()
        || metrics.get("pilot_validation_status").is_some()
        || metrics.get("customer_validation_status").is_some()
    {
        "evaluation"
    } else {
        "missing"
    }
}

fn rust_serving_evaluation_gate(metrics: &Value) -> bool {
    if metrics
        .get("model_artifact_evaluation_status")
        .and_then(|value| value.as_str())
        == Some("passed")
    {
        return true;
    }
    if metrics
        .get("model_artifact_evaluation_gate_status")
        .and_then(|value| value.as_str())
        == Some("passed")
    {
        return true;
    }
    if metrics.get("report_kind").and_then(|value| value.as_str())
        == Some("model_artifact_evaluation")
        && metrics.get("gate_status").and_then(|value| value.as_str()) == Some("passed")
    {
        return true;
    }
    metrics
        .get("model_artifact_evaluation")
        .is_some_and(|value| {
            value.get("report_kind").and_then(|value| value.as_str())
                == Some("model_artifact_evaluation")
                && value.get("gate_status").and_then(|value| value.as_str()) == Some("passed")
        })
}

fn model_artifact_evidence_summary(metrics: &Value) -> ModelArtifactEvidenceSummary {
    ModelArtifactEvidenceSummary {
        serving_manifest_uri: optional_metric_string(metrics, "serving_manifest_uri"),
        model_artifact_evaluation_report_uri: optional_metric_string(
            metrics,
            "model_artifact_evaluation_report_uri",
        ),
        permutation_importance_uri: optional_metric_string(metrics, "permutation_importance_uri"),
        rust_serving_status: optional_metric_string(metrics, "rust_serving_status"),
        rust_serving_latency_status: optional_metric_string(metrics, "rust_serving_latency_status"),
        rust_serving_p95_latency_ms: optional_metric_u64(metrics, "rust_serving_p95_latency_ms"),
        rust_serving_latency_measurement_kind: optional_metric_string(
            metrics,
            "rust_serving_latency_measurement_kind",
        ),
        rust_serving_latency_sample_count: optional_metric_u64(
            metrics,
            "rust_serving_latency_sample_count",
        ),
    }
}

fn optional_metric_string(metrics: &Value, key: &str) -> Option<String> {
    metrics
        .get(key)
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn optional_metric_u64(metrics: &Value, key: &str) -> Option<u64> {
    metrics.get(key).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_str().and_then(|value| value.parse::<u64>().ok()))
    })
}

fn source_data_quality_gate(
    metrics: &serde_json::Value,
    source_dataset: Option<&DatasetRecord>,
) -> SourceDataQualityGate {
    if let Some(dataset) = source_dataset {
        let health = build_dataset_health_record(dataset);
        return SourceDataQualityGate {
            dataset_id: health.dataset_id,
            score: Some(health.data_quality_score),
            status: health.data_quality_status,
            passed: health.data_quality_score >= 0.8,
            blocker: if health.data_quality_score >= 0.8 {
                "none"
            } else {
                "source dataset data quality below threshold"
            },
            evidence_source: "dataset",
        };
    }

    match metrics
        .get("data_quality_score")
        .and_then(|value| value.as_f64())
    {
        Some(score) => SourceDataQualityGate {
            dataset_id: "none".into(),
            score: Some(score),
            status: data_quality_status_for_score(score).into(),
            passed: score >= 0.8,
            blocker: if score >= 0.8 {
                "none"
            } else {
                "source data quality score below threshold"
            },
            evidence_source: "evaluation",
        },
        None => SourceDataQualityGate {
            dataset_id: "none".into(),
            score: None,
            status: "missing".into(),
            passed: false,
            blocker: "source data quality score missing",
            evidence_source: "missing",
        },
    }
}

fn data_quality_status_for_score(score: f64) -> &'static str {
    if score >= 0.85 {
        "ready"
    } else if score >= 0.65 {
        "watch"
    } else {
        "blocked"
    }
}

fn label_provenance_blocker(metrics: &serde_json::Value) -> &'static str {
    let status = metrics
        .get("label_provenance_status")
        .and_then(|value| value.as_str());
    let reviewer_source_present = metrics
        .get("label_reviewer_source")
        .and_then(|value| value.as_str())
        .map(|source| !source.trim().is_empty())
        .unwrap_or(false);
    if status == Some("passed") && !reviewer_source_present {
        "label reviewer source missing"
    } else {
        "label provenance missing"
    }
}

fn gate(label: &str, passed: bool, blocker: &str, evidence_source: &str) -> ModelPromotionGate {
    ModelPromotionGate {
        label: label.into(),
        passed,
        blocker: blocker.into(),
        evidence_source: evidence_source.into(),
    }
}
