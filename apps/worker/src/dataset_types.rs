use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    pub is_proxy: bool,
    pub data_source: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FeatureSetSplitSummary {
    pub split_name: String,
    pub row_count: u64,
    pub positive_count: Option<u64>,
    pub negative_count: Option<u64>,
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
