use crate::repository::{
    ClinicalCompatibilityReferenceRecord, ClinicalCompatibilityReferenceUpsertInput, DatasetRecord,
    FieldMappingRecord, ModelEvaluationRecord, ScoringFeatureContextMaterializationRecord,
    UnbundlingComparatorCandidateRecord, UnbundlingComparatorCandidateUpsertInput,
    WorkerDataPipelineExecutionReportRecord, WorkerDataPipelineReadinessReportRecord,
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

#[derive(Debug, Deserialize)]
pub struct SubmitClinicalCompatibilityReferenceRequest {
    pub actor: String,
    pub notes: String,
    pub source_report_uri: String,
    pub report_kind: String,
    pub reference_version: String,
    pub effective_date: String,
    pub source_authority: String,
    pub source_uri: String,
    pub record_count: usize,
    #[serde(default)]
    pub records: Vec<ClinicalCompatibilityReferenceUpsertInput>,
    #[serde(default)]
    pub review_tasks: Vec<Value>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

#[derive(Debug, Serialize)]
pub struct ClinicalCompatibilityReferenceSubmissionResponse {
    pub report_kind: String,
    pub source_report_uri: String,
    pub reference_version: String,
    pub record_count: usize,
    pub review_task_count: usize,
    pub persisted_records: Vec<ClinicalCompatibilityReferenceRecord>,
    pub active_scoring_policy_change: bool,
    pub claim_scoring: bool,
    pub label_assignment: bool,
    pub claim_denial: bool,
    pub medical_review_replacement: bool,
    pub governance_boundary: String,
    pub audit_event_type: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitUnbundlingComparatorCandidatesRequest {
    pub actor: String,
    pub notes: String,
    pub source_report_uri: String,
    pub report_kind: String,
    pub as_of_date: String,
    pub source_uri: String,
    pub rule_count: usize,
    pub episode_count: usize,
    pub candidate_count: usize,
    #[serde(default)]
    pub candidates: Vec<UnbundlingComparatorCandidateUpsertInput>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

#[derive(Debug, Serialize)]
pub struct UnbundlingComparatorCandidatesSubmissionResponse {
    pub report_kind: String,
    pub source_report_uri: String,
    pub as_of_date: String,
    pub rule_count: usize,
    pub episode_count: usize,
    pub candidate_count: usize,
    pub persisted_candidates: Vec<UnbundlingComparatorCandidateRecord>,
    pub active_scoring_policy_change: bool,
    pub claim_scoring: bool,
    pub label_assignment: bool,
    pub claim_denial: bool,
    pub case_creation: bool,
    pub medical_review_replacement: bool,
    pub governance_boundary: String,
    pub audit_event_type: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitWorkerDataPipelineExecutionReportRequest {
    pub actor: String,
    pub notes: String,
    pub source_report_uri: String,
    pub report_kind: String,
    pub plan_uri: String,
    pub run_status_uri: String,
    pub readiness_report_uri: Option<String>,
    pub readiness_gate_status: Option<String>,
    pub run_id: String,
    pub execution_date: String,
    pub job_count: usize,
    pub pending_or_failed_job_count: usize,
    pub review_task_count: usize,
    #[serde(default)]
    pub job_executions: Vec<Value>,
    #[serde(default)]
    pub review_tasks: Vec<Value>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

#[derive(Debug, Serialize)]
pub struct WorkerDataPipelineExecutionReportSubmissionResponse {
    pub report_kind: String,
    pub source_report_uri: String,
    pub readiness_report_uri: Option<String>,
    pub readiness_gate_status: Option<String>,
    pub run_id: String,
    pub execution_date: String,
    pub job_count: usize,
    pub pending_or_failed_job_count: usize,
    pub review_task_count: usize,
    pub active_scoring_policy_change: bool,
    pub claim_scoring: bool,
    pub label_assignment: bool,
    pub claim_denial: bool,
    pub model_activation: bool,
    pub routing_policy_change: bool,
    pub persisted_report: WorkerDataPipelineExecutionReportRecord,
    pub governance_boundary: String,
    pub audit_event_type: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitWorkerDataPipelineReadinessReportRequest {
    pub actor: String,
    pub notes: String,
    pub source_report_uri: String,
    pub report_kind: String,
    pub plan_uri: String,
    pub readiness_input_uri: String,
    pub readiness_status: String,
    pub job_count: usize,
    pub ready_job_count: usize,
    pub blocked_job_count: usize,
    pub review_task_count: usize,
    #[serde(default)]
    pub job_readiness: Vec<Value>,
    #[serde(default)]
    pub review_tasks: Vec<Value>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

#[derive(Debug, Serialize)]
pub struct WorkerDataPipelineReadinessReportSubmissionResponse {
    pub report_kind: String,
    pub source_report_uri: String,
    pub readiness_status: String,
    pub job_count: usize,
    pub ready_job_count: usize,
    pub blocked_job_count: usize,
    pub review_task_count: usize,
    pub active_scoring_policy_change: bool,
    pub claim_scoring: bool,
    pub label_assignment: bool,
    pub claim_denial: bool,
    pub model_activation: bool,
    pub routing_policy_change: bool,
    pub external_fetch_execution: bool,
    pub artifact_submission: bool,
    pub persisted_report: WorkerDataPipelineReadinessReportRecord,
    pub governance_boundary: String,
    pub audit_event_type: String,
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
