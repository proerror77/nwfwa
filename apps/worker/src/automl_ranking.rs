use anyhow::{anyhow, bail, Context};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::{fs, fs::File, path::Path};

use super::{
    column_values, ensure_parquet_path, required_manifest_str, safe_id_segment, write_json,
    AutoMlCandidateRank, AutoMlCandidateRanking, AutoMlReviewTask, FeatureImportanceRow,
};

pub fn rank_automl_candidates(
    validation_reports: &[String],
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<AutoMlCandidateRanking> {
    if validation_reports.is_empty() {
        bail!("at least one validation report is required");
    }

    let mut candidates = Vec::new();
    for report_uri in validation_reports {
        let report_path = Path::new(report_uri);
        let report_json = fs::read_to_string(report_path)
            .with_context(|| format!("read validation report {}", report_path.display()))?;
        let report: serde_json::Value =
            serde_json::from_str(&report_json).context("parse validation report")?;
        candidates.push(build_automl_candidate_rank(report_uri, &report)?);
    }

    candidates.sort_by(|left, right| {
        eligible_sort_key(right)
            .partial_cmp(&eligible_sort_key(left))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.candidate_model_version
                    .cmp(&right.candidate_model_version)
            })
    });

    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.rank = index + 1;
    }

    let recommended_candidate_model_version = candidates
        .iter()
        .find(|candidate| candidate.gate_status == "passed")
        .map(|candidate| candidate.candidate_model_version.clone());
    let review_tasks = candidates
        .iter()
        .map(|candidate| AutoMlReviewTask {
            task_kind: "model_candidate_human_review".into(),
            candidate_model_version: candidate.candidate_model_version.clone(),
            review_queue: if candidate.gate_status == "passed" {
                "model_governance_review".into()
            } else {
                "mlops_remediation_review".into()
            },
            required_review: "human_approval_required_before_shadow_or_activation".into(),
            decision_options: vec![
                "reject".into(),
                "request_more_evidence".into(),
                "approve_shadow_only".into(),
            ],
            evidence_refs: candidate.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();
    let ranking = AutoMlCandidateRanking {
        plan_kind: "automl_candidate_ranking".into(),
        plan_version: 1,
        promotion_boundary:
            "ranking opens human review only; no automatic model promotion or rule publication"
                .into(),
        generated_from_reports: validation_reports.to_vec(),
        recommended_candidate_model_version,
        candidates,
        review_tasks,
        evidence_refs: validation_reports
            .iter()
            .map(|report| format!("model_validation_reports:{report}"))
            .collect(),
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create Auto MLOps ranking output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir.as_ref().join("automl_candidate_ranking.json"),
        &ranking,
    )?;
    write_json(
        output_dir.as_ref().join("automl_review_tasks.json"),
        &ranking.review_tasks,
    )?;
    Ok(ranking)
}

pub(crate) fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

pub(crate) fn read_feature_importance(path: &Path) -> anyhow::Result<Vec<FeatureImportanceRow>> {
    ensure_parquet_path(path)?;
    let file = File::open(path)
        .with_context(|| format!("open feature importance parquet {}", path.display()))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .with_context(|| format!("read feature importance metadata {}", path.display()))?;
    let mut reader = builder.with_batch_size(4096).build()?;
    let mut rows = Vec::new();
    for batch in &mut reader {
        let batch = batch?;
        let feature_index = batch
            .schema()
            .index_of("feature")
            .context("feature importance missing feature column")?;
        let importance_index = batch
            .schema()
            .index_of("importance")
            .context("feature importance missing importance column")?;
        let kind_index = batch
            .schema()
            .index_of("importance_kind")
            .context("feature importance missing importance_kind column")?;
        let feature_values = column_values(batch.column(feature_index).as_ref());
        let importance_values = column_values(batch.column(importance_index).as_ref());
        let kind_values = column_values(batch.column(kind_index).as_ref());
        for row_index in 0..batch.num_rows() {
            let Some(feature) = feature_values.get(row_index) else {
                continue;
            };
            let Some(importance) = importance_values
                .get(row_index)
                .and_then(|value| value.parse::<f64>().ok())
            else {
                continue;
            };
            let importance_kind = kind_values
                .get(row_index)
                .cloned()
                .unwrap_or_else(|| "unknown".into());
            rows.push(FeatureImportanceRow {
                feature: feature.clone(),
                importance,
                importance_kind,
            });
        }
    }
    rows.sort_by(|left, right| {
        right
            .importance
            .partial_cmp(&left.importance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.feature.cmp(&right.feature))
    });
    Ok(rows)
}

fn build_automl_candidate_rank(
    validation_report_uri: &str,
    report: &serde_json::Value,
) -> anyhow::Result<AutoMlCandidateRank> {
    let model_key = required_manifest_str(report, "model_key")?.to_string();
    let candidate_model_version =
        required_manifest_str(report, "candidate_model_version")?.to_string();
    let algorithm = required_manifest_str(report, "algorithm")?.to_string();
    let metrics = report
        .get("metrics_json")
        .and_then(|value| value.as_object())
        .ok_or_else(|| anyhow!("validation report missing metrics_json"))?;
    let validation_metrics = report
        .get("validation_metrics")
        .unwrap_or(&serde_json::Value::Null);
    let algorithm_family = metrics
        .get("algorithm_family")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let validation_auc = metric_at(validation_metrics, "auc");
    let out_of_time_auc = metric_object_value(metrics, "out_of_time_auc");
    let out_of_time_average_precision =
        metric_object_value(metrics, "out_of_time_average_precision");
    let out_of_time_precision = metric_object_value(metrics, "out_of_time_precision");
    let out_of_time_recall = metric_object_value(metrics, "out_of_time_recall");
    let score_psi =
        metric_object_value(metrics, "score_psi").or_else(|| metric_object_value(metrics, "psi"));
    let max_feature_psi = metric_object_value(metrics, "max_feature_psi");
    let permutation_importance_passed =
        automl_permutation_importance_passed(metrics) && automl_has_permutation_importance(metrics);
    let feature_reproducibility_passed = automl_feature_reproducibility_passed(metrics);

    let blocking_reasons = automl_blocking_reasons(metrics);
    let gate_status = if blocking_reasons.is_empty() {
        "passed"
    } else {
        "blocked"
    }
    .to_string();
    let recommended_action = if gate_status == "passed" {
        "open_human_review"
    } else {
        "keep_blocked"
    }
    .to_string();

    let mut evidence_refs = vec![
        format!("model_validation_reports:{validation_report_uri}"),
        format!(
            "model_evaluations:{}",
            safe_id_segment(&candidate_model_version)
        ),
    ];
    if let Some(feature_search_report_uri) = metrics
        .get("automl_feature_search_report_uri")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
    {
        evidence_refs.push(format!(
            "automl_feature_search_reports:{feature_search_report_uri}"
        ));
    }
    if let Some(factor_ranking_report_uri) = metrics
        .get("automl_factor_ranking_report_uri")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
    {
        evidence_refs.push(format!(
            "automl_factor_rankings:{factor_ranking_report_uri}"
        ));
    }

    Ok(AutoMlCandidateRank {
        rank: 0,
        model_key,
        candidate_model_version: candidate_model_version.clone(),
        algorithm,
        algorithm_family,
        dataset_key: report
            .get("dataset_key")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        dataset_version: report
            .get("dataset_version")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        validation_report_uri: validation_report_uri.into(),
        ranking_score: automl_ranking_score(
            out_of_time_auc,
            out_of_time_average_precision,
            out_of_time_precision,
            out_of_time_recall,
            score_psi,
            max_feature_psi,
            permutation_importance_passed,
            feature_reproducibility_passed,
            &gate_status,
        ),
        validation_auc,
        out_of_time_auc,
        out_of_time_average_precision,
        out_of_time_precision,
        out_of_time_recall,
        score_psi,
        max_feature_psi,
        overfitting_penalty: automl_overfitting_penalty(
            score_psi,
            max_feature_psi,
            permutation_importance_passed,
            feature_reproducibility_passed,
        ),
        gate_status,
        blocking_reasons,
        recommended_action,
        evidence_refs,
    })
}

fn automl_blocking_reasons(metrics: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    let required_statuses = [
        "time_group_split_status",
        "leakage_check_status",
        "out_of_time_validation_status",
        "score_stability_status",
        "feature_stability_status",
        "overfitting_diagnostics_status",
        "shadow_comparison_status",
        "serving_version_lock_status",
        "artifact_integrity_status",
        "feature_store_materialization_status",
        "segment_fairness_status",
        "label_provenance_status",
    ];
    let mut reasons = Vec::new();
    for key in required_statuses {
        let status = metrics
            .get(key)
            .and_then(|value| value.as_str())
            .unwrap_or("missing");
        if status != "passed" {
            reasons.push(format!("{key}:{status}"));
        }
    }
    if metrics
        .get("rust_feature_set_status")
        .and_then(|value| value.as_str())
        != Some("passed")
    {
        reasons.push("rust_feature_set_status:missing_or_failed".into());
    }
    if !metrics
        .get("rust_feature_set_manifest_uri")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty())
    {
        reasons.push("rust_feature_set_manifest_uri:missing".into());
    }
    if !automl_feature_search_passed(metrics) {
        reasons.push("automl_feature_search_status:missing_or_failed".into());
    }
    if !automl_has_feature_search_report(metrics) {
        reasons.push("automl_feature_search_report_uri:missing".into());
    }
    if metric_object_value(metrics, "automl_selected_feature_count").unwrap_or(0.0) <= 0.0 {
        reasons.push("automl_selected_feature_count:missing_or_zero".into());
    }
    if !automl_factor_ranking_passed(metrics) {
        reasons.push("automl_factor_ranking_status:missing_or_failed".into());
    }
    if !automl_has_factor_ranking_report(metrics) {
        reasons.push("automl_factor_ranking_report_uri:missing".into());
    }
    if metric_object_value(metrics, "automl_ranked_factor_count").unwrap_or(0.0) <= 0.0 {
        reasons.push("automl_ranked_factor_count:missing_or_zero".into());
    }
    if !automl_rust_serving_evaluation_passed(metrics) {
        reasons.push("model_artifact_evaluation_status:missing_or_failed".into());
    }
    if automl_requires_onnx_parity(metrics) && !automl_onnx_parity_passed(metrics) {
        reasons.push("onnx_parity_status:missing_or_failed".into());
    }
    if automl_requires_onnx_parity(metrics)
        && !metrics
            .get("onnx_parity_report_uri")
            .and_then(|value| value.as_str())
            .is_some_and(|value| !value.trim().is_empty())
    {
        reasons.push("onnx_parity_report_uri:missing".into());
    }
    if metric_object_value(metrics, "out_of_time_auc").unwrap_or(0.0) < 0.5 {
        reasons.push("out_of_time_auc:below_0_5".into());
    }
    if metric_object_value(metrics, "out_of_time_recall").unwrap_or(0.0) <= 0.0 {
        reasons.push("out_of_time_recall:missing_or_zero".into());
    }
    if !automl_has_time_group_split_fields(metrics) {
        reasons.push("time_group_split_fields:missing".into());
    }
    if !automl_permutation_importance_passed(metrics) {
        reasons.push("permutation_importance_status:missing_or_failed".into());
    }
    if !automl_has_permutation_importance(metrics) {
        reasons.push("permutation_importance_uri:missing".into());
    }
    if !metrics
        .get("overfitting_diagnostics_report_uri")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty())
    {
        reasons.push("overfitting_diagnostics_report_uri:missing".into());
    }
    if metric_object_value(metrics, "score_psi")
        .or_else(|| metric_object_value(metrics, "psi"))
        .is_none()
    {
        reasons.push("score_psi:missing".into());
    }
    if metric_object_value(metrics, "max_feature_psi").is_none() {
        reasons.push("max_feature_psi:missing".into());
    }
    if metric_object_value(metrics, "score_psi")
        .or_else(|| metric_object_value(metrics, "psi"))
        .is_some_and(|value| value >= 0.25)
    {
        reasons.push("score_psi:drift".into());
    }
    if metric_object_value(metrics, "max_feature_psi").is_some_and(|value| value >= 0.25) {
        reasons.push("max_feature_psi:drift".into());
    }
    if !automl_feature_reproducibility_passed(metrics) {
        reasons.push("feature_reproducibility_hash:missing".into());
    }
    reasons
}

fn automl_has_time_group_split_fields(
    metrics: &serde_json::Map<String, serde_json::Value>,
) -> bool {
    let has_time = metrics
        .get("time_split_field")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty());
    let has_group = metrics
        .get("group_split_fields")
        .and_then(|value| value.as_array())
        .is_some_and(|fields| {
            fields
                .iter()
                .any(|field| field.as_str().is_some_and(|value| !value.trim().is_empty()))
        });
    has_time && has_group
}

fn automl_permutation_importance_passed(
    metrics: &serde_json::Map<String, serde_json::Value>,
) -> bool {
    metrics
        .get("permutation_importance_status")
        .and_then(|value| value.as_str())
        == Some("passed")
}

fn automl_has_permutation_importance(metrics: &serde_json::Map<String, serde_json::Value>) -> bool {
    metrics
        .get("permutation_importance_uri")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty())
}

fn automl_feature_reproducibility_passed(
    metrics: &serde_json::Map<String, serde_json::Value>,
) -> bool {
    metrics
        .get("feature_reproducibility_hash")
        .and_then(|value| value.as_str())
        .is_some_and(|value| value.starts_with("sha256:") && value.len() > "sha256:".len())
}

fn automl_feature_search_passed(metrics: &serde_json::Map<String, serde_json::Value>) -> bool {
    metrics
        .get("automl_feature_search_status")
        .and_then(|value| value.as_str())
        == Some("passed")
}

fn automl_has_feature_search_report(metrics: &serde_json::Map<String, serde_json::Value>) -> bool {
    metrics
        .get("automl_feature_search_report_uri")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty())
}

fn automl_factor_ranking_passed(metrics: &serde_json::Map<String, serde_json::Value>) -> bool {
    metrics
        .get("automl_factor_ranking_status")
        .and_then(|value| value.as_str())
        == Some("passed")
}

fn automl_has_factor_ranking_report(metrics: &serde_json::Map<String, serde_json::Value>) -> bool {
    metrics
        .get("automl_factor_ranking_report_uri")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty())
}

fn automl_requires_onnx_parity(metrics: &serde_json::Map<String, serde_json::Value>) -> bool {
    matches!(
        metrics.get("algorithm").and_then(|value| value.as_str()),
        Some("xgboost" | "lightgbm")
    ) || matches!(
        metrics.get("runtime_kind").and_then(|value| value.as_str()),
        Some("xgboost_onnx" | "lightgbm_onnx" | "deep_learning_onnx")
    )
}

fn automl_onnx_parity_passed(metrics: &serde_json::Map<String, serde_json::Value>) -> bool {
    metrics
        .get("onnx_parity_gate_status")
        .and_then(|value| value.as_str())
        == Some("passed")
        || metrics
            .get("onnx_parity_status")
            .and_then(|value| value.as_str())
            == Some("passed")
}

fn automl_rust_serving_evaluation_passed(
    metrics: &serde_json::Map<String, serde_json::Value>,
) -> bool {
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
    metrics
        .get("model_artifact_evaluation")
        .is_some_and(|value| {
            value.get("report_kind").and_then(|value| value.as_str())
                == Some("model_artifact_evaluation")
                && value.get("gate_status").and_then(|value| value.as_str()) == Some("passed")
        })
}

fn automl_ranking_score(
    out_of_time_auc: Option<f64>,
    average_precision: Option<f64>,
    precision: Option<f64>,
    recall: Option<f64>,
    score_psi: Option<f64>,
    max_feature_psi: Option<f64>,
    permutation_importance_passed: bool,
    feature_reproducibility_passed: bool,
    gate_status: &str,
) -> f64 {
    let score = out_of_time_auc.unwrap_or(0.0) * 60.0
        + average_precision.unwrap_or(0.0) * 20.0
        + precision.unwrap_or(0.0) * 10.0
        + recall.unwrap_or(0.0) * 10.0;
    let penalty = automl_overfitting_penalty(
        score_psi,
        max_feature_psi,
        permutation_importance_passed,
        feature_reproducibility_passed,
    ) + if gate_status == "passed" { 0.0 } else { 100.0 };
    ((score - penalty) * 10_000.0).round() / 10_000.0
}

fn automl_overfitting_penalty(
    score_psi: Option<f64>,
    max_feature_psi: Option<f64>,
    permutation_importance_passed: bool,
    feature_reproducibility_passed: bool,
) -> f64 {
    let stability_penalty = score_psi.unwrap_or(1.0) * 25.0 + max_feature_psi.unwrap_or(1.0) * 15.0;
    let permutation_penalty = if permutation_importance_passed {
        0.0
    } else {
        20.0
    };
    let reproducibility_penalty = if feature_reproducibility_passed {
        0.0
    } else {
        20.0
    };
    ((stability_penalty + permutation_penalty + reproducibility_penalty) * 10_000.0).round()
        / 10_000.0
}

fn eligible_sort_key(candidate: &AutoMlCandidateRank) -> f64 {
    if candidate.gate_status == "passed" {
        candidate.ranking_score
    } else {
        candidate.ranking_score - 1_000.0
    }
}

pub(crate) fn metric_at(value: &serde_json::Value, key: &str) -> Option<f64> {
    value.get(key).and_then(metric_value)
}

fn metric_object_value(
    metrics: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<f64> {
    metrics.get(key).and_then(metric_value)
}

fn metric_value(value: &serde_json::Value) -> Option<f64> {
    if let Some(value) = value.as_f64() {
        return Some(value);
    }
    value.as_str().and_then(|value| value.parse::<f64>().ok())
}
