use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use super::types_cases::*;
pub use super::types_core::*;
pub use super::types_dashboard::*;
pub use super::types_evidence::*;
pub use super::types_knowledge::*;
pub use super::types_models::*;
pub use super::types_rules::*;

pub(super) const GOVERNANCE_AUDIT_EVENT_TYPES: &[&str] = &[
    "dataset.registered",
    "dataset.field_mapping.added",
    "feature_set.registered",
    "model_dataset.registered",
    "model_evaluation.registered",
    "rule.candidate.saved",
    "rule.status.changed",
    "rule.rollback.completed",
    "rule.promotion.reviewed",
    "model.promotion.reviewed",
    "model.activation.completed",
    "model.rollback.completed",
    "agent.approval.decided",
    "audit_sample.created",
    "qa.feedback.status.updated",
    "routing_policy.candidate.saved",
    "routing_policy.status.changed",
    "routing_policy.activation.completed",
    "routing_policy.rollback.completed",
    "evidence.document.registered",
    "evidence.document_chunk.registered",
    "evidence.ocr_output.registered",
    "evidence.embedding_job.registered",
    "evidence.retrieval_audit.recorded",
];

#[derive(Debug, Clone)]
pub struct PersistedAgentRun {
    pub agent_run_id: String,
    pub claim_id: String,
    pub status: String,
    pub decision_boundary: String,
    pub output_json: Value,
    pub evidence_refs: Vec<Value>,
    pub steps: Vec<Value>,
    pub context_snapshots: Vec<AgentContextSnapshotRecord>,
    pub policy_checks: Vec<AgentPolicyCheckRecord>,
    pub tool_calls: Vec<AgentToolCallRecord>,
    pub tool_results: Vec<AgentToolResultRecord>,
    pub approvals: Vec<AgentApprovalRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunLogRecord {
    pub agent_run_id: String,
    pub claim_id: String,
    pub status: String,
    pub decision_boundary: String,
    pub output_json: Value,
    pub evidence_refs: Vec<String>,
    pub steps: Vec<Value>,
    pub context_snapshots: Vec<AgentContextSnapshotRecord>,
    pub policy_checks: Vec<AgentPolicyCheckRecord>,
    pub tool_calls: Vec<AgentToolCallRecord>,
    pub tool_results: Vec<AgentToolResultRecord>,
    pub approvals: Vec<AgentApprovalRecord>,
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContextSnapshotRecord {
    pub snapshot_id: String,
    pub redaction_status: String,
    pub context_json: Value,
    pub source_refs: Vec<String>,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCallRecord {
    pub tool_call_id: String,
    pub tool_name: String,
    pub status: String,
    pub input_json: Value,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPolicyCheckRecord {
    pub policy_check_id: String,
    pub agent_run_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    pub policy_name: String,
    pub decision: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolResultRecord {
    pub tool_result_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    pub status: String,
    pub output_json: Value,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentApprovalRecord {
    pub approval_id: String,
    pub agent_run_id: String,
    pub proposed_action: String,
    pub decision: String,
    pub approver: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

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
pub struct InvestigationResultRecord {
    pub investigation_id: String,
    pub case_id: Option<String>,
    pub claim_id: String,
    pub outcome: String,
    pub confirmed_fwa: bool,
    pub financial_impact_type: Option<String>,
    pub saving_amount: Option<Decimal>,
    pub currency: Option<String>,
    pub notes: String,
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub customer_scope_id: Option<String>,
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub actor_role: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct SavingAttributionRecord {
    pub(super) attribution_id: String,
    pub(super) claim_id: String,
    pub(super) investigation_id: String,
    pub(super) source_type: String,
    pub(super) source_id: String,
    pub(super) financial_impact_type: String,
    pub(super) action: String,
    pub(super) saving_amount: Decimal,
    pub(super) currency: String,
    pub(super) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct AuditSampleStrataContext {
    pub(super) provider_type: String,
    pub(super) provider_region: String,
    pub(super) policy_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaReviewRecord {
    pub qa_case_id: String,
    pub claim_id: String,
    pub qa_conclusion: String,
    pub issue_type: String,
    pub feedback_target: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub customer_scope_id: Option<String>,
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub actor_role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaFeedbackItemRecord {
    pub feedback_id: String,
    pub qa_case_id: String,
    pub claim_id: String,
    pub feedback_target: String,
    pub issue_type: String,
    pub qa_conclusion: String,
    pub source: String,
    pub status: String,
    pub priority: String,
    pub summary: String,
    pub note_present: bool,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
    pub status_updated_by: Option<String>,
    pub status_audit_id: Option<String>,
    pub status_updated_at: Option<String>,
    pub status_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct QaFeedbackStatusUpdate {
    pub(super) status: String,
    pub(super) actor_id: Option<String>,
    pub(super) audit_id: String,
    pub(super) updated_at: Option<String>,
    pub(super) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateQaFeedbackStatusInput {
    pub status: String,
    pub actor_id: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_deserializing)]
    pub customer_scope_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateQaFeedbackStatusRecord {
    pub item: QaFeedbackItemRecord,
    pub audit_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeLabelRecord {
    pub label_id: String,
    pub claim_id: String,
    pub label_name: String,
    pub label_value: String,
    pub source_type: String,
    pub source_id: String,
    pub governance_status: String,
    pub feedback_target: String,
    pub currency: Option<String>,
    pub evidence_refs: Vec<String>,
}

pub fn canonical_feedback_target(feedback_target: &str) -> &str {
    match feedback_target {
        "models" => "model",
        value => value,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditHistoryEventRecord {
    pub audit_id: String,
    pub run_id: String,
    pub actor_role: String,
    pub event_type: String,
    pub event_status: String,
    pub summary: String,
    pub payload: Value,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEventRecord {
    pub event_id: String,
    pub event_type: String,
    pub source_event_type: String,
    pub source_audit_id: String,
    pub customer_scope_id: String,
    pub claim_id: String,
    pub run_id: String,
    pub delivery_status: String,
    pub retry_count: u32,
    pub max_attempts: u32,
    pub next_attempt_at: Option<String>,
    pub last_attempt_at: Option<String>,
    pub last_response_status_code: Option<u16>,
    pub last_error_message: Option<String>,
    pub idempotency_key: String,
    pub signature_key_id: String,
    pub signature_algorithm: String,
    pub signature_base_string: String,
    pub payload: Value,
    pub evidence_refs: Vec<String>,
    pub occurred_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDeliveryAttemptInput {
    pub event_id: String,
    pub delivery_status: String,
    pub response_status_code: Option<u16>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDeliveryAttemptRecord {
    pub event_id: String,
    pub attempt_number: u32,
    pub delivery_status: String,
    pub response_status_code: Option<u16>,
    pub error_message: Option<String>,
    pub next_attempt_at: Option<String>,
    pub attempted_at: Option<String>,
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
