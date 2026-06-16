use anyhow::{anyhow, bail, Context};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

use super::{
    api_url, build_mock_retraining_output, build_training_retraining_output,
    enrich_retraining_output_with_model_artifact_evaluation,
    enrich_retraining_output_with_rule_candidate_workflow, retraining_job_output_path,
    retraining_job_status_path,
};

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
pub(crate) struct ClaimRetrainingJobPayload<'a> {
    actor: &'a str,
    notes: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_key: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub(crate) struct UpdateRetrainingJobStatusPayload<'a> {
    status: &'a str,
    actor: &'a str,
    notes: &'a str,
}

#[derive(Debug, Serialize)]
pub(crate) struct ModelLifecyclePayload {
    evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PromoteApprovedModelVersionResult {
    pub model_key: String,
    pub model_version: String,
    pub status: String,
    pub promotion_status: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ModelPromotionGatesResponse {
    model_key: String,
    model_version: String,
    gates: Vec<ModelPromotionGate>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ModelPromotionGate {
    label: String,
    passed: bool,
    blocker: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ModelLifecycleResponse {
    model_key: String,
    version: String,
    status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct CompleteRetrainingJobPayload {
    pub(crate) actor: String,
    pub(crate) notes: String,
    pub(crate) candidate_model_version: String,
    pub(crate) artifact_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) artifact_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) training_artifact_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) training_artifact_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) serving_manifest_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) onnx_parity_report_uri: Option<String>,
    pub(crate) endpoint_url: Option<String>,
    pub(crate) validation_report_uri: String,
    pub(crate) evaluation_run_id: String,
    pub(crate) auc: Option<String>,
    pub(crate) ks: Option<String>,
    pub(crate) precision: Option<String>,
    pub(crate) recall: Option<String>,
    pub(crate) f1: Option<String>,
    pub(crate) accuracy: Option<String>,
    pub(crate) threshold: Option<String>,
    pub(crate) confusion_matrix_json: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) feature_importance_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) permutation_importance_uri: Option<String>,
    pub(crate) metrics_json: serde_json::Value,
    pub(crate) evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) mined_rule_owner: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) mined_rule_candidates: Vec<serde_json::Value>,
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
    pub score_psi: Option<f64>,
    pub max_feature_psi: Option<f64>,
    pub overfitting_penalty: f64,
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
pub(crate) struct WorkerServingManifest {
    pub(crate) model_key: String,
    pub(crate) model_version: String,
    pub(crate) runtime_kind: String,
    pub(crate) artifact_uri: String,
    pub(crate) artifact_sha256: String,
    pub(crate) version_lock: String,
    pub(crate) feature_columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModelArtifactEvaluationRow {
    pub(crate) claim_id: String,
    pub(crate) features: BTreeMap<String, f64>,
    pub(crate) expected_probability: Option<f64>,
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
pub(crate) struct FeatureImportanceRow {
    pub(crate) feature: String,
    pub(crate) importance: f64,
    pub(crate) importance_kind: String,
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
    pub rule_library_writeback_template: serde_json::Value,
    pub condition_refs: Vec<String>,
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
pub(crate) struct RuleBacktestRow {
    pub(crate) split_name: String,
    pub(crate) label: bool,
    pub(crate) features: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TrainingCommand {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) workdir: Option<PathBuf>,
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
    let result = register_retraining_output(api_base_url, api_key, job, &output).await?;
    submit_mock_candidate_probability_calibration(api_base_url, api_key, job, actor, &output)
        .await?;
    Ok(result)
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

pub async fn promote_approved_model_version(
    api_base_url: &str,
    api_key: &str,
    model_key: &str,
    model_version: &str,
) -> anyhow::Result<PromoteApprovedModelVersionResult> {
    let gates_path =
        format!("/api/v1/ops/models/{model_key}/versions/{model_version}/promotion-gates");
    let gates = get_model_promotion_gates(api_base_url, api_key, &gates_path).await?;
    if !promotion_gate_passed(&gates, "Approval") {
        bail!("model {model_key}:{model_version} promotion gates blocked: approval missing");
    }
    let blockers = gates
        .gates
        .iter()
        .filter(|gate| gate.label != "Active version" && !gate.passed)
        .map(|gate| gate.blocker.clone())
        .collect::<Vec<_>>();
    if !blockers.is_empty() {
        bail!(
            "model {}:{} promotion gates blocked: {}",
            gates.model_key,
            gates.model_version,
            blockers.join(", ")
        );
    }

    let evidence_refs = vec![format!("model_versions:{model_key}:{model_version}")];
    let activate_path = format!("/api/v1/ops/models/{model_key}/versions/{model_version}/activate");
    let activated =
        activate_approved_model_version(api_base_url, api_key, &activate_path, &evidence_refs)
            .await?;
    Ok(PromoteApprovedModelVersionResult {
        model_key: activated.model_key,
        model_version: activated.version,
        status: activated.status,
        promotion_status: "activated_after_reviewer_approval".into(),
        evidence_refs,
    })
}

fn promotion_gate_passed(gates: &ModelPromotionGatesResponse, label: &str) -> bool {
    gates
        .gates
        .iter()
        .any(|gate| gate.label == label && gate.passed)
}

async fn get_model_promotion_gates(
    api_base_url: &str,
    api_key: &str,
    path: &str,
) -> anyhow::Result<ModelPromotionGatesResponse> {
    let response = reqwest::Client::new()
        .get(api_url(api_base_url, path))
        .header("x-api-key", api_key)
        .send()
        .await
        .with_context(|| format!("load model promotion gates from {path}"))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("load model promotion gates from {path} failed with {status}: {body}");
    }
    response
        .json::<ModelPromotionGatesResponse>()
        .await
        .context("parse model promotion gates response")
}

async fn activate_approved_model_version(
    api_base_url: &str,
    api_key: &str,
    path: &str,
    evidence_refs: &[String],
) -> anyhow::Result<ModelLifecycleResponse> {
    let response = reqwest::Client::new()
        .post(api_url(api_base_url, path))
        .header("x-api-key", api_key)
        .json(&ModelLifecyclePayload {
            evidence_refs: evidence_refs.to_vec(),
        })
        .send()
        .await
        .with_context(|| format!("activate approved model version through {path}"))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("activate approved model version through {path} failed with {status}: {body}");
    }
    response
        .json::<ModelLifecycleResponse>()
        .await
        .context("parse model activation response")
}

pub async fn complete_retraining_job_with_training_output(
    api_base_url: &str,
    api_key: &str,
    job: &ClaimedRetrainingJob,
    actor: &str,
    artifact_base_uri: &str,
    training_manifest: &str,
    trainer_python: &str,
    trainer_workdir: Option<&str>,
    algorithm: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let output = build_training_retraining_output(
        job,
        actor,
        artifact_base_uri,
        training_manifest,
        trainer_python,
        trainer_workdir,
        algorithm,
    )?;
    let output =
        enrich_retraining_output_with_model_artifact_evaluation(output, training_manifest).await?;
    let output = enrich_retraining_output_with_rule_candidate_workflow(output, training_manifest)?;
    register_retraining_output(api_base_url, api_key, job, &output).await
}

async fn submit_mock_candidate_probability_calibration(
    api_base_url: &str,
    api_key: &str,
    job: &ClaimedRetrainingJob,
    actor: &str,
    output: &CompleteRetrainingJobPayload,
) -> anyhow::Result<()> {
    let artifact_dir = output
        .validation_report_uri
        .strip_suffix("/validation.json")
        .map(str::to_string)
        .or_else(|| {
            output
                .artifact_uri
                .rsplit_once('/')
                .map(|(artifact_dir, _)| artifact_dir.to_string())
        })
        .ok_or_else(|| {
            anyhow!("candidate artifact directory is required for calibration report")
        })?;
    let report_uri = format!("{artifact_dir}/calibration/probability_calibration_report.json");
    let input_uri = format!("{artifact_dir}/calibration/holdout_predictions.json");
    let label_uri = format!("{artifact_dir}/calibration/holdout_labels.json");
    let payload = serde_json::json!({
        "actor": actor,
        "notes": "Candidate holdout probability calibration evidence registered by retraining worker.",
        "report_uri": report_uri,
        "report_kind": "probability_calibration_report",
        "model_version": output.candidate_model_version,
        "as_of_date": "2026-06-14",
        "row_count": 1000,
        "minimum_calibration_rows": 100,
        "bin_count": 2,
        "expected_calibration_error": 0.03,
        "max_expected_calibration_error": 0.05,
        "brier_score": 0.12,
        "max_brier_score": 0.20,
        "calibration_status": "passed",
        "bins": [
            {
                "bin_index": 0,
                "lower_bound": 0.0,
                "upper_bound": 0.5,
                "row_count": 680,
                "average_predicted_probability": 0.18,
                "observed_positive_rate": 0.16,
                "calibration_error": 0.02
            },
            {
                "bin_index": 1,
                "lower_bound": 0.5,
                "upper_bound": 1.0,
                "row_count": 320,
                "average_predicted_probability": 0.74,
                "observed_positive_rate": 0.70,
                "calibration_error": 0.04
            }
        ],
        "review_tasks": [],
        "evidence_refs": [
            format!("model_versions:{}:{}", job.model_key, output.candidate_model_version),
            format!("model_retraining_jobs:{}", job.job_id),
            format!("model_evaluations:{}", output.evaluation_run_id),
            format!("probability_calibration_reports:{report_uri}"),
            format!("probability_calibration_input:{input_uri}"),
            format!("calibration_labels:{label_uri}")
        ],
        "governance_boundary": "candidate probability calibration submission records model-governance evidence only; it must not activate calibrated serving, change thresholds, or assign labels"
    });
    let response = reqwest::Client::new()
        .post(api_url(
            api_base_url,
            &format!(
                "/api/v1/ops/models/{}/probability-calibration-reports",
                job.model_key
            ),
        ))
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .with_context(|| {
            format!(
                "submit probability calibration report for {}:{}",
                job.model_key, output.candidate_model_version
            )
        })?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!(
            "submit probability calibration report for {}:{} failed with {status}: {body}",
            job.model_key,
            output.candidate_model_version
        );
    }
    Ok(())
}

pub async fn run_one_retraining_job(
    api_base_url: &str,
    api_key: &str,
    actor: &str,
    model_key: Option<&str>,
    artifact_base_uri: &str,
    training_manifest: Option<&str>,
    trainer_python: &str,
    trainer_workdir: Option<&str>,
    algorithm: Option<&str>,
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
    let completion_result = if let Some(training_manifest) = training_manifest {
        complete_retraining_job_with_training_output(
            api_base_url,
            api_key,
            &validation_job,
            actor,
            artifact_base_uri,
            training_manifest,
            trainer_python,
            trainer_workdir,
            algorithm,
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
    };
    match completion_result {
        Ok(result) => Ok(result),
        Err(error) => {
            mark_retraining_job_failed_after_completion_error(
                api_base_url,
                api_key,
                &validation_job.job_id,
                actor,
                error,
            )
            .await
        }
    }
}

async fn mark_retraining_job_failed_after_completion_error(
    api_base_url: &str,
    api_key: &str,
    job_id: &str,
    actor: &str,
    error: anyhow::Error,
) -> anyhow::Result<serde_json::Value> {
    let error_message = error.to_string();
    let failure_note = format!(
        "Retraining job failed before output registration: {}",
        truncate_for_status_note(&error_message, 500)
    );
    match update_retraining_job_status(api_base_url, api_key, job_id, "failed", actor, &failure_note)
        .await
    {
        Ok(_) => Err(anyhow!(
            "model retraining job {job_id} failed and was marked failed: {error_message}"
        )),
        Err(status_error) => Err(anyhow!(
            "model retraining job {job_id} failed before output registration: {error_message}; failed status update also failed: {status_error}"
        )),
    }
}

fn truncate_for_status_note(value: &str, max_chars: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    let mut truncated = normalized.chars().take(max_chars).collect::<String>();
    truncated.push_str("...");
    truncated
}
