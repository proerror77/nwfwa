use super::ops_rules_mining_samples::MiningSample;
use anyhow::{bail, Context};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::{collections::BTreeMap, fs::File, path::PathBuf};

pub(super) fn read_parquet_mining_samples(
    dataset_uri: &str,
    label_column: Option<&str>,
    claim_id_column: Option<&str>,
    candidate_feature_fields: Option<&[String]>,
) -> anyhow::Result<Vec<MiningSample>> {
    let label_column = label_column.unwrap_or("confirmed_fwa");
    let claim_id_column = claim_id_column.unwrap_or("claim_id");
    let dataset_path = resolve_dataset_path(dataset_uri)?;
    let file =
        File::open(&dataset_path).with_context(|| format!("open {}", dataset_path.display()))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .with_context(|| format!("read parquet metadata {}", dataset_path.display()))?;
    let mut reader = builder.with_batch_size(4096).build()?;
    let mut samples = Vec::new();
    for batch in &mut reader {
        let batch = batch?;
        let schema = batch.schema();
        let label_index = schema
            .index_of(label_column)
            .with_context(|| format!("label column {label_column} not found"))?;
        let claim_id_index = schema
            .index_of(claim_id_column)
            .with_context(|| format!("claim id column {claim_id_column} not found"))?;
        let claim_amount_index = schema.index_of("claim_amount").ok();

        for row_index in 0..batch.num_rows() {
            let confirmed_fwa = bool_value_at(batch.column(label_index).as_ref(), row_index);
            let Some(confirmed_fwa) = confirmed_fwa else {
                continue;
            };
            let claim_id = string_value_at(batch.column(claim_id_index).as_ref(), row_index)
                .unwrap_or_else(|| format!("row-{row_index}"));
            let claim_amount = claim_amount_index
                .and_then(|index| numeric_value_at(batch.column(index).as_ref(), row_index))
                .and_then(Decimal::from_f64)
                .unwrap_or(Decimal::ZERO);
            let mut features = BTreeMap::new();
            for (column_index, field) in schema.fields().iter().enumerate() {
                let feature = field.name();
                if !is_candidate_feature(
                    feature,
                    label_column,
                    claim_id_column,
                    candidate_feature_fields,
                ) {
                    continue;
                }
                if let Some(value) =
                    numeric_value_at(batch.column(column_index).as_ref(), row_index)
                {
                    if value.is_finite() {
                        features.insert(feature.clone(), value);
                    }
                }
            }
            samples.push(MiningSample {
                claim_id,
                claim_amount,
                confirmed_fwa: Some(confirmed_fwa),
                features,
            });
        }
    }
    Ok(samples)
}

fn resolve_dataset_path(dataset_uri: &str) -> anyhow::Result<PathBuf> {
    if dataset_uri.starts_with("http://")
        || dataset_uri.starts_with("https://")
        || dataset_uri.starts_with("s3://")
    {
        bail!("only local parquet dataset_uri values are supported by rule discovery");
    }
    let path = PathBuf::from(dataset_uri);
    let path = if path.is_absolute() {
        path
    } else {
        let current_dir = std::env::current_dir()?;
        let mut candidate = current_dir.join(&path);
        if !candidate.exists() {
            for ancestor in current_dir.ancestors() {
                let ancestor_candidate = ancestor.join(&path);
                if ancestor_candidate.exists() {
                    candidate = ancestor_candidate;
                    break;
                }
            }
        }
        candidate
    };
    if path.extension().and_then(|value| value.to_str()) != Some("parquet") {
        bail!("dataset_uri must point to a parquet file");
    }
    if !path.exists() {
        bail!("dataset_uri not found: {}", path.display());
    }
    Ok(path)
}

fn is_candidate_feature(
    feature: &str,
    label_column: &str,
    claim_id_column: &str,
    candidate_feature_fields: Option<&[String]>,
) -> bool {
    if let Some(candidate_feature_fields) = candidate_feature_fields {
        if candidate_feature_fields.is_empty() {
            return feature != label_column
                && feature != claim_id_column
                && feature != "split"
                && feature != "service_date"
                && feature != "service_date_ord"
                && !feature.ends_with("_id");
        }
        return candidate_feature_fields
            .iter()
            .any(|candidate_feature| candidate_feature == feature);
    }
    feature != label_column
        && feature != claim_id_column
        && feature != "split"
        && feature != "service_date"
        && feature != "service_date_ord"
        && !feature.ends_with("_id")
}

fn numeric_value_at(array: &dyn arrow_array::Array, index: usize) -> Option<f64> {
    use arrow_array::{
        Float32Array, Float64Array, Int16Array, Int32Array, Int64Array, Int8Array, UInt16Array,
        UInt32Array, UInt64Array, UInt8Array,
    };
    if array.is_null(index) {
        return None;
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        return Some(values.value(index));
    }
    if let Some(values) = array.as_any().downcast_ref::<Float32Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<Int8Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<Int16Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<Int32Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt8Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt16Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt32Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt64Array>() {
        return Some(values.value(index) as f64);
    }
    None
}

fn bool_value_at(array: &dyn arrow_array::Array, index: usize) -> Option<bool> {
    use arrow_array::{BooleanArray, Float64Array, Int64Array, Int8Array, StringArray};
    if array.is_null(index) {
        return None;
    }
    if let Some(values) = array.as_any().downcast_ref::<BooleanArray>() {
        return Some(values.value(index));
    }
    if let Some(values) = array.as_any().downcast_ref::<Int8Array>() {
        return Some(values.value(index) != 0);
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return Some(values.value(index) != 0);
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        return Some(values.value(index) != 0.0);
    }
    if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        return match values.value(index).to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        };
    }
    None
}

fn string_value_at(array: &dyn arrow_array::Array, index: usize) -> Option<String> {
    use arrow_array::{LargeStringArray, StringArray};
    if array.is_null(index) {
        return None;
    }
    if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        return Some(values.value(index).into());
    }
    if let Some(values) = array.as_any().downcast_ref::<LargeStringArray>() {
        return Some(values.value(index).into());
    }
    numeric_value_at(array, index).map(format_threshold)
}

fn format_threshold(value: f64) -> String {
    format!("{:.4}", round_float(value))
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn round_float(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}
