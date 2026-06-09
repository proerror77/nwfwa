use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub use super::types_cases::*;
pub use super::types_core::*;
pub use super::types_evidence::*;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardModelScoreRecord {
    pub scored_runs: u32,
    pub average_score: f64,
    pub high_risk_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLayerScoreRecord {
    pub name: String,
    pub scored_runs: u32,
    pub average_score: f64,
    pub high_risk_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSavingAttributionRecord {
    pub source_type: String,
    pub source_id: String,
    pub financial_impact_type: String,
    pub action: String,
    pub saving_amount: String,
    pub currency: String,
    pub claim_count: u32,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSavingSegmentRecord {
    pub segment_type: String,
    pub segment_id: String,
    pub saving_amount: String,
    pub currency: String,
    pub claim_count: u32,
    pub attribution_count: u32,
    pub roi: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRiskSummaryItemRecord {
    pub provider_id: String,
    pub risk_score: u8,
    pub risk_tier: String,
    pub review_required: bool,
    pub review_route: String,
    pub claim_count: u32,
    pub specialty: Option<String>,
    pub network_status: Option<String>,
    pub review_failure_count: u32,
    pub confirmed_fwa_count: u32,
    pub false_positive_count: u32,
    pub network_risk_score: Option<u8>,
    pub latest_claim_id: Option<String>,
    pub outlier_flags: Vec<String>,
    pub graph_reasons: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRiskSummaryRecord {
    pub provider_count: u32,
    pub review_required_count: u32,
    pub high_risk_count: u32,
    pub providers: Vec<ProviderRiskSummaryItemRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLabelPoolRecord {
    pub total_labels: u32,
    pub approved_for_training: u32,
    pub needs_review: u32,
    pub rule_feedback: u32,
    pub model_feedback: u32,
    pub features_feedback: u32,
    pub provider_profile_feedback: u32,
    pub workflow_feedback: u32,
    pub case_status_labels: u32,
    pub medical_review_labels: u32,
    pub false_positive_labels: u32,
    pub evidence_backed_labels: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardQaQueueRecord {
    pub sampled_cases: u32,
    pub open_cases: u32,
    pub reviewed_cases: u32,
    pub disagreement_cases: u32,
    pub disagreement_rate: f64,
    pub feedback_open_count: u32,
    pub feedback_in_progress_count: u32,
    pub feedback_resolved_count: u32,
    pub feedback_dismissed_count: u32,
    pub unresolved_feedback_count: u32,
    pub rules_unresolved_feedback_count: u32,
    pub models_unresolved_feedback_count: u32,
    pub features_unresolved_feedback_count: u32,
    pub provider_profile_unresolved_feedback_count: u32,
    pub workflow_unresolved_feedback_count: u32,
    pub tpa_unresolved_feedback_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardCaseSlaRecord {
    pub total_cases: u32,
    pub open_cases: u32,
    pub closed_cases: u32,
    pub breached_cases: u32,
    pub sla_breach_rate: f64,
    pub average_time_to_triage_hours: f64,
    pub average_time_to_closure_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardAgentGovernanceRecord {
    pub total_runs: u32,
    pub successful_runs: u32,
    pub evidence_backed_runs: u32,
    pub tool_call_count: u32,
    pub policy_check_count: u32,
    pub denied_policy_check_count: u32,
    pub failed_tool_call_count: u32,
    pub pending_approvals: u32,
    pub approved_approvals: u32,
    pub rejected_approvals: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardModelGovernanceRecord {
    pub total_models: u32,
    pub evaluated_models: u32,
    pub drift_watch_count: u32,
    pub drift_detected_count: u32,
    pub average_precision: Option<f64>,
    pub average_recall: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardRuleGovernanceRecord {
    pub total_rules: u32,
    pub active_rules: u32,
    pub triggered_rules: u32,
    pub total_trigger_count: u32,
    pub reviewed_count: u32,
    pub confirmed_fwa_count: u32,
    pub false_positive_count: u32,
    pub precision: f64,
    pub false_positive_rate: f64,
    pub saving_amount: String,
    pub roi: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardValueMeasurementRecord {
    pub prevented_payment: String,
    pub recovered_amount: String,
    pub avoided_future_exposure: String,
    pub deterrence_estimate: String,
    pub estimated_impact: String,
    pub review_cost: String,
    pub false_positive_operational_cost: String,
    pub reviewer_capacity_hours: String,
    pub net_value: String,
    pub currency: String,
    pub evidence_caveat: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardAuditCoverageRecord {
    pub scoring_runs: u32,
    pub canonical_trace_runs: u32,
    pub canonical_trace_coverage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummaryRecord {
    pub suspected_claims: u32,
    pub confirmed_fwa: u32,
    pub risk_amount: String,
    pub saving_amount: String,
    pub rag_distribution: BTreeMap<String, u32>,
    pub scheme_distribution: BTreeMap<String, u32>,
    pub rule_hits: u32,
    pub model_scores: BTreeMap<String, DashboardModelScoreRecord>,
    pub layer_scores: BTreeMap<String, DashboardLayerScoreRecord>,
    pub saving_attributions: Vec<DashboardSavingAttributionRecord>,
    pub saving_segments: Vec<DashboardSavingSegmentRecord>,
    pub value_measurement: DashboardValueMeasurementRecord,
    pub audit_coverage: DashboardAuditCoverageRecord,
    pub label_pool: DashboardLabelPoolRecord,
    pub qa_queue: DashboardQaQueueRecord,
    pub case_sla: DashboardCaseSlaRecord,
    pub agent_governance: DashboardAgentGovernanceRecord,
    pub model_governance: DashboardModelGovernanceRecord,
    pub rule_governance: DashboardRuleGovernanceRecord,
    pub investigation_results: u32,
    pub qa_reviews: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCaseRecord {
    pub case_id: String,
    pub title: String,
    pub fwa_type: String,
    pub scheme_family: String,
    pub diagnosis_code: String,
    pub provider_region: String,
    pub provider_type: String,
    pub summary: String,
    pub outcome: String,
    pub tags: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarCaseQuery {
    pub claim_id: Option<String>,
    pub diagnosis_code: String,
    pub provider_region: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarCaseRecord {
    pub case_id: String,
    pub title: String,
    pub scheme_family: String,
    pub similarity_score: f64,
    pub matched_signals: Vec<String>,
    pub retrieval_method: String,
    pub provenance_refs: Vec<String>,
    pub summary: String,
    pub outcome: String,
    pub evidence_refs: Vec<String>,
}

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
