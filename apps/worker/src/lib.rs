use anyhow::{anyhow, bail, Context};
use arrow_array::{Float64Array, Int32Array, Int8Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use fwa_core::{ClaimId, ScoringRunId};
use fwa_features::{FeatureMap, FeatureValue};
use fwa_ml_runtime::{ModelScoreRequest, ModelScorer, ServingManifestModelScorer};
use parquet::arrow::{arrow_reader::ParquetRecordBatchReaderBuilder, ArrowWriter};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WorkerHealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
    pub checks: Vec<WorkerHealthCheck>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WorkerHealthCheck {
    pub name: &'static str,
    pub status: &'static str,
}

pub fn worker_health() -> WorkerHealthResponse {
    WorkerHealthResponse {
        status: "ok",
        service: "worker",
        version: env!("CARGO_PKG_VERSION"),
        checks: vec![
            WorkerHealthCheck {
                name: "cli_commands",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "parquet_profiler",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "feature_set_builder",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "demo_ml_dataset_builder",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "automl_candidate_ranker",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "rule_candidate_miner",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "rule_candidate_backtester",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "provider_peer_clusterer",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "claim_entity_clusterer",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "provider_graph_clusterer",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "automl_lifecycle_closure_reporter",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "demo_automl_lifecycle_evidence_builder",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "mlops_monitoring_report_submitter",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "mlops_scheduler_execution_reporter",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "model_artifact_evaluator",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "retraining_job_runner",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "pilot_readiness_checker",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "analytics_export_plan",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "ai_evidence_execution_plan",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "governance_ops_plan",
                status: "ok",
            },
        ],
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ApiHealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
    pub pilot_readiness: ApiPilotReadiness,
    pub checks: Vec<ApiHealthCheck>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ApiPilotReadiness {
    pub status: String,
    #[serde(default)]
    pub required_check_names: Vec<String>,
    #[serde(default)]
    pub required_check_count: usize,
    #[serde(default)]
    pub ready_check_count: usize,
    #[serde(default)]
    pub blocking_check_count: usize,
    #[serde(default)]
    pub ready_checks: Vec<ApiHealthCheck>,
    #[serde(default)]
    pub blocking_checks: Vec<ApiHealthCheck>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ApiHealthCheck {
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PilotReadinessReport {
    pub status: String,
    pub ready_for_customer_pilot: bool,
    pub api_status: String,
    pub api_service: String,
    pub api_version: String,
    pub required_check_count: usize,
    pub ready_check_count: usize,
    pub blocking_check_count: usize,
    pub model_runtime_kind: Option<String>,
    pub ready_checks: Vec<ApiHealthCheck>,
    pub blocking_checks: Vec<ApiHealthCheck>,
    pub remediation_summary: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MlopsMonitoringReportSubmission {
    pub actor: String,
    pub notes: String,
    pub report_uri: String,
    pub report_kind: String,
    pub model_version: String,
    pub overall_status: String,
    pub retraining_recommendation: String,
    pub triggers: Vec<String>,
    pub review_tasks: Vec<serde_json::Value>,
    pub evidence_refs: Vec<String>,
}

pub async fn check_pilot_readiness(
    api_base_url: &str,
    api_key: Option<&str>,
) -> anyhow::Result<PilotReadinessReport> {
    let mut request = reqwest::Client::new().get(api_url(api_base_url, "/api/v1/health"));
    if let Some(api_key) = api_key {
        request = request.header("x-api-key", api_key);
    }
    let response = request.send().await.context("fetch API health")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("fetch API health failed with {status}: {body}");
    }
    let health = response
        .json::<ApiHealthResponse>()
        .await
        .context("parse API health response")?;
    Ok(build_pilot_readiness_report(health))
}

pub fn build_pilot_readiness_report(health: ApiHealthResponse) -> PilotReadinessReport {
    let model_runtime_kind = health
        .checks
        .iter()
        .find(|check| check.name == "model_scorer")
        .and_then(|check| check.runtime_kind.clone());
    let remediation_summary = health
        .pilot_readiness
        .blocking_checks
        .iter()
        .map(|check| {
            check
                .remediation
                .clone()
                .unwrap_or_else(|| format!("{}={}", check.name, check.status))
        })
        .collect::<Vec<_>>();
    let evidence_refs = vec![
        "api_health:/api/v1/health".to_string(),
        "pilot_readiness:/api/v1/health#pilot_readiness".to_string(),
    ];
    PilotReadinessReport {
        status: health.pilot_readiness.status.clone(),
        ready_for_customer_pilot: health.pilot_readiness.status == "ready"
            && health.pilot_readiness.blocking_checks.is_empty(),
        api_status: health.status,
        api_service: health.service,
        api_version: health.version,
        required_check_count: health.pilot_readiness.required_check_count,
        ready_check_count: health.pilot_readiness.ready_check_count,
        blocking_check_count: health.pilot_readiness.blocking_check_count,
        model_runtime_kind,
        ready_checks: health.pilot_readiness.ready_checks,
        blocking_checks: health.pilot_readiness.blocking_checks,
        remediation_summary,
        evidence_refs,
    }
}

pub fn build_mlops_monitoring_report_submission(
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<(String, MlopsMonitoringReportSubmission)> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    let report = read_json_report(report_uri)?;
    let model_key = json_string(&report, "model_key")
        .filter(|value| !value.trim().is_empty())
        .context("MLOps monitoring report requires model_key")?;
    let model_version = json_string(&report, "model_version")
        .filter(|value| !value.trim().is_empty())
        .context("MLOps monitoring report requires model_version")?;
    let report_kind = json_string(&report, "report_kind")
        .filter(|value| value == "mlops_monitoring_report")
        .context("MLOps monitoring report_kind must be mlops_monitoring_report")?;
    let overall_status = json_string(&report, "overall_status")
        .filter(|value| !value.trim().is_empty())
        .context("MLOps monitoring report requires overall_status")?;
    let retraining_recommendation = json_string(&report, "retraining_recommendation")
        .filter(|value| !value.trim().is_empty())
        .context("MLOps monitoring report requires retraining_recommendation")?;
    let triggers = report
        .get("triggers")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let review_tasks = report
        .get("review_tasks")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let mut evidence_refs = report
        .get("evidence_refs")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    evidence_refs.push(format!("model_versions:{model_key}:{model_version}"));
    evidence_refs.push(format!("model_monitoring_reports:{report_uri}"));
    evidence_refs.sort();
    evidence_refs.dedup();

    Ok((
        model_key,
        MlopsMonitoringReportSubmission {
            actor: actor.into(),
            notes: notes.into(),
            report_uri: report_uri.into(),
            report_kind,
            model_version,
            overall_status,
            retraining_recommendation,
            triggers,
            review_tasks,
            evidence_refs,
        },
    ))
}

pub async fn submit_mlops_monitoring_report(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    let (model_key, payload) = build_mlops_monitoring_report_submission(report_uri, actor, notes)?;
    let response = reqwest::Client::new()
        .post(api_url(
            api_base_url,
            &format!("/api/v1/ops/models/{model_key}/mlops-monitoring-reports"),
        ))
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .context("submit MLOps monitoring report")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("submit MLOps monitoring report failed with {status}: {body}");
    }
    response
        .json::<serde_json::Value>()
        .await
        .context("parse MLOps monitoring report response")
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClaimedRetrainingJob {
    pub job_id: String,
    pub model_key: String,
    pub model_version: String,
    pub status: String,
    pub updated_by: String,
    pub status_note: String,
}

#[derive(Debug, Serialize)]
struct ClaimRetrainingJobPayload<'a> {
    actor: &'a str,
    notes: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_key: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct UpdateRetrainingJobStatusPayload<'a> {
    status: &'a str,
    actor: &'a str,
    notes: &'a str,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CompleteRetrainingJobPayload {
    actor: String,
    notes: String,
    candidate_model_version: String,
    artifact_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    artifact_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    training_artifact_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    training_artifact_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    serving_manifest_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    onnx_parity_report_uri: Option<String>,
    endpoint_url: Option<String>,
    validation_report_uri: String,
    evaluation_run_id: String,
    auc: Option<String>,
    ks: Option<String>,
    precision: Option<String>,
    recall: Option<String>,
    f1: Option<String>,
    accuracy: Option<String>,
    threshold: Option<String>,
    confusion_matrix_json: serde_json::Value,
    feature_importance_uri: Option<String>,
    metrics_json: serde_json::Value,
    evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AutoMlCandidateRanking {
    pub plan_kind: String,
    pub plan_version: u8,
    pub promotion_boundary: String,
    pub generated_from_reports: Vec<String>,
    pub recommended_candidate_model_version: Option<String>,
    pub candidates: Vec<AutoMlCandidateRank>,
    pub review_tasks: Vec<AutoMlReviewTask>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AutoMlCandidateRank {
    pub rank: usize,
    pub model_key: String,
    pub candidate_model_version: String,
    pub algorithm: String,
    pub algorithm_family: String,
    pub dataset_key: Option<String>,
    pub dataset_version: Option<String>,
    pub validation_report_uri: String,
    pub ranking_score: f64,
    pub validation_auc: Option<f64>,
    pub out_of_time_auc: Option<f64>,
    pub out_of_time_average_precision: Option<f64>,
    pub out_of_time_precision: Option<f64>,
    pub out_of_time_recall: Option<f64>,
    pub gate_status: String,
    pub blocking_reasons: Vec<String>,
    pub recommended_action: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AutoMlReviewTask {
    pub task_kind: String,
    pub candidate_model_version: String,
    pub review_queue: String,
    pub required_review: String,
    pub decision_options: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ModelArtifactEvaluationReport {
    pub report_kind: String,
    pub report_version: u8,
    pub model_key: String,
    pub model_version: String,
    pub runtime_kind: String,
    pub serving_manifest_uri: String,
    pub dataset_key: String,
    pub dataset_version: String,
    pub evaluated_split: String,
    pub row_count: usize,
    pub contract_status: String,
    pub rust_serving_status: String,
    pub parity_status: String,
    pub latency_status: String,
    pub gate_status: String,
    pub max_abs_probability_delta: Option<f64>,
    pub average_abs_probability_delta: Option<f64>,
    pub p95_latency_ms: u64,
    pub latency_budget_ms: u64,
    pub blocking_reasons: Vec<String>,
    pub sample_results: Vec<ModelArtifactEvaluationSample>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ModelArtifactEvaluationSample {
    pub claim_id: String,
    pub score: u8,
    pub label: String,
    pub fraud_probability: Option<f64>,
    pub expected_probability: Option<f64>,
    pub abs_probability_delta: Option<f64>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct WorkerServingManifest {
    model_key: String,
    model_version: String,
    runtime_kind: String,
    artifact_uri: String,
    artifact_sha256: String,
    version_lock: String,
    feature_columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct ModelArtifactEvaluationRow {
    claim_id: String,
    features: BTreeMap<String, f64>,
    expected_probability: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RuleCandidateMiningPlan {
    pub plan_kind: String,
    pub plan_version: u8,
    pub source_model_key: String,
    pub source_candidate_model_version: String,
    pub source_algorithm: String,
    pub promotion_boundary: String,
    pub candidate_rules: Vec<RuleCandidateDraft>,
    pub backtest_requests: Vec<RuleCandidateBacktestRequest>,
    pub review_tasks: Vec<RuleCandidateReviewTask>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RuleCandidateDraft {
    pub candidate_rule_key: String,
    pub source_feature: String,
    pub source_importance: f64,
    pub source_importance_kind: String,
    pub draft_rule_template: serde_json::Value,
    pub gate_status: String,
    pub required_before_rule_library_writeback: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RuleCandidateBacktestRequest {
    pub candidate_rule_key: String,
    pub backtest_kind: String,
    pub required_dataset_splits: Vec<String>,
    pub minimum_evidence: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RuleCandidateReviewTask {
    pub task_kind: String,
    pub candidate_rule_key: String,
    pub review_queue: String,
    pub required_review: String,
    pub decision_options: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct FeatureImportanceRow {
    feature: String,
    importance: f64,
    importance_kind: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RuleCandidateBacktestReport {
    pub report_kind: String,
    pub report_version: u8,
    pub source_plan_kind: String,
    pub source_model_key: String,
    pub source_candidate_model_version: String,
    pub dataset_key: String,
    pub dataset_version: String,
    pub label_column: String,
    pub rule_library_writeback_status: String,
    pub candidate_results: Vec<RuleCandidateBacktestResult>,
    pub review_tasks: Vec<RuleCandidateBacktestReviewTask>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RuleCandidateBacktestResult {
    pub candidate_rule_key: String,
    pub source_feature: String,
    pub selected_operator: String,
    pub selected_threshold: f64,
    pub threshold_selection_split: String,
    pub metrics_by_split: BTreeMap<String, RuleCandidateSplitMetrics>,
    pub gate_status: String,
    pub required_before_rule_library_writeback: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RuleCandidateSplitMetrics {
    pub row_count: u64,
    pub positive_count: u64,
    pub hit_count: u64,
    pub hit_rate: f64,
    pub true_positive: u64,
    pub false_positive: u64,
    pub true_negative: u64,
    pub false_negative: u64,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub manual_review_capacity_impact: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RuleCandidateBacktestReviewTask {
    pub task_kind: String,
    pub candidate_rule_key: String,
    pub review_queue: String,
    pub required_review: String,
    pub decision_options: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct RuleBacktestRow {
    split_name: String,
    label: bool,
    features: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
struct TrainingCommand {
    program: String,
    args: Vec<String>,
}

pub async fn claim_next_retraining_job(
    api_base_url: &str,
    api_key: &str,
    actor: &str,
    model_key: Option<&str>,
    notes: &str,
) -> anyhow::Result<ClaimedRetrainingJob> {
    let response = reqwest::Client::new()
        .post(api_url(
            api_base_url,
            "/api/v1/ops/model-retraining-jobs/claim-next",
        ))
        .header("x-api-key", api_key)
        .json(&ClaimRetrainingJobPayload {
            actor,
            notes,
            model_key,
        })
        .send()
        .await
        .context("claim next model retraining job")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("claim next model retraining job failed with {status}: {body}");
    }
    response
        .json::<ClaimedRetrainingJob>()
        .await
        .context("parse claimed retraining job response")
}

pub async fn update_retraining_job_status(
    api_base_url: &str,
    api_key: &str,
    job_id: &str,
    status: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<ClaimedRetrainingJob> {
    let response = reqwest::Client::new()
        .post(api_url(api_base_url, &retraining_job_status_path(job_id)))
        .header("x-api-key", api_key)
        .json(&UpdateRetrainingJobStatusPayload {
            status,
            actor,
            notes,
        })
        .send()
        .await
        .with_context(|| format!("update model retraining job {job_id} status"))?;
    let response_status = response.status();
    if !response_status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("update model retraining job {job_id} status failed with {response_status}: {body}");
    }
    response
        .json::<ClaimedRetrainingJob>()
        .await
        .context("parse retraining job status response")
}

pub async fn complete_retraining_job_with_mock_output(
    api_base_url: &str,
    api_key: &str,
    job: &ClaimedRetrainingJob,
    actor: &str,
    artifact_base_uri: &str,
) -> anyhow::Result<serde_json::Value> {
    let output = build_mock_retraining_output(job, actor, artifact_base_uri)?;
    register_retraining_output(api_base_url, api_key, job, &output).await
}

async fn register_retraining_output(
    api_base_url: &str,
    api_key: &str,
    job: &ClaimedRetrainingJob,
    output: &CompleteRetrainingJobPayload,
) -> anyhow::Result<serde_json::Value> {
    let response = reqwest::Client::new()
        .post(api_url(
            api_base_url,
            &retraining_job_output_path(&job.job_id),
        ))
        .header("x-api-key", api_key)
        .json(&output)
        .send()
        .await
        .with_context(|| format!("register model retraining job {} output", job.job_id))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!(
            "register model retraining job {} output failed with {status}: {body}",
            job.job_id
        );
    }
    response
        .json::<serde_json::Value>()
        .await
        .context("parse retraining job output response")
}

pub async fn complete_retraining_job_with_training_output(
    api_base_url: &str,
    api_key: &str,
    job: &ClaimedRetrainingJob,
    actor: &str,
    artifact_base_uri: &str,
    training_manifest: &str,
    trainer_python: &str,
) -> anyhow::Result<serde_json::Value> {
    let output = build_training_retraining_output(
        job,
        actor,
        artifact_base_uri,
        training_manifest,
        trainer_python,
    )?;
    let output =
        enrich_retraining_output_with_model_artifact_evaluation(output, training_manifest).await?;
    register_retraining_output(api_base_url, api_key, job, &output).await
}

pub async fn run_one_retraining_job(
    api_base_url: &str,
    api_key: &str,
    actor: &str,
    model_key: Option<&str>,
    artifact_base_uri: &str,
    training_manifest: Option<&str>,
    trainer_python: &str,
) -> anyhow::Result<serde_json::Value> {
    let job = claim_next_retraining_job(
        api_base_url,
        api_key,
        actor,
        model_key,
        "Worker claimed retraining job.",
    )
    .await?;
    let validation_job = update_retraining_job_status(
        api_base_url,
        api_key,
        &job.job_id,
        "validation",
        actor,
        if training_manifest.is_some() {
            "Training pipeline completed; validation metrics are ready."
        } else {
            "Mock retraining completed; validation metrics are ready."
        },
    )
    .await?;
    if let Some(training_manifest) = training_manifest {
        complete_retraining_job_with_training_output(
            api_base_url,
            api_key,
            &validation_job,
            actor,
            artifact_base_uri,
            training_manifest,
            trainer_python,
        )
        .await
    } else {
        complete_retraining_job_with_mock_output(
            api_base_url,
            api_key,
            &validation_job,
            actor,
            artifact_base_uri,
        )
        .await
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ParquetDatasetManifest {
    pub source_key: Option<String>,
    pub display_name: Option<String>,
    pub owner: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub dataset_key: String,
    pub dataset_version: String,
    pub business_domain: String,
    pub sample_grain: String,
    pub label_column: String,
    #[serde(default)]
    pub entity_keys: Vec<String>,
    pub splits: Vec<ParquetSplitManifest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ParquetSplitManifest {
    pub split_name: String,
    pub data_uri: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DemoMlDatasetPack {
    pub pack_kind: String,
    pub dataset_version: String,
    pub output_dir: String,
    pub labeled_manifest_uri: String,
    pub unlabeled_manifest_uris: Vec<String>,
    pub dataset_manifests: Vec<DemoMlDatasetSummary>,
    pub governance_boundary: String,
    pub next_worker_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DemoMlDatasetSummary {
    pub dataset_key: String,
    pub sample_grain: String,
    pub label_policy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_column: Option<String>,
    pub manifest_uri: String,
    pub split_count: usize,
    pub row_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FeatureSetManifest {
    pub manifest_kind: String,
    pub manifest_version: u8,
    pub feature_set_id: String,
    pub dataset_key: String,
    pub dataset_version: String,
    pub source_manifest_uri: String,
    pub sample_grain: String,
    pub label_column: String,
    pub entity_keys: Vec<String>,
    pub feature_columns: Vec<FeatureSetColumn>,
    pub split_summaries: Vec<FeatureSetSplitSummary>,
    pub feature_reproducibility_hash: String,
    pub governance_boundary: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FeatureSetColumn {
    pub name: String,
    pub logical_type: String,
    pub nullable: bool,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FeatureSetSplitSummary {
    pub split_name: String,
    pub row_count: u64,
    pub positive_count: Option<u64>,
    pub negative_count: Option<u64>,
}

#[derive(Debug, Clone)]
struct DemoLabeledClaim {
    claim_id: &'static str,
    member_id: &'static str,
    policy_id: &'static str,
    provider_id: &'static str,
    service_date: &'static str,
    claim_amount: f64,
    amount_to_limit_ratio: f64,
    peer_percentile: f64,
    item_count: i32,
    high_cost_item_ratio: f64,
    provider_risk_tier: i32,
    diagnosis_procedure_mismatch: i8,
    confirmed_fwa: i8,
}

#[derive(Debug, Clone)]
struct DemoUnlabeledClaim {
    claim_id: &'static str,
    member_id: &'static str,
    policy_id: &'static str,
    provider_id: &'static str,
    service_date: &'static str,
    claim_amount: f64,
    amount_to_limit_ratio: f64,
    peer_percentile: f64,
    item_count: i32,
    high_cost_item_ratio: f64,
    provider_risk_tier: i32,
    diagnosis_procedure_mismatch: i8,
}

#[derive(Debug, Clone)]
struct DemoProviderPeerRow {
    provider_id: &'static str,
    cohort_key: &'static str,
    service_month: &'static str,
    claim_count: i32,
    avg_claim_amount: f64,
    high_cost_rate: f64,
    peer_z_score: f64,
    graph_degree: i32,
    community_id: i32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderPeerClusteringReport {
    pub report_kind: String,
    pub report_version: u8,
    pub dataset_key: String,
    pub dataset_version: String,
    pub algorithm: String,
    pub label_policy: String,
    pub governance_boundary: String,
    pub feature_columns: Vec<String>,
    pub cluster_count: usize,
    pub cluster_summaries: Vec<ProviderPeerClusterSummary>,
    pub provider_assignments: Vec<ProviderPeerClusterAssignment>,
    pub anomaly_candidates: Vec<ProviderPeerAnomalyCandidate>,
    pub review_tasks: Vec<ProviderPeerReviewTask>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderPeerClusterSummary {
    pub cluster_id: usize,
    pub provider_count: usize,
    pub average_outlier_score: f64,
    pub average_claim_count: f64,
    pub average_high_cost_rate: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderPeerClusterAssignment {
    pub provider_id: String,
    pub cohort_key: String,
    pub service_month: String,
    pub cluster_id: usize,
    pub outlier_score: f64,
    pub anomaly_candidate: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderPeerAnomalyCandidate {
    pub provider_id: String,
    pub cohort_key: String,
    pub service_month: String,
    pub cluster_id: usize,
    pub outlier_score: f64,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderPeerReviewTask {
    pub task_kind: String,
    pub provider_id: String,
    pub review_queue: String,
    pub required_review: String,
    pub decision_options: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClaimEntityClusteringReport {
    pub report_kind: String,
    pub report_version: u8,
    pub dataset_key: String,
    pub dataset_version: String,
    pub algorithm: String,
    pub label_policy: String,
    pub governance_boundary: String,
    pub feature_columns: Vec<String>,
    pub cluster_count: usize,
    pub cluster_summaries: Vec<ClaimEntityClusterSummary>,
    pub entity_assignments: Vec<ClaimEntityClusterAssignment>,
    pub anomaly_candidates: Vec<ClaimEntityAnomalyCandidate>,
    pub review_tasks: Vec<ClaimEntityReviewTask>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClaimEntityClusterSummary {
    pub cluster_id: usize,
    pub claim_count: usize,
    pub average_outlier_score: f64,
    pub average_claim_amount: f64,
    pub average_provider_degree: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClaimEntityClusterAssignment {
    pub claim_id: String,
    pub member_id: String,
    pub provider_id: String,
    pub cluster_id: usize,
    pub outlier_score: f64,
    pub anomaly_candidate: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClaimEntityAnomalyCandidate {
    pub claim_id: String,
    pub member_id: String,
    pub provider_id: String,
    pub cluster_id: usize,
    pub outlier_score: f64,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClaimEntityReviewTask {
    pub task_kind: String,
    pub claim_id: String,
    pub member_id: String,
    pub provider_id: String,
    pub review_queue: String,
    pub required_review: String,
    pub decision_options: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderGraphCommunityReport {
    pub report_kind: String,
    pub report_version: u8,
    pub dataset_key: String,
    pub dataset_version: String,
    pub algorithm: String,
    pub label_policy: String,
    pub governance_boundary: String,
    pub community_summaries: Vec<ProviderGraphCommunitySummary>,
    pub provider_assignments: Vec<ProviderGraphCommunityAssignment>,
    pub anomaly_candidates: Vec<ProviderGraphAnomalyCandidate>,
    pub review_tasks: Vec<ProviderGraphReviewTask>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderGraphCommunitySummary {
    pub community_id: i32,
    pub provider_count: usize,
    pub average_graph_degree: f64,
    pub average_peer_z_score: f64,
    pub anomaly_candidate_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderGraphCommunityAssignment {
    pub provider_id: String,
    pub community_id: i32,
    pub graph_degree: f64,
    pub peer_z_score: f64,
    pub anomaly_candidate: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderGraphAnomalyCandidate {
    pub provider_id: String,
    pub community_id: i32,
    pub graph_degree: f64,
    pub peer_z_score: f64,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderGraphReviewTask {
    pub task_kind: String,
    pub provider_id: String,
    pub community_id: i32,
    pub review_queue: String,
    pub required_review: String,
    pub decision_options: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct UnlabeledDatasetManifest {
    dataset_key: String,
    dataset_version: String,
    label_policy: String,
    #[serde(default)]
    label_column: Option<String>,
    splits: Vec<ParquetSplitManifest>,
}

#[derive(Debug, Clone)]
struct ProviderPeerFeatureRow {
    provider_id: String,
    cohort_key: String,
    service_month: String,
    claim_count: f64,
    avg_claim_amount: f64,
    high_cost_rate: f64,
    peer_z_score: f64,
    graph_degree: f64,
    community_id: i32,
}

#[derive(Debug, Clone)]
struct ClaimEntityFeatureRow {
    claim_id: String,
    member_id: String,
    provider_id: String,
    claim_amount: f64,
    amount_to_limit_ratio: f64,
    peer_percentile: f64,
    item_count: f64,
    high_cost_item_ratio: f64,
    provider_risk_tier: f64,
    diagnosis_procedure_mismatch: f64,
    member_degree: f64,
    provider_degree: f64,
}

pub fn build_demo_ml_datasets(
    output_dir: impl AsRef<Path>,
    dataset_version: &str,
) -> anyhow::Result<DemoMlDatasetPack> {
    let dataset_version = required_non_empty("dataset_version", dataset_version)?;
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)
        .with_context(|| format!("create demo ML dataset dir {}", output_dir.display()))?;

    let labeled_rows = demo_labeled_claims();
    let train_rows = labeled_rows[0..8].to_vec();
    let validation_rows = labeled_rows[8..12].to_vec();
    let out_of_time_rows = labeled_rows[12..16].to_vec();
    let labeled_dir = output_dir.join("labeled_claim_risk");
    write_labeled_split(&labeled_dir, "train", &train_rows)?;
    write_labeled_split(&labeled_dir, "validation", &validation_rows)?;
    write_labeled_split(&labeled_dir, "out_of_time", &out_of_time_rows)?;
    let labeled_manifest = serde_json::json!({
        "dataset_key": "rust_demo_claim_risk_labeled",
        "dataset_version": dataset_version,
        "business_domain": "health_fwa",
        "sample_grain": "claim",
        "display_name": "Rust Demo Claim Risk Labeled",
        "owner": "ml-ops",
        "description": "Rust-generated demo dataset for supervised model training, backtesting, and promotion-gate exercises.",
        "status": "demo_only",
        "label_column": "confirmed_fwa",
        "label_policy": "weak_rust_demo_label_not_production_evidence",
        "entity_keys": ["claim_id", "member_id", "policy_id", "provider_id"],
        "time_split_field": "service_date",
        "group_split_fields": ["member_id", "policy_id", "provider_id"],
        "splits": [
            {"split_name": "train", "data_uri": "split=train/"},
            {"split_name": "validation", "data_uri": "split=validation/"},
            {"split_name": "out_of_time", "data_uri": "split=out_of_time/"}
        ],
        "governance": {
            "allowed_uses": ["pipeline_validation", "backtest_contract_validation", "human_review_workflow_demo"],
            "blocked_uses": ["production_auto_deny", "customer_validation_claim", "production_roi_claim"]
        }
    });
    write_json(labeled_dir.join("manifest.json"), &labeled_manifest)?;

    let scoring_rows = demo_unlabeled_claims();
    let scoring_dir = output_dir.join("unlabeled_shadow_scoring");
    write_unlabeled_claim_split(&scoring_dir, "scoring", &scoring_rows)?;
    let scoring_manifest = serde_json::json!({
        "dataset_key": "rust_demo_claim_shadow_unlabeled",
        "dataset_version": dataset_version,
        "business_domain": "health_fwa",
        "sample_grain": "claim",
        "display_name": "Rust Demo Claim Shadow Scoring Unlabeled",
        "owner": "ml-ops",
        "description": "Rust-generated unlabeled claims for shadow scoring and drift exercises.",
        "status": "demo_only",
        "label_policy": "unlabeled_shadow_scoring_only",
        "entity_keys": ["claim_id", "member_id", "policy_id", "provider_id"],
        "time_split_field": "service_date",
        "splits": [
            {"split_name": "scoring", "data_uri": "split=scoring/"}
        ],
        "governance": {
            "allowed_uses": ["shadow_scoring", "claim_entity_clustering", "drift_monitoring_demo", "score_distribution_demo"],
            "blocked_uses": ["supervised_training", "production_promotion_evidence", "confirmed_fwa_labeling"]
        }
    });
    write_json(scoring_dir.join("manifest.json"), &scoring_manifest)?;

    let provider_rows = demo_provider_peer_rows();
    let provider_dir = output_dir.join("unlabeled_provider_peer_clustering");
    write_provider_peer_split(&provider_dir, "analysis", &provider_rows)?;
    let provider_manifest = serde_json::json!({
        "dataset_key": "rust_demo_provider_peer_unlabeled",
        "dataset_version": dataset_version,
        "business_domain": "health_fwa",
        "sample_grain": "provider_month",
        "display_name": "Rust Demo Provider Peer Clustering Unlabeled",
        "owner": "ml-ops",
        "description": "Rust-generated provider peer features for clustering and anomaly discovery exercises.",
        "status": "demo_only",
        "label_policy": "unlabeled_clustering_discovery_only",
        "entity_keys": ["provider_id"],
        "time_split_field": "service_month",
        "splits": [
            {"split_name": "analysis", "data_uri": "split=analysis/"}
        ],
        "governance": {
            "allowed_uses": ["provider_peer_clustering", "anomaly_candidate_discovery", "manual_review_prioritization_demo"],
            "blocked_uses": ["supervised_training", "confirmed_fwa_labeling", "automatic_claim_disposition"]
        }
    });
    write_json(provider_dir.join("manifest.json"), &provider_manifest)?;

    let pack = DemoMlDatasetPack {
        pack_kind: "rust_automl_demo_datasets".into(),
        dataset_version: dataset_version.into(),
        output_dir: output_dir.to_string_lossy().into_owned(),
        labeled_manifest_uri: labeled_dir.join("manifest.json").to_string_lossy().into_owned(),
        unlabeled_manifest_uris: vec![
            scoring_dir.join("manifest.json").to_string_lossy().into_owned(),
            provider_dir.join("manifest.json").to_string_lossy().into_owned(),
        ],
        dataset_manifests: vec![
            DemoMlDatasetSummary {
                dataset_key: "rust_demo_claim_risk_labeled".into(),
                sample_grain: "claim".into(),
                label_policy: "weak_rust_demo_label_not_production_evidence".into(),
                label_column: Some("confirmed_fwa".into()),
                manifest_uri: labeled_dir.join("manifest.json").to_string_lossy().into_owned(),
                split_count: 3,
                row_count: labeled_rows.len(),
            },
            DemoMlDatasetSummary {
                dataset_key: "rust_demo_claim_shadow_unlabeled".into(),
                sample_grain: "claim".into(),
                label_policy: "unlabeled_shadow_scoring_only".into(),
                label_column: None,
                manifest_uri: scoring_dir.join("manifest.json").to_string_lossy().into_owned(),
                split_count: 1,
                row_count: scoring_rows.len(),
            },
            DemoMlDatasetSummary {
                dataset_key: "rust_demo_provider_peer_unlabeled".into(),
                sample_grain: "provider_month".into(),
                label_policy: "unlabeled_clustering_discovery_only".into(),
                label_column: None,
                manifest_uri: provider_dir.join("manifest.json").to_string_lossy().into_owned(),
                split_count: 1,
                row_count: provider_rows.len(),
            },
        ],
        governance_boundary: "demo data only; unlabeled datasets cannot train supervised models; labeled data is weak demo evidence only".into(),
        next_worker_commands: vec![
            format!(
                "cargo run --locked -p worker -- profile-parquet --manifest {} --output-dir {}/profile",
                labeled_dir.join("manifest.json").display(),
                labeled_dir.display()
            ),
            format!(
                "cargo run --locked -p worker -- build-feature-set --manifest {} --output-dir {}/feature-set",
                labeled_dir.join("manifest.json").display(),
                labeled_dir.display()
            ),
            format!(
                "cargo run --locked -p worker -- build-training-handoff --manifest {} --artifact-base-uri s3://fwa-models --model-key baseline_fwa --base-model-version 0.1.0 --job-id model_retraining_job_1 --actor trainer-worker",
                labeled_dir.join("manifest.json").display()
            ),
            format!(
                "cargo run --locked -p worker -- build-training-handoff --manifest {} --artifact-base-uri s3://fwa-models --model-key baseline_fwa --base-model-version 0.1.0 --job-id model_retraining_job_1 --actor trainer-worker --algorithm xgboost",
                labeled_dir.join("manifest.json").display()
            ),
            format!(
                "cargo run --locked -p worker -- cluster-provider-peers --manifest {} --output-dir {}/clusters",
                provider_dir.join("manifest.json").display(),
                provider_dir.display()
            ),
            format!(
                "cargo run --locked -p worker -- cluster-provider-graph --manifest {} --output-dir {}/graph-communities",
                provider_dir.join("manifest.json").display(),
                provider_dir.display()
            ),
            format!(
                "cargo run --locked -p worker -- cluster-claim-entities --manifest {} --output-dir {}/entity-clusters",
                scoring_dir.join("manifest.json").display(),
                scoring_dir.display()
            ),
        ],
    };
    write_json(output_dir.join("index.json"), &pack)?;
    Ok(pack)
}

pub fn build_feature_set(
    manifest_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    feature_set_id: Option<&str>,
) -> anyhow::Result<FeatureSetManifest> {
    let manifest_path = manifest_path.as_ref();
    let manifest_json = fs::read_to_string(manifest_path)
        .with_context(|| format!("read manifest {}", manifest_path.display()))?;
    let manifest: ParquetDatasetManifest =
        serde_json::from_str(&manifest_json).context("parse parquet dataset manifest")?;
    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let profile = profile_manifest(&manifest, base_dir)?;
    let feature_columns = profile
        .schema
        .fields
        .iter()
        .filter(|field| field.semantic_role == "feature")
        .filter(|field| is_numeric_logical_type(&field.logical_type))
        .map(|field| FeatureSetColumn {
            name: field.field_name.clone(),
            logical_type: field.logical_type.clone(),
            nullable: field.nullable,
            source: "parquet_manifest_column".into(),
        })
        .collect::<Vec<_>>();
    if feature_columns.is_empty() {
        bail!("feature set must include at least one numeric feature column");
    }

    let split_summaries = profile
        .catalog
        .splits
        .iter()
        .map(|split| FeatureSetSplitSummary {
            split_name: split.split_name.clone(),
            row_count: split.row_count,
            positive_count: split.positive_count,
            negative_count: split.negative_count,
        })
        .collect::<Vec<_>>();
    let feature_set_id = feature_set_id
        .map(|value| required_non_empty("feature_set_id", value))
        .transpose()?
        .map(str::to_string)
        .unwrap_or_else(|| {
            format!(
                "{}:{}:{}",
                manifest.dataset_key, manifest.dataset_version, manifest.sample_grain
            )
        });
    let mut feature_set = FeatureSetManifest {
        manifest_kind: "rust_feature_set_manifest".into(),
        manifest_version: 1,
        feature_set_id,
        dataset_key: manifest.dataset_key,
        dataset_version: manifest.dataset_version,
        source_manifest_uri: manifest_path.to_string_lossy().into_owned(),
        sample_grain: manifest.sample_grain,
        label_column: manifest.label_column,
        entity_keys: manifest.entity_keys,
        feature_columns,
        split_summaries,
        feature_reproducibility_hash: String::new(),
        governance_boundary:
            "feature set is training and evaluation evidence only; it does not approve labels, promote models, or publish rules"
                .into(),
        evidence_refs: vec![
            format!("dataset_manifest:{}", manifest_path.display()),
            "feature_materialization:rust_worker_build_feature_set".into(),
        ],
    };
    feature_set.feature_reproducibility_hash = feature_reproducibility_hash(&feature_set)?;

    fs::create_dir_all(output_dir.as_ref())
        .with_context(|| format!("create output dir {}", output_dir.as_ref().display()))?;
    write_json(
        output_dir.as_ref().join("feature_set_manifest.json"),
        &feature_set,
    )?;
    write_json(
        output_dir.as_ref().join("feature_columns.json"),
        &feature_set.feature_columns,
    )?;
    write_json(
        output_dir.as_ref().join("feature_split_summary.json"),
        &feature_set.split_summaries,
    )?;
    Ok(feature_set)
}

fn demo_labeled_claims() -> Vec<DemoLabeledClaim> {
    vec![
        DemoLabeledClaim {
            claim_id: "CLM-0001",
            member_id: "MBR-001",
            policy_id: "POL-001",
            provider_id: "PRV-101",
            service_date: "2026-01-03",
            claim_amount: 420.0,
            amount_to_limit_ratio: 0.18,
            peer_percentile: 0.22,
            item_count: 2,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0002",
            member_id: "MBR-002",
            policy_id: "POL-002",
            provider_id: "PRV-102",
            service_date: "2026-01-06",
            claim_amount: 1280.0,
            amount_to_limit_ratio: 0.64,
            peer_percentile: 0.79,
            item_count: 5,
            high_cost_item_ratio: 0.40,
            provider_risk_tier: 2,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0003",
            member_id: "MBR-003",
            policy_id: "POL-003",
            provider_id: "PRV-103",
            service_date: "2026-01-09",
            claim_amount: 310.0,
            amount_to_limit_ratio: 0.12,
            peer_percentile: 0.18,
            item_count: 1,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0004",
            member_id: "MBR-004",
            policy_id: "POL-004",
            provider_id: "PRV-104",
            service_date: "2026-01-12",
            claim_amount: 2420.0,
            amount_to_limit_ratio: 0.91,
            peer_percentile: 0.96,
            item_count: 8,
            high_cost_item_ratio: 0.63,
            provider_risk_tier: 3,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0005",
            member_id: "MBR-005",
            policy_id: "POL-005",
            provider_id: "PRV-105",
            service_date: "2026-01-15",
            claim_amount: 560.0,
            amount_to_limit_ratio: 0.21,
            peer_percentile: 0.34,
            item_count: 3,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0006",
            member_id: "MBR-006",
            policy_id: "POL-006",
            provider_id: "PRV-106",
            service_date: "2026-01-18",
            claim_amount: 1760.0,
            amount_to_limit_ratio: 0.83,
            peer_percentile: 0.88,
            item_count: 6,
            high_cost_item_ratio: 0.50,
            provider_risk_tier: 3,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0007",
            member_id: "MBR-007",
            policy_id: "POL-007",
            provider_id: "PRV-107",
            service_date: "2026-01-21",
            claim_amount: 690.0,
            amount_to_limit_ratio: 0.27,
            peer_percentile: 0.39,
            item_count: 2,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0008",
            member_id: "MBR-008",
            policy_id: "POL-008",
            provider_id: "PRV-108",
            service_date: "2026-01-24",
            claim_amount: 1580.0,
            amount_to_limit_ratio: 0.74,
            peer_percentile: 0.86,
            item_count: 7,
            high_cost_item_ratio: 0.57,
            provider_risk_tier: 2,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0009",
            member_id: "MBR-009",
            policy_id: "POL-009",
            provider_id: "PRV-109",
            service_date: "2026-02-04",
            claim_amount: 490.0,
            amount_to_limit_ratio: 0.16,
            peer_percentile: 0.24,
            item_count: 2,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0010",
            member_id: "MBR-010",
            policy_id: "POL-010",
            provider_id: "PRV-110",
            service_date: "2026-02-09",
            claim_amount: 2240.0,
            amount_to_limit_ratio: 0.94,
            peer_percentile: 0.97,
            item_count: 9,
            high_cost_item_ratio: 0.67,
            provider_risk_tier: 3,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0011",
            member_id: "MBR-011",
            policy_id: "POL-011",
            provider_id: "PRV-111",
            service_date: "2026-02-13",
            claim_amount: 360.0,
            amount_to_limit_ratio: 0.10,
            peer_percentile: 0.17,
            item_count: 1,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0012",
            member_id: "MBR-012",
            policy_id: "POL-012",
            provider_id: "PRV-112",
            service_date: "2026-02-16",
            claim_amount: 1430.0,
            amount_to_limit_ratio: 0.68,
            peer_percentile: 0.81,
            item_count: 5,
            high_cost_item_ratio: 0.40,
            provider_risk_tier: 2,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0013",
            member_id: "MBR-013",
            policy_id: "POL-013",
            provider_id: "PRV-113",
            service_date: "2026-03-02",
            claim_amount: 520.0,
            amount_to_limit_ratio: 0.19,
            peer_percentile: 0.29,
            item_count: 2,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0014",
            member_id: "MBR-014",
            policy_id: "POL-014",
            provider_id: "PRV-114",
            service_date: "2026-03-07",
            claim_amount: 2610.0,
            amount_to_limit_ratio: 0.98,
            peer_percentile: 0.99,
            item_count: 10,
            high_cost_item_ratio: 0.70,
            provider_risk_tier: 3,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0015",
            member_id: "MBR-015",
            policy_id: "POL-015",
            provider_id: "PRV-115",
            service_date: "2026-03-12",
            claim_amount: 450.0,
            amount_to_limit_ratio: 0.14,
            peer_percentile: 0.21,
            item_count: 2,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0016",
            member_id: "MBR-016",
            policy_id: "POL-016",
            provider_id: "PRV-116",
            service_date: "2026-03-17",
            claim_amount: 1690.0,
            amount_to_limit_ratio: 0.78,
            peer_percentile: 0.84,
            item_count: 6,
            high_cost_item_ratio: 0.50,
            provider_risk_tier: 2,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
    ]
}

fn demo_unlabeled_claims() -> Vec<DemoUnlabeledClaim> {
    vec![
        DemoUnlabeledClaim {
            claim_id: "CLM-S001",
            member_id: "MBR-S001",
            policy_id: "POL-S001",
            provider_id: "PRV-201",
            service_date: "2026-04-02",
            claim_amount: 380.0,
            amount_to_limit_ratio: 0.15,
            peer_percentile: 0.25,
            item_count: 2,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
        },
        DemoUnlabeledClaim {
            claim_id: "CLM-S002",
            member_id: "MBR-S002",
            policy_id: "POL-S002",
            provider_id: "PRV-202",
            service_date: "2026-04-03",
            claim_amount: 1980.0,
            amount_to_limit_ratio: 0.89,
            peer_percentile: 0.93,
            item_count: 8,
            high_cost_item_ratio: 0.63,
            provider_risk_tier: 3,
            diagnosis_procedure_mismatch: 1,
        },
        DemoUnlabeledClaim {
            claim_id: "CLM-S003",
            member_id: "MBR-S003",
            policy_id: "POL-S003",
            provider_id: "PRV-203",
            service_date: "2026-04-04",
            claim_amount: 740.0,
            amount_to_limit_ratio: 0.31,
            peer_percentile: 0.44,
            item_count: 3,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
        },
        DemoUnlabeledClaim {
            claim_id: "CLM-S004",
            member_id: "MBR-S004",
            policy_id: "POL-S004",
            provider_id: "PRV-204",
            service_date: "2026-04-05",
            claim_amount: 2260.0,
            amount_to_limit_ratio: 0.96,
            peer_percentile: 0.98,
            item_count: 9,
            high_cost_item_ratio: 0.67,
            provider_risk_tier: 3,
            diagnosis_procedure_mismatch: 1,
        },
        DemoUnlabeledClaim {
            claim_id: "CLM-S005",
            member_id: "MBR-S005",
            policy_id: "POL-S005",
            provider_id: "PRV-205",
            service_date: "2026-04-06",
            claim_amount: 610.0,
            amount_to_limit_ratio: 0.22,
            peer_percentile: 0.36,
            item_count: 2,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
        },
        DemoUnlabeledClaim {
            claim_id: "CLM-S006",
            member_id: "MBR-S006",
            policy_id: "POL-S006",
            provider_id: "PRV-206",
            service_date: "2026-04-07",
            claim_amount: 1510.0,
            amount_to_limit_ratio: 0.71,
            peer_percentile: 0.82,
            item_count: 6,
            high_cost_item_ratio: 0.50,
            provider_risk_tier: 2,
            diagnosis_procedure_mismatch: 1,
        },
    ]
}

fn demo_provider_peer_rows() -> Vec<DemoProviderPeerRow> {
    vec![
        DemoProviderPeerRow {
            provider_id: "PRV-201",
            cohort_key: "orthopedic_urban",
            service_month: "2026-04",
            claim_count: 42,
            avg_claim_amount: 640.0,
            high_cost_rate: 0.08,
            peer_z_score: -0.4,
            graph_degree: 3,
            community_id: 1,
        },
        DemoProviderPeerRow {
            provider_id: "PRV-202",
            cohort_key: "orthopedic_urban",
            service_month: "2026-04",
            claim_count: 136,
            avg_claim_amount: 1830.0,
            high_cost_rate: 0.41,
            peer_z_score: 2.7,
            graph_degree: 12,
            community_id: 3,
        },
        DemoProviderPeerRow {
            provider_id: "PRV-203",
            cohort_key: "primary_care_suburban",
            service_month: "2026-04",
            claim_count: 61,
            avg_claim_amount: 390.0,
            high_cost_rate: 0.03,
            peer_z_score: -0.1,
            graph_degree: 4,
            community_id: 1,
        },
        DemoProviderPeerRow {
            provider_id: "PRV-204",
            cohort_key: "primary_care_suburban",
            service_month: "2026-04",
            claim_count: 118,
            avg_claim_amount: 1420.0,
            high_cost_rate: 0.34,
            peer_z_score: 2.2,
            graph_degree: 9,
            community_id: 2,
        },
        DemoProviderPeerRow {
            provider_id: "PRV-205",
            cohort_key: "imaging_urban",
            service_month: "2026-04",
            claim_count: 54,
            avg_claim_amount: 760.0,
            high_cost_rate: 0.10,
            peer_z_score: 0.2,
            graph_degree: 5,
            community_id: 1,
        },
        DemoProviderPeerRow {
            provider_id: "PRV-206",
            cohort_key: "imaging_urban",
            service_month: "2026-04",
            claim_count: 153,
            avg_claim_amount: 2140.0,
            high_cost_rate: 0.47,
            peer_z_score: 3.1,
            graph_degree: 15,
            community_id: 3,
        },
    ]
}

fn write_labeled_split(
    dataset_dir: &Path,
    split_name: &str,
    rows: &[DemoLabeledClaim],
) -> anyhow::Result<()> {
    let split_dir = dataset_dir.join(format!("split={split_name}"));
    fs::create_dir_all(&split_dir)
        .with_context(|| format!("create labeled split dir {}", split_dir.display()))?;
    let schema = claim_schema(true);
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.claim_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.member_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.policy_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.provider_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.service_date).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.claim_amount).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.amount_to_limit_ratio)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.peer_percentile)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                rows.iter().map(|row| row.item_count).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.high_cost_item_ratio)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                rows.iter()
                    .map(|row| row.provider_risk_tier)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int8Array::from(
                rows.iter()
                    .map(|row| row.diagnosis_procedure_mismatch)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int8Array::from(
                rows.iter().map(|row| row.confirmed_fwa).collect::<Vec<_>>(),
            )),
        ],
    )?;
    write_parquet(split_dir.join("part-00000.parquet"), schema, &batch)
}

fn write_unlabeled_claim_split(
    dataset_dir: &Path,
    split_name: &str,
    rows: &[DemoUnlabeledClaim],
) -> anyhow::Result<()> {
    let split_dir = dataset_dir.join(format!("split={split_name}"));
    fs::create_dir_all(&split_dir)
        .with_context(|| format!("create unlabeled claim split dir {}", split_dir.display()))?;
    let schema = claim_schema(false);
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.claim_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.member_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.policy_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.provider_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.service_date).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.claim_amount).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.amount_to_limit_ratio)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.peer_percentile)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                rows.iter().map(|row| row.item_count).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.high_cost_item_ratio)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                rows.iter()
                    .map(|row| row.provider_risk_tier)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int8Array::from(
                rows.iter()
                    .map(|row| row.diagnosis_procedure_mismatch)
                    .collect::<Vec<_>>(),
            )),
        ],
    )?;
    write_parquet(split_dir.join("part-00000.parquet"), schema, &batch)
}

fn write_provider_peer_split(
    dataset_dir: &Path,
    split_name: &str,
    rows: &[DemoProviderPeerRow],
) -> anyhow::Result<()> {
    let split_dir = dataset_dir.join(format!("split={split_name}"));
    fs::create_dir_all(&split_dir)
        .with_context(|| format!("create provider peer split dir {}", split_dir.display()))?;
    let schema = Arc::new(Schema::new(vec![
        Field::new("provider_id", DataType::Utf8, false),
        Field::new("cohort_key", DataType::Utf8, false),
        Field::new("service_month", DataType::Utf8, false),
        Field::new("claim_count", DataType::Int32, false),
        Field::new("avg_claim_amount", DataType::Float64, false),
        Field::new("high_cost_rate", DataType::Float64, false),
        Field::new("peer_z_score", DataType::Float64, false),
        Field::new("graph_degree", DataType::Int32, false),
        Field::new("community_id", DataType::Int32, false),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.provider_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.cohort_key).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.service_month).collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                rows.iter().map(|row| row.claim_count).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.avg_claim_amount)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.high_cost_rate)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.peer_z_score).collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                rows.iter().map(|row| row.graph_degree).collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                rows.iter().map(|row| row.community_id).collect::<Vec<_>>(),
            )),
        ],
    )?;
    write_parquet(split_dir.join("part-00000.parquet"), schema, &batch)
}

fn claim_schema(include_label: bool) -> Arc<Schema> {
    let mut fields = vec![
        Field::new("claim_id", DataType::Utf8, false),
        Field::new("member_id", DataType::Utf8, false),
        Field::new("policy_id", DataType::Utf8, false),
        Field::new("provider_id", DataType::Utf8, false),
        Field::new("service_date", DataType::Utf8, false),
        Field::new("claim_amount", DataType::Float64, false),
        Field::new("amount_to_limit_ratio", DataType::Float64, false),
        Field::new("peer_percentile", DataType::Float64, false),
        Field::new("item_count", DataType::Int32, false),
        Field::new("high_cost_item_ratio", DataType::Float64, false),
        Field::new("provider_risk_tier", DataType::Int32, false),
        Field::new("diagnosis_procedure_mismatch", DataType::Int8, false),
    ];
    if include_label {
        fields.push(Field::new("confirmed_fwa", DataType::Int8, false));
    }
    Arc::new(Schema::new(fields))
}

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

#[derive(Debug, Clone, Serialize)]
pub struct DatasetSchemaOutput {
    pub dataset_key: String,
    pub dataset_version: String,
    pub business_domain: String,
    pub sample_grain: String,
    pub label_column: String,
    pub entity_keys: Vec<String>,
    pub fields: Vec<FieldSchemaOutput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldSchemaOutput {
    pub field_name: String,
    pub logical_type: String,
    pub nullable: bool,
    pub semantic_role: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatasetProfileOutput {
    pub dataset_key: String,
    pub dataset_version: String,
    pub row_count_by_split: BTreeMap<String, u64>,
    pub label_distribution_by_split: BTreeMap<String, BTreeMap<String, u64>>,
    pub fields: Vec<FieldProfileOutput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldProfileOutput {
    pub field_name: String,
    pub logical_type: String,
    pub nullable: bool,
    pub semantic_role: String,
    pub missing_count_by_split: BTreeMap<String, u64>,
    pub missing_rate_by_split: BTreeMap<String, f64>,
    pub top_values: Vec<ValueCount>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValueCount {
    pub value: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileResult {
    pub schema: DatasetSchemaOutput,
    pub profile: DatasetProfileOutput,
    pub catalog: DatasetCatalogOutput,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatasetCatalogOutput {
    pub source_key: String,
    pub display_name: String,
    pub business_domain: String,
    pub owner: String,
    pub description: String,
    pub dataset_key: String,
    pub dataset_version: String,
    pub sample_grain: String,
    pub label_column: String,
    pub entity_keys: Vec<String>,
    pub manifest_uri: String,
    pub schema_uri: String,
    pub profile_uri: String,
    pub storage_format: String,
    pub schema_hash: String,
    pub row_count: u64,
    pub status: String,
    pub splits: Vec<DatasetCatalogSplit>,
    pub fields: Vec<DatasetCatalogField>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatasetCatalogSplit {
    pub split_name: String,
    pub data_uri: String,
    pub row_count: u64,
    pub positive_count: Option<u64>,
    pub negative_count: Option<u64>,
    pub label_distribution_json: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatasetCatalogField {
    pub field_name: String,
    pub logical_type: String,
    pub nullable: bool,
    pub semantic_role: String,
    pub description: String,
    pub profile_json: serde_json::Value,
}

#[derive(Debug, Default)]
struct FieldAccumulator {
    field_name: String,
    logical_type: String,
    nullable: bool,
    semantic_role: String,
    missing_count_by_split: BTreeMap<String, u64>,
    value_counts: BTreeMap<String, u64>,
}

pub fn profile_manifest_file(
    manifest_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<ProfileResult> {
    let manifest_path = manifest_path.as_ref();
    let manifest_json = fs::read_to_string(manifest_path)
        .with_context(|| format!("read manifest {}", manifest_path.display()))?;
    let manifest: ParquetDatasetManifest =
        serde_json::from_str(&manifest_json).context("parse parquet dataset manifest")?;
    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let result = profile_manifest(&manifest, base_dir)?;

    fs::create_dir_all(output_dir.as_ref())
        .with_context(|| format!("create output dir {}", output_dir.as_ref().display()))?;
    fs::write(
        output_dir.as_ref().join("schema.json"),
        serde_json::to_string_pretty(&result.schema)?,
    )
    .context("write schema.json")?;
    fs::write(
        output_dir.as_ref().join("profile.json"),
        serde_json::to_string_pretty(&result.profile)?,
    )
    .context("write profile.json")?;
    fs::write(
        output_dir.as_ref().join("catalog.json"),
        serde_json::to_string_pretty(&result.catalog)?,
    )
    .context("write catalog.json")?;

    Ok(result)
}

pub fn profile_manifest(
    manifest: &ParquetDatasetManifest,
    base_dir: &Path,
) -> anyhow::Result<ProfileResult> {
    if manifest.splits.is_empty() {
        bail!("manifest must include at least one split");
    }

    let entity_keys = manifest
        .entity_keys
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut fields: BTreeMap<String, FieldAccumulator> = BTreeMap::new();
    let mut row_count_by_split = BTreeMap::new();
    let mut label_distribution_by_split = BTreeMap::new();
    let mut expected_schema: Option<Vec<(String, String, bool)>> = None;

    for split in &manifest.splits {
        reject_csv_uri(&split.data_uri)?;
        let parquet_files = resolve_parquet_files(base_dir, &split.data_uri)?;
        if parquet_files.is_empty() {
            bail!("split {} has no parquet files", split.split_name);
        }

        let mut split_row_count = 0_u64;
        let mut split_label_counts = BTreeMap::new();

        for parquet_file in parquet_files {
            let file = File::open(&parquet_file)
                .with_context(|| format!("open parquet file {}", parquet_file.display()))?;
            let builder = ParquetRecordBatchReaderBuilder::try_new(file)
                .with_context(|| format!("read parquet metadata {}", parquet_file.display()))?;
            let schema = builder.schema();
            let current_schema = schema
                .fields()
                .iter()
                .map(|field| {
                    (
                        field.name().clone(),
                        field.data_type().to_string(),
                        field.is_nullable(),
                    )
                })
                .collect::<Vec<_>>();
            if let Some(expected) = &expected_schema {
                if expected != &current_schema {
                    bail!("parquet schema mismatch in {}", parquet_file.display());
                }
            } else {
                expected_schema = Some(current_schema);
            }

            let mut reader = builder.with_batch_size(4096).build()?;
            for batch in &mut reader {
                let batch = batch?;
                split_row_count += batch.num_rows() as u64;
                for (column_index, field) in batch.schema().fields().iter().enumerate() {
                    let semantic_role = if field.name() == &manifest.label_column {
                        "label"
                    } else if entity_keys.contains(field.name().as_str()) {
                        "key"
                    } else {
                        "feature"
                    };
                    let accumulator =
                        fields
                            .entry(field.name().clone())
                            .or_insert_with(|| FieldAccumulator {
                                field_name: field.name().clone(),
                                logical_type: field.data_type().to_string(),
                                nullable: field.is_nullable(),
                                semantic_role: semantic_role.to_string(),
                                missing_count_by_split: BTreeMap::new(),
                                value_counts: BTreeMap::new(),
                            });

                    let column = batch.column(column_index);
                    *accumulator
                        .missing_count_by_split
                        .entry(split.split_name.clone())
                        .or_insert(0) += column.null_count() as u64;

                    for value in column_values(column.as_ref()) {
                        *accumulator.value_counts.entry(value.clone()).or_insert(0) += 1;
                        if field.name() == &manifest.label_column {
                            *split_label_counts.entry(value).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        row_count_by_split.insert(split.split_name.clone(), split_row_count);
        label_distribution_by_split.insert(split.split_name.clone(), split_label_counts);
    }

    ensure_required_columns(&fields, manifest)?;

    let schema_fields = fields
        .values()
        .map(|field| FieldSchemaOutput {
            field_name: field.field_name.clone(),
            logical_type: field.logical_type.clone(),
            nullable: field.nullable,
            semantic_role: field.semantic_role.clone(),
        })
        .collect::<Vec<_>>();

    let profile_fields = fields
        .values()
        .map(|field| {
            let mut missing_rate_by_split = BTreeMap::new();
            for (split_name, missing_count) in &field.missing_count_by_split {
                let row_count = row_count_by_split.get(split_name).copied().unwrap_or(0);
                let rate = if row_count == 0 {
                    0.0
                } else {
                    *missing_count as f64 / row_count as f64
                };
                missing_rate_by_split.insert(split_name.clone(), rate);
            }
            FieldProfileOutput {
                field_name: field.field_name.clone(),
                logical_type: field.logical_type.clone(),
                nullable: field.nullable,
                semantic_role: field.semantic_role.clone(),
                missing_count_by_split: field.missing_count_by_split.clone(),
                missing_rate_by_split,
                top_values: top_values(&field.value_counts),
            }
        })
        .collect::<Vec<_>>();

    let total_row_count = row_count_by_split.values().sum::<u64>();
    let schema_hash = schema_hash(&schema_fields);
    let catalog_splits = manifest
        .splits
        .iter()
        .map(|split| {
            let label_distribution_json = label_distribution_by_split
                .get(&split.split_name)
                .cloned()
                .unwrap_or_default();
            DatasetCatalogSplit {
                split_name: split.split_name.clone(),
                data_uri: split.data_uri.clone(),
                row_count: row_count_by_split
                    .get(&split.split_name)
                    .copied()
                    .unwrap_or_default(),
                positive_count: label_distribution_json.get("1").copied(),
                negative_count: label_distribution_json.get("0").copied(),
                label_distribution_json,
            }
        })
        .collect::<Vec<_>>();
    let catalog_fields = schema_fields
        .iter()
        .map(|field| {
            let profile = profile_fields
                .iter()
                .find(|profile| profile.field_name == field.field_name);
            DatasetCatalogField {
                field_name: field.field_name.clone(),
                logical_type: field.logical_type.clone(),
                nullable: field.nullable,
                semantic_role: field.semantic_role.clone(),
                description: String::new(),
                profile_json: serde_json::json!({
                    "missing_count_by_split": profile.map(|profile| &profile.missing_count_by_split),
                    "missing_rate_by_split": profile.map(|profile| &profile.missing_rate_by_split),
                    "top_values": profile.map(|profile| &profile.top_values),
                }),
            }
        })
        .collect::<Vec<_>>();

    Ok(ProfileResult {
        schema: DatasetSchemaOutput {
            dataset_key: manifest.dataset_key.clone(),
            dataset_version: manifest.dataset_version.clone(),
            business_domain: manifest.business_domain.clone(),
            sample_grain: manifest.sample_grain.clone(),
            label_column: manifest.label_column.clone(),
            entity_keys: manifest.entity_keys.clone(),
            fields: schema_fields,
        },
        profile: DatasetProfileOutput {
            dataset_key: manifest.dataset_key.clone(),
            dataset_version: manifest.dataset_version.clone(),
            row_count_by_split,
            label_distribution_by_split,
            fields: profile_fields,
        },
        catalog: DatasetCatalogOutput {
            source_key: manifest
                .source_key
                .clone()
                .unwrap_or_else(|| manifest.dataset_key.clone()),
            display_name: manifest
                .display_name
                .clone()
                .unwrap_or_else(|| manifest.dataset_key.clone()),
            business_domain: manifest.business_domain.clone(),
            owner: manifest.owner.clone().unwrap_or_else(|| "data-ops".into()),
            description: manifest
                .description
                .clone()
                .unwrap_or_else(|| "Generated from Parquet dataset manifest".into()),
            dataset_key: manifest.dataset_key.clone(),
            dataset_version: manifest.dataset_version.clone(),
            sample_grain: manifest.sample_grain.clone(),
            label_column: manifest.label_column.clone(),
            entity_keys: manifest.entity_keys.clone(),
            manifest_uri: "manifest.json".into(),
            schema_uri: "schema.json".into(),
            profile_uri: "profile.json".into(),
            storage_format: "parquet".into(),
            schema_hash,
            row_count: total_row_count,
            status: manifest.status.clone().unwrap_or_else(|| "draft".into()),
            splits: catalog_splits,
            fields: catalog_fields,
        },
    })
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

fn retraining_job_status_path(job_id: &str) -> String {
    format!("/api/v1/ops/model-retraining-jobs/{job_id}/status")
}

fn retraining_job_output_path(job_id: &str) -> String {
    format!("/api/v1/ops/model-retraining-jobs/{job_id}/output")
}

fn build_training_command(
    python: &str,
    manifest_path: &str,
    artifact_base_uri: &str,
    job: &ClaimedRetrainingJob,
    actor: &str,
) -> TrainingCommand {
    TrainingCommand {
        program: python.to_string(),
        args: vec![
            "-m".into(),
            "app.train".into(),
            "--manifest".into(),
            manifest_path.into(),
            "--artifact-base-uri".into(),
            artifact_base_uri.into(),
            "--model-key".into(),
            job.model_key.clone(),
            "--base-model-version".into(),
            job.model_version.clone(),
            "--job-id".into(),
            job.job_id.clone(),
            "--actor".into(),
            actor.into(),
        ],
    }
}

pub fn build_training_handoff(
    manifest_path: impl AsRef<Path>,
    artifact_base_uri: &str,
    model_key: &str,
    base_model_version: &str,
    job_id: &str,
    actor: &str,
) -> anyhow::Result<serde_json::Value> {
    build_training_handoff_with_algorithm(
        manifest_path,
        artifact_base_uri,
        model_key,
        base_model_version,
        job_id,
        actor,
        "logistic_regression",
    )
}

pub fn build_training_handoff_with_algorithm(
    manifest_path: impl AsRef<Path>,
    artifact_base_uri: &str,
    model_key: &str,
    base_model_version: &str,
    job_id: &str,
    actor: &str,
    algorithm: &str,
) -> anyhow::Result<serde_json::Value> {
    if artifact_base_uri.trim().is_empty() {
        bail!("artifact_base_uri is required");
    }
    let algorithm = normalize_training_algorithm(algorithm)?;
    let manifest_path = manifest_path.as_ref();
    let manifest_json = fs::read_to_string(manifest_path)
        .with_context(|| format!("read training manifest {}", manifest_path.display()))?;
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_json).context("parse training manifest")?;
    let dataset_key = required_manifest_str(&manifest, "dataset_key")?;
    let dataset_version = required_manifest_str(&manifest, "dataset_version")?;
    let label_column = required_manifest_str(&manifest, "label_column")?;
    let time_split_field = required_manifest_str(&manifest, "time_split_field")?;
    let entity_keys = manifest
        .get("entity_keys")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let group_split_fields = manifest
        .get("group_split_fields")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let splits = manifest
        .get("splits")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    if splits.is_empty() {
        bail!("training manifest must include splits");
    }

    let candidate_model_version = training_candidate_version(base_model_version, job_id, algorithm);
    let artifact_root = artifact_base_uri.trim().trim_end_matches('/');
    let safe_model_key = safe_path_segment(model_key);
    let artifact_dir = format!("{artifact_root}/{safe_model_key}/{candidate_model_version}");
    let onnx_algorithm = matches!(algorithm, "xgboost" | "lightgbm");
    let serving_artifact_uri = if onnx_algorithm {
        format!("{artifact_dir}/model.onnx")
    } else {
        format!("{artifact_dir}/rust_serving_artifact.json")
    };
    let runtime_kind = match algorithm {
        "logistic_regression" => "rust_logistic_regression",
        "xgboost" => "xgboost_onnx",
        "lightgbm" => "lightgbm_onnx",
        _ => unreachable!("algorithm normalized"),
    };
    let mut required_evidence_refs = vec![
        "model_retraining_jobs:<job_id>".to_string(),
        "model_artifacts:<serving_artifact_uri>".to_string(),
        "feature_set_manifests:<rust_feature_set_manifest_uri>".to_string(),
        "model_validation_reports:<validation_report_uri>".to_string(),
        "model_evaluations:<evaluation_run_id>".to_string(),
    ];
    if onnx_algorithm {
        required_evidence_refs.push("model_onnx_parity_reports:<onnx_parity_report_uri>".into());
    }

    Ok(serde_json::json!({
        "handoff_kind": "external_training_platform",
        "handoff_version": 2,
        "data_contract": {
            "source": "same_parquet_dataset_manifest",
            "manifest_uri": manifest_path.to_string_lossy(),
            "forbidden_sources": ["application_tables", "ad_hoc_feature_definitions"]
        },
        "dataset": {
            "dataset_key": dataset_key,
            "dataset_version": dataset_version,
            "manifest_uri": manifest_path.to_string_lossy(),
            "label_column": label_column,
            "entity_keys": entity_keys,
            "time_split_field": time_split_field,
            "group_split_fields": group_split_fields,
            "splits": splits
        },
        "training_job": {
            "model_key": model_key,
            "base_model_version": base_model_version,
            "candidate_model_version": candidate_model_version,
            "job_id": job_id,
            "actor": actor,
            "algorithm": algorithm,
            "runtime_kind": runtime_kind
        },
        "artifact_contract": {
            "artifact_dir": artifact_dir,
            "serving_artifact_uri": serving_artifact_uri,
            "serving_artifact_format": if onnx_algorithm { "onnx" } else { "rust_json" },
            "rust_serving_artifact_uri": format!("{artifact_dir}/rust_serving_artifact.json"),
            "onnx_artifact_uri": if onnx_algorithm {
                serde_json::Value::String(format!("{artifact_dir}/model.onnx"))
            } else {
                serde_json::Value::Null
            },
            "training_artifact_uri": format!("{artifact_dir}/model.joblib"),
            "serving_manifest_uri": format!("{artifact_dir}/serving_manifest.json"),
            "onnx_parity_report_uri": if onnx_algorithm {
                serde_json::Value::String(format!("{artifact_dir}/onnx_parity_report.json"))
            } else {
                serde_json::Value::Null
            },
            "validation_report_uri": format!("{artifact_dir}/validation.json"),
            "rust_feature_set_manifest_uri": format!("{artifact_dir}/rust_feature_set/feature_set_manifest.json"),
            "feature_store_manifest_uri": format!("{artifact_dir}/feature_store_manifest.json"),
            "shadow_report_uri": format!("{artifact_dir}/shadow_report.json"),
            "drift_report_uri": format!("{artifact_dir}/drift_report.json"),
            "fairness_report_uri": format!("{artifact_dir}/fairness_report.json")
        },
        "feature_set_contract": {
            "builder": "worker build-feature-set",
            "required_hash_field": "metrics_json.feature_reproducibility_hash",
            "required_manifest_field": "metrics_json.rust_feature_set_manifest_uri",
            "excluded_columns": ["dataset.entity_keys", "dataset.label_column"],
            "evidence_ref": "feature_set_manifests:<rust_feature_set_manifest_uri>"
        },
        "output_contract": {
            "submit_path": retraining_job_output_path(job_id),
            "artifact_uri": "artifact_contract.serving_artifact_uri",
            "serving_manifest_uri": "artifact_contract.serving_manifest_uri",
            "onnx_parity_report_uri": if onnx_algorithm {
                serde_json::Value::String("artifact_contract.onnx_parity_report_uri".into())
            } else {
                serde_json::Value::Null
            },
            "required_evidence_refs": required_evidence_refs
        }
    }))
}

fn normalize_training_algorithm(algorithm: &str) -> anyhow::Result<&'static str> {
    match algorithm.trim() {
        "" | "logistic_regression" => Ok("logistic_regression"),
        "xgboost" => Ok("xgboost"),
        "lightgbm" => Ok("lightgbm"),
        other => bail!("unsupported training algorithm: {other}"),
    }
}

fn training_candidate_version(base_model_version: &str, job_id: &str, algorithm: &str) -> String {
    let base = safe_path_segment(base_model_version);
    let job = safe_path_segment(job_id);
    if algorithm == "logistic_regression" {
        format!("{base}-candidate-{job}")
    } else {
        format!("{base}-{}-candidate-{job}", safe_path_segment(algorithm))
    }
}

pub fn build_mlops_monitoring_plan(
    manifest_uri: &str,
    artifact_uri: &str,
    model_key: &str,
    model_version: &str,
    cron: &str,
) -> anyhow::Result<serde_json::Value> {
    let manifest_uri = required_non_empty("manifest_uri", manifest_uri)?;
    let artifact_uri = required_non_empty("artifact_uri", artifact_uri)?;
    let model_key = required_non_empty("model_key", model_key)?;
    let model_version = required_non_empty("model_version", model_version)?;
    let cron = required_non_empty("cron", cron)?;
    let artifact_dir = artifact_parent_uri(artifact_uri);

    Ok(serde_json::json!({
        "plan_kind": "scheduled_mlops_monitoring",
        "plan_version": 2,
        "data_contract": {
            "source": "same_parquet_dataset_manifest",
            "manifest_uri": manifest_uri
        },
        "model": {
            "model_key": model_key,
            "model_version": model_version,
            "artifact_uri": artifact_uri
        },
        "schedule": {
            "cron": cron
        },
        "jobs": [
            {
                "job_kind": "shadow_traffic_evaluation",
                "input": "live_routing_and_qa_outcomes",
                "output_ref": "model_shadow_reports:<shadow_report_uri>",
                "shadow_report_uri": format!("{artifact_dir}/shadow_report.json")
            },
            {
                "job_kind": "drift_monitoring",
                "input": "scoring_features_and_scores",
                "output_ref": "model_drift_reports:<drift_report_uri>",
                "drift_report_uri": format!("{artifact_dir}/drift_report.json")
            },
            {
                "job_kind": "segment_fairness_review",
                "input": "customer_approved_segments",
                "output_ref": "model_fairness_reports:<fairness_report_uri>",
                "fairness_report_uri": format!("{artifact_dir}/fairness_report.json")
            },
            {
                "job_kind": "reviewer_disagreement_review",
                "input": "qa_reviews_and_investigation_outcomes",
                "output_ref": "model_reviewer_disagreement_reports:<reviewer_disagreement_report_uri>",
                "reviewer_disagreement_report_uri": format!("{artifact_dir}/reviewer_disagreement_report.json")
            },
            {
                "job_kind": "label_delay_review",
                "input": "scoring_runs_and_outcome_label_timestamps",
                "output_ref": "model_label_delay_reports:<label_delay_report_uri>",
                "label_delay_report_uri": format!("{artifact_dir}/label_delay_report.json")
            }
        ]
    }))
}

pub fn build_mlops_monitoring_report(
    model_key: &str,
    model_version: &str,
    artifact_evaluation_report_uri: &str,
    shadow_report_uri: &str,
    drift_report_uri: &str,
    fairness_report_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let model_key = required_non_empty("model_key", model_key)?;
    let model_version = required_non_empty("model_version", model_version)?;
    let artifact_evaluation_report_uri = required_non_empty(
        "artifact_evaluation_report_uri",
        artifact_evaluation_report_uri,
    )?;
    let shadow_report_uri = required_non_empty("shadow_report_uri", shadow_report_uri)?;
    let drift_report_uri = required_non_empty("drift_report_uri", drift_report_uri)?;
    let fairness_report_uri = required_non_empty("fairness_report_uri", fairness_report_uri)?;

    let artifact_evaluation_report = read_json_report(artifact_evaluation_report_uri)?;
    let shadow_report = read_json_report(shadow_report_uri)?;
    let drift_report = read_json_report(drift_report_uri)?;
    let fairness_report = read_json_report(fairness_report_uri)?;

    let artifact_gate_status =
        json_string(&artifact_evaluation_report, "gate_status").unwrap_or_else(|| "missing".into());
    let rust_serving_status = json_string(&artifact_evaluation_report, "rust_serving_status")
        .unwrap_or_else(|| "missing".into());
    let latency_status = json_string(&artifact_evaluation_report, "latency_status")
        .unwrap_or_else(|| "missing".into());
    let p95_latency_ms = json_u64(&artifact_evaluation_report, "p95_latency_ms");
    let shadow_status = json_string(&shadow_report, "status").unwrap_or_else(|| "missing".into());
    let drift_status = json_string(&drift_report, "status").unwrap_or_else(|| "missing".into());
    let fairness_status =
        json_string(&fairness_report, "status").unwrap_or_else(|| "missing".into());

    let mut triggers = Vec::new();
    if artifact_gate_status != "passed" || rust_serving_status != "passed" {
        triggers.push("rust_serving_artifact_evaluation_blocked");
    }
    if latency_status == "failed" {
        triggers.push("rust_serving_latency_budget_failed");
    }
    if shadow_status != "passed" {
        triggers.push("shadow_comparison_review_required");
    }
    match drift_status.as_str() {
        "drift" => triggers.push("model_drift_detected"),
        "watch" => triggers.push("model_drift_watch"),
        _ => {}
    }
    if fairness_status != "passed" {
        triggers.push("segment_fairness_review_required");
    }

    let retraining_recommendation =
        if artifact_gate_status != "passed" || rust_serving_status != "passed" {
            "blocked"
        } else if latency_status == "failed"
            || shadow_status != "passed"
            || drift_status == "drift"
            || fairness_status != "passed"
        {
            "prepare_retraining"
        } else {
            "monitor"
        };
    let overall_status = if retraining_recommendation == "blocked" {
        "blocked"
    } else if triggers.is_empty() {
        "passed"
    } else {
        "watch"
    };
    let review_tasks = mlops_monitoring_review_tasks(model_key, model_version, &triggers);

    let report = serde_json::json!({
        "report_kind": "mlops_monitoring_report",
        "report_version": 1,
        "model_key": model_key,
        "model_version": model_version,
        "overall_status": overall_status,
        "retraining_recommendation": retraining_recommendation,
        "signals": {
            "artifact_evaluation": {
                "report_uri": artifact_evaluation_report_uri,
                "gate_status": artifact_gate_status,
                "rust_serving_status": rust_serving_status,
                "latency_status": latency_status,
                "p95_latency_ms": p95_latency_ms
            },
            "shadow": {
                "report_uri": shadow_report_uri,
                "status": shadow_status,
                "comparison_count": json_u64(&shadow_report, "comparison_count"),
                "average_abs_probability_delta": metric_at(&shadow_report, "average_abs_probability_delta"),
                "max_abs_probability_delta": metric_at(&shadow_report, "max_abs_probability_delta")
            },
            "drift": {
                "report_uri": drift_report_uri,
                "status": drift_status,
                "score_psi": metric_at(&drift_report, "score_psi"),
                "max_feature_psi": metric_at(&drift_report, "max_feature_psi")
            },
            "fairness": {
                "report_uri": fairness_report_uri,
                "status": fairness_status,
                "segment_count": fairness_report
                    .get("segments")
                    .and_then(|value| value.as_array())
                    .map(|segments| segments.len())
                    .unwrap_or(0)
            }
        },
        "triggers": triggers,
        "review_tasks": review_tasks,
        "promotion_boundary": "monitoring can open review or retraining preparation only; it must not activate models, publish rules, or assign fraud labels",
        "evidence_refs": [
            format!("model_artifact_evaluations:{artifact_evaluation_report_uri}"),
            format!("model_shadow_reports:{shadow_report_uri}"),
            format!("model_drift_reports:{drift_report_uri}"),
            format!("model_fairness_reports:{fairness_report_uri}")
        ]
    });

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create MLOps monitoring report output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir.as_ref().join("mlops_monitoring_report.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("mlops_monitoring_review_tasks.json"),
        &report["review_tasks"],
    )?;
    Ok(report)
}

pub fn build_mlops_scheduler_execution_report(
    plan_uri: &str,
    monitoring_report_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let plan_uri = required_non_empty("plan_uri", plan_uri)?;
    let monitoring_report_uri = required_non_empty("monitoring_report_uri", monitoring_report_uri)?;
    let plan = read_json_report(plan_uri)?;
    let monitoring_report = read_json_report(monitoring_report_uri)?;
    if json_string(&plan, "plan_kind").as_deref() != Some("scheduled_mlops_monitoring") {
        bail!("MLOps scheduler execution requires a scheduled_mlops_monitoring plan");
    }
    if json_string(&monitoring_report, "report_kind").as_deref() != Some("mlops_monitoring_report")
    {
        bail!("MLOps scheduler execution requires an mlops_monitoring_report");
    }
    let model_key = nested_json_string(&plan, &["model", "model_key"])
        .context("MLOps monitoring plan requires model.model_key")?;
    let model_version = nested_json_string(&plan, &["model", "model_version"])
        .context("MLOps monitoring plan requires model.model_version")?;
    if json_string(&monitoring_report, "model_key").as_deref() != Some(model_key.as_str())
        || json_string(&monitoring_report, "model_version").as_deref()
            != Some(model_version.as_str())
    {
        bail!("MLOps monitoring report model does not match scheduler plan");
    }
    let jobs = plan
        .get("jobs")
        .and_then(|value| value.as_array())
        .context("MLOps monitoring plan requires jobs")?;
    let reported_uris = mlops_monitoring_report_uris(&monitoring_report);
    let job_executions = jobs
        .iter()
        .map(|job| {
            let job_kind = json_string(job, "job_kind").unwrap_or_else(|| "unknown".into());
            let output_uri = mlops_plan_job_output_uri(job);
            let output_status = output_uri
                .as_ref()
                .map(|uri| reported_uris.contains(uri))
                .unwrap_or(false);
            serde_json::json!({
                "job_kind": job_kind,
                "output_ref": json_string(job, "output_ref"),
                "output_uri": output_uri,
                "execution_status": if output_status {
                    "reported_in_monitoring_summary"
                } else {
                    "scheduled_pending_external_report"
                },
                "routing_impact": "none"
            })
        })
        .collect::<Vec<_>>();
    let pending_job_count = job_executions
        .iter()
        .filter(|execution| {
            execution["execution_status"].as_str() == Some("scheduled_pending_external_report")
        })
        .count();
    let triggers = monitoring_report
        .get("triggers")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let alert_delivery_tasks = triggers
        .iter()
        .map(|trigger| {
            mlops_alert_delivery_task(
                &model_key,
                &model_version,
                trigger,
                plan_uri,
                monitoring_report_uri,
            )
        })
        .collect::<Vec<_>>();
    let alert_delivery_status = if alert_delivery_tasks.is_empty() {
        "no_alerts_required"
    } else {
        "queued_for_external_alert_router"
    };
    let scheduler_status = if pending_job_count == 0 {
        "completed"
    } else {
        "completed_with_pending_external_reports"
    };
    let report = serde_json::json!({
        "report_kind": "mlops_scheduler_execution_report",
        "report_version": 1,
        "plan_uri": plan_uri,
        "monitoring_report_uri": monitoring_report_uri,
        "model_key": model_key,
        "model_version": model_version,
        "schedule": plan["schedule"].clone(),
        "scheduler_status": scheduler_status,
        "pending_external_report_count": pending_job_count,
        "job_executions": job_executions,
        "alert_delivery_status": alert_delivery_status,
        "alert_delivery_task_count": alert_delivery_tasks.len(),
        "alert_delivery_tasks": alert_delivery_tasks,
        "governance_boundary": "scheduler execution evidence may queue alert delivery and review work only; it must not create retraining jobs, activate models, rollback models, or assign fraud labels",
        "evidence_refs": [
            format!("mlops_monitoring_plans:{plan_uri}"),
            format!("model_monitoring_reports:{monitoring_report_uri}"),
            format!("model_versions:{model_key}:{model_version}")
        ]
    });

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create MLOps scheduler execution output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("mlops_scheduler_execution_report.json"),
        &report,
    )?;
    write_json(
        output_dir.as_ref().join("mlops_alert_delivery_tasks.json"),
        &report["alert_delivery_tasks"],
    )?;
    Ok(report)
}

pub fn build_automl_lifecycle_closure_report(
    demo_index_uri: &str,
    candidate_ranking_uri: &str,
    artifact_evaluation_report_uris: &[String],
    rule_backtest_report_uri: &str,
    provider_clustering_report_uri: &str,
    provider_graph_clustering_report_uri: &str,
    claim_entity_clustering_report_uri: &str,
    mlops_monitoring_report_uri: &str,
    mlops_scheduler_execution_report_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let demo_index_uri = required_non_empty("demo_index_uri", demo_index_uri)?;
    let candidate_ranking_uri = required_non_empty("candidate_ranking_uri", candidate_ranking_uri)?;
    if artifact_evaluation_report_uris.is_empty() {
        bail!("at least one artifact_evaluation_report_uri is required");
    }
    let rule_backtest_report_uri =
        required_non_empty("rule_backtest_report_uri", rule_backtest_report_uri)?;
    let provider_clustering_report_uri = required_non_empty(
        "provider_clustering_report_uri",
        provider_clustering_report_uri,
    )?;
    let provider_graph_clustering_report_uri = required_non_empty(
        "provider_graph_clustering_report_uri",
        provider_graph_clustering_report_uri,
    )?;
    let claim_entity_clustering_report_uri = required_non_empty(
        "claim_entity_clustering_report_uri",
        claim_entity_clustering_report_uri,
    )?;
    let mlops_monitoring_report_uri =
        required_non_empty("mlops_monitoring_report_uri", mlops_monitoring_report_uri)?;
    let mlops_scheduler_execution_report_uri = required_non_empty(
        "mlops_scheduler_execution_report_uri",
        mlops_scheduler_execution_report_uri,
    )?;

    let demo_index = read_json_report(demo_index_uri)?;
    let candidate_ranking = read_json_report(candidate_ranking_uri)?;
    let artifact_reports = artifact_evaluation_report_uris
        .iter()
        .map(|uri| read_json_report(uri))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let rule_backtest = read_json_report(rule_backtest_report_uri)?;
    let provider_clustering = read_json_report(provider_clustering_report_uri)?;
    let provider_graph_clustering = read_json_report(provider_graph_clustering_report_uri)?;
    let claim_entity_clustering = read_json_report(claim_entity_clustering_report_uri)?;
    let mlops_monitoring = read_json_report(mlops_monitoring_report_uri)?;
    let mlops_scheduler_execution = read_json_report(mlops_scheduler_execution_report_uri)?;

    let dataset_manifests = demo_index
        .get("dataset_manifests")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let labeled_dataset_count = dataset_manifests
        .iter()
        .filter(|dataset| json_string(dataset, "label_column").is_some())
        .count();
    let unlabeled_dataset_count = dataset_manifests
        .iter()
        .filter(|dataset| json_string(dataset, "label_column").is_none())
        .count();
    let dataset_portfolio_passed = labeled_dataset_count >= 1 && unlabeled_dataset_count >= 2;

    let candidate_algorithms = candidate_ranking
        .get("candidates")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|candidate| json_string(candidate, "algorithm"))
        .collect::<BTreeSet<_>>();
    let supervised_candidates_passed = candidate_ranking["plan_kind"] == "automl_candidate_ranking"
        && candidate_algorithms.contains("xgboost")
        && candidate_algorithms.contains("lightgbm")
        && candidate_ranking
            .get("recommended_candidate_model_version")
            .is_some_and(|value| !value.is_null());

    let runtime_kinds = artifact_reports
        .iter()
        .filter_map(|report| json_string(report, "runtime_kind"))
        .collect::<BTreeSet<_>>();
    let rust_serving_passed = artifact_reports.iter().all(|report| {
        json_string(report, "gate_status").as_deref() == Some("passed")
            && json_string(report, "rust_serving_status").as_deref() == Some("passed")
    });
    let onnx_serving_passed = rust_serving_passed
        && runtime_kinds.contains("xgboost_onnx")
        && runtime_kinds.contains("lightgbm_onnx");

    let rule_backtest_passed = rule_backtest["report_kind"]
        == "deterministic_rule_candidate_backtest"
        && json_string(&rule_backtest, "rule_library_writeback_status")
            .is_some_and(|status| status.contains("blocked_pending_human_review"))
        && json_array_len(&rule_backtest, "candidate_results") > 0
        && json_array_len(&rule_backtest, "review_tasks") > 0;

    let provider_clustering_passed = provider_clustering["report_kind"]
        == "provider_peer_clustering"
        && json_string(&provider_clustering, "governance_boundary")
            .is_some_and(|boundary| boundary.contains("must not create confirmed FWA labels"))
        && json_array_len(&provider_clustering, "anomaly_candidates") > 0;
    let provider_graph_clustering_passed = provider_graph_clustering["report_kind"]
        == "provider_graph_community_clustering"
        && json_string(&provider_graph_clustering, "governance_boundary")
            .is_some_and(|boundary| boundary.contains("must not create confirmed FWA labels"))
        && json_array_len(&provider_graph_clustering, "anomaly_candidates") > 0
        && json_array_len(&provider_graph_clustering, "review_tasks") > 0;
    let claim_entity_clustering_passed = claim_entity_clustering["report_kind"]
        == "claim_entity_clustering"
        && json_string(&claim_entity_clustering, "governance_boundary")
            .is_some_and(|boundary| boundary.contains("rule-library writeback"))
        && json_array_len(&claim_entity_clustering, "review_tasks") > 0;

    let monitoring_status =
        json_string(&mlops_monitoring, "overall_status").unwrap_or_else(|| "missing".into());
    let monitoring_loop_passed = mlops_monitoring["report_kind"] == "mlops_monitoring_report"
        && monitoring_status != "blocked"
        && json_string(&mlops_monitoring, "promotion_boundary")
            .is_some_and(|boundary| boundary.contains("must not activate models"));
    let scheduler_status = json_string(&mlops_scheduler_execution, "scheduler_status")
        .unwrap_or_else(|| "missing".into());
    let alert_delivery_status = json_string(&mlops_scheduler_execution, "alert_delivery_status")
        .unwrap_or_else(|| "missing".into());
    let scheduler_loop_passed = mlops_scheduler_execution["report_kind"]
        == "mlops_scheduler_execution_report"
        && scheduler_status.starts_with("completed")
        && json_string(&mlops_scheduler_execution, "governance_boundary")
            .is_some_and(|boundary| boundary.contains("must not create retraining jobs"));

    let stages = vec![
        lifecycle_stage(
            "demo_dataset_portfolio",
            dataset_portfolio_passed,
            format!(
                "{labeled_dataset_count} labeled dataset(s), {unlabeled_dataset_count} unlabeled dataset(s)"
            ),
            vec![format!("demo_dataset_index:{demo_index_uri}")],
        ),
        lifecycle_stage(
            "supervised_candidate_ranking",
            supervised_candidates_passed,
            format!(
                "candidate algorithms: {}",
                candidate_algorithms
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            vec![format!("automl_candidate_ranking:{candidate_ranking_uri}")],
        ),
        lifecycle_stage(
            "onnx_rust_serving_gate",
            onnx_serving_passed,
            format!(
                "artifact runtime kinds: {}",
                runtime_kinds.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
            artifact_evaluation_report_uris
                .iter()
                .map(|uri| format!("model_artifact_evaluations:{uri}"))
                .collect(),
        ),
        lifecycle_stage(
            "explainable_rule_backtest_gate",
            rule_backtest_passed,
            "rule candidates are backtested and blocked before rule-library writeback".into(),
            vec![format!("rule_candidate_backtests:{rule_backtest_report_uri}")],
        ),
        lifecycle_stage(
            "unlabeled_clustering_reviews",
            provider_clustering_passed
                && provider_graph_clustering_passed
                && claim_entity_clustering_passed,
            "provider-peer, provider graph-community, and claim/member/provider clustering create review candidates only"
                .into(),
            vec![
                format!("provider_peer_clustering:{provider_clustering_report_uri}"),
                format!("provider_graph_clustering:{provider_graph_clustering_report_uri}"),
                format!("claim_entity_clustering:{claim_entity_clustering_report_uri}"),
            ],
        ),
        lifecycle_stage(
            "mlops_monitoring_loop",
            monitoring_loop_passed && scheduler_loop_passed,
            format!(
                "monitoring status: {monitoring_status}; scheduler: {scheduler_status}; alert delivery: {alert_delivery_status}"
            ),
            vec![
                format!("mlops_monitoring_reports:{mlops_monitoring_report_uri}"),
                format!(
                    "mlops_scheduler_execution_reports:{mlops_scheduler_execution_report_uri}"
                ),
            ],
        ),
    ];
    let closure_status = if stages
        .iter()
        .all(|stage| stage["status"].as_str() == Some("passed"))
    {
        "closed_with_human_governance_gates"
    } else {
        "incomplete"
    };

    let report = serde_json::json!({
        "report_kind": "rust_automl_lifecycle_closure",
        "report_version": 1,
        "closure_status": closure_status,
        "lifecycle_stages": stages,
        "governance_boundary": "Rust lifecycle closure may open monitoring, review, retraining preparation, and rule-candidate backtest work only; it must not auto-activate models, assign fraud labels, or write back to the rule library",
        "required_human_gates": [
            "model_governance_review_before_shadow_or_activation",
            "human_rule_review_after_backtest_before_rule_library_writeback",
            "anomaly_review_before_case_creation_or_label_assignment",
            "mlops_monitoring_review_before_retraining_or_rollback_action"
        ],
        "evidence_refs": [
            format!("demo_dataset_index:{demo_index_uri}"),
            format!("automl_candidate_ranking:{candidate_ranking_uri}"),
            format!("rule_candidate_backtests:{rule_backtest_report_uri}"),
            format!("provider_peer_clustering:{provider_clustering_report_uri}"),
            format!("provider_graph_clustering:{provider_graph_clustering_report_uri}"),
            format!("claim_entity_clustering:{claim_entity_clustering_report_uri}"),
            format!("mlops_monitoring_reports:{mlops_monitoring_report_uri}"),
            format!("mlops_scheduler_execution_reports:{mlops_scheduler_execution_report_uri}")
        ],
        "artifact_evaluation_refs": artifact_evaluation_report_uris
            .iter()
            .map(|uri| format!("model_artifact_evaluations:{uri}"))
            .collect::<Vec<_>>()
    });

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create Auto MLOps lifecycle closure output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("rust_automl_lifecycle_closure_report.json"),
        &report,
    )?;
    Ok(report)
}

pub fn build_demo_automl_lifecycle_evidence(
    demo_root: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let demo_root = demo_root.as_ref();
    let output_dir = output_dir.as_ref();
    let index_uri = demo_root.join("index.json");
    let labeled_manifest = demo_root.join("labeled_claim_risk/manifest.json");
    let provider_manifest = demo_root.join("unlabeled_provider_peer_clustering/manifest.json");
    let claim_manifest = demo_root.join("unlabeled_shadow_scoring/manifest.json");
    for required_file in [
        &index_uri,
        &labeled_manifest,
        &provider_manifest,
        &claim_manifest,
    ] {
        if !required_file.is_file() {
            bail!(
                "demo lifecycle evidence requires existing demo file {}",
                required_file.display()
            );
        }
    }
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "create demo lifecycle evidence dir {}",
            output_dir.display()
        )
    })?;

    let validation_dir = output_dir.join("validation");
    fs::create_dir_all(&validation_dir)
        .with_context(|| format!("create validation dir {}", validation_dir.display()))?;
    let xgboost_validation = validation_dir.join("xgboost_validation.json");
    let lightgbm_validation = validation_dir.join("lightgbm_validation.json");
    write_demo_validation_report(
        &xgboost_validation,
        "0.2.0-xgboost-candidate",
        "xgboost",
        "xgboost_onnx",
        0.86,
        0.80,
        0.76,
    )?;
    write_demo_validation_report(
        &lightgbm_validation,
        "0.2.0-lightgbm-candidate",
        "lightgbm",
        "lightgbm_onnx",
        0.85,
        0.79,
        0.75,
    )?;

    let feature_importance_dir = output_dir.join("feature-importance");
    fs::create_dir_all(&feature_importance_dir).with_context(|| {
        format!(
            "create feature importance dir {}",
            feature_importance_dir.display()
        )
    })?;
    let feature_importance_uri = feature_importance_dir.join("feature_importance.parquet");
    write_demo_feature_importance_parquet(&feature_importance_uri)?;

    let ranking = rank_automl_candidates(
        &[
            xgboost_validation.to_string_lossy().into_owned(),
            lightgbm_validation.to_string_lossy().into_owned(),
        ],
        output_dir.join("ranking"),
    )?;

    let artifact_eval_dir = output_dir.join("artifact-evaluation");
    fs::create_dir_all(&artifact_eval_dir).with_context(|| {
        format!(
            "create artifact evaluation dir {}",
            artifact_eval_dir.display()
        )
    })?;
    let xgboost_artifact_eval = artifact_eval_dir.join("xgboost_model_artifact_evaluation.json");
    let lightgbm_artifact_eval = artifact_eval_dir.join("lightgbm_model_artifact_evaluation.json");
    write_demo_artifact_evaluation_report(
        &xgboost_artifact_eval,
        "0.2.0-xgboost-candidate",
        "xgboost_onnx",
        24,
    )?;
    write_demo_artifact_evaluation_report(
        &lightgbm_artifact_eval,
        "0.2.0-lightgbm-candidate",
        "lightgbm_onnx",
        21,
    )?;

    let rule_candidate_dir = output_dir.join("rule-candidates");
    mine_rule_candidates(
        &xgboost_validation.to_string_lossy(),
        &feature_importance_uri.to_string_lossy(),
        &rule_candidate_dir,
    )?;
    let rule_backtest_dir = rule_candidate_dir.join("backtest");
    run_rule_candidate_backtest(
        &rule_candidate_dir
            .join("rule_candidate_mining_plan.json")
            .to_string_lossy(),
        &labeled_manifest.to_string_lossy(),
        &rule_backtest_dir,
    )?;

    let provider_cluster_dir = output_dir.join("clustering/provider-peer");
    let provider_clustering =
        cluster_provider_peers(&provider_manifest.to_string_lossy(), &provider_cluster_dir)?;
    let provider_graph_dir = output_dir.join("clustering/provider-graph");
    let provider_graph = cluster_provider_graph_communities(
        &provider_manifest.to_string_lossy(),
        &provider_graph_dir,
    )?;
    let claim_cluster_dir = output_dir.join("clustering/claim-entity");
    let claim_clustering =
        cluster_claim_entities(&claim_manifest.to_string_lossy(), &claim_cluster_dir)?;

    let monitoring_inputs_dir = output_dir.join("monitoring-inputs");
    fs::create_dir_all(&monitoring_inputs_dir).with_context(|| {
        format!(
            "create monitoring input dir {}",
            monitoring_inputs_dir.display()
        )
    })?;
    let shadow_report = monitoring_inputs_dir.join("shadow_report.json");
    let drift_report = monitoring_inputs_dir.join("drift_report.json");
    let fairness_report = monitoring_inputs_dir.join("fairness_report.json");
    write_json(
        shadow_report.clone(),
        &serde_json::json!({
            "status": "passed",
            "comparison_count": 128,
            "average_abs_probability_delta": 0.04,
            "max_abs_probability_delta": 0.12
        }),
    )?;
    write_json(
        drift_report.clone(),
        &serde_json::json!({
            "status": "stable",
            "score_psi": 0.05,
            "max_feature_psi": 0.08
        }),
    )?;
    write_json(
        fairness_report.clone(),
        &serde_json::json!({
            "status": "passed",
            "segments": [
                {"segment_column": "provider_risk_tier", "segment_value": "low"},
                {"segment_column": "provider_risk_tier", "segment_value": "high"}
            ]
        }),
    )?;
    let scheduler_dir = output_dir.join("monitoring/scheduler");
    fs::create_dir_all(&scheduler_dir)
        .with_context(|| format!("create MLOps scheduler dir {}", scheduler_dir.display()))?;
    let monitoring_plan = build_mlops_monitoring_plan(
        &labeled_manifest.to_string_lossy(),
        &monitoring_inputs_dir
            .join("rust_serving_artifact.json")
            .to_string_lossy(),
        "baseline_fwa",
        "0.2.0-xgboost-candidate",
        "0 2 * * *",
    )?;
    let monitoring_plan_uri = scheduler_dir.join("mlops_monitoring_plan.json");
    write_json(monitoring_plan_uri.clone(), &monitoring_plan)?;
    build_mlops_monitoring_report(
        "baseline_fwa",
        "0.2.0-xgboost-candidate",
        &xgboost_artifact_eval.to_string_lossy(),
        &shadow_report.to_string_lossy(),
        &drift_report.to_string_lossy(),
        &fairness_report.to_string_lossy(),
        output_dir.join("monitoring"),
    )?;
    build_mlops_scheduler_execution_report(
        &monitoring_plan_uri.to_string_lossy(),
        &output_dir
            .join("monitoring/mlops_monitoring_report.json")
            .to_string_lossy(),
        &scheduler_dir,
    )?;

    let closure = build_automl_lifecycle_closure_report(
        &index_uri.to_string_lossy(),
        &output_dir
            .join("ranking/automl_candidate_ranking.json")
            .to_string_lossy(),
        &[
            xgboost_artifact_eval.to_string_lossy().into_owned(),
            lightgbm_artifact_eval.to_string_lossy().into_owned(),
        ],
        &rule_backtest_dir
            .join("rule_candidate_backtest_report.json")
            .to_string_lossy(),
        &provider_cluster_dir
            .join("provider_peer_clustering_report.json")
            .to_string_lossy(),
        &provider_graph_dir
            .join("provider_graph_community_report.json")
            .to_string_lossy(),
        &claim_cluster_dir
            .join("claim_entity_clustering_report.json")
            .to_string_lossy(),
        &output_dir
            .join("monitoring/mlops_monitoring_report.json")
            .to_string_lossy(),
        &scheduler_dir
            .join("mlops_scheduler_execution_report.json")
            .to_string_lossy(),
        output_dir.join("closure"),
    )?;

    let index = serde_json::json!({
        "evidence_pack_kind": "rust_automl_demo_lifecycle_evidence",
        "evidence_pack_version": 1,
        "demo_root": demo_root.to_string_lossy(),
        "output_dir": output_dir.to_string_lossy(),
        "recommended_candidate_model_version": ranking.recommended_candidate_model_version,
        "provider_anomaly_candidate_count": provider_clustering.anomaly_candidates.len(),
        "provider_graph_anomaly_candidate_count": provider_graph.anomaly_candidates.len(),
        "claim_entity_anomaly_candidate_count": claim_clustering.anomaly_candidates.len(),
        "closure_status": closure["closure_status"],
        "artifacts": {
            "xgboost_validation_report": xgboost_validation,
            "lightgbm_validation_report": lightgbm_validation,
            "candidate_ranking": output_dir.join("ranking/automl_candidate_ranking.json"),
            "xgboost_artifact_evaluation": xgboost_artifact_eval,
            "lightgbm_artifact_evaluation": lightgbm_artifact_eval,
            "feature_importance": feature_importance_uri,
            "rule_candidate_plan": rule_candidate_dir.join("rule_candidate_mining_plan.json"),
            "rule_backtest_report": rule_backtest_dir.join("rule_candidate_backtest_report.json"),
            "provider_clustering_report": provider_cluster_dir.join("provider_peer_clustering_report.json"),
            "provider_graph_report": provider_graph_dir.join("provider_graph_community_report.json"),
            "claim_entity_clustering_report": claim_cluster_dir.join("claim_entity_clustering_report.json"),
            "mlops_monitoring_report": output_dir.join("monitoring/mlops_monitoring_report.json"),
            "mlops_monitoring_plan": monitoring_plan_uri,
            "mlops_scheduler_execution_report": scheduler_dir.join("mlops_scheduler_execution_report.json"),
            "lifecycle_closure_report": output_dir.join("closure/rust_automl_lifecycle_closure_report.json")
        },
        "governance_boundary": "demo evidence only; the pack proves the Rust AutoML lifecycle contract but must not activate models, assign fraud labels, or write rules without the recorded human gates"
    });
    write_json(
        output_dir.join("demo_lifecycle_evidence_index.json"),
        &index,
    )?;
    Ok(index)
}

fn write_demo_validation_report(
    path: &Path,
    candidate_model_version: &str,
    algorithm: &str,
    runtime_kind: &str,
    auc: f64,
    precision: f64,
    recall: f64,
) -> anyhow::Result<()> {
    write_json(
        path.to_path_buf(),
        &serde_json::json!({
            "model_key": "baseline_fwa",
            "candidate_model_version": candidate_model_version,
            "dataset_key": "rust_demo_claim_risk_labeled",
            "dataset_version": "2026-06-rust-automl-demo",
            "algorithm": algorithm,
            "validation_metrics": {
                "auc": auc,
                "precision": precision,
                "recall": recall
            },
            "metrics_json": {
                "algorithm": algorithm,
                "algorithm_family": "gradient_boosted_tree",
                "runtime_kind": runtime_kind,
                "out_of_time_auc": auc - 0.02,
                "out_of_time_average_precision": auc - 0.05,
                "out_of_time_precision": precision,
                "out_of_time_recall": recall,
                "time_group_split_status": "passed",
                "leakage_check_status": "passed",
                "shadow_comparison_status": "passed",
                "serving_version_lock_status": "passed",
                "artifact_integrity_status": "passed",
                "feature_store_materialization_status": "passed",
                "rust_feature_set_status": "passed",
                "rust_feature_set_manifest_uri": format!("data/rust-automl-demo/labeled_claim_risk/feature-set/feature_set_manifest.json"),
                "segment_fairness_status": "passed",
                "model_artifact_evaluation_status": "passed",
                "onnx_parity_status": "passed",
                "onnx_parity_gate_status": "passed",
                "onnx_parity_report_uri": format!("data/rust-automl-demo/lifecycle-evidence/onnx-parity/{candidate_model_version}_onnx_parity_report.json"),
                "label_provenance_status": "passed"
            }
        }),
    )
}

fn write_demo_artifact_evaluation_report(
    path: &Path,
    model_version: &str,
    runtime_kind: &str,
    p95_latency_ms: u64,
) -> anyhow::Result<()> {
    write_json(
        path.to_path_buf(),
        &serde_json::json!({
            "report_kind": "model_artifact_evaluation",
            "report_version": 1,
            "model_key": "baseline_fwa",
            "model_version": model_version,
            "runtime_kind": runtime_kind,
            "serving_manifest_uri": format!("data/rust-automl-demo/lifecycle-evidence/serving/{model_version}/serving_manifest.json"),
            "dataset_key": "rust_demo_claim_risk_labeled",
            "dataset_version": "2026-06-rust-automl-demo",
            "evaluated_split": "validation",
            "row_count": 4,
            "contract_status": "passed",
            "rust_serving_status": "passed",
            "parity_status": "passed",
            "latency_status": "passed",
            "gate_status": "passed",
            "max_abs_probability_delta": 0.00001,
            "average_abs_probability_delta": 0.000004,
            "p95_latency_ms": p95_latency_ms,
            "latency_budget_ms": 100,
            "blocking_reasons": [],
            "sample_results": [],
            "evidence_refs": [
                format!("model_onnx_parity_reports:data/rust-automl-demo/lifecycle-evidence/onnx-parity/{model_version}_onnx_parity_report.json")
            ]
        }),
    )
}

fn write_demo_feature_importance_parquet(path: &Path) -> anyhow::Result<()> {
    let rows = [
        ("amount_to_limit_ratio", 0.39),
        ("peer_percentile", 0.28),
        ("high_cost_item_ratio", 0.20),
        ("provider_risk_tier", 0.08),
        ("diagnosis_procedure_mismatch", 0.05),
    ];
    let schema = Arc::new(Schema::new(vec![
        Field::new("feature", DataType::Utf8, false),
        Field::new("coefficient", DataType::Float64, true),
        Field::new("importance", DataType::Float64, false),
        Field::new("importance_kind", DataType::Utf8, false),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(
                rows.iter().map(|(feature, _)| *feature).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(vec![None; rows.len()])),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|(_, importance)| *importance)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(vec!["feature_importance"; rows.len()])),
        ],
    )?;
    write_parquet(path.to_path_buf(), schema, &batch)
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

fn json_array_len(value: &serde_json::Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(|value| value.as_array())
        .map(|items| items.len())
        .unwrap_or(0)
}

fn mlops_monitoring_review_tasks(
    model_key: &str,
    model_version: &str,
    triggers: &[&str],
) -> Vec<serde_json::Value> {
    triggers
        .iter()
        .map(|trigger| {
            let (review_queue, required_review) = match *trigger {
                "rust_serving_artifact_evaluation_blocked"
                | "rust_serving_latency_budget_failed" => {
                    ("mlops_serving_review", "review Rust serving runtime evidence")
                }
                "model_drift_detected" | "model_drift_watch" => {
                    ("mlops_drift_review", "review drift and retraining readiness")
                }
                "shadow_comparison_review_required" => {
                    ("mlops_shadow_review", "review shadow traffic comparison")
                }
                "segment_fairness_review_required" => {
                    ("model_governance_review", "review segment fairness evidence")
                }
                _ => ("mlops_review", "review MLOps monitoring trigger"),
            };
            serde_json::json!({
                "task_kind": "mlops_monitoring_review",
                "model_key": model_key,
                "model_version": model_version,
                "trigger": trigger,
                "review_queue": review_queue,
                "required_review": required_review,
                "decision_options": ["acknowledge_monitoring", "prepare_retraining", "open_governance_review"]
            })
        })
        .collect()
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

pub async fn evaluate_model_artifact(
    serving_manifest_uri: &str,
    dataset_manifest_uri: &str,
    split_name: &str,
    output_dir: impl AsRef<Path>,
    expected_probability_column: Option<&str>,
    probability_tolerance: f64,
    latency_budget_ms: u64,
    max_rows: usize,
    signing_key: Option<&str>,
) -> anyhow::Result<ModelArtifactEvaluationReport> {
    if probability_tolerance < 0.0 {
        bail!("probability_tolerance must be non-negative");
    }
    if max_rows == 0 {
        bail!("max_rows must be greater than zero");
    }

    let serving_manifest_path = Path::new(serving_manifest_uri);
    let serving_manifest_json = fs::read_to_string(serving_manifest_path)
        .with_context(|| format!("read serving manifest {}", serving_manifest_path.display()))?;
    let serving_manifest: WorkerServingManifest =
        serde_json::from_str(&serving_manifest_json).context("parse serving manifest")?;
    validate_worker_serving_manifest(&serving_manifest)?;

    let dataset_manifest_path = Path::new(dataset_manifest_uri);
    let dataset_manifest_json = fs::read_to_string(dataset_manifest_path)
        .with_context(|| format!("read dataset manifest {}", dataset_manifest_path.display()))?;
    let dataset_manifest: ParquetDatasetManifest =
        serde_json::from_str(&dataset_manifest_json).context("parse dataset manifest")?;
    let dataset_base_dir = dataset_manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let rows = read_model_artifact_evaluation_rows(
        &dataset_manifest,
        dataset_base_dir,
        split_name,
        &serving_manifest.feature_columns,
        expected_probability_column,
        max_rows,
    )?;

    let mut scorer = ServingManifestModelScorer::new(serving_manifest_uri);
    if let Some(signing_key) = signing_key {
        scorer = scorer.with_signing_key(signing_key);
    }

    let mut sample_results = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let score = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external(format!(
                    "artifact_eval_{}_{}",
                    safe_id_segment(&serving_manifest.model_key),
                    index
                )),
                claim_id: ClaimId::from_external(row.claim_id.clone()),
                model_key: serving_manifest.model_key.clone(),
                model_version: serving_manifest.model_version.clone(),
                endpoint_url: None,
                features: feature_map_from_values(&row.features),
            })
            .await
            .with_context(|| format!("score evaluation row {}", row.claim_id))?;
        let fraud_probability = score
            .metadata
            .get("fraud_probability")
            .and_then(|value| value.as_f64());
        let abs_probability_delta = match (fraud_probability, row.expected_probability) {
            (Some(actual), Some(expected)) => Some((actual - expected).abs()),
            _ => None,
        };
        sample_results.push(ModelArtifactEvaluationSample {
            claim_id: row.claim_id.clone(),
            score: score.score,
            label: score.label,
            fraud_probability,
            expected_probability: row.expected_probability,
            abs_probability_delta: abs_probability_delta.map(round4),
            latency_ms: score.latency_ms,
        });
    }

    let deltas = sample_results
        .iter()
        .filter_map(|sample| sample.abs_probability_delta)
        .collect::<Vec<_>>();
    let max_abs_probability_delta = deltas.iter().copied().reduce(f64::max).map(round4);
    let average_abs_probability_delta = if deltas.is_empty() {
        None
    } else {
        Some(round4(deltas.iter().sum::<f64>() / deltas.len() as f64))
    };
    let parity_status = if expected_probability_column.is_none() {
        "not_configured"
    } else if deltas.len() != sample_results.len() {
        "failed"
    } else if max_abs_probability_delta.unwrap_or(0.0) <= probability_tolerance {
        "passed"
    } else {
        "failed"
    }
    .to_string();
    let p95_latency_ms = percentile_latency_ms(&sample_results, 0.95);
    let latency_status = if p95_latency_ms <= latency_budget_ms {
        "passed"
    } else {
        "failed"
    }
    .to_string();
    let rust_serving_status = if sample_results.is_empty() {
        "failed"
    } else {
        "passed"
    }
    .to_string();
    let mut blocking_reasons = Vec::new();
    if parity_status == "failed" {
        blocking_reasons.push("serving_probability_parity_failed".into());
    }
    if latency_status == "failed" {
        blocking_reasons.push("serving_latency_budget_failed".into());
    }
    if rust_serving_status == "failed" {
        blocking_reasons.push("rust_serving_execution_failed".into());
    }
    let gate_status = if blocking_reasons.is_empty() {
        "passed"
    } else {
        "blocked"
    }
    .to_string();

    let report = ModelArtifactEvaluationReport {
        report_kind: "model_artifact_evaluation".into(),
        report_version: 1,
        model_key: serving_manifest.model_key.clone(),
        model_version: serving_manifest.model_version.clone(),
        runtime_kind: serving_manifest.runtime_kind.clone(),
        serving_manifest_uri: serving_manifest_uri.into(),
        dataset_key: dataset_manifest.dataset_key,
        dataset_version: dataset_manifest.dataset_version,
        evaluated_split: split_name.into(),
        row_count: sample_results.len(),
        contract_status: "passed".into(),
        rust_serving_status,
        parity_status,
        latency_status,
        gate_status,
        max_abs_probability_delta,
        average_abs_probability_delta,
        p95_latency_ms,
        latency_budget_ms,
        blocking_reasons,
        sample_results,
        evidence_refs: vec![
            format!("serving_manifests:{serving_manifest_uri}"),
            format!("model_artifacts:{}", serving_manifest.artifact_uri),
            format!(
                "model_artifact_checksums:{}",
                serving_manifest.artifact_sha256
            ),
            format!("dataset_manifests:{dataset_manifest_uri}"),
        ],
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create model artifact evaluation output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("model_artifact_evaluation_report.json"),
        &report,
    )?;
    Ok(report)
}

fn validate_worker_serving_manifest(manifest: &WorkerServingManifest) -> anyhow::Result<()> {
    required_non_empty("model_key", &manifest.model_key)?;
    required_non_empty("model_version", &manifest.model_version)?;
    required_non_empty("runtime_kind", &manifest.runtime_kind)?;
    required_non_empty("artifact_uri", &manifest.artifact_uri)?;
    required_non_empty("artifact_sha256", &manifest.artifact_sha256)?;
    if manifest.version_lock != manifest.model_version {
        bail!(
            "serving manifest version_lock mismatch: expected {}, got {}",
            manifest.model_version,
            manifest.version_lock
        );
    }
    if manifest.feature_columns.is_empty() {
        bail!("serving manifest feature_columns must not be empty");
    }
    Ok(())
}

fn read_model_artifact_evaluation_rows(
    manifest: &ParquetDatasetManifest,
    base_dir: &Path,
    split_name: &str,
    feature_columns: &[String],
    expected_probability_column: Option<&str>,
    max_rows: usize,
) -> anyhow::Result<Vec<ModelArtifactEvaluationRow>> {
    if feature_columns.is_empty() {
        bail!("model artifact evaluation requires at least one feature column");
    }
    let Some(split) = manifest
        .splits
        .iter()
        .find(|split| split.split_name == split_name)
    else {
        bail!("dataset manifest missing split {split_name}");
    };
    reject_csv_uri(&split.data_uri)?;
    let parquet_files = resolve_parquet_files(base_dir, &split.data_uri)?;
    if parquet_files.is_empty() {
        bail!("split {} has no parquet files", split.split_name);
    }

    let mut rows = Vec::new();
    for parquet_file in parquet_files {
        if rows.len() >= max_rows {
            break;
        }
        let file = File::open(&parquet_file)
            .with_context(|| format!("open parquet file {}", parquet_file.display()))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .with_context(|| format!("read parquet metadata {}", parquet_file.display()))?;
        let mut reader = builder.with_batch_size(4096).build()?;
        for batch in &mut reader {
            if rows.len() >= max_rows {
                break;
            }
            let batch = batch?;
            let feature_indexes = feature_columns
                .iter()
                .map(|feature| {
                    batch
                        .schema()
                        .index_of(feature)
                        .with_context(|| format!("missing model feature column {feature}"))
                        .map(|index| (feature.clone(), index))
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            let claim_index = batch.schema().index_of("claim_id").ok();
            let expected_index = expected_probability_column
                .map(|column| {
                    batch
                        .schema()
                        .index_of(column)
                        .with_context(|| format!("missing expected probability column {column}"))
                })
                .transpose()?;

            for row_index in 0..batch.num_rows() {
                if rows.len() >= max_rows {
                    break;
                }
                let claim_id = claim_index
                    .and_then(|index| column_value_at(batch.column(index).as_ref(), row_index))
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| format!("{}-row-{row_index}", split.split_name));
                let mut features = BTreeMap::new();
                for (feature, feature_index) in &feature_indexes {
                    let value = column_value_at(batch.column(*feature_index).as_ref(), row_index)
                        .and_then(|value| value.parse::<f64>().ok())
                        .with_context(|| {
                            format!("missing or invalid model feature {feature} at row {row_index}")
                        })?;
                    features.insert(feature.clone(), value);
                }
                let expected_probability = expected_index
                    .and_then(|index| column_value_at(batch.column(index).as_ref(), row_index))
                    .map(|value| {
                        value.parse::<f64>().with_context(|| {
                            format!(
                                "invalid expected probability value at row {row_index}: {value}"
                            )
                        })
                    })
                    .transpose()?;
                rows.push(ModelArtifactEvaluationRow {
                    claim_id,
                    features,
                    expected_probability,
                });
            }
        }
    }
    if rows.is_empty() {
        bail!("model artifact evaluation split {split_name} has no rows");
    }
    Ok(rows)
}

fn feature_map_from_values(values: &BTreeMap<String, f64>) -> FeatureMap {
    values
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                FeatureValue {
                    name: name.clone(),
                    version: 1,
                    value: serde_json::json!(value),
                    evidence_refs: Vec::new(),
                },
            )
        })
        .collect()
}

fn percentile_latency_ms(samples: &[ModelArtifactEvaluationSample], percentile: f64) -> u64 {
    if samples.is_empty() {
        return 0;
    }
    let mut latencies = samples
        .iter()
        .map(|sample| sample.latency_ms)
        .collect::<Vec<_>>();
    latencies.sort_unstable();
    let rank = ((latencies.len() as f64 * percentile).ceil() as usize).saturating_sub(1);
    latencies[rank.min(latencies.len() - 1)]
}

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
                    "scheme_family": "model_explanation_pattern",
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

    Ok(RuleCandidateBacktestResult {
        candidate_rule_key: candidate.candidate_rule_key.clone(),
        source_feature: candidate.source_feature.clone(),
        selected_operator: "gte".into(),
        selected_threshold: threshold,
        threshold_selection_split: if rows.iter().any(|row| row.split_name == "train") {
            "train".into()
        } else {
            "all_rows".into()
        },
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

pub fn cluster_provider_peers(
    manifest_path: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<ProviderPeerClusteringReport> {
    let manifest_path = Path::new(manifest_path);
    let manifest_json = fs::read_to_string(manifest_path)
        .with_context(|| format!("read provider peer manifest {}", manifest_path.display()))?;
    let manifest: UnlabeledDatasetManifest =
        serde_json::from_str(&manifest_json).context("parse provider peer manifest")?;
    if manifest.label_column.is_some() {
        bail!("provider peer clustering requires an unlabeled manifest");
    }
    if !manifest.label_policy.contains("unlabeled") {
        bail!("provider peer clustering requires an unlabeled label_policy");
    }
    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let rows = read_provider_peer_rows(&manifest, base_dir)?;
    if rows.len() < 2 {
        bail!("provider peer clustering requires at least two provider rows");
    }

    let feature_columns = vec![
        "claim_count".into(),
        "avg_claim_amount".into(),
        "high_cost_rate".into(),
        "peer_z_score".into(),
        "graph_degree".into(),
    ];
    let normalized = normalize_provider_rows(&rows);
    let cluster_count = rows.len().clamp(1, 3);
    let cluster_ids = assign_provider_clusters(&normalized, cluster_count);
    let distances = cluster_distances(&normalized, &cluster_ids, cluster_count);
    let threshold = anomaly_threshold(&distances);
    let mut provider_assignments = Vec::new();
    let mut anomaly_candidates = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let outlier_score = round4(distances[index]);
        let anomaly_candidate = distances[index] >= threshold;
        provider_assignments.push(ProviderPeerClusterAssignment {
            provider_id: row.provider_id.clone(),
            cohort_key: row.cohort_key.clone(),
            service_month: row.service_month.clone(),
            cluster_id: cluster_ids[index],
            outlier_score,
            anomaly_candidate,
        });
        if anomaly_candidate {
            anomaly_candidates.push(ProviderPeerAnomalyCandidate {
                provider_id: row.provider_id.clone(),
                cohort_key: row.cohort_key.clone(),
                service_month: row.service_month.clone(),
                cluster_id: cluster_ids[index],
                outlier_score,
                reason: "Provider-month is far from its peer-cluster centroid; review as an anomaly candidate, not a confirmed FWA label.".into(),
                evidence_refs: vec![
                    format!("dataset_manifest:{}", manifest_path.display()),
                    format!("provider_peer_cluster:{}:{}", manifest.dataset_key, row.provider_id),
                ],
            });
        }
    }
    anomaly_candidates.sort_by(|left, right| {
        right
            .outlier_score
            .partial_cmp(&left.outlier_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.provider_id.cmp(&right.provider_id))
    });

    let review_tasks = anomaly_candidates
        .iter()
        .map(|candidate| ProviderPeerReviewTask {
            task_kind: "provider_peer_anomaly_review".into(),
            provider_id: candidate.provider_id.clone(),
            review_queue: "provider_anomaly_candidate_review".into(),
            required_review: "human_review_required_before_case_creation_or_label_assignment"
                .into(),
            decision_options: vec![
                "dismiss_as_peer_variation".into(),
                "request_more_evidence".into(),
                "open_investigation_candidate".into(),
            ],
            evidence_refs: candidate.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();
    let cluster_summaries =
        summarize_provider_clusters(&rows, &cluster_ids, &distances, cluster_count);
    let report = ProviderPeerClusteringReport {
        report_kind: "provider_peer_clustering".into(),
        report_version: 1,
        dataset_key: manifest.dataset_key,
        dataset_version: manifest.dataset_version,
        algorithm: "rust_standardized_kmeans_v1".into(),
        label_policy: manifest.label_policy,
        governance_boundary:
            "unlabeled clustering creates anomaly review candidates only; it must not create confirmed FWA labels or automatic claim disposition"
                .into(),
        feature_columns,
        cluster_count,
        cluster_summaries,
        provider_assignments,
        anomaly_candidates,
        review_tasks,
        evidence_refs: vec![format!("dataset_manifest:{}", manifest_path.display())],
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create provider peer clustering output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("provider_peer_clustering_report.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("provider_anomaly_review_tasks.json"),
        &report.review_tasks,
    )?;
    Ok(report)
}

pub fn cluster_claim_entities(
    manifest_path: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<ClaimEntityClusteringReport> {
    let manifest_path = Path::new(manifest_path);
    let manifest_json = fs::read_to_string(manifest_path)
        .with_context(|| format!("read claim entity manifest {}", manifest_path.display()))?;
    let manifest: UnlabeledDatasetManifest =
        serde_json::from_str(&manifest_json).context("parse claim entity manifest")?;
    if manifest.label_column.is_some() {
        bail!("claim entity clustering requires an unlabeled manifest");
    }
    if !manifest.label_policy.contains("unlabeled") {
        bail!("claim entity clustering requires an unlabeled label_policy");
    }
    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let rows = read_claim_entity_rows(&manifest, base_dir)?;
    if rows.len() < 2 {
        bail!("claim entity clustering requires at least two claim rows");
    }

    let feature_columns = vec![
        "claim_amount".into(),
        "amount_to_limit_ratio".into(),
        "peer_percentile".into(),
        "item_count".into(),
        "high_cost_item_ratio".into(),
        "provider_risk_tier".into(),
        "diagnosis_procedure_mismatch".into(),
        "member_degree".into(),
        "provider_degree".into(),
    ];
    let normalized = normalize_claim_entity_rows(&rows);
    let cluster_count = rows.len().clamp(1, 4);
    let cluster_ids = assign_standardized_clusters(&normalized, 2, cluster_count);
    let distances = standardized_cluster_distances(&normalized, &cluster_ids, cluster_count);
    let threshold = anomaly_threshold(&distances);
    let mut entity_assignments = Vec::new();
    let mut anomaly_candidates = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let outlier_score = round4(distances[index]);
        let anomaly_candidate = distances[index] >= threshold;
        entity_assignments.push(ClaimEntityClusterAssignment {
            claim_id: row.claim_id.clone(),
            member_id: row.member_id.clone(),
            provider_id: row.provider_id.clone(),
            cluster_id: cluster_ids[index],
            outlier_score,
            anomaly_candidate,
        });
        if anomaly_candidate {
            anomaly_candidates.push(ClaimEntityAnomalyCandidate {
                claim_id: row.claim_id.clone(),
                member_id: row.member_id.clone(),
                provider_id: row.provider_id.clone(),
                cluster_id: cluster_ids[index],
                outlier_score,
                reason: "Claim/member/provider entity context is far from its cluster centroid; review as an anomaly candidate, not a confirmed FWA label.".into(),
                evidence_refs: vec![
                    format!("dataset_manifest:{}", manifest_path.display()),
                    format!("claim_entity_cluster:{}:{}", manifest.dataset_key, row.claim_id),
                ],
            });
        }
    }
    anomaly_candidates.sort_by(|left, right| {
        right
            .outlier_score
            .partial_cmp(&left.outlier_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.claim_id.cmp(&right.claim_id))
    });

    let review_tasks = anomaly_candidates
        .iter()
        .map(|candidate| ClaimEntityReviewTask {
            task_kind: "claim_entity_anomaly_review".into(),
            claim_id: candidate.claim_id.clone(),
            member_id: candidate.member_id.clone(),
            provider_id: candidate.provider_id.clone(),
            review_queue: "claim_entity_anomaly_candidate_review".into(),
            required_review:
                "human_review_required_before_case_creation_label_assignment_or_rule_writeback"
                    .into(),
            decision_options: vec![
                "dismiss_as_entity_variation".into(),
                "request_more_evidence".into(),
                "open_investigation_candidate".into(),
                "prepare_rule_candidate_backtest".into(),
            ],
            evidence_refs: candidate.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();
    let cluster_summaries =
        summarize_claim_entity_clusters(&rows, &cluster_ids, &distances, cluster_count);
    let report = ClaimEntityClusteringReport {
        report_kind: "claim_entity_clustering".into(),
        report_version: 1,
        dataset_key: manifest.dataset_key,
        dataset_version: manifest.dataset_version,
        algorithm: "rust_standardized_entity_kmeans_v1".into(),
        label_policy: manifest.label_policy,
        governance_boundary:
            "unlabeled entity clustering creates anomaly review candidates only; it must not create confirmed FWA labels, automatic claim disposition, or rule-library writeback"
                .into(),
        feature_columns,
        cluster_count,
        cluster_summaries,
        entity_assignments,
        anomaly_candidates,
        review_tasks,
        evidence_refs: vec![format!("dataset_manifest:{}", manifest_path.display())],
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create claim entity clustering output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("claim_entity_clustering_report.json"),
        &report,
    )?;
    write_json(
        output_dir.as_ref().join("claim_entity_review_tasks.json"),
        &report.review_tasks,
    )?;
    Ok(report)
}

pub fn cluster_provider_graph_communities(
    manifest_path: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<ProviderGraphCommunityReport> {
    let manifest_path = Path::new(manifest_path);
    let manifest_json = fs::read_to_string(manifest_path)
        .with_context(|| format!("read provider graph manifest {}", manifest_path.display()))?;
    let manifest: UnlabeledDatasetManifest =
        serde_json::from_str(&manifest_json).context("parse provider graph manifest")?;
    if manifest.label_column.is_some() {
        bail!("provider graph clustering requires an unlabeled manifest");
    }
    if !manifest.label_policy.contains("unlabeled") {
        bail!("provider graph clustering requires an unlabeled label_policy");
    }
    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let rows = read_provider_peer_rows(&manifest, base_dir)?;
    if rows.len() < 2 {
        bail!("provider graph clustering requires at least two provider rows");
    }

    let graph_degree_threshold =
        anomaly_threshold(&rows.iter().map(|row| row.graph_degree).collect::<Vec<_>>());
    let mut provider_assignments = Vec::new();
    let mut anomaly_candidates = Vec::new();
    for row in &rows {
        let anomaly_candidate =
            row.graph_degree >= graph_degree_threshold || row.peer_z_score >= 2.0;
        provider_assignments.push(ProviderGraphCommunityAssignment {
            provider_id: row.provider_id.clone(),
            community_id: row.community_id,
            graph_degree: row.graph_degree,
            peer_z_score: row.peer_z_score,
            anomaly_candidate,
        });
        if anomaly_candidate {
            anomaly_candidates.push(ProviderGraphAnomalyCandidate {
                provider_id: row.provider_id.clone(),
                community_id: row.community_id,
                graph_degree: row.graph_degree,
                peer_z_score: row.peer_z_score,
                reason: "Provider is unusually central or high-risk inside the provider graph community; review as a graph anomaly candidate, not a confirmed FWA label.".into(),
                evidence_refs: vec![
                    format!("dataset_manifest:{}", manifest_path.display()),
                    format!(
                        "provider_graph_community:{}:{}",
                        manifest.dataset_key, row.provider_id
                    ),
                ],
            });
        }
    }
    anomaly_candidates.sort_by(|left, right| {
        right
            .graph_degree
            .partial_cmp(&left.graph_degree)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .peer_z_score
                    .partial_cmp(&left.peer_z_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.provider_id.cmp(&right.provider_id))
    });
    let review_tasks = anomaly_candidates
        .iter()
        .map(|candidate| ProviderGraphReviewTask {
            task_kind: "provider_graph_anomaly_review".into(),
            provider_id: candidate.provider_id.clone(),
            community_id: candidate.community_id,
            review_queue: "provider_graph_anomaly_candidate_review".into(),
            required_review: "human_review_required_before_case_creation_or_label_assignment"
                .into(),
            decision_options: vec![
                "dismiss_as_network_variation".into(),
                "request_more_evidence".into(),
                "open_investigation_candidate".into(),
            ],
            evidence_refs: candidate.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();
    let community_summaries = summarize_provider_graph_communities(&rows, &provider_assignments);
    let report = ProviderGraphCommunityReport {
        report_kind: "provider_graph_community_clustering".into(),
        report_version: 1,
        dataset_key: manifest.dataset_key,
        dataset_version: manifest.dataset_version,
        algorithm: "rust_provider_graph_community_v1".into(),
        label_policy: manifest.label_policy,
        governance_boundary:
            "unlabeled graph clustering creates anomaly review candidates only; it must not create confirmed FWA labels or automatic claim disposition"
                .into(),
        community_summaries,
        provider_assignments,
        anomaly_candidates,
        review_tasks,
        evidence_refs: vec![format!("dataset_manifest:{}", manifest_path.display())],
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create provider graph clustering output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("provider_graph_community_report.json"),
        &report,
    )?;
    write_json(
        output_dir.as_ref().join("provider_graph_review_tasks.json"),
        &report.review_tasks,
    )?;
    Ok(report)
}

fn read_provider_peer_rows(
    manifest: &UnlabeledDatasetManifest,
    base_dir: &Path,
) -> anyhow::Result<Vec<ProviderPeerFeatureRow>> {
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
                for row_index in 0..batch.num_rows() {
                    rows.push(ProviderPeerFeatureRow {
                        provider_id: required_string_cell(&batch, "provider_id", row_index)?,
                        cohort_key: required_string_cell(&batch, "cohort_key", row_index)?,
                        service_month: required_string_cell(&batch, "service_month", row_index)?,
                        claim_count: required_numeric_cell(&batch, "claim_count", row_index)?,
                        avg_claim_amount: required_numeric_cell(
                            &batch,
                            "avg_claim_amount",
                            row_index,
                        )?,
                        high_cost_rate: required_numeric_cell(&batch, "high_cost_rate", row_index)?,
                        peer_z_score: required_numeric_cell(&batch, "peer_z_score", row_index)?,
                        graph_degree: required_numeric_cell(&batch, "graph_degree", row_index)?,
                        community_id: required_numeric_cell(&batch, "community_id", row_index)?
                            as i32,
                    });
                }
            }
        }
    }
    Ok(rows)
}

fn read_claim_entity_rows(
    manifest: &UnlabeledDatasetManifest,
    base_dir: &Path,
) -> anyhow::Result<Vec<ClaimEntityFeatureRow>> {
    let mut raw_rows = Vec::new();
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
                for row_index in 0..batch.num_rows() {
                    raw_rows.push((
                        required_string_cell(&batch, "claim_id", row_index)?,
                        required_string_cell(&batch, "member_id", row_index)?,
                        required_string_cell(&batch, "provider_id", row_index)?,
                        required_numeric_cell(&batch, "claim_amount", row_index)?,
                        required_numeric_cell(&batch, "amount_to_limit_ratio", row_index)?,
                        required_numeric_cell(&batch, "peer_percentile", row_index)?,
                        required_numeric_cell(&batch, "item_count", row_index)?,
                        required_numeric_cell(&batch, "high_cost_item_ratio", row_index)?,
                        required_numeric_cell(&batch, "provider_risk_tier", row_index)?,
                        required_numeric_cell(&batch, "diagnosis_procedure_mismatch", row_index)?,
                    ));
                }
            }
        }
    }

    let mut member_counts = BTreeMap::<String, u64>::new();
    let mut provider_counts = BTreeMap::<String, u64>::new();
    for (_, member_id, provider_id, ..) in &raw_rows {
        *member_counts.entry(member_id.clone()).or_default() += 1;
        *provider_counts.entry(provider_id.clone()).or_default() += 1;
    }

    Ok(raw_rows
        .into_iter()
        .map(
            |(
                claim_id,
                member_id,
                provider_id,
                claim_amount,
                amount_to_limit_ratio,
                peer_percentile,
                item_count,
                high_cost_item_ratio,
                provider_risk_tier,
                diagnosis_procedure_mismatch,
            )| {
                let member_degree = member_counts.get(&member_id).copied().unwrap_or(1) as f64;
                let provider_degree =
                    provider_counts.get(&provider_id).copied().unwrap_or(1) as f64;
                ClaimEntityFeatureRow {
                    claim_id,
                    member_id,
                    provider_id,
                    claim_amount,
                    amount_to_limit_ratio,
                    peer_percentile,
                    item_count,
                    high_cost_item_ratio,
                    provider_risk_tier,
                    diagnosis_procedure_mismatch,
                    member_degree,
                    provider_degree,
                }
            },
        )
        .collect())
}

fn required_string_cell(
    batch: &RecordBatch,
    column_name: &str,
    row_index: usize,
) -> anyhow::Result<String> {
    let column_index = batch
        .schema()
        .index_of(column_name)
        .with_context(|| format!("missing provider peer column {column_name}"))?;
    column_value_at(batch.column(column_index).as_ref(), row_index)
        .filter(|value| !value.trim().is_empty())
        .with_context(|| format!("missing provider peer value {column_name} at row {row_index}"))
}

fn required_numeric_cell(
    batch: &RecordBatch,
    column_name: &str,
    row_index: usize,
) -> anyhow::Result<f64> {
    let value = required_string_cell(batch, column_name, row_index)?;
    value
        .parse::<f64>()
        .with_context(|| format!("invalid numeric provider peer value {column_name}: {value}"))
}

fn normalize_provider_rows(rows: &[ProviderPeerFeatureRow]) -> Vec<[f64; 5]> {
    let raw = rows
        .iter()
        .map(|row| {
            [
                row.claim_count,
                row.avg_claim_amount,
                row.high_cost_rate,
                row.peer_z_score,
                row.graph_degree,
            ]
        })
        .collect::<Vec<_>>();
    let mut means = [0.0; 5];
    for values in &raw {
        for index in 0..5 {
            means[index] += values[index];
        }
    }
    for mean in &mut means {
        *mean /= raw.len() as f64;
    }
    let mut stddevs = [0.0; 5];
    for values in &raw {
        for index in 0..5 {
            stddevs[index] += (values[index] - means[index]).powi(2);
        }
    }
    for stddev in &mut stddevs {
        *stddev = (*stddev / raw.len() as f64).sqrt();
        if *stddev == 0.0 {
            *stddev = 1.0;
        }
    }
    raw.iter()
        .map(|values| {
            let mut normalized = [0.0; 5];
            for index in 0..5 {
                normalized[index] = (values[index] - means[index]) / stddevs[index];
            }
            normalized
        })
        .collect()
}

fn normalize_claim_entity_rows(rows: &[ClaimEntityFeatureRow]) -> Vec<[f64; 9]> {
    let raw = rows
        .iter()
        .map(|row| {
            [
                row.claim_amount,
                row.amount_to_limit_ratio,
                row.peer_percentile,
                row.item_count,
                row.high_cost_item_ratio,
                row.provider_risk_tier,
                row.diagnosis_procedure_mismatch,
                row.member_degree,
                row.provider_degree,
            ]
        })
        .collect::<Vec<_>>();
    let mut means = [0.0; 9];
    for values in &raw {
        for index in 0..9 {
            means[index] += values[index];
        }
    }
    for mean in &mut means {
        *mean /= raw.len() as f64;
    }
    let mut stddevs = [0.0; 9];
    for values in &raw {
        for index in 0..9 {
            stddevs[index] += (values[index] - means[index]).powi(2);
        }
    }
    for stddev in &mut stddevs {
        *stddev = (*stddev / raw.len() as f64).sqrt();
        if *stddev == 0.0 {
            *stddev = 1.0;
        }
    }
    raw.iter()
        .map(|values| {
            let mut normalized = [0.0; 9];
            for index in 0..9 {
                normalized[index] = (values[index] - means[index]) / stddevs[index];
            }
            normalized
        })
        .collect()
}

fn assign_provider_clusters(rows: &[[f64; 5]], cluster_count: usize) -> Vec<usize> {
    assign_standardized_clusters(rows, 3, cluster_count)
}

fn assign_standardized_clusters<const N: usize>(
    rows: &[[f64; N]],
    ordering_feature_index: usize,
    cluster_count: usize,
) -> Vec<usize> {
    let mut ordered = rows.iter().enumerate().collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left.1[ordering_feature_index]
            .partial_cmp(&right.1[ordering_feature_index])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut centroids = (0..cluster_count)
        .map(|cluster_index| {
            let source_index = cluster_index * (ordered.len() - 1) / cluster_count.max(1);
            *ordered[source_index].1
        })
        .collect::<Vec<_>>();
    let mut assignments = vec![0; rows.len()];
    for _ in 0..12 {
        for (row_index, row) in rows.iter().enumerate() {
            assignments[row_index] = nearest_centroid(row, &centroids);
        }
        let mut sums = vec![[0.0; 5]; cluster_count];
        let mut counts = vec![0_usize; cluster_count];
        for (row, cluster_id) in rows.iter().zip(assignments.iter()) {
            counts[*cluster_id] += 1;
            for index in 0..5 {
                sums[*cluster_id][index] += row[index];
            }
        }
        for cluster_id in 0..cluster_count {
            if counts[cluster_id] == 0 {
                continue;
            }
            for index in 0..5 {
                centroids[cluster_id][index] = sums[cluster_id][index] / counts[cluster_id] as f64;
            }
        }
    }
    assignments
}

fn nearest_centroid<const N: usize>(row: &[f64; N], centroids: &[[f64; N]]) -> usize {
    centroids
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            squared_distance(row, *left)
                .partial_cmp(&squared_distance(row, *right))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn cluster_distances(rows: &[[f64; 5]], assignments: &[usize], cluster_count: usize) -> Vec<f64> {
    standardized_cluster_distances(rows, assignments, cluster_count)
}

fn standardized_cluster_distances<const N: usize>(
    rows: &[[f64; N]],
    assignments: &[usize],
    cluster_count: usize,
) -> Vec<f64> {
    let mut sums = vec![[0.0; N]; cluster_count];
    let mut counts = vec![0_usize; cluster_count];
    for (row, cluster_id) in rows.iter().zip(assignments.iter()) {
        counts[*cluster_id] += 1;
        for index in 0..N {
            sums[*cluster_id][index] += row[index];
        }
    }
    let mut centroids = vec![[0.0; N]; cluster_count];
    for cluster_id in 0..cluster_count {
        if counts[cluster_id] == 0 {
            continue;
        }
        for index in 0..N {
            centroids[cluster_id][index] = sums[cluster_id][index] / counts[cluster_id] as f64;
        }
    }
    rows.iter()
        .zip(assignments.iter())
        .map(|(row, cluster_id)| squared_distance(row, &centroids[*cluster_id]).sqrt())
        .collect()
}

fn squared_distance(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| (*left - *right).powi(2))
        .sum()
}

fn anomaly_threshold(distances: &[f64]) -> f64 {
    let mean = distances.iter().sum::<f64>() / distances.len() as f64;
    let variance = distances
        .iter()
        .map(|distance| (distance - mean).powi(2))
        .sum::<f64>()
        / distances.len() as f64;
    let threshold = mean + variance.sqrt();
    if distances.iter().any(|distance| *distance >= threshold) {
        threshold
    } else {
        distances
            .iter()
            .copied()
            .fold(0.0, |current, distance| current.max(distance))
    }
}

fn summarize_provider_clusters(
    rows: &[ProviderPeerFeatureRow],
    assignments: &[usize],
    distances: &[f64],
    cluster_count: usize,
) -> Vec<ProviderPeerClusterSummary> {
    (0..cluster_count)
        .map(|cluster_id| {
            let indexes = assignments
                .iter()
                .enumerate()
                .filter_map(|(index, assigned)| (*assigned == cluster_id).then_some(index))
                .collect::<Vec<_>>();
            let provider_count = indexes.len();
            let divisor = provider_count.max(1) as f64;
            ProviderPeerClusterSummary {
                cluster_id,
                provider_count,
                average_outlier_score: round4(
                    indexes.iter().map(|index| distances[*index]).sum::<f64>() / divisor,
                ),
                average_claim_count: round4(
                    indexes
                        .iter()
                        .map(|index| rows[*index].claim_count)
                        .sum::<f64>()
                        / divisor,
                ),
                average_high_cost_rate: round4(
                    indexes
                        .iter()
                        .map(|index| rows[*index].high_cost_rate)
                        .sum::<f64>()
                        / divisor,
                ),
            }
        })
        .collect()
}

fn summarize_claim_entity_clusters(
    rows: &[ClaimEntityFeatureRow],
    assignments: &[usize],
    distances: &[f64],
    cluster_count: usize,
) -> Vec<ClaimEntityClusterSummary> {
    (0..cluster_count)
        .map(|cluster_id| {
            let indexes = assignments
                .iter()
                .enumerate()
                .filter_map(|(index, assigned)| (*assigned == cluster_id).then_some(index))
                .collect::<Vec<_>>();
            let claim_count = indexes.len();
            let divisor = claim_count.max(1) as f64;
            ClaimEntityClusterSummary {
                cluster_id,
                claim_count,
                average_outlier_score: round4(
                    indexes.iter().map(|index| distances[*index]).sum::<f64>() / divisor,
                ),
                average_claim_amount: round4(
                    indexes
                        .iter()
                        .map(|index| rows[*index].claim_amount)
                        .sum::<f64>()
                        / divisor,
                ),
                average_provider_degree: round4(
                    indexes
                        .iter()
                        .map(|index| rows[*index].provider_degree)
                        .sum::<f64>()
                        / divisor,
                ),
            }
        })
        .collect()
}

fn summarize_provider_graph_communities(
    rows: &[ProviderPeerFeatureRow],
    assignments: &[ProviderGraphCommunityAssignment],
) -> Vec<ProviderGraphCommunitySummary> {
    let mut community_ids = rows
        .iter()
        .map(|row| row.community_id)
        .collect::<BTreeSet<_>>();
    if community_ids.is_empty() {
        community_ids.insert(0);
    }
    community_ids
        .into_iter()
        .map(|community_id| {
            let indexes = rows
                .iter()
                .enumerate()
                .filter_map(|(index, row)| (row.community_id == community_id).then_some(index))
                .collect::<Vec<_>>();
            let provider_count = indexes.len();
            let divisor = provider_count.max(1) as f64;
            let anomaly_candidate_count = assignments
                .iter()
                .filter(|assignment| {
                    assignment.community_id == community_id && assignment.anomaly_candidate
                })
                .count();
            ProviderGraphCommunitySummary {
                community_id,
                provider_count,
                average_graph_degree: round4(
                    indexes
                        .iter()
                        .map(|index| rows[*index].graph_degree)
                        .sum::<f64>()
                        / divisor,
                ),
                average_peer_z_score: round4(
                    indexes
                        .iter()
                        .map(|index| rows[*index].peer_z_score)
                        .sum::<f64>()
                        / divisor,
                ),
                anomaly_candidate_count,
            }
        })
        .collect()
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn read_feature_importance(path: &Path) -> anyhow::Result<Vec<FeatureImportanceRow>> {
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
            &gate_status,
        ),
        validation_auc,
        out_of_time_auc,
        out_of_time_average_precision,
        out_of_time_precision,
        out_of_time_recall,
        gate_status,
        blocking_reasons,
        recommended_action,
        evidence_refs: vec![
            format!("model_validation_reports:{validation_report_uri}"),
            format!(
                "model_evaluations:{}",
                safe_id_segment(&candidate_model_version)
            ),
        ],
    })
}

fn automl_blocking_reasons(metrics: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    let required_statuses = [
        "time_group_split_status",
        "leakage_check_status",
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
    reasons
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

fn mlops_monitoring_report_uris(report: &serde_json::Value) -> BTreeSet<String> {
    let mut uris = BTreeSet::new();
    if let Some(signals) = report.get("signals").and_then(|value| value.as_object()) {
        for signal in signals.values() {
            if let Some(uri) = json_string(signal, "report_uri") {
                uris.insert(uri);
            }
        }
    }
    if let Some(evidence_refs) = report
        .get("evidence_refs")
        .and_then(|value| value.as_array())
    {
        for evidence_ref in evidence_refs {
            let Some(evidence_ref) = evidence_ref.as_str() else {
                continue;
            };
            if let Some((_, uri)) = evidence_ref.split_once(':') {
                uris.insert(uri.to_string());
            }
        }
    }
    uris
}

fn mlops_plan_job_output_uri(job: &serde_json::Value) -> Option<String> {
    job.as_object().and_then(|object| {
        object.iter().find_map(|(key, value)| {
            if key.ends_with("_uri") {
                value.as_str().map(str::to_string)
            } else {
                None
            }
        })
    })
}

fn mlops_alert_delivery_task(
    model_key: &str,
    model_version: &str,
    trigger: &str,
    plan_uri: &str,
    monitoring_report_uri: &str,
) -> serde_json::Value {
    let (severity, route_key, recommended_action) = match trigger {
        "rust_serving_artifact_evaluation_blocked" => (
            "critical",
            "mlops_serving_runtime",
            "open serving artifact governance review",
        ),
        "rust_serving_latency_budget_failed" => (
            "high",
            "mlops_serving_runtime",
            "review latency budget before rollout or rollback decision",
        ),
        "model_drift_detected" => (
            "high",
            "mlops_retraining_readiness",
            "prepare retraining review after human approval",
        ),
        "model_drift_watch" => (
            "medium",
            "mlops_retraining_readiness",
            "monitor drift and schedule next comparison",
        ),
        "shadow_comparison_review_required" => (
            "high",
            "mlops_shadow_review",
            "review shadow comparison before promotion",
        ),
        "segment_fairness_review_required" => (
            "high",
            "model_governance",
            "open segment fairness governance review",
        ),
        _ => ("medium", "mlops_review", "review monitoring trigger"),
    };
    serde_json::json!({
        "task_kind": "mlops_alert_delivery",
        "model_key": model_key,
        "model_version": model_version,
        "trigger": trigger,
        "severity": severity,
        "route_key": route_key,
        "delivery_status": "queued_for_external_alert_router",
        "recommended_action": recommended_action,
        "evidence_refs": [
            format!("mlops_monitoring_plans:{plan_uri}"),
            format!("model_monitoring_reports:{monitoring_report_uri}"),
            format!("model_versions:{model_key}:{model_version}")
        ]
    })
}

fn automl_ranking_score(
    out_of_time_auc: Option<f64>,
    average_precision: Option<f64>,
    precision: Option<f64>,
    recall: Option<f64>,
    gate_status: &str,
) -> f64 {
    let score = out_of_time_auc.unwrap_or(0.0) * 60.0
        + average_precision.unwrap_or(0.0) * 20.0
        + precision.unwrap_or(0.0) * 10.0
        + recall.unwrap_or(0.0) * 10.0;
    let penalty = if gate_status == "passed" { 0.0 } else { 100.0 };
    ((score - penalty) * 10_000.0).round() / 10_000.0
}

fn eligible_sort_key(candidate: &AutoMlCandidateRank) -> f64 {
    if candidate.gate_status == "passed" {
        candidate.ranking_score
    } else {
        candidate.ranking_score - 1_000.0
    }
}

fn metric_at(value: &serde_json::Value, key: &str) -> Option<f64> {
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

pub fn build_analytics_export_plan(
    object_storage_uri: &str,
    clickhouse_url: &str,
    customer_scope_id: &str,
    cron: &str,
) -> anyhow::Result<serde_json::Value> {
    let object_storage_uri =
        required_non_empty("object_storage_uri", object_storage_uri)?.trim_end_matches('/');
    let clickhouse_url = required_non_empty("clickhouse_url", clickhouse_url)?;
    let customer_scope_id = required_non_empty("customer_scope_id", customer_scope_id)?;
    let cron = required_non_empty("cron", cron)?;
    let export_root = format!("{object_storage_uri}/analytics-exports/{customer_scope_id}");

    Ok(serde_json::json!({
        "plan_kind": "scheduled_analytics_export",
        "plan_version": 1,
        "customer_scope_id": customer_scope_id,
        "data_contract": {
            "source_of_truth": "postgresql_operational_tables",
            "derived_store": "clickhouse",
            "clickhouse_url": clickhouse_url,
            "schema_ref": "analytics/clickhouse/schema.sql",
            "dashboard_queries_ref": "analytics/clickhouse/dashboard_queries.sql",
            "pii_policy": "masked_ids_and_evidence_refs_only"
        },
        "schedule": {
            "cron": cron,
            "concurrency_policy": "forbid",
            "idempotency_key": "customer_scope_id + export_window_start + sink_table"
        },
        "export_root_uri": export_root,
        "jobs": [
            {
                "job_kind": "scoring_events_export",
                "source_tables": ["scoring_runs", "claims"],
                "sink_table": "fwa_analytics.analytics_scoring_events",
                "output_uri": format!("{export_root}/scoring_events/{{window_start}}.ndjson")
            },
            {
                "job_kind": "rule_events_export",
                "source_tables": ["rule_runs", "scoring_runs", "qa_reviews"],
                "sink_table": "fwa_analytics.analytics_rule_events",
                "output_uri": format!("{export_root}/rule_events/{{window_start}}.ndjson")
            },
            {
                "job_kind": "model_events_export",
                "source_tables": ["model_scores", "model_evaluation_runs", "scoring_runs"],
                "sink_table": "fwa_analytics.analytics_model_events",
                "output_uri": format!("{export_root}/model_events/{{window_start}}.ndjson")
            },
            {
                "job_kind": "case_sla_events_export",
                "source_tables": ["investigation_cases", "audit_events"],
                "sink_table": "fwa_analytics.analytics_case_sla_events",
                "output_uri": format!("{export_root}/case_sla_events/{{window_start}}.ndjson")
            },
            {
                "job_kind": "value_events_export",
                "source_tables": ["saving_attributions", "investigation_cases", "qa_reviews"],
                "sink_table": "fwa_analytics.analytics_value_events",
                "output_uri": format!("{export_root}/value_events/{{window_start}}.ndjson")
            },
            {
                "job_kind": "reviewer_capacity_events_export",
                "source_tables": ["investigation_cases", "qa_reviews"],
                "sink_table": "fwa_analytics.analytics_reviewer_capacity_events",
                "output_uri": format!("{export_root}/reviewer_capacity_events/{{window_start}}.ndjson")
            },
            {
                "job_kind": "provider_graph_snapshots_export",
                "source_tables": ["providers", "claims", "rule_runs"],
                "sink_table": "fwa_analytics.analytics_provider_graph_snapshots",
                "output_uri": format!("{export_root}/provider_graph_snapshots/{{window_start}}.ndjson")
            }
        ],
        "dashboard_coverage": [
            "rule_drift",
            "model_drift",
            "sla_reporting",
            "roi_reporting",
            "reviewer_capacity",
            "false_positive_cost",
            "provider_graph_snapshots"
        ]
    }))
}

pub fn build_ai_evidence_execution_plan(
    api_base_url: &str,
    object_storage_uri: &str,
    vector_store_kind: &str,
    vector_store_ref: &str,
    customer_scope_id: &str,
    cron: &str,
) -> anyhow::Result<serde_json::Value> {
    let api_base_url = required_non_empty("api_base_url", api_base_url)?.trim_end_matches('/');
    let object_storage_uri =
        required_non_empty("object_storage_uri", object_storage_uri)?.trim_end_matches('/');
    let vector_store_kind = required_non_empty("vector_store_kind", vector_store_kind)?;
    let vector_store_ref = required_non_empty("vector_store_ref", vector_store_ref)?;
    let customer_scope_id = required_non_empty("customer_scope_id", customer_scope_id)?;
    let cron = required_non_empty("cron", cron)?;
    let evidence_root = format!("{object_storage_uri}/ai-evidence/{customer_scope_id}");

    Ok(serde_json::json!({
        "plan_kind": "scheduled_ai_evidence_execution",
        "plan_version": 1,
        "customer_scope_id": customer_scope_id,
        "runtime_boundary": {
            "raw_document_text": "customer_approved_object_storage_only",
            "raw_ocr_text": "customer_approved_object_storage_only",
            "embedding_vectors": "customer_approved_vector_store_only",
            "retrieval_queries": "query_checksum_only"
        },
        "api_contract": {
            "base_url": api_base_url,
            "document_registry_path": "/api/v1/ops/evidence/documents",
            "chunk_registry_path": "/api/v1/ops/evidence/documents/{document_id}/chunks",
            "ocr_output_registry_path": "/api/v1/ops/evidence/documents/{document_id}/ocr-outputs",
            "embedding_job_registry_path": "/api/v1/ops/evidence/embedding-jobs",
            "retrieval_audit_path": "/api/v1/ops/evidence/retrieval-audit-events"
        },
        "artifact_contract": {
            "document_manifest_uri": format!("{evidence_root}/documents/{{window_start}}/document_manifest.ndjson"),
            "ocr_output_manifest_uri": format!("{evidence_root}/ocr/{{window_start}}/ocr_outputs.ndjson"),
            "chunk_manifest_uri": format!("{evidence_root}/chunks/{{window_start}}/chunks.ndjson"),
            "embedding_manifest_uri": format!("{evidence_root}/embeddings/{{window_start}}/embedding_jobs.ndjson"),
            "retrieval_eval_report_uri": format!("{evidence_root}/retrieval-eval/{{window_start}}/retrieval_eval_report.json")
        },
        "vector_store": {
            "kind": vector_store_kind,
            "ref": vector_store_ref,
            "write_policy": "customer_scope_partitioned_and_redacted"
        },
        "schedule": {
            "cron": cron,
            "concurrency_policy": "forbid",
            "idempotency_key": "customer_scope_id + source_record_ref + content_checksum + execution_window"
        },
        "jobs": [
            {
                "job_kind": "document_ingestion_metadata_sync",
                "input": "customer_approved_document_drop_or_document_management_export",
                "api_path": "/api/v1/ops/evidence/documents",
                "output_ref": "evidence_documents:<document_id>",
                "manifest_uri": format!("{evidence_root}/documents/{{window_start}}/document_manifest.ndjson")
            },
            {
                "job_kind": "ocr_output_registration",
                "input": "customer_ocr_output_uri_and_checksum",
                "api_path": "/api/v1/ops/evidence/documents/{document_id}/ocr-outputs",
                "output_ref": "evidence_ocr_outputs:<ocr_output_id>",
                "manifest_uri": format!("{evidence_root}/ocr/{{window_start}}/ocr_outputs.ndjson")
            },
            {
                "job_kind": "document_chunk_registration",
                "input": "redacted_ocr_output_or_redacted_document_text_uri",
                "api_path": "/api/v1/ops/evidence/documents/{document_id}/chunks",
                "output_ref": "evidence_chunks:<chunk_id>",
                "manifest_uri": format!("{evidence_root}/chunks/{{window_start}}/chunks.ndjson")
            },
            {
                "job_kind": "embedding_job_dispatch",
                "input": "redacted_document_chunk_refs",
                "api_path": "/api/v1/ops/evidence/embedding-jobs",
                "output_ref": "evidence_embedding_jobs:<embedding_job_id>",
                "manifest_uri": format!("{evidence_root}/embeddings/{{window_start}}/embedding_jobs.ndjson")
            },
            {
                "job_kind": "retrieval_ranking_evaluation",
                "input": "retrieval_audit_events_and_reviewer_outcomes",
                "api_path": "/api/v1/ops/evidence/retrieval-audit-events",
                "output_ref": "evidence_retrieval_eval_reports:<retrieval_eval_report_uri>",
                "report_uri": format!("{evidence_root}/retrieval-eval/{{window_start}}/retrieval_eval_report.json")
            }
        ],
        "downstream_contracts": {
            "analytics_export_plan": "build-analytics-export-plan",
            "observability_dashboard": "retrieval_audit_and_agent_workspace_artifacts"
        }
    }))
}

pub fn build_governance_ops_plan(
    object_storage_uri: &str,
    database_ref: &str,
    customer_scope_id: &str,
    retention_policy_id: &str,
    backup_restore_plan_id: &str,
    legal_hold_policy_id: &str,
    cron: &str,
) -> anyhow::Result<serde_json::Value> {
    let object_storage_uri =
        required_non_empty("object_storage_uri", object_storage_uri)?.trim_end_matches('/');
    let database_ref = required_non_empty("database_ref", database_ref)?;
    let customer_scope_id = required_non_empty("customer_scope_id", customer_scope_id)?;
    let retention_policy_id = required_non_empty("retention_policy_id", retention_policy_id)?;
    let backup_restore_plan_id =
        required_non_empty("backup_restore_plan_id", backup_restore_plan_id)?;
    let legal_hold_policy_id = required_non_empty("legal_hold_policy_id", legal_hold_policy_id)?;
    let cron = required_non_empty("cron", cron)?;
    let governance_root = format!("{object_storage_uri}/governance-ops/{customer_scope_id}");

    Ok(serde_json::json!({
        "plan_kind": "scheduled_governance_ops",
        "plan_version": 1,
        "customer_scope_id": customer_scope_id,
        "policies": {
            "retention_policy_id": retention_policy_id,
            "backup_restore_plan_id": backup_restore_plan_id,
            "legal_hold_policy_id": legal_hold_policy_id
        },
        "runtime_boundary": {
            "raw_payloads": "customer_approved_object_storage_only",
            "destructive_actions": "approval_required_plan_only",
            "customer_data": "customer_scope_partitioned"
        },
        "schedule": {
            "cron": cron,
            "concurrency_policy": "forbid",
            "idempotency_key": "customer_scope_id + policy_ids + execution_window"
        },
        "artifact_contract": {
            "backup_manifest_uri": format!("{governance_root}/backup/{{window_start}}/backup_manifest.json"),
            "restore_drill_report_uri": format!("{governance_root}/restore-drills/{{window_start}}/restore_drill_report.json"),
            "retention_scan_report_uri": format!("{governance_root}/retention/{{window_start}}/retention_scan_report.json"),
            "legal_hold_report_uri": format!("{governance_root}/legal-hold/{{window_start}}/legal_hold_report.json"),
            "destruction_review_report_uri": format!("{governance_root}/destruction-review/{{window_start}}/destruction_review_report.json")
        },
        "jobs": [
            {
                "job_kind": "backup_snapshot_manifest",
                "input": database_ref,
                "output_ref": "backup_manifests:<backup_manifest_uri>",
                "backup_uri": format!("{governance_root}/backup/{{window_start}}/postgres.dump"),
                "manifest_uri": format!("{governance_root}/backup/{{window_start}}/backup_manifest.json")
            },
            {
                "job_kind": "restore_drill_validation",
                "input": "latest_backup_manifest",
                "output_ref": "restore_drill_reports:<restore_drill_report_uri>",
                "restore_target": "staging-restore-validation",
                "report_uri": format!("{governance_root}/restore-drills/{{window_start}}/restore_drill_report.json")
            },
            {
                "job_kind": "retention_policy_scan",
                "input": ["audit_events", "api_call_records", "evidence_documents", "agent_workspace_artifacts"],
                "output_ref": "retention_scan_reports:<retention_scan_report_uri>",
                "report_uri": format!("{governance_root}/retention/{{window_start}}/retention_scan_report.json")
            },
            {
                "job_kind": "legal_hold_reconciliation",
                "input": ["investigation_cases", "audit_samples", "evidence_documents"],
                "output_ref": "legal_hold_reports:<legal_hold_report_uri>",
                "report_uri": format!("{governance_root}/legal-hold/{{window_start}}/legal_hold_report.json")
            },
            {
                "job_kind": "destruction_candidate_review",
                "input": "expired_retention_items_without_legal_hold",
                "output_ref": "destruction_review_reports:<destruction_review_report_uri>",
                "approval_gate": "human_approval_required_before_destroy",
                "report_uri": format!("{governance_root}/destruction-review/{{window_start}}/destruction_review_report.json")
            }
        ],
        "downstream_contracts": {
            "pilot_readiness": "check-pilot-readiness",
            "observability_alert": "database.backup.failed | retention.scan.failed | legal_hold.reconciliation.failed"
        }
    }))
}

fn required_non_empty<'a>(field: &str, value: &'a str) -> anyhow::Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        bail!("{field} is required");
    }
    Ok(value)
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

fn build_training_retraining_output(
    job: &ClaimedRetrainingJob,
    actor: &str,
    artifact_base_uri: &str,
    training_manifest: &str,
    trainer_python: &str,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    let training_command = build_training_command(
        trainer_python,
        training_manifest,
        artifact_base_uri,
        job,
        actor,
    );
    let output = Command::new(&training_command.program)
        .args(&training_command.args)
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

async fn enrich_retraining_output_with_model_artifact_evaluation(
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

fn build_mock_retraining_output(
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
    let evaluation_run_id = format!(
        "eval_{}_{}",
        safe_id_segment(&job.model_key),
        safe_id_segment(&candidate_model_version)
    );
    let evidence_refs = vec![
        format!("model_retraining_jobs:{}", job.job_id),
        format!("model_artifacts:{artifact_uri}"),
        format!("model_validation_reports:{validation_report_uri}"),
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
        metrics_json: serde_json::json!({
            "out_of_time_auc": 0.82,
            "score_psi": 0.04,
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
            "review_capacity_threshold_status": "passed"
        }),
        evidence_refs,
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

fn resolve_parquet_files(base_dir: &Path, data_uri: &str) -> anyhow::Result<Vec<PathBuf>> {
    let path = PathBuf::from(data_uri);
    let path = if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    };

    if path.is_file() {
        ensure_parquet_path(&path)?;
        return Ok(vec![path]);
    }

    if path.is_dir() {
        let mut files = fs::read_dir(&path)
            .with_context(|| format!("read parquet directory {}", path.display()))?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_file())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "parquet")
            })
            .collect::<Vec<_>>();
        files.sort();
        return Ok(files);
    }

    Err(anyhow!(
        "parquet data_uri does not exist: {}",
        path.display()
    ))
}

fn ensure_parquet_path(path: &Path) -> anyhow::Result<()> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("parquet") => Ok(()),
        _ => bail!("data_uri file must end with .parquet: {}", path.display()),
    }
}

fn ensure_required_columns(
    fields: &BTreeMap<String, FieldAccumulator>,
    manifest: &ParquetDatasetManifest,
) -> anyhow::Result<()> {
    if !fields.contains_key(&manifest.label_column) {
        bail!(
            "label_column {} not found in parquet schema",
            manifest.label_column
        );
    }
    for key in &manifest.entity_keys {
        let Some(field) = fields.get(key) else {
            bail!("entity_key {key} not found in parquet schema");
        };
        if field.logical_type != "Utf8" && field.logical_type != "LargeUtf8" {
            bail!("entity_key {key} must be a string parquet field");
        }
    }
    Ok(())
}

fn column_value_at(array: &dyn arrow_array::Array, index: usize) -> Option<String> {
    use arrow_array::{
        BooleanArray, Float64Array, Int16Array, Int32Array, Int64Array, Int8Array,
        LargeStringArray, StringArray, UInt16Array, UInt32Array, UInt64Array, UInt8Array,
    };

    if array.is_null(index) {
        return None;
    }
    if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<LargeStringArray>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<BooleanArray>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int8Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int16Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int32Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt8Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt16Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt32Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt64Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        return Some(values.value(index).to_string());
    }
    None
}

fn column_values(array: &dyn arrow_array::Array) -> Vec<String> {
    use arrow_array::{
        Array, BooleanArray, Float64Array, Int16Array, Int32Array, Int64Array, Int8Array,
        LargeStringArray, StringArray, UInt16Array, UInt32Array, UInt64Array, UInt8Array,
    };

    if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        return (0..values.len())
            .filter(|index| !values.is_null(*index))
            .map(|index| values.value(index).to_string())
            .collect();
    }
    if let Some(values) = array.as_any().downcast_ref::<LargeStringArray>() {
        return (0..values.len())
            .filter(|index| !values.is_null(*index))
            .map(|index| values.value(index).to_string())
            .collect();
    }
    if let Some(values) = array.as_any().downcast_ref::<BooleanArray>() {
        return (0..values.len())
            .filter(|index| !values.is_null(*index))
            .map(|index| values.value(index).to_string())
            .collect();
    }
    if let Some(values) = array.as_any().downcast_ref::<Int8Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int16Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int32Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt8Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt16Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt32Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt64Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        return primitive_values(values, |value| value.to_string());
    }

    Vec::new()
}

fn primitive_values<T, F>(array: &arrow_array::PrimitiveArray<T>, format: F) -> Vec<String>
where
    T: arrow_array::ArrowPrimitiveType,
    F: Fn(T::Native) -> String,
{
    use arrow_array::Array;

    (0..array.len())
        .filter(|index| !array.is_null(*index))
        .map(|index| format(array.value(index)))
        .collect()
}

fn top_values(counts: &BTreeMap<String, u64>) -> Vec<ValueCount> {
    let mut values = counts
        .iter()
        .map(|(value, count)| ValueCount {
            value: value.clone(),
            count: *count,
        })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.value.cmp(&right.value))
    });
    values.truncate(10);
    values
}

fn schema_hash(fields: &[FieldSchemaOutput]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for field in fields {
        for byte in format!(
            "{}:{}:{}:{};",
            field.field_name, field.logical_type, field.nullable, field.semantic_role
        )
        .as_bytes()
        {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    format!("fnv64:{hash:016x}")
}

fn is_numeric_logical_type(logical_type: &str) -> bool {
    matches!(
        logical_type,
        "Float64"
            | "Float32"
            | "Int8"
            | "Int16"
            | "Int32"
            | "Int64"
            | "UInt8"
            | "UInt16"
            | "UInt32"
            | "UInt64"
    )
}

fn feature_reproducibility_hash(feature_set: &FeatureSetManifest) -> anyhow::Result<String> {
    let mut hash_input = feature_set.clone();
    hash_input.feature_reproducibility_hash.clear();
    let bytes = serde_json::to_vec(&hash_input).context("serialize feature set for hash")?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{digest:x}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn builds_worker_api_url_without_double_slashes() {
        assert_eq!(
            api_url(
                "http://127.0.0.1:8080/",
                "/api/v1/ops/model-retraining-jobs/claim-next"
            ),
            "http://127.0.0.1:8080/api/v1/ops/model-retraining-jobs/claim-next"
        );
        assert_eq!(
            api_url(
                "http://127.0.0.1:8080/",
                &retraining_job_status_path("model_retraining_job_1")
            ),
            "http://127.0.0.1:8080/api/v1/ops/model-retraining-jobs/model_retraining_job_1/status"
        );
        assert_eq!(
            api_url(
                "http://127.0.0.1:8080/",
                &retraining_job_output_path("model_retraining_job_1")
            ),
            "http://127.0.0.1:8080/api/v1/ops/model-retraining-jobs/model_retraining_job_1/output"
        );
    }

    #[test]
    fn returns_worker_health_metadata() {
        let health = worker_health();

        assert_eq!(health.status, "ok");
        assert_eq!(health.service, "worker");
        assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
        assert!(health.checks.contains(&WorkerHealthCheck {
            name: "cli_commands",
            status: "ok"
        }));
        assert!(health.checks.contains(&WorkerHealthCheck {
            name: "parquet_profiler",
            status: "ok"
        }));
        assert!(health.checks.contains(&WorkerHealthCheck {
            name: "feature_set_builder",
            status: "ok"
        }));
        assert!(health.checks.contains(&WorkerHealthCheck {
            name: "demo_ml_dataset_builder",
            status: "ok"
        }));
        assert!(health.checks.contains(&WorkerHealthCheck {
            name: "automl_candidate_ranker",
            status: "ok"
        }));
        assert!(health.checks.contains(&WorkerHealthCheck {
            name: "rule_candidate_miner",
            status: "ok"
        }));
        assert!(health.checks.contains(&WorkerHealthCheck {
            name: "rule_candidate_backtester",
            status: "ok"
        }));
        assert!(health.checks.contains(&WorkerHealthCheck {
            name: "provider_peer_clusterer",
            status: "ok"
        }));
        assert!(health.checks.contains(&WorkerHealthCheck {
            name: "retraining_job_runner",
            status: "ok"
        }));
        assert!(health.checks.contains(&WorkerHealthCheck {
            name: "pilot_readiness_checker",
            status: "ok"
        }));
    }

    #[test]
    fn builds_pilot_readiness_report_from_api_health() {
        let report = build_pilot_readiness_report(ApiHealthResponse {
            status: "ok".into(),
            service: "api-server".into(),
            version: "0.1.0".into(),
            checks: vec![ApiHealthCheck {
                name: "model_scorer".into(),
                status: "ok".into(),
                runtime_kind: Some("rust_artifact".into()),
                remediation: None,
            }],
            pilot_readiness: ApiPilotReadiness {
                status: "not_ready".into(),
                required_check_names: vec![
                    "api_key_configuration".into(),
                    "object_storage_configuration".into(),
                ],
                required_check_count: 2,
                ready_check_count: 1,
                blocking_check_count: 1,
                ready_checks: vec![ApiHealthCheck {
                    name: "api_key_configuration".into(),
                    status: "configured".into(),
                    runtime_kind: None,
                    remediation: None,
                }],
                blocking_checks: vec![ApiHealthCheck {
                    name: "object_storage_configuration".into(),
                    status: "local_demo_object_storage".into(),
                    runtime_kind: None,
                    remediation: Some("Set FWA_OBJECT_STORAGE_URI.".into()),
                }],
            },
        });

        assert_eq!(report.status, "not_ready");
        assert!(!report.ready_for_customer_pilot);
        assert_eq!(report.api_service, "api-server");
        assert_eq!(report.required_check_count, 2);
        assert_eq!(report.ready_check_count, 1);
        assert_eq!(report.blocking_check_count, 1);
        assert_eq!(report.model_runtime_kind.as_deref(), Some("rust_artifact"));
        assert_eq!(
            report.remediation_summary,
            vec!["Set FWA_OBJECT_STORAGE_URI."]
        );
        assert!(report
            .evidence_refs
            .contains(&"api_health:/api/v1/health".to_string()));
    }

    #[test]
    fn marks_pilot_readiness_report_ready_only_without_blockers() {
        let report = build_pilot_readiness_report(ApiHealthResponse {
            status: "ok".into(),
            service: "api-server".into(),
            version: "0.1.0".into(),
            checks: vec![ApiHealthCheck {
                name: "model_scorer".into(),
                status: "ok".into(),
                runtime_kind: Some("python_http".into()),
                remediation: None,
            }],
            pilot_readiness: ApiPilotReadiness {
                status: "ready".into(),
                required_check_names: vec!["api_key_configuration".into()],
                required_check_count: 1,
                ready_check_count: 1,
                blocking_check_count: 0,
                ready_checks: vec![ApiHealthCheck {
                    name: "api_key_configuration".into(),
                    status: "configured".into(),
                    runtime_kind: None,
                    remediation: None,
                }],
                blocking_checks: Vec::new(),
            },
        });

        assert!(report.ready_for_customer_pilot);
        assert_eq!(report.blocking_check_count, 0);
        assert!(report.remediation_summary.is_empty());
    }

    #[test]
    fn builds_deterministic_mock_retraining_output() {
        let job = ClaimedRetrainingJob {
            job_id: "model retraining/job#1".into(),
            model_key: "baseline/fwa".into(),
            model_version: "0.1.0".into(),
            status: "validation".into(),
            updated_by: "trainer-worker".into(),
            status_note: "Validation metrics are ready.".into(),
        };

        let output = build_mock_retraining_output(&job, "trainer-worker", "s3://fwa-models/")
            .expect("mock retraining output");

        assert_eq!(output.actor, "trainer-worker");
        assert_eq!(
            output.candidate_model_version,
            "0.1.0-candidate-model_retraining_job_1"
        );
        assert_eq!(
            output.artifact_uri,
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/model.onnx"
        );
        assert_eq!(
            output.validation_report_uri,
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/validation.json"
        );
        assert_eq!(
            output.evaluation_run_id,
            "eval_baseline_fwa_0_1_0_candidate_model_retraining_job_1"
        );
        assert_eq!(output.auc.as_deref(), Some("0.86"));
        assert_eq!(output.endpoint_url, None);
        assert_eq!(output.confusion_matrix_json["tp"], 24);
        assert_eq!(output.metrics_json["shadow_comparison_status"], "passed");
        assert_eq!(output.metrics_json["leakage_check_status"], "passed");
        assert_eq!(output.metrics_json["time_group_split_status"], "passed");
        assert_eq!(output.metrics_json["time_split_field"], "service_date");
        assert_eq!(
            output.metrics_json["group_split_fields"],
            serde_json::json!(["member_id", "policy_id", "provider_id"])
        );
        assert_eq!(output.metrics_json["label_provenance_status"], "passed");
        assert_eq!(output.metrics_json["pilot_validation_status"], "passed");
        assert_eq!(output.metrics_json["serving_version_lock_status"], "passed");
        assert_eq!(output.metrics_json["artifact_integrity_status"], "passed");
        assert_eq!(
            output.metrics_json["feature_store_materialization_status"],
            "passed"
        );
        assert_eq!(output.metrics_json["segment_fairness_status"], "passed");
        assert_eq!(
            output.evidence_refs,
            vec![
                "model_retraining_jobs:model retraining/job#1",
                "model_artifacts:s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/model.onnx",
                "model_validation_reports:s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/validation.json",
                "model_evaluations:eval_baseline_fwa_0_1_0_candidate_model_retraining_job_1"
            ]
        );
    }

    #[test]
    fn rejects_empty_artifact_base_uri_for_mock_retraining_output() {
        let job = ClaimedRetrainingJob {
            job_id: "model_retraining_job_1".into(),
            model_key: "baseline_fwa".into(),
            model_version: "0.1.0".into(),
            status: "validation".into(),
            updated_by: "trainer-worker".into(),
            status_note: "Validation metrics are ready.".into(),
        };

        let error = build_mock_retraining_output(&job, "trainer-worker", " ").unwrap_err();

        assert!(error.to_string().contains("artifact_base_uri"));
    }

    #[test]
    fn builds_training_command_for_retraining_job() {
        let job = ClaimedRetrainingJob {
            job_id: "model_retraining_job_1".into(),
            model_key: "baseline_fwa".into(),
            model_version: "0.1.0".into(),
            status: "validation".into(),
            updated_by: "trainer-worker".into(),
            status_note: "Validation metrics are ready.".into(),
        };

        let command = build_training_command(
            "python3",
            "data/training/manifest.json",
            "artifacts/models",
            &job,
            "trainer-worker",
        );

        assert_eq!(command.program, "python3");
        assert_eq!(
            command.args,
            vec![
                "-m",
                "app.train",
                "--manifest",
                "data/training/manifest.json",
                "--artifact-base-uri",
                "artifacts/models",
                "--model-key",
                "baseline_fwa",
                "--base-model-version",
                "0.1.0",
                "--job-id",
                "model_retraining_job_1",
                "--actor",
                "trainer-worker",
            ]
        );
    }

    #[test]
    fn builds_external_training_handoff_from_manifest() {
        let root = temp_root("training-handoff");
        let manifest_path = root.join("manifest.json");
        fs::write(
            &manifest_path,
            serde_json::json!({
                "dataset_key": "claims_model",
                "dataset_version": "2026-06-02",
                "business_domain": "health_fwa",
                "sample_grain": "claim",
                "label_column": "confirmed_fwa",
                "entity_keys": ["claim_id", "member_id", "policy_id", "provider_id"],
                "time_split_field": "service_date",
                "group_split_fields": ["member_id", "policy_id", "provider_id"],
                "splits": [
                    {"split_name": "train", "data_uri": "train.parquet"},
                    {"split_name": "validation", "data_uri": "validation.parquet"},
                    {"split_name": "out_of_time", "data_uri": "out_of_time.parquet"}
                ]
            })
            .to_string(),
        )
        .unwrap();

        let handoff = build_training_handoff(
            &manifest_path,
            "s3://fwa-models",
            "baseline_fwa",
            "0.1.0",
            "model_retraining_job_1",
            "trainer-worker",
        )
        .expect("training handoff");

        assert_eq!(handoff["handoff_kind"], "external_training_platform");
        assert_eq!(handoff["handoff_version"], 2);
        assert_eq!(handoff["dataset"]["dataset_key"], "claims_model");
        assert_eq!(handoff["dataset"]["dataset_version"], "2026-06-02");
        assert_eq!(
            handoff["dataset"]["manifest_uri"],
            serde_json::json!(manifest_path.to_string_lossy())
        );
        assert_eq!(handoff["training_job"]["model_key"], "baseline_fwa");
        assert_eq!(handoff["training_job"]["algorithm"], "logistic_regression");
        assert_eq!(
            handoff["training_job"]["runtime_kind"],
            "rust_logistic_regression"
        );
        assert_eq!(
            handoff["training_job"]["candidate_model_version"],
            "0.1.0-candidate-model_retraining_job_1"
        );
        assert_eq!(
            handoff["artifact_contract"]["serving_artifact_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/rust_serving_artifact.json"
        );
        assert_eq!(
            handoff["artifact_contract"]["serving_artifact_format"],
            "rust_json"
        );
        assert_eq!(
            handoff["artifact_contract"]["rust_serving_artifact_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/rust_serving_artifact.json"
        );
        assert_eq!(
            handoff["artifact_contract"]["rust_feature_set_manifest_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/rust_feature_set/feature_set_manifest.json"
        );
        assert_eq!(
            handoff["feature_set_contract"]["builder"],
            "worker build-feature-set"
        );
        assert_eq!(
            handoff["output_contract"]["submit_path"],
            "/api/v1/ops/model-retraining-jobs/model_retraining_job_1/output"
        );
        assert_eq!(
            handoff["output_contract"]["artifact_uri"],
            "artifact_contract.serving_artifact_uri"
        );
        assert_eq!(
            handoff["data_contract"]["source"],
            "same_parquet_dataset_manifest"
        );
        assert!(handoff["output_contract"]["required_evidence_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reference| reference
                .as_str()
                .unwrap()
                .contains("feature_set_manifests")));
    }

    #[test]
    fn builds_xgboost_training_handoff_with_onnx_contract() {
        let root = temp_root("xgboost-training-handoff");
        let pack =
            build_demo_ml_datasets(&root, "2026-06-xgboost-handoff").expect("demo ML datasets");

        let handoff = build_training_handoff_with_algorithm(
            &pack.labeled_manifest_uri,
            "s3://fwa-models",
            "baseline_fwa",
            "0.1.0",
            "model_retraining_job_1",
            "trainer-worker",
            "xgboost",
        )
        .expect("xgboost handoff");

        assert_eq!(handoff["handoff_version"], 2);
        assert_eq!(handoff["training_job"]["algorithm"], "xgboost");
        assert_eq!(handoff["training_job"]["runtime_kind"], "xgboost_onnx");
        assert_eq!(
            handoff["training_job"]["candidate_model_version"],
            "0.1.0-xgboost-candidate-model_retraining_job_1"
        );
        assert_eq!(
            handoff["artifact_contract"]["serving_artifact_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-xgboost-candidate-model_retraining_job_1/model.onnx"
        );
        assert_eq!(
            handoff["artifact_contract"]["serving_artifact_format"],
            "onnx"
        );
        assert_eq!(
            handoff["artifact_contract"]["onnx_artifact_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-xgboost-candidate-model_retraining_job_1/model.onnx"
        );
        assert_eq!(
            handoff["artifact_contract"]["onnx_parity_report_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-xgboost-candidate-model_retraining_job_1/onnx_parity_report.json"
        );
        assert_eq!(
            handoff["output_contract"]["onnx_parity_report_uri"],
            "artifact_contract.onnx_parity_report_uri"
        );
        assert!(handoff["output_contract"]["required_evidence_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reference| reference
                .as_str()
                .unwrap()
                .contains("model_onnx_parity_reports")));
    }

    #[test]
    fn builds_rust_demo_ml_datasets_with_labeled_and_unlabeled_manifests() {
        let root = temp_root("demo-ml-datasets");
        let pack = build_demo_ml_datasets(&root, "2026-06-rust-demo").expect("demo ML datasets");

        assert_eq!(pack.pack_kind, "rust_automl_demo_datasets");
        assert_eq!(pack.dataset_version, "2026-06-rust-demo");
        assert_eq!(pack.dataset_manifests.len(), 3);
        assert_eq!(pack.unlabeled_manifest_uris.len(), 2);
        assert!(root.join("index.json").is_file());

        let labeled_manifest_path = root.join("labeled_claim_risk/manifest.json");
        let scoring_manifest_path = root.join("unlabeled_shadow_scoring/manifest.json");
        let provider_manifest_path = root.join("unlabeled_provider_peer_clustering/manifest.json");
        assert!(labeled_manifest_path.is_file());
        assert!(scoring_manifest_path.is_file());
        assert!(provider_manifest_path.is_file());
        assert!(root
            .join("labeled_claim_risk/split=train/part-00000.parquet")
            .is_file());
        assert!(root
            .join("labeled_claim_risk/split=validation/part-00000.parquet")
            .is_file());
        assert!(root
            .join("labeled_claim_risk/split=out_of_time/part-00000.parquet")
            .is_file());
        assert!(root
            .join("unlabeled_shadow_scoring/split=scoring/part-00000.parquet")
            .is_file());
        assert!(root
            .join("unlabeled_provider_peer_clustering/split=analysis/part-00000.parquet")
            .is_file());

        let labeled_manifest = serde_json::from_str::<serde_json::Value>(
            &fs::read_to_string(&labeled_manifest_path).unwrap(),
        )
        .unwrap();
        assert_eq!(labeled_manifest["label_column"], "confirmed_fwa");
        assert_eq!(
            labeled_manifest["label_policy"],
            "weak_rust_demo_label_not_production_evidence"
        );
        assert_eq!(labeled_manifest["splits"].as_array().unwrap().len(), 3);

        let scoring_manifest = serde_json::from_str::<serde_json::Value>(
            &fs::read_to_string(&scoring_manifest_path).unwrap(),
        )
        .unwrap();
        assert!(scoring_manifest.get("label_column").is_none());
        assert_eq!(
            scoring_manifest["label_policy"],
            "unlabeled_shadow_scoring_only"
        );

        let provider_manifest = serde_json::from_str::<serde_json::Value>(
            &fs::read_to_string(&provider_manifest_path).unwrap(),
        )
        .unwrap();
        assert!(provider_manifest.get("label_column").is_none());
        assert_eq!(
            provider_manifest["label_policy"],
            "unlabeled_clustering_discovery_only"
        );

        let profile_dir = root.join("profile");
        let profile = profile_manifest_file(&labeled_manifest_path, &profile_dir).unwrap();
        assert_eq!(profile.profile.row_count_by_split["train"], 8);
        assert_eq!(profile.profile.row_count_by_split["validation"], 4);
        assert_eq!(profile.profile.row_count_by_split["out_of_time"], 4);
        assert_eq!(profile.profile.label_distribution_by_split["train"]["1"], 4);
        assert_eq!(profile.profile.label_distribution_by_split["train"]["0"], 4);
        assert!(profile_dir.join("schema.json").is_file());
        assert!(profile_dir.join("profile.json").is_file());
        assert!(profile_dir.join("catalog.json").is_file());
        assert!(pack
            .next_worker_commands
            .iter()
            .any(|command| command.contains("build-feature-set")));
        assert!(pack
            .next_worker_commands
            .iter()
            .any(|command| command.contains("cluster-provider-peers")));
        assert!(pack
            .next_worker_commands
            .iter()
            .any(|command| command.contains("cluster-provider-graph")));
        assert!(pack
            .next_worker_commands
            .iter()
            .any(|command| command.contains("cluster-claim-entities")));
    }

    #[test]
    fn builds_feature_set_manifest_from_labeled_parquet_manifest() {
        let root = temp_root("feature-set");
        let pack = build_demo_ml_datasets(&root, "2026-06-feature-set").expect("demo ML datasets");
        let output_dir = root.join("feature-set-output");

        let feature_set = build_feature_set(
            &pack.labeled_manifest_uri,
            &output_dir,
            Some("claims-risk-demo-features-v1"),
        )
        .expect("feature set");
        let repeated = build_feature_set(
            &pack.labeled_manifest_uri,
            root.join("feature-set-output-repeat"),
            Some("claims-risk-demo-features-v1"),
        )
        .expect("repeat feature set");

        assert_eq!(feature_set.manifest_kind, "rust_feature_set_manifest");
        assert_eq!(feature_set.feature_set_id, "claims-risk-demo-features-v1");
        assert_eq!(feature_set.dataset_key, "rust_demo_claim_risk_labeled");
        assert_eq!(feature_set.label_column, "confirmed_fwa");
        assert_eq!(
            feature_set.entity_keys,
            vec![
                "claim_id".to_string(),
                "member_id".to_string(),
                "policy_id".to_string(),
                "provider_id".to_string()
            ]
        );
        let feature_names = feature_set
            .feature_columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>();
        assert!(feature_names.contains(&"claim_amount"));
        assert!(feature_names.contains(&"amount_to_limit_ratio"));
        assert!(!feature_names.contains(&"confirmed_fwa"));
        assert!(!feature_names.contains(&"claim_id"));
        assert_eq!(feature_set.split_summaries.len(), 3);
        assert_eq!(feature_set.split_summaries[0].row_count, 8);
        assert!(feature_set
            .feature_reproducibility_hash
            .starts_with("sha256:"));
        assert_eq!(
            feature_set.feature_reproducibility_hash,
            repeated.feature_reproducibility_hash
        );
        assert!(feature_set
            .governance_boundary
            .contains("does not approve labels"));
        assert!(output_dir.join("feature_set_manifest.json").is_file());
        assert!(output_dir.join("feature_columns.json").is_file());
        assert!(output_dir.join("feature_split_summary.json").is_file());
    }

    #[test]
    fn enriches_training_output_with_rust_feature_set_evidence() {
        let root = temp_root("training-output-feature-set");
        let pack = build_demo_ml_datasets(&root, "2026-06-training-feature-set")
            .expect("demo ML datasets");
        let artifact_dir = root.join("artifacts/baseline_fwa/0.1.0-candidate-job");
        fs::create_dir_all(&artifact_dir).unwrap();
        let output = CompleteRetrainingJobPayload {
            actor: "trainer-worker".into(),
            notes: "training output".into(),
            candidate_model_version: "0.1.0-candidate-job".into(),
            artifact_uri: artifact_dir
                .join("model.onnx")
                .to_string_lossy()
                .into_owned(),
            artifact_sha256: Some("sha256:serving".into()),
            training_artifact_uri: Some(
                artifact_dir
                    .join("model.joblib")
                    .to_string_lossy()
                    .into_owned(),
            ),
            training_artifact_sha256: Some("sha256:training".into()),
            serving_manifest_uri: None,
            onnx_parity_report_uri: None,
            endpoint_url: None,
            validation_report_uri: artifact_dir
                .join("validation.json")
                .to_string_lossy()
                .into_owned(),
            evaluation_run_id: "eval_baseline_fwa_candidate".into(),
            auc: Some("0.82".into()),
            ks: None,
            precision: Some("0.70".into()),
            recall: Some("0.68".into()),
            f1: None,
            accuracy: None,
            threshold: Some("0.50".into()),
            confusion_matrix_json: serde_json::json!({}),
            feature_importance_uri: Some(
                artifact_dir
                    .join("feature_importance.parquet")
                    .to_string_lossy()
                    .into_owned(),
            ),
            metrics_json: serde_json::json!({
                "feature_reproducibility_hash": "sha256:trainer-hash",
                "feature_store_materialization_status": "passed"
            }),
            evidence_refs: vec![
                format!(
                    "model_artifacts:{}",
                    artifact_dir.join("model.onnx").display()
                ),
                format!(
                    "model_validation_reports:{}",
                    artifact_dir.join("validation.json").display()
                ),
                "model_evaluations:eval_baseline_fwa_candidate".into(),
            ],
        };

        let output =
            enrich_retraining_output_with_rust_feature_set(output, &pack.labeled_manifest_uri)
                .expect("enriched training output");

        assert_eq!(output.artifact_sha256.as_deref(), Some("sha256:serving"));
        assert_eq!(
            output.training_artifact_sha256.as_deref(),
            Some("sha256:training")
        );
        assert_eq!(
            output.metrics_json["trainer_feature_reproducibility_hash"],
            "sha256:trainer-hash"
        );
        assert!(output.metrics_json["feature_reproducibility_hash"]
            .as_str()
            .unwrap()
            .starts_with("sha256:"));
        assert_ne!(
            output.metrics_json["feature_reproducibility_hash"],
            "sha256:trainer-hash"
        );
        assert_eq!(output.metrics_json["rust_feature_set_status"], "passed");
        let feature_set_manifest_uri = output.metrics_json["rust_feature_set_manifest_uri"]
            .as_str()
            .expect("rust feature set manifest uri");
        assert!(Path::new(feature_set_manifest_uri).is_file());
        assert!(output
            .evidence_refs
            .iter()
            .any(|reference| reference
                == &format!("feature_set_manifests:{feature_set_manifest_uri}")));
    }

    #[tokio::test]
    async fn enriches_training_output_with_rust_serving_evaluation_evidence() {
        let root = temp_root("training-output-artifact-evaluation");
        let pack = build_demo_ml_datasets(&root, "2026-06-training-artifact-eval")
            .expect("demo ML datasets");
        let artifact_dir = root.join("artifacts/baseline_fwa/0.2.0-candidate-job");
        fs::create_dir_all(&artifact_dir).unwrap();
        let artifact_path = artifact_dir.join("rust_serving_artifact.json");
        write_json(
            artifact_path.clone(),
            &serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.2.0-candidate-job",
                "runtime_kind": "rust_logistic_regression",
                "execution_provider": "cpu",
                "threshold": 0.5,
                "feature_columns": ["amount_to_limit_ratio", "peer_percentile"],
                "intercept": -2.0,
                "coefficients": {
                    "amount_to_limit_ratio": 2.4,
                    "peer_percentile": 1.1
                }
            }),
        )
        .unwrap();
        let artifact_sha256 = test_sha256(&artifact_path);
        let serving_manifest_path = artifact_dir.join("serving_manifest.json");
        write_json(
            serving_manifest_path.clone(),
            &serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.2.0-candidate-job",
                "runtime_kind": "rust_logistic_regression",
                "artifact_uri": artifact_path.to_string_lossy(),
                "artifact_sha256": artifact_sha256,
                "version_lock": "0.2.0-candidate-job",
                "feature_columns": ["amount_to_limit_ratio", "peer_percentile"],
                "threshold": 0.5
            }),
        )
        .unwrap();
        let output = CompleteRetrainingJobPayload {
            actor: "trainer-worker".into(),
            notes: "training output".into(),
            candidate_model_version: "0.2.0-candidate-job".into(),
            artifact_uri: artifact_path.to_string_lossy().into_owned(),
            artifact_sha256: Some(test_sha256(&artifact_path)),
            training_artifact_uri: Some(
                artifact_dir
                    .join("model.joblib")
                    .to_string_lossy()
                    .into_owned(),
            ),
            training_artifact_sha256: Some("sha256:training".into()),
            serving_manifest_uri: Some(serving_manifest_path.to_string_lossy().into_owned()),
            onnx_parity_report_uri: None,
            endpoint_url: None,
            validation_report_uri: artifact_dir
                .join("validation.json")
                .to_string_lossy()
                .into_owned(),
            evaluation_run_id: "eval_baseline_fwa_candidate".into(),
            auc: Some("0.84".into()),
            ks: None,
            precision: Some("0.72".into()),
            recall: Some("0.69".into()),
            f1: None,
            accuracy: None,
            threshold: Some("0.50".into()),
            confusion_matrix_json: serde_json::json!({}),
            feature_importance_uri: None,
            metrics_json: serde_json::json!({
                "feature_reproducibility_hash": "sha256:rust-feature-hash",
                "rust_feature_set_status": "passed",
                "rust_feature_set_manifest_uri": artifact_dir
                    .join("rust_feature_set/feature_set_manifest.json")
                    .to_string_lossy(),
                "feature_store_materialization_status": "passed"
            }),
            evidence_refs: vec![format!("model_artifacts:{}", artifact_path.display())],
        };

        let output = enrich_retraining_output_with_model_artifact_evaluation(
            output,
            &pack.labeled_manifest_uri,
        )
        .await
        .expect("enriched training output");

        assert_eq!(
            output.metrics_json["model_artifact_evaluation_status"],
            "passed"
        );
        assert_eq!(output.metrics_json["rust_serving_status"], "passed");
        assert_eq!(output.metrics_json["rust_serving_latency_status"], "passed");
        let report_uri = output.metrics_json["model_artifact_evaluation_report_uri"]
            .as_str()
            .expect("artifact evaluation report uri");
        assert!(Path::new(report_uri).is_file());
        assert!(output
            .evidence_refs
            .iter()
            .any(|reference| reference == &format!("model_artifact_evaluations:{report_uri}")));
    }

    #[test]
    fn clusters_unlabeled_provider_peers_without_label_assignment() {
        let root = temp_root("provider-peer-clustering");
        let pack =
            build_demo_ml_datasets(&root, "2026-06-clustering-demo").expect("demo ML datasets");
        let provider_manifest = pack
            .unlabeled_manifest_uris
            .iter()
            .find(|uri| uri.contains("unlabeled_provider_peer_clustering"))
            .expect("provider peer manifest");
        let output_dir = root.join("clusters");

        let report =
            cluster_provider_peers(provider_manifest, &output_dir).expect("provider clustering");

        assert_eq!(report.report_kind, "provider_peer_clustering");
        assert_eq!(report.dataset_key, "rust_demo_provider_peer_unlabeled");
        assert_eq!(report.algorithm, "rust_standardized_kmeans_v1");
        assert_eq!(report.label_policy, "unlabeled_clustering_discovery_only");
        assert!(report
            .governance_boundary
            .contains("must not create confirmed FWA labels"));
        assert_eq!(report.cluster_count, 3);
        assert_eq!(report.provider_assignments.len(), 6);
        assert!(!report.anomaly_candidates.is_empty());
        assert_eq!(report.review_tasks.len(), report.anomaly_candidates.len());
        assert_eq!(
            report.review_tasks[0].required_review,
            "human_review_required_before_case_creation_or_label_assignment"
        );
        assert!(output_dir
            .join("provider_peer_clustering_report.json")
            .is_file());
        assert!(output_dir
            .join("provider_anomaly_review_tasks.json")
            .is_file());
    }

    #[test]
    fn clusters_provider_graph_communities_without_label_assignment() {
        let root = temp_root("provider-graph-clustering");
        let pack =
            build_demo_ml_datasets(&root, "2026-06-provider-graph-demo").expect("demo ML datasets");
        let provider_manifest = pack
            .unlabeled_manifest_uris
            .iter()
            .find(|uri| uri.contains("unlabeled_provider_peer_clustering"))
            .expect("provider peer manifest");
        let output_dir = root.join("graph-communities");

        let report = cluster_provider_graph_communities(provider_manifest, &output_dir)
            .expect("provider graph clustering");

        assert_eq!(report.report_kind, "provider_graph_community_clustering");
        assert_eq!(report.dataset_key, "rust_demo_provider_peer_unlabeled");
        assert_eq!(report.algorithm, "rust_provider_graph_community_v1");
        assert_eq!(report.label_policy, "unlabeled_clustering_discovery_only");
        assert!(report
            .governance_boundary
            .contains("must not create confirmed FWA labels"));
        assert!(!report.community_summaries.is_empty());
        assert_eq!(report.provider_assignments.len(), 6);
        assert!(!report.anomaly_candidates.is_empty());
        assert_eq!(report.review_tasks.len(), report.anomaly_candidates.len());
        assert!(output_dir
            .join("provider_graph_community_report.json")
            .is_file());
        assert!(output_dir
            .join("provider_graph_review_tasks.json")
            .is_file());
    }

    #[test]
    fn clusters_unlabeled_claim_entities_without_rule_writeback() {
        let root = temp_root("claim-entity-clustering");
        let pack = build_demo_ml_datasets(&root, "2026-06-entity-clustering-demo")
            .expect("demo ML datasets");
        let scoring_manifest = pack
            .unlabeled_manifest_uris
            .iter()
            .find(|uri| uri.contains("unlabeled_shadow_scoring"))
            .expect("shadow scoring manifest");
        let output_dir = root.join("entity-clusters");

        let report =
            cluster_claim_entities(scoring_manifest, &output_dir).expect("entity clustering");

        assert_eq!(report.report_kind, "claim_entity_clustering");
        assert_eq!(report.dataset_key, "rust_demo_claim_shadow_unlabeled");
        assert_eq!(report.algorithm, "rust_standardized_entity_kmeans_v1");
        assert_eq!(report.label_policy, "unlabeled_shadow_scoring_only");
        assert!(report
            .governance_boundary
            .contains("must not create confirmed FWA labels"));
        assert!(report
            .governance_boundary
            .contains("rule-library writeback"));
        assert_eq!(report.cluster_count, 4);
        assert_eq!(report.entity_assignments.len(), 6);
        assert!(!report.anomaly_candidates.is_empty());
        assert_eq!(report.review_tasks.len(), report.anomaly_candidates.len());
        assert_eq!(
            report.review_tasks[0].required_review,
            "human_review_required_before_case_creation_label_assignment_or_rule_writeback"
        );
        assert!(output_dir
            .join("claim_entity_clustering_report.json")
            .is_file());
        assert!(output_dir.join("claim_entity_review_tasks.json").is_file());
    }

    #[test]
    fn builds_scheduled_mlops_monitoring_plan() {
        let plan = build_mlops_monitoring_plan(
            "data/training/manifest.json",
            "s3://fwa-models/baseline_fwa/0.2.0/rust_serving_artifact.json",
            "baseline_fwa",
            "0.2.0",
            "0 2 * * *",
        )
        .expect("mlops monitoring plan");

        assert_eq!(plan["plan_kind"], "scheduled_mlops_monitoring");
        assert_eq!(plan["plan_version"], 2);
        assert_eq!(plan["model"]["model_key"], "baseline_fwa");
        assert_eq!(plan["model"]["model_version"], "0.2.0");
        assert_eq!(plan["schedule"]["cron"], "0 2 * * *");
        assert_eq!(
            plan["data_contract"]["source"],
            "same_parquet_dataset_manifest"
        );
        assert_eq!(plan["jobs"][0]["job_kind"], "shadow_traffic_evaluation");
        assert_eq!(plan["jobs"][1]["job_kind"], "drift_monitoring");
        assert_eq!(plan["jobs"][2]["job_kind"], "segment_fairness_review");
        assert_eq!(plan["jobs"][3]["job_kind"], "reviewer_disagreement_review");
        assert_eq!(plan["jobs"][4]["job_kind"], "label_delay_review");
        assert_eq!(
            plan["jobs"][1]["drift_report_uri"],
            "s3://fwa-models/baseline_fwa/0.2.0/drift_report.json"
        );
        assert_eq!(
            plan["jobs"][3]["reviewer_disagreement_report_uri"],
            "s3://fwa-models/baseline_fwa/0.2.0/reviewer_disagreement_report.json"
        );
        assert_eq!(
            plan["jobs"][4]["label_delay_report_uri"],
            "s3://fwa-models/baseline_fwa/0.2.0/label_delay_report.json"
        );
    }

    #[test]
    fn builds_mlops_monitoring_report_from_runtime_reports() {
        let root = temp_root("mlops-monitoring-report");
        let artifact_eval = root.join("artifact-evaluation.json");
        let shadow = root.join("shadow.json");
        let drift = root.join("drift.json");
        let fairness = root.join("fairness.json");
        write_json(
            artifact_eval.clone(),
            &serde_json::json!({
                "report_kind": "model_artifact_evaluation",
                "gate_status": "passed",
                "rust_serving_status": "passed",
                "latency_status": "passed",
                "p95_latency_ms": 18
            }),
        )
        .unwrap();
        write_json(
            shadow.clone(),
            &serde_json::json!({
                "status": "passed",
                "comparison_count": 100,
                "average_abs_probability_delta": 0.08,
                "max_abs_probability_delta": 0.18
            }),
        )
        .unwrap();
        write_json(
            drift.clone(),
            &serde_json::json!({
                "status": "stable",
                "score_psi": 0.04,
                "max_feature_psi": 0.06
            }),
        )
        .unwrap();
        write_json(
            fairness.clone(),
            &serde_json::json!({
                "status": "passed",
                "segments": [
                    {"segment_column": "provider_type", "segment_value": "clinic"}
                ]
            }),
        )
        .unwrap();

        let report = build_mlops_monitoring_report(
            "baseline_fwa",
            "0.2.0",
            &artifact_eval.to_string_lossy(),
            &shadow.to_string_lossy(),
            &drift.to_string_lossy(),
            &fairness.to_string_lossy(),
            root.join("out"),
        )
        .expect("mlops monitoring report");

        assert_eq!(report["report_kind"], "mlops_monitoring_report");
        assert_eq!(report["overall_status"], "passed");
        assert_eq!(report["retraining_recommendation"], "monitor");
        assert_eq!(
            report["signals"]["artifact_evaluation"]["p95_latency_ms"],
            18
        );
        assert_eq!(report["signals"]["fairness"]["segment_count"], 1);
        assert!(report["triggers"].as_array().unwrap().is_empty());
        assert!(root.join("out/mlops_monitoring_report.json").is_file());
        assert!(root
            .join("out/mlops_monitoring_review_tasks.json")
            .is_file());

        let (model_key, submission) = build_mlops_monitoring_report_submission(
            &root
                .join("out/mlops_monitoring_report.json")
                .to_string_lossy(),
            "mlops-worker",
            "submit monitoring report",
        )
        .expect("monitoring report submission");
        assert_eq!(model_key, "baseline_fwa");
        assert_eq!(submission.report_kind, "mlops_monitoring_report");
        assert_eq!(submission.model_version, "0.2.0");
        assert_eq!(submission.overall_status, "passed");
        assert!(submission
            .evidence_refs
            .contains(&"model_versions:baseline_fwa:0.2.0".into()));
        assert!(submission
            .evidence_refs
            .iter()
            .any(|reference| reference.starts_with("model_monitoring_reports:")));
    }

    #[test]
    fn mlops_monitoring_report_opens_reviews_for_drift_and_latency() {
        let root = temp_root("mlops-monitoring-report-watch");
        let artifact_eval = root.join("artifact-evaluation.json");
        let shadow = root.join("shadow.json");
        let drift = root.join("drift.json");
        let fairness = root.join("fairness.json");
        write_json(
            artifact_eval.clone(),
            &serde_json::json!({
                "gate_status": "passed",
                "rust_serving_status": "passed",
                "latency_status": "failed",
                "p95_latency_ms": 250
            }),
        )
        .unwrap();
        write_json(
            shadow.clone(),
            &serde_json::json!({
                "status": "watch",
                "comparison_count": 100,
                "average_abs_probability_delta": 0.42
            }),
        )
        .unwrap();
        write_json(
            drift.clone(),
            &serde_json::json!({
                "status": "drift",
                "score_psi": 0.34,
                "max_feature_psi": 0.41
            }),
        )
        .unwrap();
        write_json(
            fairness.clone(),
            &serde_json::json!({
                "status": "passed",
                "segments": []
            }),
        )
        .unwrap();

        let report = build_mlops_monitoring_report(
            "baseline_fwa",
            "0.2.0",
            &artifact_eval.to_string_lossy(),
            &shadow.to_string_lossy(),
            &drift.to_string_lossy(),
            &fairness.to_string_lossy(),
            root.join("out"),
        )
        .expect("mlops monitoring report");

        assert_eq!(report["overall_status"], "watch");
        assert_eq!(report["retraining_recommendation"], "prepare_retraining");
        let triggers = report["triggers"].as_array().unwrap();
        assert!(triggers.contains(&serde_json::json!("rust_serving_latency_budget_failed")));
        assert!(triggers.contains(&serde_json::json!("model_drift_detected")));
        assert!(triggers.contains(&serde_json::json!("shadow_comparison_review_required")));
        assert_eq!(report["review_tasks"].as_array().unwrap().len(), 3);
        assert_eq!(
            report["promotion_boundary"],
            "monitoring can open review or retraining preparation only; it must not activate models, publish rules, or assign fraud labels"
        );
    }

    #[test]
    fn builds_mlops_scheduler_execution_report_and_alert_delivery_tasks() {
        let root = temp_root("mlops-scheduler-execution");
        let plan = build_mlops_monitoring_plan(
            "data/training/manifest.json",
            &root.join("rust_serving_artifact.json").to_string_lossy(),
            "baseline_fwa",
            "0.2.0",
            "0 2 * * *",
        )
        .expect("monitoring plan");
        let plan_uri = root.join("mlops_monitoring_plan.json");
        write_json(plan_uri.clone(), &plan).unwrap();
        let artifact_eval = root.join("artifact-evaluation.json");
        let shadow = root.join("shadow_report.json");
        let drift = root.join("drift_report.json");
        let fairness = root.join("fairness_report.json");
        write_json(
            artifact_eval.clone(),
            &serde_json::json!({
                "gate_status": "passed",
                "rust_serving_status": "passed",
                "latency_status": "failed",
                "p95_latency_ms": 250
            }),
        )
        .unwrap();
        write_json(
            shadow.clone(),
            &serde_json::json!({"status": "passed", "comparison_count": 100}),
        )
        .unwrap();
        write_json(
            drift.clone(),
            &serde_json::json!({"status": "drift", "score_psi": 0.34}),
        )
        .unwrap();
        write_json(
            fairness.clone(),
            &serde_json::json!({"status": "passed", "segments": []}),
        )
        .unwrap();
        build_mlops_monitoring_report(
            "baseline_fwa",
            "0.2.0",
            &artifact_eval.to_string_lossy(),
            &shadow.to_string_lossy(),
            &drift.to_string_lossy(),
            &fairness.to_string_lossy(),
            root.join("monitoring"),
        )
        .expect("monitoring report");

        let report = build_mlops_scheduler_execution_report(
            &plan_uri.to_string_lossy(),
            &root
                .join("monitoring/mlops_monitoring_report.json")
                .to_string_lossy(),
            root.join("scheduler"),
        )
        .expect("scheduler execution report");

        assert_eq!(report["report_kind"], "mlops_scheduler_execution_report");
        assert_eq!(report["model_key"], "baseline_fwa");
        assert_eq!(
            report["alert_delivery_status"],
            "queued_for_external_alert_router"
        );
        assert_eq!(report["alert_delivery_task_count"], 2);
        assert!(report["governance_boundary"]
            .as_str()
            .unwrap()
            .contains("must not create retraining jobs"));
        assert!(report["job_executions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|job| {
                job["job_kind"] == "drift_monitoring"
                    && job["execution_status"] == "reported_in_monitoring_summary"
            }));
        assert!(report["alert_delivery_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|task| task["trigger"] == "model_drift_detected"
                && task["route_key"] == "mlops_retraining_readiness"));
        assert!(root
            .join("scheduler/mlops_scheduler_execution_report.json")
            .is_file());
        assert!(root
            .join("scheduler/mlops_alert_delivery_tasks.json")
            .is_file());
    }

    #[test]
    fn onnx_runtime_requires_passed_parity_report() {
        let root = temp_root("onnx-parity-gate");
        let parity_report = root.join("onnx_parity_report.json");
        write_json(
            parity_report.clone(),
            &serde_json::json!({
                "report_kind": "onnx_probability_parity",
                "status": "passed",
                "serving_runtime_kind": "xgboost_onnx",
                "max_abs_probability_delta": 0.00001,
                "tolerance": 0.0001
            }),
        )
        .unwrap();

        let gate = validate_onnx_parity_for_runtime(
            "xgboost_onnx",
            Some(&parity_report.to_string_lossy()),
        )
        .expect("onnx parity")
        .expect("onnx gate");
        assert_eq!(gate.gate_status, "passed");
        assert_eq!(gate.status, "passed");
        assert_eq!(gate.serving_runtime_kind, "xgboost_onnx");

        let missing = validate_onnx_parity_for_runtime("lightgbm_onnx", None);
        assert!(missing.is_err());
        let deep_learning_missing = validate_onnx_parity_for_runtime("deep_learning_onnx", None);
        assert!(deep_learning_missing.is_err());

        write_json(
            parity_report.clone(),
            &serde_json::json!({
                "report_kind": "onnx_probability_parity",
                "status": "failed",
                "serving_runtime_kind": "xgboost_onnx"
            }),
        )
        .unwrap();
        let blocked = validate_onnx_parity_for_runtime(
            "xgboost_onnx",
            Some(&parity_report.to_string_lossy()),
        )
        .expect("blocked parity")
        .expect("onnx gate");
        assert_eq!(blocked.gate_status, "blocked");
    }

    #[test]
    fn builds_automl_lifecycle_closure_report_from_governed_evidence() {
        let root = temp_root("automl-lifecycle-closure");
        let pack = build_demo_ml_datasets(&root, "2026-06-closure-demo").expect("demo ML datasets");

        let xgboost_validation = root.join("xgboost-validation.json");
        let lightgbm_validation = root.join("lightgbm-validation.json");
        write_validation_report(
            &xgboost_validation,
            "0.2.0-xgboost-candidate",
            "xgboost",
            "gradient_boosted_tree",
            0.86,
            0.80,
            0.76,
            "passed",
        );
        write_validation_report(
            &lightgbm_validation,
            "0.2.0-lightgbm-candidate",
            "lightgbm",
            "gradient_boosted_tree",
            0.85,
            0.79,
            0.75,
            "passed",
        );
        let ranking = rank_automl_candidates(
            &[
                xgboost_validation.to_string_lossy().into_owned(),
                lightgbm_validation.to_string_lossy().into_owned(),
            ],
            root.join("ranking"),
        )
        .expect("ranking");
        assert_eq!(
            ranking.recommended_candidate_model_version.as_deref(),
            Some("0.2.0-xgboost-candidate")
        );

        let xgboost_artifact_eval = root.join("xgboost-artifact-evaluation.json");
        let lightgbm_artifact_eval = root.join("lightgbm-artifact-evaluation.json");
        write_json(
            xgboost_artifact_eval.clone(),
            &serde_json::json!({
                "report_kind": "model_artifact_evaluation",
                "runtime_kind": "xgboost_onnx",
                "gate_status": "passed",
                "rust_serving_status": "passed",
                "latency_status": "passed",
                "p95_latency_ms": 24
            }),
        )
        .unwrap();
        write_json(
            lightgbm_artifact_eval.clone(),
            &serde_json::json!({
                "report_kind": "model_artifact_evaluation",
                "runtime_kind": "lightgbm_onnx",
                "gate_status": "passed",
                "rust_serving_status": "passed",
                "latency_status": "passed",
                "p95_latency_ms": 21
            }),
        )
        .unwrap();

        let rule_backtest = root.join("rule-backtest.json");
        write_json(
            rule_backtest.clone(),
            &serde_json::json!({
                "report_kind": "deterministic_rule_candidate_backtest",
                "rule_library_writeback_status": "blocked_pending_human_review_and_policy_governance_approval",
                "candidate_results": [
                    {"candidate_rule_key": "rule_candidate_high_amount", "gate_status": "passed"}
                ],
                "review_tasks": [
                    {"task_kind": "rule_candidate_backtest_review"}
                ]
            }),
        )
        .unwrap();

        let provider_manifest = pack
            .unlabeled_manifest_uris
            .iter()
            .find(|uri| uri.contains("unlabeled_provider_peer_clustering"))
            .expect("provider manifest");
        let provider_cluster_dir = root.join("provider-clusters");
        cluster_provider_peers(provider_manifest, &provider_cluster_dir)
            .expect("provider clustering");
        let provider_graph_dir = root.join("provider-graph");
        cluster_provider_graph_communities(provider_manifest, &provider_graph_dir)
            .expect("provider graph clustering");
        let claim_manifest = pack
            .unlabeled_manifest_uris
            .iter()
            .find(|uri| uri.contains("unlabeled_shadow_scoring"))
            .expect("claim manifest");
        let claim_cluster_dir = root.join("claim-entity-clusters");
        cluster_claim_entities(claim_manifest, &claim_cluster_dir).expect("claim clustering");

        let mlops_monitoring = build_mlops_monitoring_report(
            "baseline_fwa",
            "0.2.0",
            &xgboost_artifact_eval.to_string_lossy(),
            &root.join("shadow.json").to_string_lossy(),
            &root.join("drift.json").to_string_lossy(),
            &root.join("fairness.json").to_string_lossy(),
            root.join("monitoring"),
        );
        assert!(mlops_monitoring.is_err());
        write_json(
            root.join("shadow.json"),
            &serde_json::json!({"status": "passed"}),
        )
        .unwrap();
        write_json(
            root.join("drift.json"),
            &serde_json::json!({"status": "stable"}),
        )
        .unwrap();
        write_json(
            root.join("fairness.json"),
            &serde_json::json!({"status": "passed", "segments": []}),
        )
        .unwrap();
        build_mlops_monitoring_report(
            "baseline_fwa",
            "0.2.0",
            &xgboost_artifact_eval.to_string_lossy(),
            &root.join("shadow.json").to_string_lossy(),
            &root.join("drift.json").to_string_lossy(),
            &root.join("fairness.json").to_string_lossy(),
            root.join("monitoring"),
        )
        .expect("monitoring report");
        let monitoring_plan = build_mlops_monitoring_plan(
            &pack.labeled_manifest_uri,
            &root.join("rust_serving_artifact.json").to_string_lossy(),
            "baseline_fwa",
            "0.2.0",
            "0 2 * * *",
        )
        .expect("monitoring plan");
        let monitoring_plan_uri = root.join("monitoring-plan.json");
        write_json(monitoring_plan_uri.clone(), &monitoring_plan).unwrap();
        build_mlops_scheduler_execution_report(
            &monitoring_plan_uri.to_string_lossy(),
            &root
                .join("monitoring/mlops_monitoring_report.json")
                .to_string_lossy(),
            root.join("scheduler"),
        )
        .expect("scheduler execution report");

        let report = build_automl_lifecycle_closure_report(
            &root.join("index.json").to_string_lossy(),
            &root
                .join("ranking/automl_candidate_ranking.json")
                .to_string_lossy(),
            &[
                xgboost_artifact_eval.to_string_lossy().into_owned(),
                lightgbm_artifact_eval.to_string_lossy().into_owned(),
            ],
            &rule_backtest.to_string_lossy(),
            &provider_cluster_dir
                .join("provider_peer_clustering_report.json")
                .to_string_lossy(),
            &provider_graph_dir
                .join("provider_graph_community_report.json")
                .to_string_lossy(),
            &claim_cluster_dir
                .join("claim_entity_clustering_report.json")
                .to_string_lossy(),
            &root
                .join("monitoring/mlops_monitoring_report.json")
                .to_string_lossy(),
            &root
                .join("scheduler/mlops_scheduler_execution_report.json")
                .to_string_lossy(),
            root.join("closure"),
        )
        .expect("lifecycle closure report");

        assert_eq!(report["report_kind"], "rust_automl_lifecycle_closure");
        assert_eq!(
            report["closure_status"],
            "closed_with_human_governance_gates"
        );
        assert_eq!(report["lifecycle_stages"].as_array().unwrap().len(), 6);
        assert!(report["lifecycle_stages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|stage| stage["status"] == "passed"));
        assert!(report["governance_boundary"]
            .as_str()
            .unwrap()
            .contains("must not auto-activate models"));
        let clustering_stage = report["lifecycle_stages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|stage| stage["stage"] == "unlabeled_clustering_reviews")
            .expect("clustering stage");
        assert!(clustering_stage["evidence_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|evidence_ref| evidence_ref
                .as_str()
                .unwrap()
                .starts_with("provider_graph_clustering:")));
        let monitoring_stage = report["lifecycle_stages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|stage| stage["stage"] == "mlops_monitoring_loop")
            .expect("monitoring stage");
        assert!(monitoring_stage["evidence_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|evidence_ref| evidence_ref
                .as_str()
                .unwrap()
                .starts_with("mlops_scheduler_execution_reports:")));
        assert!(root
            .join("closure/rust_automl_lifecycle_closure_report.json")
            .is_file());
    }

    #[test]
    fn builds_demo_automl_lifecycle_evidence_pack() {
        let root = temp_root("demo-automl-lifecycle-evidence");
        let demo_root = root.join("demo");
        build_demo_ml_datasets(&demo_root, "2026-06-rust-automl-demo").expect("demo ML datasets");
        let output_dir = root.join("lifecycle-evidence");

        let index = build_demo_automl_lifecycle_evidence(&demo_root, &output_dir)
            .expect("demo lifecycle evidence");

        assert_eq!(
            index["evidence_pack_kind"],
            "rust_automl_demo_lifecycle_evidence"
        );
        assert_eq!(
            index["closure_status"],
            "closed_with_human_governance_gates"
        );
        assert_eq!(
            index["recommended_candidate_model_version"],
            "0.2.0-xgboost-candidate"
        );
        assert!(output_dir
            .join("ranking/automl_candidate_ranking.json")
            .is_file());
        assert!(output_dir
            .join("rule-candidates/backtest/rule_candidate_backtest_report.json")
            .is_file());
        assert!(output_dir
            .join("clustering/provider-peer/provider_peer_clustering_report.json")
            .is_file());
        assert!(output_dir
            .join("clustering/provider-graph/provider_graph_community_report.json")
            .is_file());
        assert!(output_dir
            .join("clustering/claim-entity/claim_entity_clustering_report.json")
            .is_file());
        assert!(output_dir
            .join("monitoring/mlops_monitoring_report.json")
            .is_file());
        assert!(output_dir
            .join("monitoring/scheduler/mlops_monitoring_plan.json")
            .is_file());
        assert!(output_dir
            .join("monitoring/scheduler/mlops_scheduler_execution_report.json")
            .is_file());
        assert!(output_dir
            .join("monitoring/scheduler/mlops_alert_delivery_tasks.json")
            .is_file());
        assert!(output_dir
            .join("closure/rust_automl_lifecycle_closure_report.json")
            .is_file());
        assert!(output_dir
            .join("demo_lifecycle_evidence_index.json")
            .is_file());
    }

    #[test]
    fn ranks_automl_candidates_and_blocks_missing_governance_gates() {
        let root = temp_root("automl-ranking");
        let logistic_report = root.join("logistic-validation.json");
        let xgboost_report = root.join("xgboost-validation.json");
        let lightgbm_report = root.join("lightgbm-validation.json");
        write_validation_report(
            &logistic_report,
            "0.1.0-candidate-logistic",
            "logistic_regression",
            "linear_baseline",
            0.72,
            0.68,
            0.66,
            "passed",
        );
        write_validation_report(
            &xgboost_report,
            "0.1.0-xgboost-candidate",
            "xgboost",
            "gradient_boosted_tree",
            0.84,
            0.78,
            0.74,
            "passed",
        );
        write_validation_report(
            &lightgbm_report,
            "0.1.0-lightgbm-candidate",
            "lightgbm",
            "gradient_boosted_tree",
            0.86,
            0.80,
            0.77,
            "failed",
        );

        let report_uris = vec![
            logistic_report.to_string_lossy().into_owned(),
            xgboost_report.to_string_lossy().into_owned(),
            lightgbm_report.to_string_lossy().into_owned(),
        ];
        let output_dir = root.join("out");
        let ranking = rank_automl_candidates(&report_uris, &output_dir).expect("ranking");

        assert_eq!(ranking.plan_kind, "automl_candidate_ranking");
        assert_eq!(
            ranking.recommended_candidate_model_version.as_deref(),
            Some("0.1.0-xgboost-candidate")
        );
        assert_eq!(
            ranking.candidates[0].candidate_model_version,
            "0.1.0-xgboost-candidate"
        );
        assert_eq!(ranking.candidates[0].gate_status, "passed");
        assert_eq!(
            ranking.candidates[0].recommended_action,
            "open_human_review"
        );
        assert_eq!(
            ranking.candidates[1].candidate_model_version,
            "0.1.0-candidate-logistic"
        );
        assert_eq!(
            ranking.candidates[2].candidate_model_version,
            "0.1.0-lightgbm-candidate"
        );
        assert_eq!(ranking.candidates[2].gate_status, "blocked");
        assert!(ranking.candidates[2]
            .blocking_reasons
            .contains(&"leakage_check_status:failed".to_string()));
        assert_eq!(ranking.review_tasks.len(), 3);
        assert_eq!(
            ranking.review_tasks[0].required_review,
            "human_approval_required_before_shadow_or_activation"
        );
        assert!(output_dir.join("automl_candidate_ranking.json").is_file());
        assert!(output_dir.join("automl_review_tasks.json").is_file());
    }

    #[test]
    fn automl_candidate_ranking_requires_rust_lifecycle_evidence() {
        let root = temp_root("automl-rust-evidence");
        let validation_report = root.join("validation.json");
        fs::write(
            &validation_report,
            serde_json::json!({
                "model_key": "baseline_fwa",
                "candidate_model_version": "0.1.0-candidate-without-rust-evidence",
                "dataset_key": "claims_model",
                "dataset_version": "2026-06-demo",
                "algorithm": "xgboost",
                "validation_metrics": {
                    "auc": 0.84,
                    "precision": 0.78,
                    "recall": 0.74
                },
                "metrics_json": {
                    "algorithm": "xgboost",
                    "algorithm_family": "gradient_boosted_tree",
                    "out_of_time_auc": 0.84,
                    "out_of_time_average_precision": 0.80,
                    "out_of_time_precision": 0.78,
                    "out_of_time_recall": 0.74,
                    "time_group_split_status": "passed",
                    "leakage_check_status": "passed",
                    "shadow_comparison_status": "passed",
                    "serving_version_lock_status": "passed",
                    "artifact_integrity_status": "passed",
                    "feature_store_materialization_status": "passed",
                    "segment_fairness_status": "passed",
                    "label_provenance_status": "passed"
                }
            })
            .to_string(),
        )
        .unwrap();

        let ranking = rank_automl_candidates(
            &[validation_report.to_string_lossy().into_owned()],
            root.join("out"),
        )
        .expect("ranking");

        assert_eq!(ranking.recommended_candidate_model_version, None);
        assert_eq!(ranking.candidates[0].gate_status, "blocked");
        assert!(ranking.candidates[0]
            .blocking_reasons
            .contains(&"rust_feature_set_status:missing_or_failed".into()));
        assert!(ranking.candidates[0]
            .blocking_reasons
            .contains(&"rust_feature_set_manifest_uri:missing".into()));
        assert!(ranking.candidates[0]
            .blocking_reasons
            .contains(&"model_artifact_evaluation_status:missing_or_failed".into()));
        assert!(ranking.candidates[0]
            .blocking_reasons
            .contains(&"onnx_parity_status:missing_or_failed".into()));
        assert!(ranking.candidates[0]
            .blocking_reasons
            .contains(&"onnx_parity_report_uri:missing".into()));
    }

    #[tokio::test]
    async fn evaluates_model_artifact_with_rust_serving_parity_gate() {
        let root = temp_root("model-artifact-evaluation");
        let artifact_path = root.join("rust_serving_artifact.json");
        fs::write(
            &artifact_path,
            serde_json::to_vec(&serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.2.0-rust",
                "runtime_kind": "rust_logistic_regression",
                "execution_provider": "cpu",
                "threshold": 0.5,
                "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
                "intercept": -2.0,
                "coefficients": {
                    "claim_amount_to_limit_ratio": 4.0,
                    "provider_profile_score": 0.01
                }
            }))
            .unwrap(),
        )
        .unwrap();
        let artifact_sha256 = test_sha256(&artifact_path);
        let serving_manifest_path = root.join("serving_manifest.json");
        write_json(
            serving_manifest_path.clone(),
            &serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.2.0-rust",
                "runtime_kind": "rust_logistic_regression",
                "artifact_uri": artifact_path.to_string_lossy(),
                "artifact_sha256": artifact_sha256,
                "version_lock": "0.2.0-rust",
                "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
                "threshold": 0.5
            }),
        )
        .unwrap();

        let dataset_dir = root.join("dataset");
        let validation_dir = dataset_dir.join("split=validation");
        fs::create_dir_all(&validation_dir).unwrap();
        let schema = Arc::new(Schema::new(vec![
            Field::new("claim_id", DataType::Utf8, false),
            Field::new("claim_amount_to_limit_ratio", DataType::Float64, false),
            Field::new("provider_profile_score", DataType::Float64, false),
            Field::new("expected_probability", DataType::Float64, false),
            Field::new("confirmed_fwa", DataType::Int8, false),
        ]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(vec!["CLM-EVAL-1", "CLM-EVAL-2"])),
                Arc::new(Float64Array::from(vec![0.8, 0.2])),
                Arc::new(Float64Array::from(vec![20.0, 10.0])),
                Arc::new(Float64Array::from(vec![0.8022, 0.2497])),
                Arc::new(Int8Array::from(vec![1, 0])),
            ],
        )
        .unwrap();
        write_parquet(validation_dir.join("part-00000.parquet"), schema, &batch).unwrap();
        let dataset_manifest_path = dataset_dir.join("manifest.json");
        write_json(
            dataset_manifest_path.clone(),
            &serde_json::json!({
                "dataset_key": "claims_model",
                "dataset_version": "2026-06-eval",
                "business_domain": "health_fwa",
                "sample_grain": "claim",
                "label_column": "confirmed_fwa",
                "entity_keys": ["claim_id"],
                "splits": [
                    {"split_name": "validation", "data_uri": "split=validation/"}
                ]
            }),
        )
        .unwrap();

        let output_dir = root.join("out");
        let report = evaluate_model_artifact(
            &serving_manifest_path.to_string_lossy(),
            &dataset_manifest_path.to_string_lossy(),
            "validation",
            &output_dir,
            Some("expected_probability"),
            0.0001,
            100,
            10,
            None,
        )
        .await
        .expect("model artifact evaluation");

        assert_eq!(report.report_kind, "model_artifact_evaluation");
        assert_eq!(report.runtime_kind, "rust_logistic_regression");
        assert_eq!(report.row_count, 2);
        assert_eq!(report.contract_status, "passed");
        assert_eq!(report.rust_serving_status, "passed");
        assert_eq!(report.parity_status, "passed");
        assert_eq!(report.latency_status, "passed");
        assert_eq!(report.gate_status, "passed");
        assert_eq!(report.max_abs_probability_delta, Some(0.0));
        assert_eq!(report.sample_results[0].score, 80);
        assert_eq!(report.sample_results[1].label, "LOW_RISK");
        assert!(output_dir
            .join("model_artifact_evaluation_report.json")
            .is_file());
    }

    #[test]
    fn mines_rule_candidates_from_feature_importance_without_rule_library_writeback() {
        let root = temp_root("rule-candidate-mining");
        let validation_report = root.join("validation.json");
        let feature_importance = root.join("feature_importance.parquet");
        write_validation_report(
            &validation_report,
            "0.1.0-xgboost-candidate",
            "xgboost",
            "gradient_boosted_tree",
            0.84,
            0.78,
            0.74,
            "passed",
        );
        write_feature_importance_parquet(
            &feature_importance,
            &[
                ("claim_amount_to_limit_ratio", 0.91),
                ("provider_profile_score", 0.72),
                ("high_cost_item_ratio", 0.53),
                ("service_date_ord", 0.12),
            ],
        );

        let output_dir = root.join("out");
        let plan = mine_rule_candidates(
            &validation_report.to_string_lossy(),
            &feature_importance.to_string_lossy(),
            &output_dir,
        )
        .expect("rule candidate mining");

        assert_eq!(plan.plan_kind, "explainable_model_rule_candidate_mining");
        assert_eq!(plan.source_algorithm, "xgboost");
        assert_eq!(plan.candidate_rules.len(), 3);
        assert_eq!(
            plan.candidate_rules
                .iter()
                .map(|candidate| candidate.source_feature.as_str())
                .collect::<Vec<_>>(),
            vec![
                "claim_amount_to_limit_ratio",
                "provider_profile_score",
                "high_cost_item_ratio"
            ]
        );
        assert!(plan
            .promotion_boundary
            .contains("backtest and human review"));
        assert_eq!(
            plan.candidate_rules[0].gate_status,
            "blocked_until_backtest_and_human_review"
        );
        assert!(plan.candidate_rules[0]
            .required_before_rule_library_writeback
            .contains(&"deterministic_backtest".to_string()));
        assert_eq!(
            plan.candidate_rules[0].draft_rule_template["conditions"][0]["operator"],
            "threshold_selected_by_backtest"
        );
        assert_eq!(plan.backtest_requests.len(), 3);
        assert_eq!(
            plan.backtest_requests[0].backtest_kind,
            "deterministic_rule_candidate_backtest"
        );
        assert_eq!(plan.review_tasks.len(), 3);
        assert_eq!(
            plan.review_tasks[0].required_review,
            "human_approval_required_before_rule_library_writeback"
        );
        assert!(output_dir.join("rule_candidate_mining_plan.json").is_file());
        assert!(output_dir
            .join("rule_candidate_backtest_requests.json")
            .is_file());
        assert!(output_dir
            .join("rule_candidate_review_tasks.json")
            .is_file());
    }

    #[test]
    fn backtests_rule_candidates_before_rule_library_writeback() {
        let root = temp_root("rule-candidate-backtest");
        let dataset_pack = build_demo_ml_datasets(root.join("datasets"), "2026-06-backtest")
            .expect("demo ML datasets");
        let validation_report = root.join("validation.json");
        let feature_importance = root.join("feature_importance.parquet");
        write_validation_report(
            &validation_report,
            "0.2.0-xgboost-candidate",
            "xgboost",
            "gradient_boosted_tree",
            0.86,
            0.8,
            0.75,
            "passed",
        );
        write_feature_importance_parquet(
            &feature_importance,
            &[
                ("amount_to_limit_ratio", 0.91),
                ("high_cost_item_ratio", 0.72),
                ("provider_risk_tier", 0.53),
            ],
        );
        let mining_dir = root.join("mining");
        mine_rule_candidates(
            &validation_report.to_string_lossy(),
            &feature_importance.to_string_lossy(),
            &mining_dir,
        )
        .expect("rule candidate mining");

        let output_dir = root.join("backtest");
        let report = run_rule_candidate_backtest(
            &mining_dir
                .join("rule_candidate_mining_plan.json")
                .to_string_lossy(),
            &dataset_pack.labeled_manifest_uri,
            &output_dir,
        )
        .expect("rule candidate backtest");

        assert_eq!(report.report_kind, "deterministic_rule_candidate_backtest");
        assert_eq!(report.dataset_key, "rust_demo_claim_risk_labeled");
        assert_eq!(
            report.rule_library_writeback_status,
            "blocked_pending_human_review_and_policy_governance_approval"
        );
        assert_eq!(report.candidate_results.len(), 3);
        assert_eq!(
            report.candidate_results[0].gate_status,
            "backtested_but_blocked_until_human_review"
        );
        assert!(report.candidate_results[0].selected_threshold.is_finite());
        assert!(report.candidate_results[0]
            .metrics_by_split
            .contains_key("train"));
        assert!(report.candidate_results[0]
            .metrics_by_split
            .contains_key("validation"));
        assert!(report.candidate_results[0]
            .metrics_by_split
            .contains_key("out_of_time"));
        assert_eq!(report.review_tasks.len(), 3);
        assert_eq!(
            report.review_tasks[0].required_review,
            "human_approval_required_after_backtest_before_rule_library_writeback"
        );
        assert!(output_dir
            .join("rule_candidate_backtest_report.json")
            .is_file());
        assert!(output_dir
            .join("rule_candidate_backtest_review_tasks.json")
            .is_file());
    }

    #[test]
    fn builds_scheduled_analytics_export_plan() {
        let plan = build_analytics_export_plan(
            "s3://nwfwa-staging-artifacts",
            "http://clickhouse:8123",
            "staging-customer",
            "15 * * * *",
        )
        .expect("analytics export plan");

        assert_eq!(plan["plan_kind"], "scheduled_analytics_export");
        assert_eq!(plan["plan_version"], 1);
        assert_eq!(plan["customer_scope_id"], "staging-customer");
        assert_eq!(plan["data_contract"]["derived_store"], "clickhouse");
        assert_eq!(
            plan["data_contract"]["pii_policy"],
            "masked_ids_and_evidence_refs_only"
        );
        assert_eq!(plan["schedule"]["cron"], "15 * * * *");
        assert_eq!(plan["jobs"][0]["job_kind"], "scoring_events_export");
        assert_eq!(plan["jobs"][1]["job_kind"], "rule_events_export");
        assert_eq!(plan["jobs"][2]["job_kind"], "model_events_export");
        assert_eq!(plan["jobs"][3]["job_kind"], "case_sla_events_export");
        assert_eq!(plan["jobs"][4]["job_kind"], "value_events_export");
        assert_eq!(
            plan["jobs"][5]["job_kind"],
            "reviewer_capacity_events_export"
        );
        assert_eq!(
            plan["jobs"][6]["job_kind"],
            "provider_graph_snapshots_export"
        );
        assert_eq!(
            plan["jobs"][6]["sink_table"],
            "fwa_analytics.analytics_provider_graph_snapshots"
        );
        assert!(plan["dashboard_coverage"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("false_positive_cost")));
    }

    #[test]
    fn builds_scheduled_ai_evidence_execution_plan() {
        let plan = build_ai_evidence_execution_plan(
            "http://api-server:8080",
            "s3://nwfwa-staging-artifacts",
            "pgvector",
            "postgres://evidence_vectors",
            "staging-customer",
            "*/20 * * * *",
        )
        .expect("ai evidence execution plan");

        assert_eq!(plan["plan_kind"], "scheduled_ai_evidence_execution");
        assert_eq!(plan["plan_version"], 1);
        assert_eq!(plan["customer_scope_id"], "staging-customer");
        assert_eq!(
            plan["runtime_boundary"]["raw_document_text"],
            "customer_approved_object_storage_only"
        );
        assert_eq!(plan["vector_store"]["kind"], "pgvector");
        assert_eq!(plan["schedule"]["concurrency_policy"], "forbid");
        assert_eq!(
            plan["api_contract"]["embedding_job_registry_path"],
            "/api/v1/ops/evidence/embedding-jobs"
        );
        assert_eq!(
            plan["jobs"][0]["job_kind"],
            "document_ingestion_metadata_sync"
        );
        assert_eq!(plan["jobs"][1]["job_kind"], "ocr_output_registration");
        assert_eq!(plan["jobs"][2]["job_kind"], "document_chunk_registration");
        assert_eq!(plan["jobs"][3]["job_kind"], "embedding_job_dispatch");
        assert_eq!(plan["jobs"][4]["job_kind"], "retrieval_ranking_evaluation");
        assert_eq!(
            plan["artifact_contract"]["retrieval_eval_report_uri"],
            "s3://nwfwa-staging-artifacts/ai-evidence/staging-customer/retrieval-eval/{window_start}/retrieval_eval_report.json"
        );
        assert_eq!(
            plan["downstream_contracts"]["analytics_export_plan"],
            "build-analytics-export-plan"
        );
    }

    #[test]
    fn builds_scheduled_governance_ops_plan() {
        let plan = build_governance_ops_plan(
            "s3://nwfwa-staging-artifacts",
            "postgres://postgres:5432/fwa",
            "staging-customer",
            "staging-retention-v1",
            "staging-backup-restore-v1",
            "staging-legal-hold-v1",
            "45 1 * * *",
        )
        .expect("governance ops plan");

        assert_eq!(plan["plan_kind"], "scheduled_governance_ops");
        assert_eq!(plan["plan_version"], 1);
        assert_eq!(plan["customer_scope_id"], "staging-customer");
        assert_eq!(
            plan["policies"]["retention_policy_id"],
            "staging-retention-v1"
        );
        assert_eq!(
            plan["policies"]["backup_restore_plan_id"],
            "staging-backup-restore-v1"
        );
        assert_eq!(
            plan["policies"]["legal_hold_policy_id"],
            "staging-legal-hold-v1"
        );
        assert_eq!(
            plan["runtime_boundary"]["destructive_actions"],
            "approval_required_plan_only"
        );
        assert_eq!(plan["schedule"]["concurrency_policy"], "forbid");
        assert_eq!(plan["jobs"][0]["job_kind"], "backup_snapshot_manifest");
        assert_eq!(plan["jobs"][1]["job_kind"], "restore_drill_validation");
        assert_eq!(plan["jobs"][2]["job_kind"], "retention_policy_scan");
        assert_eq!(plan["jobs"][3]["job_kind"], "legal_hold_reconciliation");
        assert_eq!(plan["jobs"][4]["job_kind"], "destruction_candidate_review");
        assert_eq!(
            plan["jobs"][4]["approval_gate"],
            "human_approval_required_before_destroy"
        );
        assert_eq!(
            plan["artifact_contract"]["retention_scan_report_uri"],
            "s3://nwfwa-staging-artifacts/governance-ops/staging-customer/retention/{window_start}/retention_scan_report.json"
        );
    }

    #[test]
    fn profiles_parquet_manifest_and_writes_schema_and_profile() {
        let root = temp_root("parquet-profile");
        let train_dir = root.join("split=train");
        let validation_dir = root.join("split=validation");
        fs::create_dir_all(&train_dir).unwrap();
        fs::create_dir_all(&validation_dir).unwrap();
        write_fixture_parquet(&train_dir.join("part-00000.parquet"), &["P1", "P2", "P3"]);
        write_fixture_parquet(
            &validation_dir.join("part-00000.parquet"),
            &["P4", "P5", "P6"],
        );

        let manifest_path = root.join("manifest.json");
        fs::write(
            &manifest_path,
            serde_json::json!({
                "dataset_key": "renewal_automl_20211105",
                "dataset_version": "v1",
                "business_domain": "renewal_retention",
                "sample_grain": "policy_order",
                "label_column": "m_2_keep_status",
                "entity_keys": ["policy_no", "order_no"],
                "splits": [
                    { "split_name": "train", "data_uri": "split=train/" },
                    { "split_name": "validation", "data_uri": "split=validation/" }
                ]
            })
            .to_string(),
        )
        .unwrap();

        let output_dir = root.join("out");
        let result = profile_manifest_file(&manifest_path, &output_dir).unwrap();

        assert_eq!(result.profile.row_count_by_split["train"], 3);
        assert_eq!(result.profile.row_count_by_split["validation"], 3);
        assert_eq!(result.profile.label_distribution_by_split["train"]["1"], 2);
        assert_eq!(result.profile.label_distribution_by_split["train"]["0"], 1);
        let policy_field = result
            .schema
            .fields
            .iter()
            .find(|field| field.field_name == "policy_no")
            .unwrap();
        assert_eq!(policy_field.logical_type, "Utf8");
        assert_eq!(policy_field.semantic_role, "key");
        let premium_profile = result
            .profile
            .fields
            .iter()
            .find(|field| field.field_name == "sum_premium")
            .unwrap();
        assert_eq!(premium_profile.missing_count_by_split["train"], 1);
        assert_eq!(result.catalog.storage_format, "parquet");
        assert_eq!(result.catalog.row_count, 6);
        assert_eq!(result.catalog.splits[0].positive_count, Some(2));
        assert!(result.catalog.schema_hash.starts_with("fnv64:"));
        assert!(output_dir.join("schema.json").is_file());
        assert!(output_dir.join("profile.json").is_file());
        assert!(output_dir.join("catalog.json").is_file());
    }

    #[test]
    fn rejects_csv_manifest_split() {
        let manifest = ParquetDatasetManifest {
            source_key: None,
            display_name: None,
            owner: None,
            description: None,
            status: None,
            dataset_key: "bad".into(),
            dataset_version: "v1".into(),
            business_domain: "renewal_retention".into(),
            sample_grain: "policy_order".into(),
            label_column: "m_2_keep_status".into(),
            entity_keys: vec!["policy_no".into()],
            splits: vec![ParquetSplitManifest {
                split_name: "train".into(),
                data_uri: "train.csv".into(),
            }],
        };

        let error = profile_manifest(&manifest, Path::new(".")).unwrap_err();

        assert!(error.to_string().contains("rejects csv"));
    }

    fn write_fixture_parquet(path: &Path, policy_ids: &[&str]) {
        let schema = Arc::new(Schema::new(vec![
            Field::new("policy_no", DataType::Utf8, false),
            Field::new("order_no", DataType::Utf8, false),
            Field::new("sum_premium", DataType::Float64, true),
            Field::new("m_2_keep_status", DataType::Int8, false),
        ]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(policy_ids.to_vec())),
                Arc::new(StringArray::from(vec!["O1", "O2", "O3"])),
                Arc::new(Float64Array::from(vec![Some(100.0), None, Some(300.0)])),
                Arc::new(Int8Array::from(vec![Some(1), Some(0), Some(1)])),
            ],
        )
        .unwrap();
        let file = File::create(path).unwrap();
        let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();
    }

    fn write_validation_report(
        path: &Path,
        candidate_model_version: &str,
        algorithm: &str,
        algorithm_family: &str,
        auc: f64,
        precision: f64,
        recall: f64,
        leakage_status: &str,
    ) {
        fs::write(
            path,
            serde_json::json!({
                "model_key": "baseline_fwa",
                "candidate_model_version": candidate_model_version,
                "dataset_key": "claims_model",
                "dataset_version": "2026-06-demo",
                "algorithm": algorithm,
                "validation_metrics": {
                    "auc": auc,
                    "precision": precision,
                    "recall": recall
                },
                "metrics_json": {
                    "algorithm": algorithm,
                    "algorithm_family": algorithm_family,
                    "out_of_time_auc": auc,
                    "out_of_time_average_precision": auc - 0.04,
                    "out_of_time_precision": precision,
                    "out_of_time_recall": recall,
                    "time_group_split_status": "passed",
                    "leakage_check_status": leakage_status,
                    "shadow_comparison_status": "passed",
                    "serving_version_lock_status": "passed",
                    "artifact_integrity_status": "passed",
                    "feature_store_materialization_status": "passed",
                    "rust_feature_set_status": "passed",
                    "rust_feature_set_manifest_uri": format!(
                        "s3://fwa-models/baseline_fwa/{candidate_model_version}/rust_feature_set/feature_set_manifest.json"
                    ),
                    "onnx_parity_status": if algorithm == "xgboost" || algorithm == "lightgbm" {
                        "passed"
                    } else {
                        "not_required"
                    },
                    "onnx_parity_report_uri": if algorithm == "xgboost" || algorithm == "lightgbm" {
                        format!("s3://fwa-models/baseline_fwa/{candidate_model_version}/onnx_parity_report.json")
                    } else {
                        String::new()
                    },
                    "segment_fairness_status": "passed",
                    "model_artifact_evaluation_status": "passed",
                    "label_provenance_status": "passed"
                }
            })
            .to_string(),
        )
        .unwrap();
    }

    fn write_feature_importance_parquet(path: &Path, rows: &[(&str, f64)]) {
        let schema = Arc::new(Schema::new(vec![
            Field::new("feature", DataType::Utf8, false),
            Field::new("coefficient", DataType::Float64, true),
            Field::new("importance", DataType::Float64, false),
            Field::new("importance_kind", DataType::Utf8, false),
        ]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(
                    rows.iter().map(|(feature, _)| *feature).collect::<Vec<_>>(),
                )),
                Arc::new(Float64Array::from(vec![None; rows.len()])),
                Arc::new(Float64Array::from(
                    rows.iter()
                        .map(|(_, importance)| *importance)
                        .collect::<Vec<_>>(),
                )),
                Arc::new(StringArray::from(vec!["feature_importance"; rows.len()])),
            ],
        )
        .unwrap();
        let file = File::create(path).unwrap();
        let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();
    }

    fn test_sha256(path: &Path) -> String {
        use sha2::{Digest, Sha256};

        let digest = Sha256::digest(fs::read(path).unwrap());
        format!("sha256:{digest:x}")
    }

    fn temp_root(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{stamp}"));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
