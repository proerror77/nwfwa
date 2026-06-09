use anyhow::{bail, Context};
use fwa_core::{ClaimId, ScoringRunId};
use fwa_features::{FeatureMap, FeatureValue};
use fwa_ml_runtime::{ModelScoreRequest, ModelScorer, ServingManifestModelScorer};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::{collections::BTreeMap, fs, fs::File, path::Path};

use super::{
    column_value_at, reject_csv_uri, required_non_empty, resolve_parquet_files, round4,
    safe_id_segment, write_json, ModelArtifactEvaluationReport, ModelArtifactEvaluationRow,
    ModelArtifactEvaluationSample, ParquetDatasetManifest, WorkerServingManifest,
};

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
