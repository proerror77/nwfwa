use anyhow::{anyhow, bail, Context};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
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
                name: "retraining_job_runner",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "pilot_readiness_checker",
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
        .map(|check| format!("{}={}", check.name, check.status))
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
    use arrow_array::{Float64Array, Int8Array, RecordBatch, StringArray};
    use arrow_schema::{DataType, Field, Schema};
    use parquet::arrow::ArrowWriter;
    use std::{
        fs::File,
        sync::Arc,
        time::{SystemTime, UNIX_EPOCH},
    };

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
                }],
                blocking_checks: vec![ApiHealthCheck {
                    name: "object_storage_configuration".into(),
                    status: "local_demo_object_storage".into(),
                    runtime_kind: None,
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
            vec!["object_storage_configuration=local_demo_object_storage"]
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
