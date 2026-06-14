use anyhow::{bail, Context};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    path::Path,
};

use super::{
    column_values, reject_csv_uri, resolve_parquet_files, DatasetCatalogField,
    DatasetCatalogOutput, DatasetCatalogSplit, DatasetProfileOutput, DatasetSchemaOutput,
    FieldProfileOutput, FieldSchemaOutput, ParquetDatasetManifest, ProfileResult, ValueCount,
};

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
