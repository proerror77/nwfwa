use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSplitRecord {
    pub split_name: String,
    pub data_uri: String,
    pub row_count: u64,
    pub positive_count: Option<u64>,
    pub negative_count: Option<u64>,
    pub label_distribution_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaFieldRecord {
    pub field_name: String,
    pub logical_type: String,
    pub nullable: bool,
    pub semantic_role: String,
    pub description: String,
    pub profile_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRecord {
    pub dataset_id: String,
    pub source_key: String,
    pub display_name: String,
    pub business_domain: String,
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
    pub splits: Vec<DatasetSplitRecord>,
    pub fields: Vec<SchemaFieldRecord>,
    pub mappings: Vec<FieldMappingRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterDatasetInput {
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
    pub splits: Vec<DatasetSplitRecord>,
    pub fields: Vec<SchemaFieldRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMappingRecord {
    pub mapping_id: String,
    pub dataset_id: String,
    pub external_field: String,
    pub canonical_target: String,
    pub feature_name: Option<String>,
    pub transform_kind: String,
    pub transform_json: Value,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFieldMappingInput {
    pub external_field: String,
    pub canonical_target: String,
    pub feature_name: Option<String>,
    pub transform_kind: String,
    pub transform_json: Value,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSetRecord {
    pub feature_set_id: String,
    pub business_domain: String,
    pub feature_set_key: String,
    pub version: String,
    pub dataset_id: String,
    pub features_uri: String,
    pub feature_list_json: Value,
    pub row_count: u64,
    pub label_column: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterFeatureSetInput {
    pub business_domain: String,
    pub feature_set_key: String,
    pub version: String,
    pub dataset_id: String,
    pub features_uri: String,
    pub feature_list_json: Value,
    pub row_count: u64,
    pub label_column: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDatasetRecord {
    pub model_dataset_id: String,
    pub business_domain: String,
    pub task_type: String,
    pub label_name: String,
    pub feature_set_id: String,
    pub train_uri: String,
    pub validation_uri: String,
    pub test_uri: Option<String>,
    pub row_counts_json: Value,
    pub label_distribution_json: Value,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterModelDatasetInput {
    pub business_domain: String,
    pub task_type: String,
    pub label_name: String,
    pub feature_set_id: String,
    pub train_uri: String,
    pub validation_uri: String,
    pub test_uri: Option<String>,
    pub row_counts_json: Value,
    pub label_distribution_json: Value,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEvaluationRecord {
    pub evaluation_run_id: String,
    pub model_key: String,
    pub model_version: String,
    pub model_dataset_id: String,
    pub scheme_family: String,
    pub auc: Option<Decimal>,
    pub ks: Option<Decimal>,
    pub precision: Option<Decimal>,
    pub recall: Option<Decimal>,
    pub f1: Option<Decimal>,
    pub accuracy: Option<Decimal>,
    pub threshold: Option<Decimal>,
    pub confusion_matrix_json: Value,
    pub feature_importance_uri: Option<String>,
    pub permutation_importance_uri: Option<String>,
    pub metrics_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterModelEvaluationInput {
    pub evaluation_run_id: String,
    pub model_key: String,
    pub model_version: String,
    pub model_dataset_id: String,
    pub scheme_family: String,
    pub auc: Option<Decimal>,
    pub ks: Option<Decimal>,
    pub precision: Option<Decimal>,
    pub recall: Option<Decimal>,
    pub f1: Option<Decimal>,
    pub accuracy: Option<Decimal>,
    pub threshold: Option<Decimal>,
    pub confusion_matrix_json: Value,
    pub feature_importance_uri: Option<String>,
    pub permutation_importance_uri: Option<String>,
    pub metrics_json: Value,
}
