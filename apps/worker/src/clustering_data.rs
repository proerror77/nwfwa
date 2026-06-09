use anyhow::{bail, Context};
use arrow_array::RecordBatch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde::Deserialize;
use std::{collections::BTreeMap, fs::File, path::Path};

use super::{column_value_at, reject_csv_uri, resolve_parquet_files, ParquetSplitManifest};

#[derive(Debug, Clone, Deserialize)]
pub(super) struct UnlabeledDatasetManifest {
    pub(super) dataset_key: String,
    pub(super) dataset_version: String,
    pub(super) label_policy: String,
    #[serde(default)]
    pub(super) label_column: Option<String>,
    pub(super) splits: Vec<ParquetSplitManifest>,
}

#[derive(Debug, Clone)]
pub(super) struct ProviderPeerFeatureRow {
    pub(super) provider_id: String,
    pub(super) cohort_key: String,
    pub(super) service_month: String,
    pub(super) claim_count: f64,
    pub(super) avg_claim_amount: f64,
    pub(super) high_cost_rate: f64,
    pub(super) peer_z_score: f64,
    pub(super) graph_degree: f64,
    pub(super) community_id: i32,
}

#[derive(Debug, Clone)]
pub(super) struct ClaimEntityFeatureRow {
    pub(super) claim_id: String,
    pub(super) member_id: String,
    pub(super) provider_id: String,
    pub(super) claim_amount: f64,
    pub(super) amount_to_limit_ratio: f64,
    pub(super) peer_percentile: f64,
    pub(super) item_count: f64,
    pub(super) high_cost_item_ratio: f64,
    pub(super) provider_risk_tier: f64,
    pub(super) diagnosis_procedure_mismatch: f64,
    pub(super) member_degree: f64,
    pub(super) provider_degree: f64,
}

pub(super) fn read_provider_peer_rows(
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

pub(super) fn read_claim_entity_rows(
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
