use anyhow::{anyhow, bail, Context};
use arrow_array::{Float64Array, Int32Array, Int8Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::{arrow_reader::ParquetRecordBatchReaderBuilder, ArrowWriter};
use serde::{Deserialize, Serialize};
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
                name: "demo_ml_dataset_builder",
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
            "allowed_uses": ["shadow_scoring", "drift_monitoring_demo", "score_distribution_demo"],
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
                "cargo run --locked -p worker -- build-training-handoff --manifest {} --artifact-base-uri s3://fwa-models --model-key baseline_fwa --base-model-version 0.1.0 --job-id model_retraining_job_1 --actor trainer-worker",
                labeled_dir.join("manifest.json").display()
            ),
        ],
    };
    write_json(output_dir.join("index.json"), &pack)?;
    Ok(pack)
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
    if artifact_base_uri.trim().is_empty() {
        bail!("artifact_base_uri is required");
    }
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

    let candidate_model_version = format!(
        "{}-candidate-{}",
        safe_path_segment(base_model_version),
        safe_path_segment(job_id)
    );
    let artifact_root = artifact_base_uri.trim().trim_end_matches('/');
    let safe_model_key = safe_path_segment(model_key);
    let artifact_dir = format!("{artifact_root}/{safe_model_key}/{candidate_model_version}");

    Ok(serde_json::json!({
        "handoff_kind": "external_training_platform",
        "handoff_version": 1,
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
            "actor": actor
        },
        "artifact_contract": {
            "artifact_dir": artifact_dir,
            "rust_serving_artifact_uri": format!("{artifact_dir}/rust_serving_artifact.json"),
            "training_artifact_uri": format!("{artifact_dir}/model.joblib"),
            "serving_manifest_uri": format!("{artifact_dir}/serving_manifest.json"),
            "validation_report_uri": format!("{artifact_dir}/validation.json"),
            "feature_store_manifest_uri": format!("{artifact_dir}/feature_store_manifest.json"),
            "shadow_report_uri": format!("{artifact_dir}/shadow_report.json"),
            "drift_report_uri": format!("{artifact_dir}/drift_report.json"),
            "fairness_report_uri": format!("{artifact_dir}/fairness_report.json")
        },
        "output_contract": {
            "submit_path": retraining_job_output_path(job_id),
            "artifact_uri": "artifact_contract.rust_serving_artifact_uri",
            "required_evidence_refs": [
                "model_retraining_jobs:<job_id>",
                "model_artifacts:<rust_serving_artifact_uri>",
                "model_validation_reports:<validation_report_uri>",
                "model_evaluations:<evaluation_run_id>"
            ]
        }
    }))
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
    serde_json::from_slice::<CompleteRetrainingJobPayload>(&output.stdout)
        .context("parse model training output")
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
            name: "demo_ml_dataset_builder",
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
        assert_eq!(handoff["dataset"]["dataset_key"], "claims_model");
        assert_eq!(handoff["dataset"]["dataset_version"], "2026-06-02");
        assert_eq!(
            handoff["dataset"]["manifest_uri"],
            serde_json::json!(manifest_path.to_string_lossy())
        );
        assert_eq!(handoff["training_job"]["model_key"], "baseline_fwa");
        assert_eq!(
            handoff["training_job"]["candidate_model_version"],
            "0.1.0-candidate-model_retraining_job_1"
        );
        assert_eq!(
            handoff["artifact_contract"]["rust_serving_artifact_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/rust_serving_artifact.json"
        );
        assert_eq!(
            handoff["output_contract"]["submit_path"],
            "/api/v1/ops/model-retraining-jobs/model_retraining_job_1/output"
        );
        assert_eq!(
            handoff["data_contract"]["source"],
            "same_parquet_dataset_manifest"
        );
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
