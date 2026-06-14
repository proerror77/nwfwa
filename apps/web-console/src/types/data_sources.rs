use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DatasetListResponse {
    pub(crate) datasets: Vec<DatasetRecord>,
    pub(crate) health: Vec<DatasetHealthRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DatasetRecord {
    pub(crate) dataset_id: String,
    pub(crate) source_key: String,
    pub(crate) display_name: String,
    pub(crate) business_domain: String,
    pub(crate) dataset_key: String,
    pub(crate) dataset_version: String,
    pub(crate) sample_grain: String,
    pub(crate) label_column: String,
    pub(crate) entity_keys: Vec<String>,
    pub(crate) manifest_uri: String,
    pub(crate) schema_uri: String,
    pub(crate) profile_uri: String,
    pub(crate) storage_format: String,
    pub(crate) schema_hash: String,
    pub(crate) row_count: u64,
    pub(crate) status: String,
    pub(crate) splits: Vec<DatasetSplitRecord>,
    pub(crate) fields: Vec<SchemaFieldRecord>,
    pub(crate) mappings: Vec<FieldMappingRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DatasetSplitRecord {
    pub(crate) split_name: String,
    pub(crate) data_uri: String,
    pub(crate) row_count: u64,
    pub(crate) positive_count: Option<u64>,
    pub(crate) negative_count: Option<u64>,
    pub(crate) label_distribution_json: Value,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct SchemaFieldRecord {
    pub(crate) field_name: String,
    pub(crate) logical_type: String,
    pub(crate) nullable: bool,
    pub(crate) semantic_role: String,
    pub(crate) description: String,
    pub(crate) profile_json: Value,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct FieldMappingRecord {
    pub(crate) mapping_id: String,
    pub(crate) dataset_id: String,
    pub(crate) external_field: String,
    pub(crate) canonical_target: String,
    pub(crate) feature_name: Option<String>,
    pub(crate) transform_kind: String,
    pub(crate) transform_json: Value,
    pub(crate) status: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DatasetHealthRecord {
    pub(crate) dataset_id: String,
    pub(crate) dataset_key: String,
    pub(crate) dataset_version: String,
    pub(crate) data_quality_score: f64,
    pub(crate) data_quality_status: String,
    pub(crate) field_count: u32,
    pub(crate) label_count: u32,
    pub(crate) entity_key_count: u32,
    pub(crate) high_missing_count: u32,
    pub(crate) unstable_field_count: u32,
    pub(crate) unowned_field_count: u32,
    pub(crate) online_ready_count: u32,
    pub(crate) issue_count: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ModelEvaluationListResponse {
    pub(crate) evaluations: Vec<ModelEvaluationRecord>,
    pub(crate) lineage: Vec<ModelEvaluationLineageRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ModelEvaluationRecord {
    pub(crate) evaluation_run_id: String,
    pub(crate) model_key: String,
    pub(crate) model_version: String,
    pub(crate) model_dataset_id: String,
    pub(crate) scheme_family: String,
    pub(crate) auc: Option<Value>,
    pub(crate) ks: Option<Value>,
    pub(crate) precision: Option<Value>,
    pub(crate) recall: Option<Value>,
    pub(crate) f1: Option<Value>,
    pub(crate) accuracy: Option<Value>,
    pub(crate) threshold: Option<Value>,
    pub(crate) confusion_matrix_json: Value,
    pub(crate) feature_importance_uri: Option<String>,
    pub(crate) permutation_importance_uri: Option<String>,
    pub(crate) metrics_json: Value,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ModelEvaluationLineageRecord {
    pub(crate) evaluation_run_id: String,
    pub(crate) model_key: String,
    pub(crate) model_version: String,
    pub(crate) model_dataset_id: String,
    pub(crate) source_dataset_id: Option<String>,
    pub(crate) source_dataset_key: Option<String>,
    pub(crate) source_dataset_version: Option<String>,
    pub(crate) source_data_quality_score: Option<f64>,
    pub(crate) source_data_quality_status: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DataSourcesSnapshot {
    pub(crate) datasets: Vec<DatasetRecord>,
    pub(crate) health: Vec<DatasetHealthRecord>,
    pub(crate) evaluations: Vec<ModelEvaluationRecord>,
    pub(crate) lineage: Vec<ModelEvaluationLineageRecord>,
}
