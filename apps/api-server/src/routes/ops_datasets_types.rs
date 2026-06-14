use crate::repository::{
    DatasetRecord, FieldMappingRecord, ModelEvaluationRecord,
    ScoringFeatureContextMaterializationRecord,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Serialize)]
pub struct DatasetListResponse {
    pub datasets: Vec<DatasetRecord>,
    pub health: Vec<DatasetHealthRecord>,
}

#[derive(Debug, Serialize)]
pub struct DatasetHealthRecord {
    pub dataset_id: String,
    pub dataset_key: String,
    pub dataset_version: String,
    pub data_quality_score: f64,
    pub data_quality_status: String,
    pub field_count: u32,
    pub label_count: u32,
    pub entity_key_count: u32,
    pub high_missing_count: u32,
    pub unstable_field_count: u32,
    pub unowned_field_count: u32,
    pub online_ready_count: u32,
    pub issue_count: u32,
}

#[derive(Debug, Serialize)]
pub struct FieldMappingResponse {
    pub mapping: FieldMappingRecord,
}

#[derive(Debug, Serialize)]
pub struct ModelEvaluationResponse {
    pub evaluation: ModelEvaluationRecord,
}

#[derive(Debug, Serialize)]
pub struct ModelEvaluationListResponse {
    pub evaluations: Vec<ModelEvaluationRecord>,
    pub lineage: Vec<ModelEvaluationLineageRecord>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitScoringFeatureContextMaterializationRequest {
    pub materialization_id: String,
    pub actor: String,
    pub notes: String,
    pub report_uri: String,
    pub report_kind: String,
    pub as_of_date: String,
    pub source_uris: Value,
    pub claim_count: u64,
    pub context_count: u64,
    pub contexts: Vec<Value>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

#[derive(Debug, Serialize)]
pub struct ScoringFeatureContextMaterializationResponse {
    pub materialization: ScoringFeatureContextMaterializationRecord,
}

#[derive(Debug, Serialize)]
pub struct ModelEvaluationLineageRecord {
    pub evaluation_run_id: String,
    pub model_key: String,
    pub model_version: String,
    pub model_dataset_id: String,
    pub source_dataset_id: Option<String>,
    pub source_dataset_key: Option<String>,
    pub source_dataset_version: Option<String>,
    pub source_data_quality_score: Option<f64>,
    pub source_data_quality_status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FactorReadinessResponse {
    pub dataset_count: u32,
    pub factor_count: u32,
    pub label_count: u32,
    pub entity_key_count: u32,
    pub data_quality_score: f64,
    pub data_quality_status: String,
    pub online_ready_count: u32,
    pub rule_convertible_count: u32,
    pub mapped_factor_count: u32,
    pub high_missing_count: u32,
    pub unstable_factor_count: u32,
    pub unowned_factor_count: u32,
    pub ready_factor_count: u32,
    pub review_factor_count: u32,
    pub readiness_issue_counts: Map<String, Value>,
    pub scheme_readiness: Vec<FactorSchemeReadinessRecord>,
    pub factor_cards: Vec<FactorCardRecord>,
}

#[derive(Debug, Serialize)]
pub struct FactorSchemeReadinessRecord {
    pub scheme_family: String,
    pub factor_count: u32,
    pub ready_factor_count: u32,
    pub review_factor_count: u32,
    pub online_ready_count: u32,
    pub rule_convertible_count: u32,
    pub readiness_issue_counts: Map<String, Value>,
}

#[derive(Debug, Serialize)]
pub struct FactorCardRecord {
    pub dataset_id: String,
    pub dataset_key: String,
    pub dataset_version: String,
    pub factor_name: String,
    pub scheme_family: String,
    pub chinese_name: String,
    pub entity_type: String,
    pub semantic_role: String,
    pub logical_type: String,
    pub calculation_window: String,
    pub calculation_logic: String,
    pub source_table: String,
    pub source_fields: Vec<String>,
    pub business_meaning: String,
    pub risk_direction: String,
    pub missing_rate: Option<f64>,
    pub iv: Option<f64>,
    pub auc_gain: Option<f64>,
    pub lift: Option<f64>,
    pub psi: Option<f64>,
    pub stability: String,
    pub model_contribution: Option<f64>,
    pub rule_convertible: bool,
    pub online_available: bool,
    pub readiness_status: String,
    pub readiness_issues: Vec<String>,
    pub version: String,
    pub owner: String,
    pub is_label: bool,
    pub is_entity_key: bool,
    pub evidence_refs: Vec<String>,
}
