use anyhow::{bail, Context};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

use super::{
    profile_manifest, required_non_empty, write_json, FeatureSetColumn, FeatureSetManifest,
    FeatureSetSplitSummary, ParquetDatasetManifest,
};

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
