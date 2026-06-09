use anyhow::{anyhow, bail, Context};
use arrow_array::RecordBatch;
#[cfg(test)]
use arrow_array::{Float64Array, Int8Array, StringArray};
use arrow_schema::Schema;
#[cfg(test)]
use arrow_schema::{DataType, Field};
use hmac::Hmac;
use parquet::arrow::ArrowWriter;
use serde::Serialize;
use sha2::Sha256;
use std::{
    collections::BTreeSet,
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

type HmacSha256 = Hmac<Sha256>;

mod dataset_types;
pub use dataset_types::*;

mod demo_datasets;
pub use demo_datasets::build_demo_ml_datasets;

mod demo_lifecycle;
pub use demo_lifecycle::{build_demo_automl_lifecycle_evidence, verify_demo_automl_lifecycle};

mod dataset_profile;
pub use dataset_profile::{profile_manifest, profile_manifest_file};

mod feature_set;
pub use feature_set::build_feature_set;

mod health;
pub use health::{
    build_pilot_readiness_report, check_pilot_readiness, worker_health, ApiHealthCheck,
    ApiHealthResponse, ApiPilotReadiness, PilotReadinessReport, WorkerHealthCheck,
    WorkerHealthResponse,
};

mod mlops_delivery;
pub use mlops_delivery::{
    build_mlops_alert_delivery_submission, build_mlops_alert_receiver_payload,
    build_mlops_monitoring_report_submission, deliver_mlops_alert_receiver_webhook,
    submit_mlops_alert_delivery_tasks, submit_mlops_monitoring_report,
    MlopsAlertDeliverySubmission, MlopsMonitoringReportSubmission,
};

mod anomaly_clustering;
pub use anomaly_clustering::{
    build_anomaly_clustering_report_submission, submit_anomaly_clustering_report,
    AnomalyClusteringReportSubmission, AnomalyClusteringReviewTaskSubmission,
};

mod alertmanager;
#[cfg(test)]
pub(crate) use alertmanager::alertmanager_webhook_is_authorized;
pub use alertmanager::{
    build_alertmanager_mlops_alert_delivery_submission, serve_mlops_alert_router,
    submit_alertmanager_webhook_to_fwa, AlertmanagerAlert, AlertmanagerWebhook,
    MlopsAlertRouterConfig,
};

mod mlops_monitoring;
pub use mlops_monitoring::{
    build_mlops_monitoring_plan, build_mlops_monitoring_report,
    build_mlops_scheduler_execution_report, run_mlops_monitoring_plan,
    run_mlops_monitoring_plan_with_inputs, run_scheduled_mlops_monitoring,
    run_scheduled_mlops_monitoring_with_artifact_base_uri,
    run_scheduled_mlops_monitoring_with_options,
};
#[cfg(test)]
pub(crate) use mlops_monitoring_runtime::sha256_prefixed_hex;
mod mlops_monitoring_runtime;

mod mlops_cycle;
pub use mlops_cycle::{build_mlops_monitoring_cycle_evidence, run_mlops_monitoring_cycle};

mod model_artifact_evaluation;
pub use model_artifact_evaluation::evaluate_model_artifact;

mod rule_candidates;
pub use rule_candidates::{mine_rule_candidates, run_rule_candidate_backtest};

mod clustering;
mod clustering_data;
mod clustering_math;
mod clustering_types;
pub use clustering::{
    cluster_claim_entities, cluster_provider_graph_communities, cluster_provider_peers,
};
pub use clustering_types::{
    ClaimEntityAnomalyCandidate, ClaimEntityClusterAssignment, ClaimEntityClusterSummary,
    ClaimEntityClusteringReport, ClaimEntityReviewTask, ProviderGraphAnomalyCandidate,
    ProviderGraphCommunityAssignment, ProviderGraphCommunityReport, ProviderGraphCommunitySummary,
    ProviderGraphReviewTask, ProviderPeerAnomalyCandidate, ProviderPeerClusterAssignment,
    ProviderPeerClusterSummary, ProviderPeerClusteringReport, ProviderPeerReviewTask,
    UnsupervisedFactorRank, UnsupervisedFactorRanking,
};

mod automl_lifecycle;
pub use automl_lifecycle::{
    build_automl_lifecycle_closure_report, build_model_promotion_orchestration_report,
};

mod automl_ranking;
pub use automl_ranking::rank_automl_candidates;
pub(crate) use automl_ranking::{metric_at, read_feature_importance, round4};

mod ops_plans;
pub use ops_plans::{
    build_ai_evidence_execution_plan, build_analytics_export_plan, build_governance_ops_plan,
};

mod parquet_utils;
pub(crate) use parquet_utils::{
    column_value_at, column_values, ensure_parquet_path, resolve_parquet_files,
};

mod training_handoff;
pub(crate) use training_handoff::build_training_command;
pub use training_handoff::{build_training_handoff, build_training_handoff_with_algorithm};

mod retraining;
pub use retraining::{
    claim_next_retraining_job, complete_retraining_job_with_mock_output,
    complete_retraining_job_with_training_output, promote_approved_model_version,
    run_one_retraining_job, update_retraining_job_status, AutoMlCandidateRank,
    AutoMlCandidateRanking, AutoMlReviewTask, ClaimedRetrainingJob, ModelArtifactEvaluationReport,
    ModelArtifactEvaluationSample, PromoteApprovedModelVersionResult, RuleCandidateBacktestReport,
    RuleCandidateBacktestRequest, RuleCandidateBacktestResult, RuleCandidateBacktestReviewTask,
    RuleCandidateDraft, RuleCandidateMiningPlan, RuleCandidateReviewTask,
    RuleCandidateSplitMetrics,
};
pub(crate) use retraining::{
    CompleteRetrainingJobPayload, FeatureImportanceRow, ModelArtifactEvaluationRow,
    RuleBacktestRow, TrainingCommand, WorkerServingManifest,
};

fn write_parquet(path: PathBuf, schema: Arc<Schema>, batch: &RecordBatch) -> anyhow::Result<()> {
    let file = File::create(&path).with_context(|| format!("create parquet {}", path.display()))?;
    let mut writer = ArrowWriter::try_new(file, schema, None)
        .with_context(|| format!("open parquet writer {}", path.display()))?;
    writer
        .write(batch)
        .with_context(|| format!("write parquet batch {}", path.display()))?;
    writer
        .close()
        .with_context(|| format!("close parquet writer {}", path.display()))?;
    Ok(())
}

fn write_json(path: PathBuf, value: &impl Serialize) -> anyhow::Result<()> {
    fs::write(&path, serde_json::to_string_pretty(value)?)
        .with_context(|| format!("write json {}", path.display()))
}

fn reject_csv_uri(uri: &str) -> anyhow::Result<()> {
    if uri.to_ascii_lowercase().contains(".csv") {
        bail!("parquet profiler rejects csv data_uri: {uri}");
    }
    Ok(())
}

fn api_url(base_url: &str, path: &str) -> String {
    format!("{}{}", base_url.trim_end_matches('/'), path)
}

pub(crate) fn retraining_job_status_path(job_id: &str) -> String {
    format!("/api/v1/ops/model-retraining-jobs/{job_id}/status")
}

pub(crate) fn retraining_job_output_path(job_id: &str) -> String {
    format!("/api/v1/ops/model-retraining-jobs/{job_id}/output")
}

fn lifecycle_stage(
    stage: &str,
    passed: bool,
    summary: String,
    evidence_refs: Vec<String>,
) -> serde_json::Value {
    serde_json::json!({
        "stage": stage,
        "status": if passed { "passed" } else { "missing_or_blocked" },
        "summary": summary,
        "evidence_refs": evidence_refs
    })
}

fn unsupervised_factor_ranking_passed(report: &serde_json::Value, report_kind: &str) -> bool {
    let Some(factor_ranking) = report.get("factor_ranking") else {
        return false;
    };
    factor_ranking["report_kind"] == report_kind
        && factor_ranking
            .get("ranked_factor_count")
            .and_then(|value| value.as_u64())
            .is_some_and(|count| count > 0)
        && json_array_len(factor_ranking, "ranked_factors") > 0
}

fn json_array_len(value: &serde_json::Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(|value| value.as_array())
        .map(|items| items.len())
        .unwrap_or(0)
}

fn read_json_report(uri: &str) -> anyhow::Result<serde_json::Value> {
    let path = Path::new(uri);
    let report_json =
        fs::read_to_string(path).with_context(|| format!("read report {}", path.display()))?;
    serde_json::from_str(&report_json).with_context(|| format!("parse report {}", path.display()))
}

fn nested_json_string(value: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(key)?;
    }
    current
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn nested_json_array_contains(value: &serde_json::Value, path: &[&str], expected: &str) -> bool {
    let mut current = value;
    for key in path {
        let Some(next) = current.get(key) else {
            return false;
        };
        current = next;
    }
    current
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
}

fn json_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn json_u64(value: &serde_json::Value, key: &str) -> Option<u64> {
    value.get(key).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_str().and_then(|value| value.parse::<u64>().ok()))
    })
}

fn required_non_empty<'a>(field: &str, value: &'a str) -> anyhow::Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        bail!("{field} is required");
    }
    Ok(value)
}

fn required_optional<'a>(field: &str, value: Option<&'a str>) -> anyhow::Result<&'a str> {
    value
        .map(|value| required_non_empty(field, value))
        .transpose()?
        .with_context(|| format!("{field} is required when API submission is requested"))
}

fn artifact_parent_uri(artifact_uri: &str) -> &str {
    artifact_uri
        .trim()
        .rsplit_once('/')
        .map(|(parent, _)| parent)
        .unwrap_or_else(|| artifact_uri.trim())
}

fn artifact_parent_path(artifact_uri: &str) -> PathBuf {
    let parent_uri = artifact_parent_uri(artifact_uri);
    let local_parent = parent_uri
        .strip_prefix("artifact://")
        .or_else(|| parent_uri.strip_prefix("file://"))
        .unwrap_or(parent_uri);
    PathBuf::from(local_parent)
}

fn required_manifest_str<'a>(
    manifest: &'a serde_json::Value,
    key: &str,
) -> anyhow::Result<&'a str> {
    manifest
        .get(key)
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("training manifest missing {key}"))
}

pub(crate) fn build_training_retraining_output(
    job: &ClaimedRetrainingJob,
    actor: &str,
    artifact_base_uri: &str,
    training_manifest: &str,
    trainer_python: &str,
    trainer_workdir: Option<&str>,
    algorithm: Option<&str>,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    let training_command = build_training_command(
        trainer_python,
        training_manifest,
        artifact_base_uri,
        job,
        actor,
        trainer_workdir,
        algorithm,
    );
    let mut command = Command::new(&training_command.program);
    command.args(&training_command.args);
    if let Some(workdir) = &training_command.workdir {
        command.current_dir(workdir);
    }
    let output = command
        .output()
        .with_context(|| format!("run model training command {}", training_command.program))?;
    if !output.status.success() {
        bail!(
            "model training command failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = serde_json::from_slice::<CompleteRetrainingJobPayload>(&output.stdout)
        .context("parse model training output")?;
    enrich_retraining_output_with_rust_feature_set(output, training_manifest)
}

fn enrich_retraining_output_with_rust_feature_set(
    mut output: CompleteRetrainingJobPayload,
    training_manifest: &str,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    let feature_set_output_dir =
        artifact_parent_path(&output.artifact_uri).join("rust_feature_set");
    let feature_set_id = format!(
        "{}:{}",
        output.candidate_model_version, "rust_feature_set_v1"
    );
    let feature_set = build_feature_set(
        training_manifest,
        &feature_set_output_dir,
        Some(&feature_set_id),
    )
    .context("build Rust feature set for retraining output")?;
    let feature_set_manifest_uri = feature_set_output_dir
        .join("feature_set_manifest.json")
        .to_string_lossy()
        .into_owned();

    let Some(metrics) = output.metrics_json.as_object_mut() else {
        bail!("training output metrics_json must be an object");
    };
    if let Some(existing_hash) = metrics
        .get("feature_reproducibility_hash")
        .and_then(|value| value.as_str())
    {
        metrics.insert(
            "trainer_feature_reproducibility_hash".into(),
            serde_json::Value::String(existing_hash.to_string()),
        );
    }
    metrics.insert(
        "feature_reproducibility_hash".into(),
        serde_json::Value::String(feature_set.feature_reproducibility_hash.clone()),
    );
    metrics.insert(
        "rust_feature_set_manifest_uri".into(),
        serde_json::Value::String(feature_set_manifest_uri.clone()),
    );
    metrics.insert(
        "rust_feature_set_status".into(),
        serde_json::Value::String("passed".into()),
    );
    metrics.insert(
        "feature_store_materialization_status".into(),
        serde_json::Value::String("passed".into()),
    );

    let evidence_ref = format!("feature_set_manifests:{feature_set_manifest_uri}");
    if !output
        .evidence_refs
        .iter()
        .any(|reference| reference == &evidence_ref)
    {
        output.evidence_refs.push(evidence_ref);
    }
    Ok(output)
}

pub(crate) async fn enrich_retraining_output_with_model_artifact_evaluation(
    mut output: CompleteRetrainingJobPayload,
    training_manifest: &str,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    let Some(serving_manifest_uri) = output.serving_manifest_uri.clone() else {
        return Ok(output);
    };
    let artifact_eval_output_dir =
        artifact_parent_path(&output.artifact_uri).join("artifact-evaluation");
    let report = evaluate_model_artifact(
        &serving_manifest_uri,
        training_manifest,
        "validation",
        &artifact_eval_output_dir,
        None,
        0.0001,
        100,
        100,
        Some(&model_artifact_evaluation_signing_key()),
    )
    .await
    .context("evaluate Rust serving artifact before retraining registration")?;
    let onnx_parity = validate_onnx_parity_for_runtime(
        report.runtime_kind.as_str(),
        output.onnx_parity_report_uri.as_deref(),
    )?;
    let gate_status = if report.gate_status == "passed"
        && onnx_parity
            .as_ref()
            .is_none_or(|parity| parity.gate_status == "passed")
    {
        "passed".to_string()
    } else {
        "blocked".to_string()
    };
    let report_uri = artifact_eval_output_dir
        .join("model_artifact_evaluation_report.json")
        .to_string_lossy()
        .into_owned();

    let Some(metrics) = output.metrics_json.as_object_mut() else {
        bail!("training output metrics_json must be an object");
    };
    metrics.insert(
        "model_artifact_evaluation_status".into(),
        serde_json::Value::String(gate_status.clone()),
    );
    metrics.insert(
        "model_artifact_evaluation_gate_status".into(),
        serde_json::Value::String(gate_status.clone()),
    );
    metrics.insert(
        "model_artifact_evaluation_report_uri".into(),
        serde_json::Value::String(report_uri.clone()),
    );
    metrics.insert(
        "rust_serving_status".into(),
        serde_json::Value::String(report.rust_serving_status),
    );
    metrics.insert(
        "rust_serving_latency_status".into(),
        serde_json::Value::String(report.latency_status),
    );
    metrics.insert(
        "rust_serving_p95_latency_ms".into(),
        serde_json::json!(report.p95_latency_ms),
    );
    metrics.insert(
        "rust_serving_runtime_kind".into(),
        serde_json::Value::String(report.runtime_kind),
    );
    if let Some(parity) = &onnx_parity {
        metrics.insert(
            "onnx_parity_gate_status".into(),
            serde_json::Value::String(parity.gate_status.clone()),
        );
        metrics.insert(
            "onnx_parity_status".into(),
            serde_json::Value::String(parity.status.clone()),
        );
        metrics.insert(
            "onnx_parity_report_uri".into(),
            serde_json::Value::String(parity.report_uri.clone()),
        );
        metrics.insert(
            "onnx_serving_runtime_kind".into(),
            serde_json::Value::String(parity.serving_runtime_kind.clone()),
        );
        metrics.insert(
            "onnx_max_abs_probability_delta".into(),
            serde_json::json!(parity.max_abs_probability_delta),
        );
        metrics.insert(
            "onnx_probability_tolerance".into(),
            serde_json::json!(parity.tolerance),
        );
    }

    let evidence_ref = format!("model_artifact_evaluations:{report_uri}");
    if !output
        .evidence_refs
        .iter()
        .any(|reference| reference == &evidence_ref)
    {
        output.evidence_refs.push(evidence_ref);
    }
    if let Some(parity) = onnx_parity {
        let evidence_ref = format!("model_onnx_parity_reports:{}", parity.report_uri);
        if !output
            .evidence_refs
            .iter()
            .any(|reference| reference == &evidence_ref)
        {
            output.evidence_refs.push(evidence_ref);
        }
    }
    Ok(output)
}

pub(crate) fn enrich_retraining_output_with_rule_candidate_workflow(
    mut output: CompleteRetrainingJobPayload,
    training_manifest: &str,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    let Some(feature_importance_uri) = output.feature_importance_uri.clone() else {
        return Ok(output);
    };
    let training_platform_candidates = output.mined_rule_candidates.clone();
    let training_platform_candidate_count = training_platform_candidates.len();
    let rule_candidate_dir = artifact_parent_path(&output.artifact_uri).join("rule-candidates");
    let plan = mine_rule_candidates(
        &output.validation_report_uri,
        &feature_importance_uri,
        &rule_candidate_dir,
    )
    .context("mine explainable rule candidates before retraining output registration")?;
    let candidate_plan_uri = rule_candidate_dir
        .join("rule_candidate_mining_plan.json")
        .to_string_lossy()
        .into_owned();
    let candidate_review_tasks_uri = rule_candidate_dir
        .join("rule_candidate_review_tasks.json")
        .to_string_lossy()
        .into_owned();
    let backtest_dir = rule_candidate_dir.join("backtest");
    let backtest =
        run_rule_candidate_backtest(&candidate_plan_uri, training_manifest, &backtest_dir)
            .context(
                "backtest explainable rule candidates before retraining output registration",
            )?;
    let backtest_report_uri = backtest_dir
        .join("rule_candidate_backtest_report.json")
        .to_string_lossy()
        .into_owned();
    let backtest_review_tasks_uri = backtest_dir
        .join("rule_candidate_backtest_review_tasks.json")
        .to_string_lossy()
        .into_owned();
    let mut rule_candidates = training_platform_candidates;
    let mut existing_rule_ids = rule_candidates
        .iter()
        .filter_map(|candidate| {
            candidate
                .get("rule_id")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .collect::<BTreeSet<_>>();
    let backtested_rule_candidates = backtest
        .candidate_results
        .iter()
        .map(|result| result.rule_library_writeback_template.clone())
        .collect::<Vec<_>>();
    let backtested_rule_candidate_count = backtested_rule_candidates.len();
    for candidate in backtested_rule_candidates {
        let Some(rule_id) = candidate.get("rule_id").and_then(|value| value.as_str()) else {
            continue;
        };
        if existing_rule_ids.insert(rule_id.to_string()) {
            rule_candidates.push(candidate);
        }
    }
    output.mined_rule_candidates = rule_candidates;

    let Some(metrics) = output.metrics_json.as_object_mut() else {
        bail!("training output metrics_json must be an object");
    };
    metrics.insert(
        "rule_candidate_mining_status".into(),
        serde_json::Value::String("passed".into()),
    );
    metrics.insert(
        "rule_candidate_mining_plan_uri".into(),
        serde_json::Value::String(candidate_plan_uri.clone()),
    );
    metrics.insert(
        "rule_candidate_source_count".into(),
        serde_json::json!(plan.candidate_rules.len()),
    );
    metrics.insert(
        "rule_candidate_backtest_status".into(),
        serde_json::Value::String("passed".into()),
    );
    metrics.insert(
        "rule_candidate_backtest_report_uri".into(),
        serde_json::Value::String(backtest_report_uri.clone()),
    );
    metrics.insert(
        "rule_candidate_review_tasks_uri".into(),
        serde_json::Value::String(backtest_review_tasks_uri.clone()),
    );
    metrics.insert(
        "rule_candidate_review_task_count".into(),
        serde_json::json!(backtest.review_tasks.len()),
    );
    metrics.insert(
        "mined_rule_candidates_source".into(),
        serde_json::Value::String(
            "training_platform_and_deterministic_rule_candidate_backtest".into(),
        ),
    );
    metrics.insert(
        "training_platform_mined_rule_candidate_count".into(),
        serde_json::json!(training_platform_candidate_count),
    );
    metrics.insert(
        "mined_rule_candidates_backtested_count".into(),
        serde_json::json!(backtested_rule_candidate_count),
    );
    metrics.insert(
        "rule_library_writeback_status".into(),
        serde_json::Value::String(backtest.rule_library_writeback_status.clone()),
    );
    metrics.insert(
        "rule_candidate_workflow_boundary".into(),
        serde_json::Value::String(
            "rule candidates are backtested and handed to human review only; worker must not write active rules".into(),
        ),
    );

    push_unique_evidence_ref(
        &mut output.evidence_refs,
        format!("rule_candidate_mining_plans:{candidate_plan_uri}"),
    );
    push_unique_evidence_ref(
        &mut output.evidence_refs,
        format!("rule_candidate_review_tasks:{candidate_review_tasks_uri}"),
    );
    push_unique_evidence_ref(
        &mut output.evidence_refs,
        format!("rule_candidate_backtests:{backtest_report_uri}"),
    );
    push_unique_evidence_ref(
        &mut output.evidence_refs,
        format!("rule_candidate_review_tasks:{backtest_review_tasks_uri}"),
    );
    Ok(output)
}

fn push_unique_evidence_ref(evidence_refs: &mut Vec<String>, evidence_ref: String) {
    if !evidence_refs
        .iter()
        .any(|reference| reference == &evidence_ref)
    {
        evidence_refs.push(evidence_ref);
    }
}

#[derive(Debug, Clone, PartialEq)]
struct OnnxParityGate {
    report_uri: String,
    gate_status: String,
    status: String,
    serving_runtime_kind: String,
    max_abs_probability_delta: Option<f64>,
    tolerance: Option<f64>,
}

fn validate_onnx_parity_for_runtime(
    runtime_kind: &str,
    onnx_parity_report_uri: Option<&str>,
) -> anyhow::Result<Option<OnnxParityGate>> {
    if !runtime_kind.ends_with("_onnx") {
        return Ok(None);
    }
    let report_uri = onnx_parity_report_uri
        .filter(|uri| !uri.trim().is_empty())
        .ok_or_else(|| anyhow!("ONNX runtime {runtime_kind} requires onnx_parity_report_uri"))?;
    let report = read_json_report(report_uri)?;
    if report["report_kind"] != "onnx_probability_parity" {
        bail!("ONNX parity report {report_uri} has invalid report_kind");
    }
    let serving_runtime_kind =
        json_string(&report, "serving_runtime_kind").unwrap_or_else(|| "missing".into());
    let status = json_string(&report, "status").unwrap_or_else(|| "missing".into());
    let max_abs_probability_delta = metric_at(&report, "max_abs_probability_delta");
    let tolerance = metric_at(&report, "tolerance");
    let gate_status = if status == "passed" && serving_runtime_kind == runtime_kind {
        "passed"
    } else {
        "blocked"
    }
    .to_string();
    Ok(Some(OnnxParityGate {
        report_uri: report_uri.into(),
        gate_status,
        status,
        serving_runtime_kind,
        max_abs_probability_delta,
        tolerance,
    }))
}

fn model_artifact_evaluation_signing_key() -> String {
    std::env::var("FWA_MODEL_SIGNATURE_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "local-dev-model-signing-key".into())
}

pub(crate) fn build_mock_retraining_output(
    job: &ClaimedRetrainingJob,
    actor: &str,
    artifact_base_uri: &str,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    if artifact_base_uri.trim().is_empty() {
        bail!("artifact_base_uri is required");
    }
    let safe_model_key = safe_path_segment(&job.model_key);
    let candidate_model_version = format!(
        "{}-candidate-{}",
        safe_path_segment(&job.model_version),
        safe_path_segment(&job.job_id)
    );
    let artifact_root = artifact_base_uri.trim().trim_end_matches('/');
    let artifact_uri =
        format!("{artifact_root}/{safe_model_key}/{candidate_model_version}/model.onnx");
    let validation_report_uri =
        format!("{artifact_root}/{safe_model_key}/{candidate_model_version}/validation.json");
    let feature_importance_uri = format!(
        "{artifact_root}/{safe_model_key}/{candidate_model_version}/feature_importance.parquet"
    );
    let permutation_importance_uri = format!(
        "{artifact_root}/{safe_model_key}/{candidate_model_version}/permutation_importance.parquet"
    );
    let artifact_evaluation_report_uri = format!(
        "{artifact_root}/{safe_model_key}/{candidate_model_version}/artifact-evaluation/model_artifact_evaluation_report.json"
    );
    let rule_backtest_report_uri = format!(
        "{artifact_root}/{safe_model_key}/{candidate_model_version}/rule-candidates/backtest/rule_candidate_backtest_report.json"
    );
    let rule_review_tasks_uri = format!(
        "{artifact_root}/{safe_model_key}/{candidate_model_version}/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json"
    );
    let evaluation_run_id = format!(
        "eval_{}_{}",
        safe_id_segment(&job.model_key),
        safe_id_segment(&candidate_model_version)
    );
    let evidence_refs = vec![
        format!("model_retraining_jobs:{}", job.job_id),
        format!("model_artifacts:{artifact_uri}"),
        format!("model_validation_reports:{validation_report_uri}"),
        format!("model_feature_importance:{feature_importance_uri}"),
        format!("model_permutation_importance:{permutation_importance_uri}"),
        format!("model_artifact_evaluations:{artifact_evaluation_report_uri}"),
        format!("rule_candidate_backtests:{rule_backtest_report_uri}"),
        format!("rule_candidate_review_tasks:{rule_review_tasks_uri}"),
        format!("model_evaluations:{evaluation_run_id}"),
    ];

    Ok(CompleteRetrainingJobPayload {
        actor: actor.to_string(),
        notes: "Candidate model and validation report registered by worker.".into(),
        candidate_model_version,
        artifact_uri,
        artifact_sha256: None,
        training_artifact_uri: None,
        training_artifact_sha256: None,
        serving_manifest_uri: None,
        onnx_parity_report_uri: None,
        endpoint_url: None,
        validation_report_uri,
        evaluation_run_id,
        auc: Some("0.86".into()),
        ks: Some("0.48".into()),
        precision: Some("0.78".into()),
        recall: Some("0.71".into()),
        f1: Some("0.74".into()),
        accuracy: Some("0.79".into()),
        threshold: Some("0.52".into()),
        confusion_matrix_json: serde_json::json!({
            "tp": 24,
            "fp": 6,
            "tn": 52,
            "fn": 8
        }),
        feature_importance_uri: Some(feature_importance_uri),
        permutation_importance_uri: Some(permutation_importance_uri),
        metrics_json: serde_json::json!({
            "out_of_time_auc": 0.82,
            "out_of_time_precision": 0.76,
            "out_of_time_recall": 0.71,
            "score_psi": 0.04,
            "max_feature_psi": 0.08,
            "leakage_check_status": "passed",
            "time_group_split_status": "passed",
            "time_split_field": "service_date",
            "group_split_fields": ["member_id", "policy_id", "provider_id"],
            "feature_reproducibility_hash": "sha256:demo-retraining-feature-reproducibility",
            "label_provenance_status": "passed",
            "label_reviewer_source": "investigation_results",
            "pilot_validation_status": "passed",
            "shadow_comparison_status": "passed",
            "serving_version_lock_status": "passed",
            "artifact_integrity_status": "passed",
            "feature_store_materialization_status": "passed",
            "segment_fairness_status": "passed",
            "review_capacity_threshold_status": "passed",
            "model_artifact_evaluation_status": "passed",
            "model_artifact_evaluation_report_uri": artifact_evaluation_report_uri,
            "rule_candidate_backtest_status": "passed",
            "rule_candidate_backtest_report_uri": rule_backtest_report_uri,
            "rule_candidate_review_tasks_uri": rule_review_tasks_uri,
            "rule_library_writeback_status": "blocked_pending_human_review_and_policy_governance_approval"
        }),
        evidence_refs,
        mined_rule_owner: None,
        mined_rule_candidates: Vec::new(),
    })
}

fn safe_path_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if sanitized.is_empty() {
        "unknown".into()
    } else {
        sanitized
    }
}

fn safe_id_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if sanitized.is_empty() {
        "unknown".into()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests;
