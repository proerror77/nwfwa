use anyhow::{anyhow, bail, Context};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    fs::File,
    path::Path,
};

use super::{
    column_value_at, read_feature_importance, reject_csv_uri, required_manifest_str,
    resolve_parquet_files, safe_id_segment, write_json, ParquetDatasetManifest, RuleBacktestRow,
    RuleCandidateBacktestReport, RuleCandidateBacktestRequest, RuleCandidateBacktestResult,
    RuleCandidateBacktestReviewTask, RuleCandidateDraft, RuleCandidateMiningPlan,
    RuleCandidateReviewTask, RuleCandidateSplitMetrics,
};

pub fn mine_rule_candidates(
    validation_report: &str,
    feature_importance_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<RuleCandidateMiningPlan> {
    let validation_path = Path::new(validation_report);
    let report_json = fs::read_to_string(validation_path)
        .with_context(|| format!("read validation report {}", validation_path.display()))?;
    let report: serde_json::Value =
        serde_json::from_str(&report_json).context("parse validation report")?;
    let model_key = required_manifest_str(&report, "model_key")?.to_string();
    let candidate_model_version =
        required_manifest_str(&report, "candidate_model_version")?.to_string();
    let algorithm = required_manifest_str(&report, "algorithm")?.to_string();
    let feature_importance = read_feature_importance(Path::new(feature_importance_uri))?;
    if feature_importance.is_empty() {
        bail!("feature importance artifact contains no candidate features");
    }

    let candidate_rules = feature_importance
        .into_iter()
        .take(3)
        .map(|feature| {
            let candidate_rule_key = format!(
                "model_pattern_{}_{}",
                safe_id_segment(&candidate_model_version),
                safe_id_segment(&feature.feature)
            );
            RuleCandidateDraft {
                candidate_rule_key: candidate_rule_key.clone(),
                source_feature: feature.feature.clone(),
                source_importance: feature.importance,
                source_importance_kind: feature.importance_kind.clone(),
                draft_rule_template: serde_json::json!({
                    "rule_id": candidate_rule_key,
                    "version": 0,
                    "name": format!("Model pattern: {}", feature.feature),
                    "review_mode": "both",
                    "scheme_family": "high_risk_claim",
                    "conditions": [
                        {
                            "field": feature.feature,
                            "operator": "threshold_selected_by_backtest",
                            "value": {
                                "threshold_source": "run_rule_candidate_backtest_required"
                            }
                        }
                    ],
                    "action": {
                        "score": "selected_by_backtest",
                        "recommended_action": "manual_review",
                        "action_class": "score_only_or_manual_review_after_approval",
                        "required_evidence": [],
                        "reason": "Explainable model pattern candidate; not publishable before deterministic backtest and human approval."
                    }
                }),
                gate_status: "blocked_until_backtest_and_human_review".into(),
                required_before_rule_library_writeback: vec![
                    "deterministic_backtest".into(),
                    "false_positive_review".into(),
                    "human_rule_promotion_review".into(),
                    "customer_policy_or_model_governance_approval".into(),
                    "shadow_or_limited_rollout_if_high_impact".into(),
                ],
                evidence_refs: vec![
                    format!("model_validation_reports:{validation_report}"),
                    format!("model_feature_importance:{feature_importance_uri}"),
                    format!("model_evaluations:{}", safe_id_segment(&candidate_model_version)),
                ],
            }
        })
        .collect::<Vec<_>>();
    let backtest_requests = candidate_rules
        .iter()
        .map(|candidate| RuleCandidateBacktestRequest {
            candidate_rule_key: candidate.candidate_rule_key.clone(),
            backtest_kind: "deterministic_rule_candidate_backtest".into(),
            required_dataset_splits: vec![
                "train".into(),
                "validation".into(),
                "out_of_time".into(),
            ],
            minimum_evidence: vec![
                "hit_rate_by_split".into(),
                "precision_recall_by_split".into(),
                "false_positive_review".into(),
                "rule_only_baseline_comparison".into(),
                "manual_review_capacity_impact".into(),
            ],
            evidence_refs: candidate.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();
    let review_tasks = candidate_rules
        .iter()
        .map(|candidate| RuleCandidateReviewTask {
            task_kind: "rule_candidate_human_review".into(),
            candidate_rule_key: candidate.candidate_rule_key.clone(),
            review_queue: "rule_studio_candidate_review".into(),
            required_review: "human_approval_required_before_rule_library_writeback".into(),
            decision_options: vec![
                "reject".into(),
                "request_backtest_changes".into(),
                "approve_draft_for_backtest".into(),
            ],
            evidence_refs: candidate.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();
    let plan = RuleCandidateMiningPlan {
        plan_kind: "explainable_model_rule_candidate_mining".into(),
        plan_version: 1,
        source_model_key: model_key,
        source_candidate_model_version: candidate_model_version,
        source_algorithm: algorithm,
        promotion_boundary:
            "candidate rules are drafts only; backtest and human review are required before rule library writeback"
                .into(),
        candidate_rules,
        backtest_requests,
        review_tasks,
        evidence_refs: vec![
            format!("model_validation_reports:{validation_report}"),
            format!("model_feature_importance:{feature_importance_uri}"),
        ],
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create rule candidate mining output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir.as_ref().join("rule_candidate_mining_plan.json"),
        &plan,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("rule_candidate_backtest_requests.json"),
        &plan.backtest_requests,
    )?;
    write_json(
        output_dir.as_ref().join("rule_candidate_review_tasks.json"),
        &plan.review_tasks,
    )?;
    Ok(plan)
}

pub fn run_rule_candidate_backtest(
    candidate_plan: &str,
    dataset_manifest: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<RuleCandidateBacktestReport> {
    let candidate_plan_path = Path::new(candidate_plan);
    let plan_json = fs::read_to_string(candidate_plan_path).with_context(|| {
        format!(
            "read rule candidate mining plan {}",
            candidate_plan_path.display()
        )
    })?;
    let plan: RuleCandidateMiningPlan =
        serde_json::from_str(&plan_json).context("parse rule candidate mining plan")?;
    if plan.candidate_rules.is_empty() {
        bail!("candidate plan contains no rule candidates");
    }

    let manifest_path = Path::new(dataset_manifest);
    let manifest_json = fs::read_to_string(manifest_path)
        .with_context(|| format!("read dataset manifest {}", manifest_path.display()))?;
    let manifest: ParquetDatasetManifest =
        serde_json::from_str(&manifest_json).context("parse parquet dataset manifest")?;
    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let candidate_features = plan
        .candidate_rules
        .iter()
        .map(|candidate| candidate.source_feature.clone())
        .collect::<BTreeSet<_>>();
    let rows = read_rule_backtest_rows(&manifest, base_dir, &candidate_features)?;
    if rows.is_empty() {
        bail!("dataset manifest contains no rows for rule candidate backtest");
    }

    let candidate_results = plan
        .candidate_rules
        .iter()
        .map(|candidate| backtest_rule_candidate(candidate, &rows))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let review_tasks = candidate_results
        .iter()
        .map(|result| RuleCandidateBacktestReviewTask {
            task_kind: "rule_candidate_backtest_review".into(),
            candidate_rule_key: result.candidate_rule_key.clone(),
            review_queue: "rule_studio_candidate_review".into(),
            required_review: "human_approval_required_after_backtest_before_rule_library_writeback"
                .into(),
            decision_options: vec![
                "reject".into(),
                "request_threshold_or_feature_changes".into(),
                "approve_for_policy_governance_review".into(),
            ],
            evidence_refs: result.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();

    let report = RuleCandidateBacktestReport {
        report_kind: "deterministic_rule_candidate_backtest".into(),
        report_version: 1,
        source_plan_kind: plan.plan_kind,
        source_model_key: plan.source_model_key,
        source_candidate_model_version: plan.source_candidate_model_version,
        dataset_key: manifest.dataset_key,
        dataset_version: manifest.dataset_version,
        label_column: manifest.label_column,
        rule_library_writeback_status:
            "blocked_pending_human_review_and_policy_governance_approval".into(),
        candidate_results,
        review_tasks,
        evidence_refs: vec![
            format!("rule_candidate_mining_plan:{candidate_plan}"),
            format!("dataset_manifest:{dataset_manifest}"),
        ],
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create rule candidate backtest output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("rule_candidate_backtest_report.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("rule_candidate_backtest_review_tasks.json"),
        &report.review_tasks,
    )?;
    Ok(report)
}

fn read_rule_backtest_rows(
    manifest: &ParquetDatasetManifest,
    base_dir: &Path,
    candidate_features: &BTreeSet<String>,
) -> anyhow::Result<Vec<RuleBacktestRow>> {
    if candidate_features.is_empty() {
        bail!("rule candidate backtest requires at least one feature");
    }

    let mut rows = Vec::new();
    for split in &manifest.splits {
        reject_csv_uri(&split.data_uri)?;
        let parquet_files = resolve_parquet_files(base_dir, &split.data_uri)?;
        if parquet_files.is_empty() {
            bail!("split {} has no parquet files", split.split_name);
        }

        for parquet_file in parquet_files {
            let file = File::open(&parquet_file)
                .with_context(|| format!("open parquet file {}", parquet_file.display()))?;
            let builder = ParquetRecordBatchReaderBuilder::try_new(file)
                .with_context(|| format!("read parquet metadata {}", parquet_file.display()))?;
            let mut reader = builder.with_batch_size(4096).build()?;
            for batch in &mut reader {
                let batch = batch?;
                let label_index = batch
                    .schema()
                    .index_of(&manifest.label_column)
                    .with_context(|| format!("missing label column {}", manifest.label_column))?;
                let feature_indexes = candidate_features
                    .iter()
                    .map(|feature| {
                        batch
                            .schema()
                            .index_of(feature)
                            .with_context(|| format!("missing candidate feature column {feature}"))
                            .map(|index| (feature.clone(), index))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;

                for row_index in 0..batch.num_rows() {
                    let label = column_value_at(batch.column(label_index).as_ref(), row_index)
                        .and_then(|value| parse_label(&value))
                        .with_context(|| {
                            format!(
                                "missing or invalid label {} at row {}",
                                manifest.label_column, row_index
                            )
                        })?;
                    let mut features = BTreeMap::new();
                    for (feature, feature_index) in &feature_indexes {
                        if let Some(value) =
                            column_value_at(batch.column(*feature_index).as_ref(), row_index)
                                .and_then(|value| value.parse::<f64>().ok())
                        {
                            features.insert(feature.clone(), value);
                        }
                    }
                    rows.push(RuleBacktestRow {
                        split_name: split.split_name.clone(),
                        label,
                        features,
                    });
                }
            }
        }
    }
    Ok(rows)
}

fn backtest_rule_candidate(
    candidate: &RuleCandidateDraft,
    rows: &[RuleBacktestRow],
) -> anyhow::Result<RuleCandidateBacktestResult> {
    let train_rows = rows
        .iter()
        .filter(|row| row.split_name == "train")
        .collect::<Vec<_>>();
    let selection_rows = if train_rows.is_empty() {
        rows.iter().collect::<Vec<_>>()
    } else {
        train_rows
    };
    let threshold = select_threshold(&candidate.source_feature, &selection_rows)?;
    let mut split_names = rows
        .iter()
        .map(|row| row.split_name.clone())
        .collect::<BTreeSet<_>>();
    if split_names.is_empty() {
        split_names.insert("all".into());
    }

    let mut metrics_by_split = BTreeMap::new();
    for split_name in split_names {
        let split_rows = rows
            .iter()
            .filter(|row| row.split_name == split_name)
            .collect::<Vec<_>>();
        metrics_by_split.insert(
            split_name,
            compute_split_metrics(&candidate.source_feature, threshold, &split_rows),
        );
    }

    let condition_ref = format!("rule_conditions:{}_v1_c1", candidate.candidate_rule_key);
    Ok(RuleCandidateBacktestResult {
        candidate_rule_key: candidate.candidate_rule_key.clone(),
        source_feature: candidate.source_feature.clone(),
        selected_operator: ">=".into(),
        selected_threshold: threshold,
        threshold_selection_split: if rows.iter().any(|row| row.split_name == "train") {
            "train".into()
        } else {
            "all_rows".into()
        },
        rule_library_writeback_template: serde_json::json!({
            "rule_id": candidate.candidate_rule_key,
            "version": 1,
            "name": format!("Model pattern: {}", candidate.source_feature),
            "review_mode": "both",
            "scheme_family": "high_risk_claim",
            "conditions": [
                {
                    "field": candidate.source_feature,
                    "operator": ">=",
                    "value": threshold
                }
            ],
            "action": {
                "score": 20,
                "alert_code": format!("MODEL_PATTERN_{}", safe_id_segment(&candidate.source_feature).to_uppercase()),
                "recommended_action": "ManualReview",
                "action_class": "manual_review",
                "required_evidence": [],
                "reason": "Explainable model pattern candidate; not publishable before deterministic backtest, false-positive review, human approval, and policy governance approval."
            }
        }),
        condition_refs: vec![condition_ref.clone()],
        metrics_by_split,
        gate_status: "backtested_but_blocked_until_human_review".into(),
        required_before_rule_library_writeback: vec![
            "false_positive_review".into(),
            "human_rule_promotion_review".into(),
            "customer_policy_or_model_governance_approval".into(),
            "shadow_or_limited_rollout_if_high_impact".into(),
        ],
        evidence_refs: vec![
            format!("rule_candidate_backtest:{}", candidate.candidate_rule_key),
            condition_ref,
            format!("source_feature:{}", candidate.source_feature),
        ],
    })
}

fn select_threshold(feature: &str, rows: &[&RuleBacktestRow]) -> anyhow::Result<f64> {
    let mut thresholds = rows
        .iter()
        .filter_map(|row| row.features.get(feature).copied())
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    if thresholds.is_empty() {
        bail!("no numeric values available for candidate feature {feature}");
    }
    thresholds.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    thresholds.dedup_by(|left, right| (*left - *right).abs() < f64::EPSILON);

    thresholds
        .into_iter()
        .map(|threshold| {
            let metrics = compute_split_metrics(feature, threshold, rows);
            (threshold, metrics)
        })
        .max_by(
            |(left_threshold, left_metrics), (right_threshold, right_metrics)| {
                left_metrics
                    .f1
                    .partial_cmp(&right_metrics.f1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        left_metrics
                            .precision
                            .partial_cmp(&right_metrics.precision)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .then_with(|| {
                        right_threshold
                            .partial_cmp(left_threshold)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            },
        )
        .map(|(threshold, _)| threshold)
        .ok_or_else(|| anyhow!("no threshold selected for candidate feature {feature}"))
}

fn compute_split_metrics(
    feature: &str,
    threshold: f64,
    rows: &[&RuleBacktestRow],
) -> RuleCandidateSplitMetrics {
    let mut true_positive = 0_u64;
    let mut false_positive = 0_u64;
    let mut true_negative = 0_u64;
    let mut false_negative = 0_u64;

    for row in rows {
        let hit = row
            .features
            .get(feature)
            .is_some_and(|value| *value >= threshold);
        match (hit, row.label) {
            (true, true) => true_positive += 1,
            (true, false) => false_positive += 1,
            (false, true) => false_negative += 1,
            (false, false) => true_negative += 1,
        }
    }

    let row_count = rows.len() as u64;
    let hit_count = true_positive + false_positive;
    let positive_count = true_positive + false_negative;
    let precision = ratio(true_positive, true_positive + false_positive);
    let recall = ratio(true_positive, true_positive + false_negative);
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };

    RuleCandidateSplitMetrics {
        row_count,
        positive_count,
        hit_count,
        hit_rate: ratio(hit_count, row_count),
        true_positive,
        false_positive,
        true_negative,
        false_negative,
        precision,
        recall,
        f1,
        manual_review_capacity_impact: ratio(hit_count, row_count),
    }
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn parse_label(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "positive" => Some(true),
        "0" | "false" | "no" | "negative" => Some(false),
        _ => None,
    }
}
