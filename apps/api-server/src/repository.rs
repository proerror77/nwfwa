use async_trait::async_trait;
use chrono::NaiveDate;
use fwa_core::{
    assess_evidence_sufficiency, canonical_scheme_family, AuditEventId, Claim, ClaimContext,
    ClaimId, ClaimItem, Member, MemberId, Money, Policy, PolicyId, Provider, ProviderId,
    ProviderRiskTier, RecommendedAction, RuleActionClass,
};
use fwa_rules::{Condition, RequiredEvidence, Rule, RuleAction};
use fwa_scoring::RoutingPolicy;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, postgres::PgRow, PgPool, Postgres, Row, Transaction};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    hash::{Hash, Hasher},
    sync::Arc,
};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct PersistedScoringRun {
    pub run_id: String,
    pub audit_id: String,
    pub claim_id: String,
    pub source_system: String,
    pub actor_id: String,
    pub risk_score: u8,
    pub rag: String,
    pub risk_level: String,
    pub recommended_action: String,
    pub confidence_score: u8,
    pub confidence: String,
    pub routing_reason: String,
    pub routing_policy: Value,
    pub score_breakdown: Value,
    pub feature_values: Vec<Value>,
    pub rule_runs: Vec<Value>,
    pub model_score: Value,
    pub audit_event: Value,
    pub evidence_refs: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberProfileSummaryRecord {
    pub member_id: String,
    pub claim_count: u32,
    pub policy_count: u32,
    pub total_claim_amount: Decimal,
    pub currency: String,
    pub high_risk_claim_count: u32,
    pub latest_claim_id: Option<String>,
    pub risk_level_summary: String,
    pub profile_summary: String,
    pub evidence_refs: Vec<String>,
}

struct MemberProfileSummaryInput {
    member_id: String,
    claim_count: u32,
    policy_count: u32,
    total_claim_amount: Decimal,
    currency: String,
    high_risk_claim_count: u32,
    latest_claim_id: Option<String>,
    evidence_refs: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub struct PersistedAuditEvent {
    pub audit_id: String,
    pub run_id: String,
    pub claim_id: String,
    pub source_system: String,
    pub actor_id: String,
    pub actor_role: String,
    pub event_type: String,
    pub event_status: String,
    pub summary: String,
    pub payload: Value,
    pub evidence_refs: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct PersistedInboxClaimRun {
    pub run_id: String,
    pub audit_id: String,
    pub external_message_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub external_message_fingerprint: Option<String>,
    pub raw_payload_checksum: String,
    pub raw_payload_ref: Option<String>,
    pub mapping_version: String,
    pub validation_result: String,
    pub scoring_ready: bool,
    pub claim_id: String,
    pub source_system: String,
    pub customer_scope_id: String,
    pub canonical_claim_context: Value,
    pub validation_errors: Value,
    pub data_quality_signals: Value,
    pub evidence_refs: Value,
}

#[derive(Debug, Clone, Default)]
pub struct AuditEventListFilter {
    pub limit: u32,
    pub event_group: Option<String>,
    pub event_type: Option<String>,
    pub actor_id: Option<String>,
    pub run_id: Option<String>,
    pub claim_id: Option<String>,
    pub rule_id: Option<String>,
    pub rule_version: Option<String>,
    pub model_key: Option<String>,
    pub model_version: Option<String>,
    pub routing_policy_id: Option<String>,
    pub routing_policy_version: Option<String>,
    pub review_mode: Option<String>,
    pub feedback_id: Option<String>,
    pub qa_case_id: Option<String>,
    pub sample_id: Option<String>,
    pub agent_run_id: Option<String>,
    pub dataset_id: Option<String>,
    pub feature_set_id: Option<String>,
    pub model_dataset_id: Option<String>,
    pub evaluation_run_id: Option<String>,
    pub has_canonical_trace: Option<bool>,
    pub customer_scope_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceDocumentRecord {
    pub document_id: String,
    pub customer_scope_id: String,
    pub source_system: String,
    pub source_record_ref: String,
    pub claim_id: Option<String>,
    pub external_document_id: Option<String>,
    pub document_type: String,
    pub storage_uri: String,
    pub content_checksum: String,
    pub ingestion_status: String,
    pub redaction_status: String,
    pub retention_policy_id: String,
    pub evidence_refs: Vec<String>,
    pub metadata_json: Value,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateEvidenceDocumentInput {
    pub document_id: String,
    pub customer_scope_id: String,
    pub source_system: String,
    pub source_record_ref: String,
    pub claim_id: Option<String>,
    pub external_document_id: Option<String>,
    pub document_type: String,
    pub storage_uri: String,
    pub content_checksum: String,
    pub ingestion_status: String,
    pub redaction_status: String,
    pub retention_policy_id: String,
    pub evidence_refs: Vec<String>,
    pub metadata_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceDocumentChunkRecord {
    pub chunk_id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub chunking_version: String,
    pub redaction_status: String,
    pub text_checksum: String,
    pub token_count: i32,
    pub storage_uri: String,
    pub source_offsets_json: Value,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateEvidenceDocumentChunkInput {
    pub chunk_id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub chunking_version: String,
    pub redaction_status: String,
    pub text_checksum: String,
    pub token_count: i32,
    pub storage_uri: String,
    pub source_offsets_json: Value,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceOcrOutputRecord {
    pub ocr_output_id: String,
    pub document_id: String,
    pub ocr_engine: String,
    pub ocr_engine_version: String,
    pub output_uri: String,
    pub output_checksum: String,
    pub confidence_score: Option<Decimal>,
    pub quality_status: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateEvidenceOcrOutputInput {
    pub ocr_output_id: String,
    pub document_id: String,
    pub ocr_engine: String,
    pub ocr_engine_version: String,
    pub output_uri: String,
    pub output_checksum: String,
    pub confidence_score: Option<Decimal>,
    pub quality_status: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceEmbeddingJobRecord {
    pub embedding_job_id: String,
    pub customer_scope_id: String,
    pub target_kind: String,
    pub target_ref: String,
    pub embedding_model: String,
    pub embedding_model_version: String,
    pub chunking_version: String,
    pub redaction_status: String,
    pub vector_store_kind: String,
    pub vector_store_ref: String,
    pub embedding_checksum: String,
    pub status: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateEvidenceEmbeddingJobInput {
    pub embedding_job_id: String,
    pub customer_scope_id: String,
    pub target_kind: String,
    pub target_ref: String,
    pub embedding_model: String,
    pub embedding_model_version: String,
    pub chunking_version: String,
    pub redaction_status: String,
    pub vector_store_kind: String,
    pub vector_store_ref: String,
    pub embedding_checksum: String,
    pub status: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceRetrievalAuditEventRecord {
    pub retrieval_id: String,
    pub customer_scope_id: String,
    pub actor_id: String,
    pub actor_role: String,
    pub query_kind: String,
    pub query_checksum: String,
    pub retrieval_method: String,
    pub embedding_model_version: Option<String>,
    pub top_k: i32,
    pub source_refs: Vec<String>,
    pub result_refs: Vec<String>,
    pub redaction_status: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateEvidenceRetrievalAuditEventInput {
    pub retrieval_id: String,
    pub customer_scope_id: String,
    pub actor_id: String,
    pub actor_role: String,
    pub query_kind: String,
    pub query_checksum: String,
    pub retrieval_method: String,
    pub embedding_model_version: Option<String>,
    pub top_k: i32,
    pub source_refs: Vec<String>,
    pub result_refs: Vec<String>,
    pub redaction_status: String,
    pub evidence_refs: Vec<String>,
}

const GOVERNANCE_AUDIT_EVENT_TYPES: &[&str] = &[
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
pub struct RoutingPolicyRecord {
    pub policy_id: String,
    pub version: u32,
    pub review_mode: String,
    pub status: String,
    pub owner: String,
    pub risk_thresholds: fwa_scoring::RiskThresholds,
    pub confidence_thresholds: fwa_scoring::ConfidenceThresholds,
    pub provider_review_threshold: u8,
    pub activated_at: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSummaryRecord {
    pub rule_id: String,
    pub name: String,
    pub status: String,
    pub owner: String,
    pub active_version: Option<u32>,
    pub latest_version: u32,
    pub review_mode: String,
    pub scheme_family: String,
    pub score: u8,
    pub alert_code: String,
    pub recommended_action: RecommendedAction,
    pub applicability_scope: RuleApplicabilityScopeRecord,
    pub backtest_result: RuleBacktestSummaryRecord,
    pub estimated_saving: String,
    pub false_positive_history: RuleFalsePositiveHistoryRecord,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleApplicabilityScopeRecord {
    pub review_mode: String,
    pub scheme_family: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleBacktestSummaryRecord {
    pub status: String,
    pub sample_count: u32,
    pub matched_count: u32,
    pub precision: f64,
    pub recall: f64,
    pub lift: f64,
    pub false_positive_rate: f64,
    pub estimated_saving: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleFalsePositiveHistoryRecord {
    pub status: String,
    pub false_positive_count: u32,
    pub false_positive_rate: f64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleVersionRecord {
    pub version: u32,
    pub status: String,
    pub dsl: Value,
    pub review_mode: String,
    pub scheme_family: String,
    pub score: u8,
    pub alert_code: String,
    pub recommended_action: RecommendedAction,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDetailRecord {
    pub summary: RuleSummaryRecord,
    pub versions: Vec<RuleVersionRecord>,
    pub audit_events: Vec<AuditHistoryEventRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePerformanceRecord {
    pub rule_id: String,
    pub alert_code: String,
    pub trigger_count: u32,
    pub reviewed_count: u32,
    pub confirmed_fwa_count: u32,
    pub false_positive_count: u32,
    pub mark_rate: f64,
    pub precision: f64,
    pub false_positive_rate: f64,
    pub saving_amount: String,
    pub roi: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePromotionReviewRecord {
    pub rule_id: String,
    pub rule_version: u32,
    pub decision: String,
    pub reviewer: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleBacktestRecord {
    pub rule_id: String,
    pub rule_version: u32,
    pub sample_count: u32,
    pub matched_count: u32,
    pub reviewed_count: u32,
    pub confirmed_fwa_count: u32,
    pub false_positive_count: u32,
    pub precision: f64,
    pub recall: f64,
    pub lift: f64,
    pub false_positive_rate: f64,
    pub estimated_saving: String,
    pub promotion_recommendation: String,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadRecord {
    pub lead_id: String,
    pub run_id: String,
    pub claim_id: String,
    pub member_id: String,
    pub provider_id: String,
    pub source_system: String,
    #[serde(default = "default_review_mode")]
    pub review_mode: String,
    pub scheme_family: String,
    pub lead_source: String,
    pub status: String,
    pub disposition: String,
    pub risk_score: u8,
    pub rag: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseRecord {
    pub case_id: String,
    pub lead_id: String,
    pub claim_id: String,
    pub member_id: String,
    pub provider_id: String,
    pub source_system: String,
    #[serde(default = "default_review_mode")]
    pub review_mode: String,
    pub scheme_family: String,
    pub lead_source: String,
    pub status: String,
    pub assignee: String,
    pub reviewer: String,
    pub priority: String,
    pub routing_reason: String,
    pub evidence_package: Value,
    pub sla_target_hours: u32,
    pub sla_status: String,
    pub time_to_triage_hours: f64,
    pub time_to_closure_hours: Option<f64>,
    pub final_outcome: Option<String>,
    pub reviewer_notes: Option<String>,
    pub investigation_result_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageLeadInput {
    pub decision: String,
    pub merge_target_lead_id: Option<String>,
    pub assignee: String,
    pub reviewer: String,
    pub priority: String,
    pub notes: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_deserializing)]
    pub customer_scope_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageLeadRecord {
    pub lead: LeadRecord,
    pub case: Option<CaseRecord>,
    pub audit_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCaseStatusInput {
    pub status: String,
    pub actor_id: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_deserializing)]
    pub customer_scope_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCaseStatusRecord {
    pub case: CaseRecord,
    pub audit_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSampleLeadRecord {
    pub lead_id: String,
    pub claim_id: String,
    pub scheme_family: String,
    #[serde(default = "default_review_mode")]
    pub review_mode: String,
    #[serde(default)]
    pub provider_id: String,
    #[serde(default)]
    pub provider_type: String,
    #[serde(default)]
    pub provider_region: String,
    #[serde(default)]
    pub policy_type: String,
    #[serde(default)]
    pub risk_band: String,
    #[serde(default)]
    pub strata_key: String,
    #[serde(default)]
    pub prior_reviewer_sample_count: u32,
    pub risk_score: u8,
    pub rag: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuditSampleInput {
    pub sample_mode: String,
    pub population_definition: String,
    pub inclusion_criteria: Value,
    pub deterministic_seed: Option<String>,
    pub sample_size: usize,
    pub reviewer: String,
    pub assignment_queue: String,
    #[serde(default, skip_deserializing)]
    pub customer_scope_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSampleRecord {
    pub sample_id: String,
    pub customer_scope_id: String,
    pub sample_mode: String,
    pub population_definition: String,
    pub inclusion_criteria: Value,
    pub deterministic_seed: Option<String>,
    pub selection_method: String,
    pub sample_size: usize,
    pub reviewer: String,
    pub assignment_queue: String,
    pub selected_leads: Vec<AuditSampleLeadRecord>,
    pub outcome_distribution: Value,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersionRecord {
    pub model_key: String,
    pub version: String,
    pub model_type: String,
    pub runtime_kind: String,
    pub execution_provider: String,
    pub status: String,
    pub review_mode: String,
    pub artifact_uri: Option<String>,
    pub endpoint_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformanceRecord {
    pub model_key: String,
    pub data_status: String,
    pub scored_runs: u32,
    pub average_score: f64,
    pub high_risk_count: u32,
    pub score_psi: Option<f64>,
    pub drift_status: String,
    pub latest_scored_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPromotionReviewRecord {
    pub model_key: String,
    pub model_version: String,
    pub decision: String,
    pub reviewer: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRetrainingJobRecord {
    pub job_id: String,
    pub model_key: String,
    pub model_version: String,
    pub status: String,
    pub requested_by: String,
    pub request_notes: String,
    pub status_note: String,
    pub updated_by: String,
    pub readiness_recommendation: String,
    pub latest_evaluation_id: String,
    pub source_dataset_id: String,
    pub source_data_quality_score: Option<f64>,
    pub source_data_quality_status: String,
    pub trigger_summary: Vec<String>,
    pub blocker_summary: Vec<String>,
    pub candidate_model_version: Option<String>,
    pub candidate_artifact_uri: Option<String>,
    pub candidate_endpoint_url: Option<String>,
    pub validation_report_uri: Option<String>,
    pub output_evaluation_id: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompleteModelRetrainingJobInput<'a> {
    pub job_id: &'a str,
    pub actor: &'a str,
    pub status_note: &'a str,
    pub candidate_model_version: &'a str,
    pub candidate_artifact_uri: &'a str,
    pub candidate_endpoint_url: Option<&'a str>,
    pub validation_report_uri: &'a str,
    pub output_evaluation_id: &'a str,
}

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
struct SavingAttributionRecord {
    attribution_id: String,
    claim_id: String,
    investigation_id: String,
    source_type: String,
    source_id: String,
    financial_impact_type: String,
    action: String,
    saving_amount: Decimal,
    currency: String,
    evidence_refs: Vec<String>,
}

#[derive(Debug, Clone)]
struct AuditSampleStrataContext {
    provider_type: String,
    provider_region: String,
    policy_type: String,
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
struct QaFeedbackStatusUpdate {
    status: String,
    actor_id: Option<String>,
    audit_id: String,
    updated_at: Option<String>,
    evidence_refs: Vec<String>,
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
    pub metrics_json: Value,
}

#[derive(sqlx::FromRow)]
struct ClaimContextRow {
    external_claim_id: String,
    diagnosis_code: String,
    service_date: NaiveDate,
    claim_amount: Decimal,
    claim_currency: String,
    external_member_id: String,
    dob: Option<NaiveDate>,
    gender: Option<String>,
    external_policy_id: String,
    product_code: String,
    coverage_start_date: NaiveDate,
    coverage_end_date: NaiveDate,
    coverage_limit_amount: Decimal,
    policy_currency: String,
    external_provider_id: String,
    provider_name: String,
    provider_type: String,
    provider_region: String,
    provider_risk_tier: String,
}

type ClaimItemRow = (String, String, String, i32, Decimal, Decimal, String);
type LeadRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    i32,
    String,
    String,
    Value,
);
#[derive(sqlx::FromRow)]
struct CaseRow {
    case_id: String,
    lead_id: String,
    claim_id: String,
    member_id: String,
    provider_id: String,
    source_system: String,
    review_mode: String,
    scheme_family: String,
    lead_source: String,
    status: String,
    assignee: String,
    reviewer: String,
    priority: String,
    routing_reason: String,
    evidence_package_json: Value,
    final_outcome: Option<String>,
    reviewer_notes: Option<String>,
    investigation_result_id: Option<String>,
    lead_created_at: chrono::DateTime<chrono::Utc>,
    case_created_at: chrono::DateTime<chrono::Utc>,
    case_updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct AgentApprovalRow {
    approval_id: String,
    proposed_action: String,
    decision: String,
    approver: String,
    reason: String,
    evidence_refs: Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct AgentPolicyCheckRow {
    policy_check_id: String,
    tool_call_id: String,
    tool_name: String,
    policy_name: String,
    decision: String,
    reason: String,
    evidence_refs: Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

trait IntoClaimContext {
    fn into_context(self, items: Vec<ClaimItemRow>) -> ClaimContext;
}

impl IntoClaimContext for ClaimContextRow {
    fn into_context(self, items: Vec<ClaimItemRow>) -> ClaimContext {
        let member_id = MemberId::from_external(self.external_member_id.clone());
        let policy_id = PolicyId::from_external(self.external_policy_id.clone());
        let provider_id = ProviderId::from_external(self.external_provider_id.clone());

        ClaimContext {
            claim: Claim {
                id: ClaimId::from_external(self.external_claim_id.clone()),
                external_claim_id: self.external_claim_id,
                member_id: member_id.clone(),
                policy_id: policy_id.clone(),
                provider_id: provider_id.clone(),
                diagnosis_code: self.diagnosis_code,
                service_date: self.service_date,
                amount: Money::new(self.claim_amount, self.claim_currency),
            },
            items: items
                .into_iter()
                .map(
                    |(
                        item_code,
                        item_type,
                        description,
                        quantity,
                        unit_amount,
                        total_amount,
                        currency,
                    )| ClaimItem {
                        item_code,
                        item_type,
                        description,
                        quantity: quantity.max(0) as u32,
                        unit_amount: Money::new(unit_amount, currency.clone()),
                        total_amount: Money::new(total_amount, currency),
                    },
                )
                .collect(),
            member: Member {
                id: member_id.clone(),
                external_member_id: self.external_member_id,
                dob: self.dob,
                gender: self.gender,
            },
            policy: Policy {
                id: policy_id,
                external_policy_id: self.external_policy_id,
                member_id,
                product_code: self.product_code,
                coverage_start_date: self.coverage_start_date,
                coverage_end_date: self.coverage_end_date,
                coverage_limit: Money::new(self.coverage_limit_amount, self.policy_currency),
            },
            provider: Provider {
                id: provider_id,
                external_provider_id: self.external_provider_id,
                name: self.provider_name,
                provider_type: self.provider_type,
                region: self.provider_region,
                risk_tier: provider_risk_tier_from_text(&self.provider_risk_tier),
            },
        }
    }
}

fn provider_risk_tier_from_text(value: &str) -> ProviderRiskTier {
    match value {
        "Low" => ProviderRiskTier::Low,
        "High" => ProviderRiskTier::High,
        _ => ProviderRiskTier::Medium,
    }
}

fn inbox_claim_run_from_row(row: PgRow) -> PersistedInboxClaimRun {
    PersistedInboxClaimRun {
        run_id: row.try_get("run_id").unwrap_or_default(),
        audit_id: row.try_get("audit_id").unwrap_or_default(),
        external_message_id: row
            .try_get::<Option<String>, _>("external_message_id")
            .unwrap_or(None),
        idempotency_key: row
            .try_get::<Option<String>, _>("idempotency_key")
            .unwrap_or(None),
        external_message_fingerprint: row
            .try_get::<Option<String>, _>("external_message_fingerprint")
            .unwrap_or(None),
        raw_payload_checksum: row.try_get("raw_payload_checksum").unwrap_or_default(),
        raw_payload_ref: row
            .try_get::<Option<String>, _>("raw_payload_ref")
            .unwrap_or(None),
        mapping_version: row.try_get("mapping_version").unwrap_or_default(),
        validation_result: row.try_get("validation_result").unwrap_or_default(),
        scoring_ready: row.try_get("scoring_ready").unwrap_or(false),
        claim_id: row.try_get("claim_id").unwrap_or_default(),
        source_system: row.try_get("source_system").unwrap_or_default(),
        customer_scope_id: row.try_get("customer_scope_id").unwrap_or_default(),
        canonical_claim_context: row
            .try_get("canonical_claim_context")
            .unwrap_or_else(|_| serde_json::json!({})),
        validation_errors: row
            .try_get("validation_errors")
            .unwrap_or_else(|_| serde_json::json!([])),
        data_quality_signals: row
            .try_get("data_quality_signals")
            .unwrap_or_else(|_| serde_json::json!([])),
        evidence_refs: row
            .try_get("evidence_refs")
            .unwrap_or_else(|_| serde_json::json!([])),
    }
}

#[async_trait]
pub trait ScoringRepository: Send + Sync {
    async fn upsert_claim_context(
        &self,
        context: ClaimContext,
        raw_payload: Value,
    ) -> anyhow::Result<()>;

    async fn load_claim_context(
        &self,
        external_claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<ClaimContext>>;

    async fn member_profile_summary(
        &self,
        member_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<MemberProfileSummaryRecord>>;

    async fn save_scoring_run(&self, run: PersistedScoringRun) -> anyhow::Result<()>;

    async fn save_audit_event(&self, event: PersistedAuditEvent) -> anyhow::Result<()>;

    async fn save_inbox_claim_run(&self, run: PersistedInboxClaimRun) -> anyhow::Result<()>;

    async fn get_inbox_claim_run_by_idempotency_key(
        &self,
        idempotency_key: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>>;

    async fn get_inbox_claim_run_by_run_id(
        &self,
        run_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>>;

    async fn active_routing_policy(
        &self,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicy>>;

    async fn list_routing_policies(&self) -> anyhow::Result<Vec<RoutingPolicyRecord>>;

    async fn save_routing_policy_candidate(
        &self,
        policy: RoutingPolicy,
        owner: String,
    ) -> anyhow::Result<RoutingPolicyRecord>;

    async fn get_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>>;

    async fn update_routing_policy_status(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
        status: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>>;

    async fn activate_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>>;

    async fn list_rules(&self) -> anyhow::Result<Vec<RuleSummaryRecord>>;

    async fn list_active_rules(&self) -> anyhow::Result<Vec<Rule>>;

    async fn get_rule(&self, rule_id: &str) -> anyhow::Result<Option<RuleDetailRecord>>;

    async fn rule_audit_history(
        &self,
        rule_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>>;

    async fn save_rule_candidate(
        &self,
        rule: Rule,
        owner: String,
    ) -> anyhow::Result<RuleDetailRecord>;

    async fn update_rule_status(
        &self,
        rule_id: &str,
        status: &str,
    ) -> anyhow::Result<Option<RuleSummaryRecord>>;

    async fn rule_performance(&self) -> anyhow::Result<Vec<RulePerformanceRecord>>;

    async fn save_rule_backtest(
        &self,
        record: RuleBacktestRecord,
    ) -> anyhow::Result<RuleBacktestRecord>;

    async fn latest_rule_backtest(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleBacktestRecord>>;

    async fn save_rule_promotion_review(
        &self,
        record: RulePromotionReviewRecord,
    ) -> anyhow::Result<RulePromotionReviewRecord>;

    async fn latest_rule_promotion_review(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RulePromotionReviewRecord>>;

    async fn list_leads(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<LeadRecord>>;

    async fn triage_lead(
        &self,
        lead_id: &str,
        input: TriageLeadInput,
    ) -> anyhow::Result<Option<TriageLeadRecord>>;

    async fn list_cases(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<CaseRecord>>;

    async fn update_case_status(
        &self,
        case_id: &str,
        input: UpdateCaseStatusInput,
    ) -> anyhow::Result<Option<UpdateCaseStatusRecord>>;

    async fn create_audit_sample(
        &self,
        input: CreateAuditSampleInput,
    ) -> anyhow::Result<AuditSampleRecord>;

    async fn list_audit_samples(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditSampleRecord>>;

    async fn list_models(&self) -> anyhow::Result<Vec<ModelVersionRecord>>;

    async fn save_model_version(
        &self,
        record: ModelVersionRecord,
    ) -> anyhow::Result<ModelVersionRecord>;

    async fn update_model_status(
        &self,
        model_key: &str,
        model_version: &str,
        status: &str,
    ) -> anyhow::Result<Option<ModelVersionRecord>>;

    async fn model_performance(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Option<ModelPerformanceRecord>>;

    async fn save_model_promotion_review(
        &self,
        record: ModelPromotionReviewRecord,
    ) -> anyhow::Result<ModelPromotionReviewRecord>;

    async fn latest_model_promotion_review(
        &self,
        model_key: &str,
        model_version: &str,
    ) -> anyhow::Result<Option<ModelPromotionReviewRecord>>;

    async fn save_model_retraining_job(
        &self,
        record: ModelRetrainingJobRecord,
    ) -> anyhow::Result<ModelRetrainingJobRecord>;

    async fn list_model_retraining_jobs(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Vec<ModelRetrainingJobRecord>>;

    async fn get_model_retraining_job(
        &self,
        job_id: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>>;

    async fn claim_next_model_retraining_job(
        &self,
        model_key: Option<&str>,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>>;

    async fn update_model_retraining_job_status(
        &self,
        job_id: &str,
        status: &str,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>>;

    async fn complete_model_retraining_job(
        &self,
        input: CompleteModelRetrainingJobInput<'_>,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>>;

    async fn dashboard_summary(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<DashboardSummaryRecord>;

    async fn provider_risk_summary(&self) -> anyhow::Result<ProviderRiskSummaryRecord>;

    async fn list_knowledge_cases(&self) -> anyhow::Result<Vec<KnowledgeCaseRecord>>;

    async fn save_knowledge_case(
        &self,
        record: KnowledgeCaseRecord,
    ) -> anyhow::Result<KnowledgeCaseRecord>;

    async fn search_similar_cases(
        &self,
        query: SimilarCaseQuery,
    ) -> anyhow::Result<Vec<SimilarCaseRecord>>;

    async fn save_agent_run(&self, run: PersistedAgentRun) -> anyhow::Result<()>;

    async fn list_agent_runs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AgentRunLogRecord>>;

    async fn save_agent_approval(
        &self,
        approval: AgentApprovalRecord,
    ) -> anyhow::Result<AgentApprovalRecord>;

    async fn register_dataset(&self, input: RegisterDatasetInput) -> anyhow::Result<DatasetRecord>;

    async fn list_datasets(&self) -> anyhow::Result<Vec<DatasetRecord>>;

    async fn get_dataset(&self, dataset_id: &str) -> anyhow::Result<Option<DatasetRecord>>;

    async fn add_field_mapping(
        &self,
        dataset_id: &str,
        input: CreateFieldMappingInput,
    ) -> anyhow::Result<Option<FieldMappingRecord>>;

    async fn save_investigation_result(
        &self,
        record: InvestigationResultRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord>;

    async fn save_qa_review(
        &self,
        record: QaReviewRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord>;

    async fn list_qa_feedback_items(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaFeedbackItemRecord>>;

    async fn update_qa_feedback_status(
        &self,
        feedback_id: &str,
        input: UpdateQaFeedbackStatusInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<UpdateQaFeedbackStatusRecord>>;

    async fn list_qa_reviews(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaReviewRecord>>;

    async fn list_outcome_labels(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<OutcomeLabelRecord>>;

    async fn claim_audit_history(
        &self,
        claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>>;

    async fn list_audit_events(
        &self,
        filter: AuditEventListFilter,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>>;

    async fn list_webhook_events(&self) -> anyhow::Result<Vec<WebhookEventRecord>>;

    async fn save_webhook_delivery_attempt(
        &self,
        input: WebhookDeliveryAttemptInput,
    ) -> anyhow::Result<WebhookDeliveryAttemptRecord>;

    async fn register_feature_set(
        &self,
        input: RegisterFeatureSetInput,
    ) -> anyhow::Result<Option<FeatureSetRecord>>;

    async fn register_model_dataset(
        &self,
        input: RegisterModelDatasetInput,
    ) -> anyhow::Result<Option<ModelDatasetRecord>>;

    async fn get_model_dataset_source_dataset(
        &self,
        model_dataset_id: &str,
    ) -> anyhow::Result<Option<DatasetRecord>>;

    async fn register_model_evaluation(
        &self,
        input: RegisterModelEvaluationInput,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>>;

    async fn get_model_evaluation(
        &self,
        evaluation_run_id: &str,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>>;

    async fn list_model_evaluations(&self) -> anyhow::Result<Vec<ModelEvaluationRecord>>;

    async fn save_evidence_document(
        &self,
        input: CreateEvidenceDocumentInput,
    ) -> anyhow::Result<EvidenceDocumentRecord>;

    async fn list_evidence_documents(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentRecord>>;

    async fn get_evidence_document(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentRecord>>;

    async fn save_evidence_document_chunk(
        &self,
        input: CreateEvidenceDocumentChunkInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentChunkRecord>>;

    async fn list_evidence_document_chunks(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentChunkRecord>>;

    async fn save_evidence_ocr_output(
        &self,
        input: CreateEvidenceOcrOutputInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceOcrOutputRecord>>;

    async fn list_evidence_ocr_outputs(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceOcrOutputRecord>>;

    async fn save_evidence_embedding_job(
        &self,
        input: CreateEvidenceEmbeddingJobInput,
    ) -> anyhow::Result<EvidenceEmbeddingJobRecord>;

    async fn list_evidence_embedding_jobs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceEmbeddingJobRecord>>;

    async fn save_evidence_retrieval_audit_event(
        &self,
        input: CreateEvidenceRetrievalAuditEventInput,
    ) -> anyhow::Result<EvidenceRetrievalAuditEventRecord>;

    async fn list_evidence_retrieval_audit_events(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceRetrievalAuditEventRecord>>;
}

pub type SharedRepository = Arc<dyn ScoringRepository>;

#[derive(Debug, Default)]
pub struct InMemoryScoringRepository {
    claims: Mutex<HashMap<String, ClaimContext>>,
    inbox_claim_runs: Mutex<HashMap<String, PersistedInboxClaimRun>>,
    runs: Mutex<Vec<PersistedScoringRun>>,
    audit_events: Mutex<Vec<PersistedAuditEvent>>,
    agent_runs: Mutex<Vec<PersistedAgentRun>>,
    leads: Mutex<HashMap<String, LeadRecord>>,
    cases: Mutex<HashMap<String, CaseRecord>>,
    audit_samples: Mutex<HashMap<String, AuditSampleRecord>>,
    audit_sample_sequence: Mutex<u64>,
    candidate_rules: Mutex<HashMap<String, RuleDetailRecord>>,
    rule_statuses: Mutex<HashMap<String, String>>,
    rule_backtests: Mutex<Vec<RuleBacktestRecord>>,
    rule_promotion_reviews: Mutex<Vec<RulePromotionReviewRecord>>,
    knowledge_cases: Mutex<HashMap<String, KnowledgeCaseRecord>>,
    datasets: Mutex<HashMap<String, DatasetRecord>>,
    dataset_sequence: Mutex<u64>,
    mapping_sequence: Mutex<u64>,
    pilot_audit_events: Mutex<Vec<(String, AuditHistoryEventRecord)>>,
    feature_sets: Mutex<HashMap<String, FeatureSetRecord>>,
    feature_set_sequence: Mutex<u64>,
    model_datasets: Mutex<HashMap<String, ModelDatasetRecord>>,
    model_dataset_sequence: Mutex<u64>,
    model_versions: Mutex<HashMap<String, ModelVersionRecord>>,
    model_evaluations: Mutex<HashMap<String, ModelEvaluationRecord>>,
    model_promotion_reviews: Mutex<Vec<ModelPromotionReviewRecord>>,
    model_retraining_jobs: Mutex<HashMap<String, ModelRetrainingJobRecord>>,
    model_retraining_job_sequence: Mutex<u64>,
    model_statuses: Mutex<HashMap<String, String>>,
    routing_policies: Mutex<Vec<RoutingPolicyRecord>>,
    webhook_delivery_attempts: Mutex<HashMap<String, Vec<WebhookDeliveryAttemptRecord>>>,
    saving_attributions: Mutex<Vec<SavingAttributionRecord>>,
    evidence_documents: Mutex<HashMap<String, EvidenceDocumentRecord>>,
    evidence_document_chunks: Mutex<HashMap<String, EvidenceDocumentChunkRecord>>,
    evidence_ocr_outputs: Mutex<HashMap<String, EvidenceOcrOutputRecord>>,
    evidence_embedding_jobs: Mutex<HashMap<String, EvidenceEmbeddingJobRecord>>,
    evidence_retrieval_audit_events: Mutex<HashMap<String, EvidenceRetrievalAuditEventRecord>>,
}

async fn upsert_pilot_audit_event(
    events: &Mutex<Vec<(String, AuditHistoryEventRecord)>>,
    claim_id: String,
    event: AuditHistoryEventRecord,
) {
    let mut events = events.lock().await;
    if let Some((stored_claim_id, stored_event)) = events
        .iter_mut()
        .find(|(_, stored_event)| stored_event.audit_id == event.audit_id)
    {
        *stored_claim_id = claim_id;
        *stored_event = event;
    } else {
        events.push((claim_id, event));
    }
}

impl InMemoryScoringRepository {
    pub fn shared() -> SharedRepository {
        Arc::new(Self::default())
    }

    pub fn shared_with_routing_policies(policies: Vec<RoutingPolicy>) -> SharedRepository {
        Arc::new(Self {
            routing_policies: Mutex::new(
                policies
                    .into_iter()
                    .map(|policy| routing_policy_record(policy, "active", "system", None, None))
                    .collect(),
            ),
            ..Self::default()
        })
    }

    async fn claim_visible_to_scope(
        &self,
        claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> bool {
        let Some(scope) = customer_scope_id else {
            return true;
        };
        scoped_claim_ids_from_audit_events(self.audit_events.lock().await.iter(), scope)
            .contains(claim_id)
    }
}

#[async_trait]
impl ScoringRepository for InMemoryScoringRepository {
    async fn upsert_claim_context(
        &self,
        context: ClaimContext,
        _raw_payload: Value,
    ) -> anyhow::Result<()> {
        self.claims
            .lock()
            .await
            .insert(context.claim.external_claim_id.clone(), context);
        Ok(())
    }

    async fn load_claim_context(
        &self,
        external_claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<ClaimContext>> {
        if !self
            .claim_visible_to_scope(external_claim_id, customer_scope_id)
            .await
        {
            return Ok(None);
        }
        Ok(self.claims.lock().await.get(external_claim_id).cloned())
    }

    async fn member_profile_summary(
        &self,
        member_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<MemberProfileSummaryRecord>> {
        let visible_claim_ids = match customer_scope_id {
            Some(scope) => Some(scoped_claim_ids_from_audit_events(
                self.audit_events.lock().await.iter(),
                scope,
            )),
            None => None,
        };
        let member_claims = self
            .claims
            .lock()
            .await
            .values()
            .filter(|context| context.member.external_member_id == member_id)
            .filter(|context| {
                visible_claim_ids
                    .as_ref()
                    .is_none_or(|claim_ids| claim_ids.contains(&context.claim.external_claim_id))
            })
            .cloned()
            .collect::<Vec<_>>();
        Ok(member_profile_from_contexts(
            member_id,
            &member_claims,
            self.runs.lock().await.as_slice(),
        ))
    }

    async fn save_scoring_run(&self, run: PersistedScoringRun) -> anyhow::Result<()> {
        let context = self.claims.lock().await.get(&run.claim_id).cloned();
        if let Some(lead) = lead_from_scoring_run(&run, context.as_ref()) {
            self.leads.lock().await.insert(lead.lead_id.clone(), lead);
        }
        self.audit_events.lock().await.push(PersistedAuditEvent {
            audit_id: run.audit_id.clone(),
            run_id: run.run_id.clone(),
            claim_id: run.claim_id.clone(),
            source_system: run.source_system.clone(),
            actor_id: run.actor_id.clone(),
            actor_role: "tpa_system".into(),
            event_type: run
                .audit_event
                .get("event_type")
                .and_then(Value::as_str)
                .unwrap_or("scoring.completed")
                .to_string(),
            event_status: run
                .audit_event
                .get("event_status")
                .and_then(Value::as_str)
                .unwrap_or("succeeded")
                .to_string(),
            summary: "FWA scoring completed".into(),
            payload: run.audit_event.clone(),
            evidence_refs: run.evidence_refs.clone(),
        });
        self.runs.lock().await.push(run);
        Ok(())
    }

    async fn save_audit_event(&self, event: PersistedAuditEvent) -> anyhow::Result<()> {
        let mut audit_events = self.audit_events.lock().await;
        if let Some(existing) = audit_events
            .iter_mut()
            .find(|existing| existing.audit_id == event.audit_id)
        {
            *existing = event;
        } else {
            audit_events.push(event);
        }
        Ok(())
    }

    async fn save_inbox_claim_run(&self, run: PersistedInboxClaimRun) -> anyhow::Result<()> {
        self.inbox_claim_runs
            .lock()
            .await
            .insert(run.run_id.clone(), run);
        Ok(())
    }

    async fn get_inbox_claim_run_by_idempotency_key(
        &self,
        idempotency_key: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
        Ok(self
            .inbox_claim_runs
            .lock()
            .await
            .values()
            .find(|run| {
                run.idempotency_key.as_deref() == Some(idempotency_key)
                    && customer_scope_id.is_none_or(|scope| run.customer_scope_id == scope)
            })
            .cloned())
    }

    async fn get_inbox_claim_run_by_run_id(
        &self,
        run_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
        Ok(self
            .inbox_claim_runs
            .lock()
            .await
            .get(run_id)
            .filter(|run| customer_scope_id.is_none_or(|scope| run.customer_scope_id == scope))
            .cloned())
    }

    async fn active_routing_policy(
        &self,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicy>> {
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        Ok(policies
            .iter()
            .filter(|policy| policy.status == "active")
            .filter(|policy| routing_policy_review_mode_applies(&policy.review_mode, review_mode))
            .max_by_key(|policy| (policy.review_mode == review_mode, policy.version))
            .map(routing_policy_from_record))
    }

    async fn list_routing_policies(&self) -> anyhow::Result<Vec<RoutingPolicyRecord>> {
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        Ok(policies.clone())
    }

    async fn save_routing_policy_candidate(
        &self,
        policy: RoutingPolicy,
        owner: String,
    ) -> anyhow::Result<RoutingPolicyRecord> {
        let record = routing_policy_record(policy, "draft", &owner, None, None);
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        policies.retain(|existing| {
            !(existing.policy_id == record.policy_id
                && existing.version == record.version
                && existing.review_mode == record.review_mode)
        });
        policies.push(record.clone());
        Ok(record)
    }

    async fn get_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        Ok(policies
            .iter()
            .find(|policy| {
                policy.policy_id == policy_id
                    && policy.version == version
                    && policy.review_mode == review_mode
            })
            .cloned())
    }

    async fn update_routing_policy_status(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
        status: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        let Some(policy) = policies.iter_mut().find(|policy| {
            policy.policy_id == policy_id
                && policy.version == version
                && policy.review_mode == review_mode
        }) else {
            return Ok(None);
        };
        policy.status = status.into();
        Ok(Some(policy.clone()))
    }

    async fn activate_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        if !policies.iter().any(|policy| {
            policy.policy_id == policy_id
                && policy.version == version
                && policy.review_mode == review_mode
        }) {
            return Ok(None);
        }
        for policy in policies
            .iter_mut()
            .filter(|policy| policy.review_mode == review_mode && policy.status == "active")
        {
            policy.status = "approved".into();
        }
        let policy = policies
            .iter_mut()
            .find(|policy| {
                policy.policy_id == policy_id
                    && policy.version == version
                    && policy.review_mode == review_mode
            })
            .expect("routing policy existence checked before activation");
        policy.status = "active".into();
        Ok(Some(policy.clone()))
    }

    async fn list_rules(&self) -> anyhow::Result<Vec<RuleSummaryRecord>> {
        let statuses = self.rule_statuses.lock().await;
        let backtests = self.rule_backtests.lock().await.clone();
        let mut details = default_rule_details();
        details.extend(self.candidate_rules.lock().await.values().cloned());
        let mut rules = details
            .into_iter()
            .map(|mut detail| {
                apply_rule_status(&mut detail, &statuses);
                if let Some(backtest) = latest_rule_backtest_for(
                    &backtests,
                    &detail.summary.rule_id,
                    detail.summary.latest_version,
                ) {
                    apply_rule_backtest_metadata(&mut detail.summary, Some(backtest));
                }
                detail.summary
            })
            .collect::<Vec<_>>();
        rules.sort_by(|left, right| left.rule_id.cmp(&right.rule_id));
        Ok(rules)
    }

    async fn list_active_rules(&self) -> anyhow::Result<Vec<Rule>> {
        let statuses = self.rule_statuses.lock().await;
        let mut details = default_rule_details();
        details.extend(self.candidate_rules.lock().await.values().cloned());
        details
            .into_iter()
            .filter_map(|mut detail| {
                apply_rule_status(&mut detail, &statuses);
                (detail.summary.status == "active").then_some(detail)
            })
            .map(runtime_rule_from_detail)
            .collect()
    }

    async fn get_rule(&self, rule_id: &str) -> anyhow::Result<Option<RuleDetailRecord>> {
        let statuses = self.rule_statuses.lock().await;
        let backtests = self.rule_backtests.lock().await.clone();
        let mut details = default_rule_details();
        details.extend(self.candidate_rules.lock().await.values().cloned());
        let audit_events = self.rule_audit_history(rule_id).await?;
        Ok(details
            .into_iter()
            .find(|detail| detail.summary.rule_id == rule_id)
            .map(|mut detail| {
                apply_rule_status(&mut detail, &statuses);
                if let Some(backtest) = latest_rule_backtest_for(
                    &backtests,
                    &detail.summary.rule_id,
                    detail.summary.latest_version,
                ) {
                    apply_rule_backtest_metadata(&mut detail.summary, Some(backtest));
                }
                detail.audit_events = audit_events;
                detail
            }))
    }

    async fn rule_audit_history(
        &self,
        rule_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        Ok(self
            .audit_events
            .lock()
            .await
            .iter()
            .filter(|event| event.payload["rule_id"].as_str() == Some(rule_id))
            .map(|event| AuditHistoryEventRecord {
                audit_id: event.audit_id.clone(),
                run_id: event.run_id.clone(),
                actor_role: event.actor_role.clone(),
                event_type: event.event_type.clone(),
                event_status: event.event_status.clone(),
                summary: event.summary.clone(),
                payload: event.payload.clone(),
                evidence_refs: evidence_values_to_strings(&event.evidence_refs),
                created_at: None,
            })
            .collect())
    }

    async fn save_rule_candidate(
        &self,
        rule: Rule,
        owner: String,
    ) -> anyhow::Result<RuleDetailRecord> {
        let detail = rule_detail_from_rule(rule, "draft", owner);
        self.candidate_rules
            .lock()
            .await
            .insert(detail.summary.rule_id.clone(), detail.clone());
        Ok(detail)
    }

    async fn update_rule_status(
        &self,
        rule_id: &str,
        status: &str,
    ) -> anyhow::Result<Option<RuleSummaryRecord>> {
        if self.get_rule(rule_id).await?.is_none() {
            return Ok(None);
        }
        self.rule_statuses
            .lock()
            .await
            .insert(rule_id.to_string(), status.to_string());
        Ok(self.get_rule(rule_id).await?.map(|detail| detail.summary))
    }

    async fn rule_performance(&self) -> anyhow::Result<Vec<RulePerformanceRecord>> {
        let rules = self.list_rules().await?;
        let runs = self.runs.lock().await;
        let pilot_events = self.pilot_audit_events.lock().await;
        let mut outcomes = HashMap::new();
        for (_, event) in pilot_events.iter() {
            if event.event_type != "investigation.result.received" {
                continue;
            }
            let Some(claim_id) = event.payload["claim_id"].as_str() else {
                continue;
            };
            outcomes.insert(
                claim_id.to_string(),
                InvestigationOutcome {
                    confirmed_fwa: event.payload["confirmed_fwa"].as_bool().unwrap_or(false),
                    saving_amount: decimal_from_json(&event.payload["saving_amount"]),
                },
            );
        }

        let mut accumulators = rule_accumulators_from_rules(&rules);
        for run in runs.iter() {
            for rule_run in &run.rule_runs {
                let Some(rule_id) = rule_run["rule_id"].as_str() else {
                    continue;
                };
                let Some(accumulator) = accumulators.get_mut(rule_id) else {
                    continue;
                };
                accumulator.trigger_count += 1;
                accumulator.triggered_claim_ids.insert(run.claim_id.clone());
            }
        }

        Ok(rule_performance_records(
            accumulators,
            &outcomes,
            runs.len() as u32,
        ))
    }

    async fn save_rule_backtest(
        &self,
        mut record: RuleBacktestRecord,
    ) -> anyhow::Result<RuleBacktestRecord> {
        record.created_at = Some(chrono::Utc::now().to_rfc3339());
        self.rule_backtests.lock().await.push(record.clone());
        Ok(record)
    }

    async fn latest_rule_backtest(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleBacktestRecord>> {
        Ok(self
            .rule_backtests
            .lock()
            .await
            .iter()
            .rev()
            .find(|record| record.rule_id == rule_id && record.rule_version == rule_version)
            .cloned())
    }

    async fn save_rule_promotion_review(
        &self,
        mut record: RulePromotionReviewRecord,
    ) -> anyhow::Result<RulePromotionReviewRecord> {
        record.created_at = Some(chrono::Utc::now().to_rfc3339());
        self.rule_promotion_reviews
            .lock()
            .await
            .push(record.clone());
        Ok(record)
    }

    async fn latest_rule_promotion_review(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RulePromotionReviewRecord>> {
        Ok(self
            .rule_promotion_reviews
            .lock()
            .await
            .iter()
            .rev()
            .find(|review| review.rule_id == rule_id && review.rule_version == rule_version)
            .cloned())
    }

    async fn list_leads(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<LeadRecord>> {
        let visible_claim_ids = match customer_scope_id {
            Some(scope) => Some(scoped_claim_ids_from_audit_events(
                self.audit_events.lock().await.iter(),
                scope,
            )),
            None => None,
        };
        let mut leads = self
            .leads
            .lock()
            .await
            .values()
            .filter(|lead| {
                visible_claim_ids
                    .as_ref()
                    .is_none_or(|claim_ids| claim_ids.contains(&lead.claim_id))
            })
            .cloned()
            .collect::<Vec<_>>();
        leads.sort_by(|left, right| left.lead_id.cmp(&right.lead_id));
        Ok(leads)
    }

    async fn triage_lead(
        &self,
        lead_id: &str,
        input: TriageLeadInput,
    ) -> anyhow::Result<Option<TriageLeadRecord>> {
        let mut leads = self.leads.lock().await;
        let visible_claim_ids = match input.customer_scope_id.as_deref() {
            Some(scope) => Some(scoped_claim_ids_from_audit_events(
                self.audit_events.lock().await.iter(),
                scope,
            )),
            None => None,
        };
        if input.decision == "merge_lead"
            && !merge_target_exists_in_memory(&leads, &input, visible_claim_ids.as_ref())
        {
            return Ok(None);
        }
        let Some(lead) = leads.get_mut(lead_id) else {
            return Ok(None);
        };
        if visible_claim_ids
            .as_ref()
            .is_some_and(|claim_ids| !claim_ids.contains(&lead.claim_id))
        {
            return Ok(None);
        }
        lead.status = triage_status_for_decision(&input.decision).into();
        lead.disposition = triage_disposition_for_decision(&input.decision).into();
        let lead = lead.clone();
        let case = if input.decision == "open_case" {
            let case = case_from_lead(&lead, &input);
            self.cases
                .lock()
                .await
                .insert(case.case_id.clone(), case.clone());
            Some(case)
        } else {
            None
        };
        let audit_id = AuditEventId::new().to_string();
        self.audit_events.lock().await.push(PersistedAuditEvent {
            audit_id: audit_id.clone(),
            run_id: lead.run_id.clone(),
            claim_id: lead.claim_id.clone(),
            source_system: lead.source_system.clone(),
            actor_id: input.assignee.clone(),
            actor_role: "fwa_operator".into(),
            event_type: "lead.triaged".into(),
            event_status: "succeeded".into(),
            summary: format!("Lead triaged: {}", input.decision),
            payload: triage_audit_payload(&lead, &input, case.as_ref()),
            evidence_refs: input
                .evidence_refs
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        });
        Ok(Some(TriageLeadRecord {
            lead,
            case,
            audit_id,
        }))
    }

    async fn list_cases(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<CaseRecord>> {
        let visible_claim_ids = match customer_scope_id {
            Some(scope) => Some(scoped_claim_ids_from_audit_events(
                self.audit_events.lock().await.iter(),
                scope,
            )),
            None => None,
        };
        let mut cases = self
            .cases
            .lock()
            .await
            .values()
            .filter(|case| {
                visible_claim_ids
                    .as_ref()
                    .is_none_or(|claim_ids| claim_ids.contains(&case.claim_id))
            })
            .cloned()
            .collect::<Vec<_>>();
        cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));
        Ok(cases)
    }

    async fn update_case_status(
        &self,
        case_id: &str,
        input: UpdateCaseStatusInput,
    ) -> anyhow::Result<Option<UpdateCaseStatusRecord>> {
        let mut cases = self.cases.lock().await;
        let Some(case) = cases.get_mut(case_id) else {
            return Ok(None);
        };
        if !self
            .claim_visible_to_scope(&case.claim_id, input.customer_scope_id.as_deref())
            .await
        {
            return Ok(None);
        }
        let from_status = case.status.clone();
        case.status = input.status.clone();
        if is_terminal_case_status(&case.status) {
            case.time_to_closure_hours = Some(0.0);
        } else {
            case.time_to_closure_hours = None;
        }
        let elapsed_hours = case.time_to_closure_hours.unwrap_or(0.0);
        case.sla_status = case_sla_status(&case.status, case.sla_target_hours, elapsed_hours);
        let case = case.clone();
        drop(cases);
        let audit_run_id = self
            .leads
            .lock()
            .await
            .get(&case.lead_id)
            .map(|lead| lead.run_id.clone())
            .unwrap_or_else(|| format!("case_status_{}", case.case_id));
        let audit_id = AuditEventId::new().to_string();
        self.audit_events.lock().await.push(PersistedAuditEvent {
            audit_id: audit_id.clone(),
            run_id: audit_run_id,
            claim_id: case.claim_id.clone(),
            source_system: case.source_system.clone(),
            actor_id: input.actor_id.clone(),
            actor_role: "fwa_operator".into(),
            event_type: "case.status.updated".into(),
            event_status: "succeeded".into(),
            summary: format!("Case status updated: {} -> {}", from_status, case.status),
            payload: serde_json::json!({
                "claim_id": case.claim_id,
                "case_id": case.case_id,
                "lead_id": case.lead_id,
                "from_status": from_status,
                "to_status": case.status,
                "notes": input.notes,
                "customer_scope_id": input.customer_scope_id
            }),
            evidence_refs: input
                .evidence_refs
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        });
        Ok(Some(UpdateCaseStatusRecord { case, audit_id }))
    }

    async fn create_audit_sample(
        &self,
        input: CreateAuditSampleInput,
    ) -> anyhow::Result<AuditSampleRecord> {
        let mut sequence = self.audit_sample_sequence.lock().await;
        *sequence += 1;
        let sample_id = format!("sample_{}", *sequence);
        let customer_scope_id = input.customer_scope_id.as_deref();
        let leads = if input.sample_mode == "random_control" {
            let visible_claim_ids = match customer_scope_id {
                Some(scope) => Some(scoped_claim_ids_from_audit_events(
                    self.audit_events.lock().await.iter(),
                    scope,
                )),
                None => None,
            };
            let claims = self.claims.lock().await;
            self.runs
                .lock()
                .await
                .iter()
                .filter(|run| {
                    visible_claim_ids
                        .as_ref()
                        .is_none_or(|claim_ids| claim_ids.contains(&run.claim_id))
                })
                .map(|run| control_lead_from_scoring_run(run, claims.get(&run.claim_id)))
                .collect()
        } else {
            self.list_leads(customer_scope_id).await?
        };
        let claims = self.claims.lock().await;
        let strata_contexts = audit_sample_strata_contexts_from_claims(&claims);
        drop(claims);
        let samples = self.audit_samples.lock().await;
        let reviewer_history = reviewer_lead_sample_counts(samples.values(), &input.reviewer);
        drop(samples);
        let sample = build_audit_sample(
            sample_id,
            input,
            leads,
            &strata_contexts,
            &reviewer_history,
            None,
        );
        self.audit_samples
            .lock()
            .await
            .insert(sample.sample_id.clone(), sample.clone());
        Ok(sample)
    }

    async fn list_audit_samples(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditSampleRecord>> {
        let mut samples = self
            .audit_samples
            .lock()
            .await
            .values()
            .filter(|sample| {
                customer_scope_id.is_none_or(|scope| sample.customer_scope_id == scope)
            })
            .cloned()
            .collect::<Vec<_>>();
        samples.sort_by(|left, right| left.sample_id.cmp(&right.sample_id));
        let reviews = self.list_qa_reviews(customer_scope_id).await?;
        Ok(with_sample_outcome_distributions(samples, &reviews))
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelVersionRecord>> {
        let statuses = self.model_statuses.lock().await;
        let mut models = default_model_versions();
        models.extend(self.model_versions.lock().await.values().cloned());
        models.sort_by(|left, right| {
            left.model_key
                .cmp(&right.model_key)
                .then_with(|| right.version.cmp(&left.version))
        });
        Ok(models
            .into_iter()
            .map(|mut model| {
                if let Some(status) =
                    statuses.get(&model_version_key(&model.model_key, &model.version))
                {
                    model.status = status.clone();
                }
                model
            })
            .collect())
    }

    async fn save_model_version(
        &self,
        record: ModelVersionRecord,
    ) -> anyhow::Result<ModelVersionRecord> {
        self.model_versions.lock().await.insert(
            model_version_key(&record.model_key, &record.version),
            record.clone(),
        );
        Ok(record)
    }

    async fn update_model_status(
        &self,
        model_key: &str,
        model_version: &str,
        status: &str,
    ) -> anyhow::Result<Option<ModelVersionRecord>> {
        let mut models = self.list_models().await?;
        let Some(model) = models
            .iter_mut()
            .find(|model| model.model_key == model_key && model.version == model_version)
        else {
            return Ok(None);
        };
        model.status = status.to_string();
        self.model_statuses.lock().await.insert(
            model_version_key(model_key, model_version),
            status.to_string(),
        );
        Ok(Some(model.clone()))
    }

    async fn model_performance(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Option<ModelPerformanceRecord>> {
        if default_model_versions()
            .iter()
            .any(|model| model.model_key == model_key)
        {
            let evaluations = self.model_evaluations.lock().await;
            let drift = evaluations
                .values()
                .filter(|evaluation| evaluation.model_key == model_key)
                .max_by(|left, right| left.evaluation_run_id.cmp(&right.evaluation_run_id))
                .map(|evaluation| drift_summary(&evaluation.metrics_json))
                .unwrap_or_else(|| drift_summary(&Value::Null));
            Ok(Some(model_performance_with_drift(
                empty_model_performance(model_key),
                drift,
            )))
        } else {
            Ok(None)
        }
    }

    async fn save_model_promotion_review(
        &self,
        mut record: ModelPromotionReviewRecord,
    ) -> anyhow::Result<ModelPromotionReviewRecord> {
        record.created_at = Some(chrono::Utc::now().to_rfc3339());
        self.model_promotion_reviews
            .lock()
            .await
            .push(record.clone());
        Ok(record)
    }

    async fn latest_model_promotion_review(
        &self,
        model_key: &str,
        model_version: &str,
    ) -> anyhow::Result<Option<ModelPromotionReviewRecord>> {
        Ok(self
            .model_promotion_reviews
            .lock()
            .await
            .iter()
            .rev()
            .find(|review| review.model_key == model_key && review.model_version == model_version)
            .cloned())
    }

    async fn save_model_retraining_job(
        &self,
        mut record: ModelRetrainingJobRecord,
    ) -> anyhow::Result<ModelRetrainingJobRecord> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut sequence = self.model_retraining_job_sequence.lock().await;
        *sequence += 1;
        record.job_id = format!("model_retraining_job_{}", *sequence);
        record.created_at = Some(now.clone());
        record.updated_at = Some(now);
        self.model_retraining_jobs
            .lock()
            .await
            .insert(record.job_id.clone(), record.clone());
        Ok(record)
    }

    async fn list_model_retraining_jobs(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Vec<ModelRetrainingJobRecord>> {
        let mut jobs = self
            .model_retraining_jobs
            .lock()
            .await
            .values()
            .filter(|job| job.model_key == model_key)
            .cloned()
            .collect::<Vec<_>>();
        jobs.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(jobs)
    }

    async fn get_model_retraining_job(
        &self,
        job_id: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        Ok(self.model_retraining_jobs.lock().await.get(job_id).cloned())
    }

    async fn claim_next_model_retraining_job(
        &self,
        model_key: Option<&str>,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let mut jobs = self.model_retraining_jobs.lock().await;
        let next_job_id = jobs
            .values()
            .filter(|job| job.status == "queued")
            .filter(|job| model_key.map(|key| job.model_key == key).unwrap_or(true))
            .min_by(|left, right| left.created_at.cmp(&right.created_at))
            .map(|job| job.job_id.clone());
        let Some(job_id) = next_job_id else {
            return Ok(None);
        };
        let Some(job) = jobs.get_mut(&job_id) else {
            return Ok(None);
        };
        job.status = "running".into();
        job.updated_by = actor.to_string();
        job.status_note = status_note.to_string();
        job.updated_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(Some(job.clone()))
    }

    async fn update_model_retraining_job_status(
        &self,
        job_id: &str,
        status: &str,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let mut jobs = self.model_retraining_jobs.lock().await;
        let Some(job) = jobs.get_mut(job_id) else {
            return Ok(None);
        };
        job.status = status.to_string();
        job.updated_by = actor.to_string();
        job.status_note = status_note.to_string();
        job.updated_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(Some(job.clone()))
    }

    async fn complete_model_retraining_job(
        &self,
        input: CompleteModelRetrainingJobInput<'_>,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let mut jobs = self.model_retraining_jobs.lock().await;
        let Some(job) = jobs.get_mut(input.job_id) else {
            return Ok(None);
        };
        job.status = "completed".into();
        job.updated_by = input.actor.to_string();
        job.status_note = input.status_note.to_string();
        job.candidate_model_version = Some(input.candidate_model_version.to_string());
        job.candidate_artifact_uri = Some(input.candidate_artifact_uri.to_string());
        job.candidate_endpoint_url = input.candidate_endpoint_url.map(ToString::to_string);
        job.validation_report_uri = Some(input.validation_report_uri.to_string());
        job.output_evaluation_id = Some(input.output_evaluation_id.to_string());
        job.updated_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(Some(job.clone()))
    }

    async fn dashboard_summary(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<DashboardSummaryRecord> {
        let runs = self.runs.lock().await;
        let claims = self.claims.lock().await;
        let pilot_events = self.pilot_audit_events.lock().await;
        let saving_attribution_records = self.saving_attributions.lock().await.clone();

        let mut risk_amount = Decimal::ZERO;
        let mut rag_distribution = BTreeMap::new();
        let mut model_accumulators = BTreeMap::<String, (u32, u32, u32)>::new();
        let mut layer_accumulators = BTreeMap::<String, (String, u32, u32, u32)>::new();
        let mut rule_hits = 0_u32;

        let scoped_runs = runs
            .iter()
            .filter(|run| {
                customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&run.audit_event, scope)
                })
            })
            .collect::<Vec<_>>();
        let scoped_pilot_events = pilot_events
            .iter()
            .filter(|(_, event)| {
                customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&event.payload, scope)
                })
            })
            .collect::<Vec<_>>();
        let scoped_claim_ids = scoped_runs
            .iter()
            .map(|run| run.claim_id.clone())
            .chain(
                scoped_pilot_events
                    .iter()
                    .map(|(claim_id, _)| claim_id.clone()),
            )
            .collect::<BTreeSet<_>>();
        let scoped_saving_attributions = saving_attribution_records
            .iter()
            .filter(|attribution| {
                customer_scope_id.is_none() || scoped_claim_ids.contains(&attribution.claim_id)
            })
            .cloned()
            .collect::<Vec<_>>();

        for run in scoped_runs.iter() {
            if run.risk_score >= 70 {
                if let Some(context) = claims.get(&run.claim_id) {
                    risk_amount += context.claim.amount.amount;
                }
            }
            *rag_distribution.entry(run.rag.clone()).or_insert(0) += 1;
            rule_hits += run.rule_runs.len() as u32;

            let model_key = run.model_score["model_key"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let score = run.model_score["score"].as_u64().unwrap_or(0) as u32;
            let entry = model_accumulators.entry(model_key).or_insert((0, 0, 0));
            entry.0 += 1;
            entry.1 += score;
            if score >= 70 {
                entry.2 += 1;
            }

            for layer in run
                .audit_event
                .get("layers")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                let layer_id = layer["layer_id"].as_str().unwrap_or("UNKNOWN").to_string();
                let layer_name = layer["name"].as_str().unwrap_or("Unknown").to_string();
                let layer_score = layer["score"].as_u64().unwrap_or(0) as u32;
                let entry =
                    layer_accumulators
                        .entry(layer_id)
                        .or_insert((layer_name.clone(), 0, 0, 0));
                entry.0 = layer_name;
                entry.1 += 1;
                entry.2 += layer_score;
                if layer_score >= 70 {
                    entry.3 += 1;
                }
            }
        }

        let mut saving_amount = Decimal::ZERO;
        let mut confirmed_fwa = 0_u32;
        let mut investigation_results = 0_u32;
        let mut qa_reviews = 0_u32;
        let mut outcome_labels = Vec::new();
        let mut financial_impacts = Vec::new();
        let feedback_statuses = latest_qa_feedback_statuses(
            &scoped_pilot_events
                .iter()
                .map(|(claim_id, event)| ((*claim_id).clone(), (*event).clone()))
                .collect::<Vec<_>>(),
        );

        for (_, event) in scoped_pilot_events.iter() {
            match event.event_type.as_str() {
                "investigation.result.received" => {
                    investigation_results += 1;
                    if let Ok(record) =
                        serde_json::from_value::<InvestigationResultRecord>(event.payload.clone())
                    {
                        outcome_labels.extend(labels_from_investigation_result(record.clone()));
                        if let Some(impact) = financial_impact_from_investigation(&record) {
                            financial_impacts.push(impact);
                        }
                    }
                    if event.payload["confirmed_fwa"].as_bool().unwrap_or(false) {
                        confirmed_fwa += 1;
                    }
                    if let Some(value) = event.payload["saving_amount"].as_str() {
                        saving_amount += value.parse::<Decimal>().unwrap_or(Decimal::ZERO);
                    }
                }
                "qa.result.received" => {
                    qa_reviews += 1;
                    if let Ok(record) =
                        serde_json::from_value::<QaReviewRecord>(event.payload.clone())
                    {
                        let feedback_id = qa_feedback_id(&record.qa_case_id);
                        let feedback_status = feedback_statuses
                            .get(&feedback_id)
                            .map(|update| update.status.as_str())
                            .unwrap_or("open");
                        outcome_labels.push(label_from_qa_review(record, feedback_status));
                    }
                }
                "medical.review.recorded" => {
                    outcome_labels.extend(labels_from_medical_review_event(event));
                }
                _ => {}
            }
        }
        let runtime_events = self.audit_events.lock().await;
        let scoring_audit_runs = runtime_events
            .iter()
            .filter(|event| {
                event.event_type == "scoring.completed"
                    && event.event_status == "succeeded"
                    && customer_scope_id.is_none_or(|scope| {
                        audit_event_payload_matches_customer_scope(&event.payload, scope)
                    })
            })
            .count() as u32;
        let canonical_trace_audit_runs = runtime_events
            .iter()
            .filter(|event| {
                event.event_type == "scoring.completed"
                    && event.event_status == "succeeded"
                    && customer_scope_id.is_none_or(|scope| {
                        audit_event_payload_matches_customer_scope(&event.payload, scope)
                    })
                    && event
                        .payload
                        .get("canonical_claim_context_trace")
                        .and_then(Value::as_object)
                        .is_some()
            })
            .count() as u32;
        let audit_coverage =
            summarize_dashboard_audit_coverage(scoring_audit_runs, canonical_trace_audit_runs);
        for event in runtime_events.iter().filter(|event| {
            event.event_type == "medical.review.recorded"
                && customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&event.payload, scope)
                })
        }) {
            let audit_event = AuditHistoryEventRecord {
                audit_id: event.audit_id.clone(),
                run_id: event.run_id.clone(),
                actor_role: event.actor_role.clone(),
                event_type: event.event_type.clone(),
                event_status: event.event_status.clone(),
                summary: event.summary.clone(),
                payload: event.payload.clone(),
                evidence_refs: evidence_values_to_strings(&event.evidence_refs),
                created_at: None,
            };
            outcome_labels.extend(labels_from_medical_review_event(&audit_event));
        }
        let suspected_claims = scoped_runs
            .iter()
            .filter(|run| run.risk_score >= 70)
            .count() as u32;
        let saving_attributions = summarize_saving_attributions(&scoped_saving_attributions);
        drop(runtime_events);
        drop(pilot_events);
        drop(claims);
        drop(runs);

        let audit_samples = self.list_audit_samples(customer_scope_id).await?;
        let qa_review_records = self.list_qa_reviews(customer_scope_id).await?;
        let qa_feedback_items = self.list_qa_feedback_items(customer_scope_id).await?;
        let cases = self.list_cases(customer_scope_id).await?;
        let agent_runs = self.list_agent_runs(customer_scope_id).await?;
        let models = self.list_models().await?;
        let model_evaluations = self.list_model_evaluations().await?;
        let rules = self.list_rules().await?;
        let rule_performance = self.rule_performance().await?;
        let leads = self.list_leads(customer_scope_id).await?;
        let scheme_distribution = leads
            .iter()
            .fold(BTreeMap::new(), |mut distribution, lead| {
                *distribution.entry(lead.scheme_family.clone()).or_insert(0) += 1;
                distribution
            });
        let saving_segments = summarize_saving_segments(&scoped_saving_attributions, &leads);
        let false_positive_count = rule_performance
            .iter()
            .map(|record| record.false_positive_count)
            .sum::<u32>();
        let value_measurement = summarize_dashboard_value_measurement(
            &financial_impacts,
            rule_hits,
            false_positive_count,
        );

        Ok(DashboardSummaryRecord {
            suspected_claims,
            confirmed_fwa,
            risk_amount: risk_amount.to_string(),
            saving_amount: saving_amount.to_string(),
            rag_distribution,
            scheme_distribution,
            rule_hits,
            model_scores: model_accumulators
                .into_iter()
                .map(|(model_key, (scored_runs, score_sum, high_risk_count))| {
                    let average_score = if scored_runs == 0 {
                        0.0
                    } else {
                        score_sum as f64 / scored_runs as f64
                    };
                    (
                        model_key,
                        DashboardModelScoreRecord {
                            scored_runs,
                            average_score,
                            high_risk_count,
                        },
                    )
                })
                .collect(),
            layer_scores: layer_accumulators
                .into_iter()
                .map(
                    |(layer_id, (name, scored_runs, score_sum, high_risk_count))| {
                        let average_score = if scored_runs == 0 {
                            0.0
                        } else {
                            score_sum as f64 / scored_runs as f64
                        };
                        (
                            layer_id,
                            DashboardLayerScoreRecord {
                                name,
                                scored_runs,
                                average_score,
                                high_risk_count,
                            },
                        )
                    },
                )
                .collect(),
            saving_attributions,
            saving_segments,
            value_measurement,
            audit_coverage,
            label_pool: summarize_dashboard_label_pool(&outcome_labels),
            qa_queue: summarize_dashboard_qa_queue(
                &audit_samples,
                &qa_review_records,
                &qa_feedback_items,
            ),
            case_sla: summarize_dashboard_case_sla(&cases),
            agent_governance: summarize_dashboard_agent_governance(&agent_runs),
            model_governance: summarize_dashboard_model_governance(&models, &model_evaluations),
            rule_governance: summarize_dashboard_rule_governance(&rules, &rule_performance),
            investigation_results,
            qa_reviews,
        })
    }

    async fn provider_risk_summary(&self) -> anyhow::Result<ProviderRiskSummaryRecord> {
        let runs = self.runs.lock().await;
        Ok(summarize_provider_risk_profiles(
            runs.iter().map(|run| &run.audit_event),
        ))
    }

    async fn list_knowledge_cases(&self) -> anyhow::Result<Vec<KnowledgeCaseRecord>> {
        let mut cases = default_knowledge_cases()
            .into_iter()
            .map(|case| (case.case_id.clone(), case))
            .collect::<HashMap<_, _>>();
        cases.extend(self.knowledge_cases.lock().await.clone());
        let mut cases = cases.into_values().collect::<Vec<_>>();
        cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));
        Ok(cases)
    }

    async fn save_knowledge_case(
        &self,
        record: KnowledgeCaseRecord,
    ) -> anyhow::Result<KnowledgeCaseRecord> {
        self.knowledge_cases
            .lock()
            .await
            .insert(record.case_id.clone(), record.clone());
        Ok(record)
    }

    async fn search_similar_cases(
        &self,
        query: SimilarCaseQuery,
    ) -> anyhow::Result<Vec<SimilarCaseRecord>> {
        Ok(search_cases(self.list_knowledge_cases().await?, &query))
    }

    async fn save_agent_run(&self, run: PersistedAgentRun) -> anyhow::Result<()> {
        self.agent_runs.lock().await.push(run);
        Ok(())
    }

    async fn list_agent_runs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AgentRunLogRecord>> {
        let scoped_claim_ids = match customer_scope_id {
            Some(scope) => Some(scoped_claim_ids_from_audit_events(
                self.audit_events.lock().await.iter(),
                scope,
            )),
            None => None,
        };
        let mut runs = self
            .agent_runs
            .lock()
            .await
            .iter()
            .filter(|run| {
                scoped_claim_ids
                    .as_ref()
                    .is_none_or(|claim_ids| claim_ids.contains(&run.claim_id))
            })
            .map(agent_run_log_from_persisted)
            .collect::<Vec<_>>();
        runs.sort_by(|left, right| left.agent_run_id.cmp(&right.agent_run_id));
        Ok(runs)
    }

    async fn save_agent_approval(
        &self,
        approval: AgentApprovalRecord,
    ) -> anyhow::Result<AgentApprovalRecord> {
        let mut runs = self.agent_runs.lock().await;
        let Some(run) = runs
            .iter_mut()
            .find(|run| run.agent_run_id == approval.agent_run_id)
        else {
            anyhow::bail!("agent run not found: {}", approval.agent_run_id);
        };
        if let Some(existing) = run
            .approvals
            .iter_mut()
            .find(|existing| existing.approval_id == approval.approval_id)
        {
            *existing = approval.clone();
        } else {
            run.approvals.push(approval.clone());
        }
        Ok(approval)
    }

    async fn register_dataset(&self, input: RegisterDatasetInput) -> anyhow::Result<DatasetRecord> {
        let mut sequence = self.dataset_sequence.lock().await;
        *sequence += 1;
        let dataset_id = format!("dataset_{}", *sequence);
        let record = DatasetRecord {
            dataset_id: dataset_id.clone(),
            source_key: input.source_key,
            display_name: input.display_name,
            business_domain: input.business_domain,
            dataset_key: input.dataset_key,
            dataset_version: input.dataset_version,
            sample_grain: input.sample_grain,
            label_column: input.label_column,
            entity_keys: input.entity_keys,
            manifest_uri: input.manifest_uri,
            schema_uri: input.schema_uri,
            profile_uri: input.profile_uri,
            storage_format: input.storage_format,
            schema_hash: input.schema_hash,
            row_count: input.row_count,
            status: input.status,
            splits: input.splits,
            fields: input.fields,
            mappings: vec![],
        };
        self.datasets
            .lock()
            .await
            .insert(dataset_id, record.clone());
        Ok(record)
    }

    async fn list_datasets(&self) -> anyhow::Result<Vec<DatasetRecord>> {
        let mut datasets = self
            .datasets
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        datasets.sort_by(|left, right| left.dataset_key.cmp(&right.dataset_key));
        Ok(datasets)
    }

    async fn get_dataset(&self, dataset_id: &str) -> anyhow::Result<Option<DatasetRecord>> {
        Ok(self.datasets.lock().await.get(dataset_id).cloned())
    }

    async fn add_field_mapping(
        &self,
        dataset_id: &str,
        input: CreateFieldMappingInput,
    ) -> anyhow::Result<Option<FieldMappingRecord>> {
        let mut datasets = self.datasets.lock().await;
        let Some(dataset) = datasets.get_mut(dataset_id) else {
            return Ok(None);
        };
        let mut sequence = self.mapping_sequence.lock().await;
        *sequence += 1;
        let mapping = FieldMappingRecord {
            mapping_id: format!("mapping_{}", *sequence),
            dataset_id: dataset_id.to_string(),
            external_field: input.external_field,
            canonical_target: input.canonical_target,
            feature_name: input.feature_name,
            transform_kind: input.transform_kind,
            transform_json: input.transform_json,
            status: input.status,
        };
        dataset.mappings.push(mapping.clone());
        Ok(Some(mapping))
    }

    async fn save_investigation_result(
        &self,
        record: InvestigationResultRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        let saving_attributions = derive_saving_attributions(&record);
        let audit_id = format!("audit_investigation_{}", record.investigation_id);
        let previous_case_id = {
            let events = self.pilot_audit_events.lock().await;
            events
                .iter()
                .find(|(_, event)| event.audit_id == audit_id)
                .and_then(|(_, event)| event.payload["case_id"].as_str())
                .map(str::to_string)
        };
        let mut cases = self.cases.lock().await;
        if previous_case_id.as_deref() != record.case_id.as_deref() {
            if let Some(case_id) = previous_case_id.as_deref() {
                if let Some(case) = cases.get_mut(case_id) {
                    if case.investigation_result_id.as_deref()
                        == Some(record.investigation_id.as_str())
                    {
                        case.final_outcome = None;
                        case.reviewer_notes = None;
                        case.investigation_result_id = None;
                    }
                }
            }
        }
        if let Some(case_id) = record.case_id.as_deref() {
            if let Some(case) = cases.get_mut(case_id) {
                case.final_outcome = Some(record.outcome.clone());
                case.reviewer_notes = Some(record.notes.clone());
                case.investigation_result_id = Some(record.investigation_id.clone());
            } else {
                anyhow::bail!("case not found for investigation result: {case_id}");
            }
        }
        drop(cases);
        let event = AuditHistoryEventRecord {
            audit_id,
            run_id: format!("pilot_investigation_{}", record.investigation_id),
            actor_role: record
                .actor_role
                .clone()
                .unwrap_or_else(|| "tpa_system".into()),
            event_type: "investigation.result.received".into(),
            event_status: "succeeded".into(),
            summary: format!("Investigation result received: {}", record.outcome),
            payload: serde_json::to_value(&record)?,
            evidence_refs: record.evidence_refs.clone(),
            created_at: None,
        };
        upsert_pilot_audit_event(
            &self.pilot_audit_events,
            record.claim_id.clone(),
            event.clone(),
        )
        .await;
        let mut stored_attributions = self.saving_attributions.lock().await;
        stored_attributions
            .retain(|attribution| attribution.investigation_id != record.investigation_id);
        stored_attributions.extend(saving_attributions);
        Ok(event)
    }

    async fn save_qa_review(
        &self,
        mut record: QaReviewRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        record.feedback_target = canonical_feedback_target(&record.feedback_target).into();
        let event = AuditHistoryEventRecord {
            audit_id: format!("audit_qa_{}", record.qa_case_id),
            run_id: format!("pilot_qa_{}", record.qa_case_id),
            actor_role: record
                .actor_role
                .clone()
                .unwrap_or_else(|| "tpa_system".into()),
            event_type: "qa.result.received".into(),
            event_status: "succeeded".into(),
            summary: format!("QA result received: {}", record.qa_conclusion),
            payload: serde_json::to_value(&record)?,
            evidence_refs: record.evidence_refs.clone(),
            created_at: None,
        };
        upsert_pilot_audit_event(&self.pilot_audit_events, record.claim_id, event.clone()).await;
        Ok(event)
    }

    async fn list_qa_feedback_items(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaFeedbackItemRecord>> {
        let events = self.pilot_audit_events.lock().await.clone();
        let scoped_events = events
            .into_iter()
            .filter(|(_, event)| {
                customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&event.payload, scope)
                })
            })
            .collect::<Vec<_>>();
        let feedback_statuses = latest_qa_feedback_statuses(&scoped_events);
        let mut items = scoped_events
            .iter()
            .filter_map(|(_, event)| {
                (event.event_type == "qa.result.received")
                    .then(|| serde_json::from_value::<QaReviewRecord>(event.payload.clone()).ok())
                    .flatten()
            })
            .filter(|review| review.qa_conclusion != "pass")
            .map(|review| {
                let feedback_id = qa_feedback_id(&review.qa_case_id);
                let status_update = feedback_statuses.get(&feedback_id);
                let status = status_update
                    .map(|update| update.status.as_str())
                    .unwrap_or("open");
                qa_review_to_feedback_item(review, None, status, status_update)
            })
            .collect::<Vec<_>>();
        sort_qa_feedback_items(&mut items);
        Ok(items)
    }

    async fn update_qa_feedback_status(
        &self,
        feedback_id: &str,
        input: UpdateQaFeedbackStatusInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<UpdateQaFeedbackStatusRecord>> {
        let Some(mut item) = self
            .list_qa_feedback_items(customer_scope_id)
            .await?
            .into_iter()
            .find(|item| item.feedback_id == feedback_id)
        else {
            return Ok(None);
        };
        let from_status = item.status.clone();
        item.status = input.status.clone();
        let audit_id = AuditEventId::new().to_string();
        item.status_updated_by = Some(input.actor_id.clone());
        item.status_audit_id = Some(audit_id.clone());
        item.status_updated_at = None;
        item.status_evidence_refs = input.evidence_refs.clone();
        let event = AuditHistoryEventRecord {
            audit_id: audit_id.clone(),
            run_id: format!("qa_feedback_status_{}", item.feedback_id),
            actor_role: "fwa_operator".into(),
            event_type: "qa.feedback.status.updated".into(),
            event_status: "succeeded".into(),
            summary: format!(
                "QA feedback status updated: {} -> {}",
                from_status, item.status
            ),
            payload: serde_json::json!({
                "feedback_id": item.feedback_id,
                "qa_case_id": item.qa_case_id,
                "claim_id": item.claim_id,
                "feedback_target": item.feedback_target,
                "from_status": from_status,
                "to_status": item.status,
                "actor_id": input.actor_id,
                "notes": input.notes,
                "customer_scope_id": input.customer_scope_id
            }),
            evidence_refs: input.evidence_refs,
            created_at: None,
        };
        self.pilot_audit_events
            .lock()
            .await
            .push((item.claim_id.clone(), event));
        Ok(Some(UpdateQaFeedbackStatusRecord { item, audit_id }))
    }

    async fn list_qa_reviews(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaReviewRecord>> {
        let mut reviews = self
            .pilot_audit_events
            .lock()
            .await
            .iter()
            .filter(|(_, event)| {
                customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&event.payload, scope)
                })
            })
            .filter_map(|(_, event)| {
                (event.event_type == "qa.result.received")
                    .then(|| serde_json::from_value::<QaReviewRecord>(event.payload.clone()).ok())
                    .flatten()
            })
            .map(|mut review| {
                review.feedback_target = canonical_feedback_target(&review.feedback_target).into();
                review
            })
            .collect::<Vec<_>>();
        reviews.sort_by(|left, right| left.qa_case_id.cmp(&right.qa_case_id));
        Ok(reviews)
    }

    async fn list_outcome_labels(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<OutcomeLabelRecord>> {
        let events = self.pilot_audit_events.lock().await.clone();
        let scoped_events = events
            .into_iter()
            .filter(|(_, event)| {
                customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&event.payload, scope)
                })
            })
            .collect::<Vec<_>>();
        let feedback_statuses = latest_qa_feedback_statuses(&scoped_events);
        let mut labels = scoped_events
            .iter()
            .filter_map(|(_, event)| match event.event_type.as_str() {
                "investigation.result.received" => {
                    serde_json::from_value::<InvestigationResultRecord>(event.payload.clone())
                        .ok()
                        .map(labels_from_investigation_result)
                }
                "qa.result.received" => {
                    serde_json::from_value::<QaReviewRecord>(event.payload.clone())
                        .ok()
                        .map(|review| {
                            let feedback_id = qa_feedback_id(&review.qa_case_id);
                            let feedback_status = feedback_statuses
                                .get(&feedback_id)
                                .map(|update| update.status.as_str())
                                .unwrap_or("open");
                            vec![label_from_qa_review(review, feedback_status)]
                        })
                }
                "medical.review.recorded" => Some(labels_from_medical_review_event(event)),
                _ => None,
            })
            .flatten()
            .collect::<Vec<_>>();
        labels.extend(
            self.audit_events
                .lock()
                .await
                .iter()
                .filter(|event| {
                    event.event_type == "medical.review.recorded"
                        && customer_scope_id.is_none_or(|scope| {
                            audit_event_payload_matches_customer_scope(&event.payload, scope)
                        })
                })
                .flat_map(|event| {
                    labels_from_medical_review_event(&AuditHistoryEventRecord {
                        audit_id: event.audit_id.clone(),
                        run_id: event.run_id.clone(),
                        actor_role: event.actor_role.clone(),
                        event_type: event.event_type.clone(),
                        event_status: event.event_status.clone(),
                        summary: event.summary.clone(),
                        payload: event.payload.clone(),
                        evidence_refs: evidence_values_to_strings(&event.evidence_refs),
                        created_at: None,
                    })
                }),
        );
        labels.extend(
            self.audit_events
                .lock()
                .await
                .iter()
                .filter(|event| {
                    event.event_type == "label.bootstrap.reviewed"
                        && event.event_status == "succeeded"
                        && customer_scope_id.is_none_or(|scope| {
                            audit_event_payload_matches_customer_scope(&event.payload, scope)
                        })
                })
                .filter_map(|event| {
                    label_from_bootstrap_review_event(&audit_history_from_persisted(event))
                }),
        );
        let lead_triage_events = self
            .audit_events
            .lock()
            .await
            .iter()
            .filter(|event| {
                event.event_type == "lead.triaged"
                    && event.event_status == "succeeded"
                    && customer_scope_id.is_none_or(|scope| {
                        audit_event_payload_matches_customer_scope(&event.payload, scope)
                    })
            })
            .map(audit_history_from_persisted)
            .collect::<Vec<_>>();
        labels.extend(labels_from_lead_triage_events(lead_triage_events));
        labels.extend(
            self.list_cases(customer_scope_id)
                .await?
                .into_iter()
                .flat_map(labels_from_case_status),
        );
        sort_outcome_labels(&mut labels);
        Ok(labels)
    }

    async fn claim_audit_history(
        &self,
        claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        let mut events = self
            .audit_events
            .lock()
            .await
            .iter()
            .filter(|event| event.claim_id == claim_id)
            .filter(|event| {
                customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&event.payload, scope)
                })
            })
            .map(|event| AuditHistoryEventRecord {
                audit_id: event.audit_id.clone(),
                run_id: event.run_id.clone(),
                actor_role: event.actor_role.clone(),
                event_type: event.event_type.clone(),
                event_status: event.event_status.clone(),
                summary: event.summary.clone(),
                payload: event.payload.clone(),
                evidence_refs: evidence_values_to_strings(&event.evidence_refs),
                created_at: None,
            })
            .collect::<Vec<_>>();

        events.extend(
            self.pilot_audit_events
                .lock()
                .await
                .iter()
                .filter(|(event_claim_id, event)| {
                    event_claim_id == claim_id
                        && customer_scope_id.is_none_or(|scope| {
                            audit_event_payload_matches_customer_scope(&event.payload, scope)
                        })
                })
                .map(|(_, event)| event.clone()),
        );
        Ok(events)
    }

    async fn list_audit_events(
        &self,
        filter: AuditEventListFilter,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        let mut events = self
            .audit_events
            .lock()
            .await
            .iter()
            .filter(|event| persisted_audit_event_matches_filter(event, &filter))
            .map(|event| AuditHistoryEventRecord {
                audit_id: event.audit_id.clone(),
                run_id: event.run_id.clone(),
                actor_role: event.actor_role.clone(),
                event_type: event.event_type.clone(),
                event_status: event.event_status.clone(),
                summary: event.summary.clone(),
                payload: event.payload.clone(),
                evidence_refs: evidence_values_to_strings(&event.evidence_refs),
                created_at: None,
            })
            .collect::<Vec<_>>();
        events.extend(
            self.pilot_audit_events
                .lock()
                .await
                .iter()
                .filter(|(claim_id, event)| {
                    pilot_audit_event_matches_filter(claim_id, event, &filter)
                })
                .map(|(_, event)| event.clone()),
        );
        events.reverse();
        events.truncate(filter.limit as usize);
        Ok(events)
    }

    async fn list_webhook_events(&self) -> anyhow::Result<Vec<WebhookEventRecord>> {
        let mut events = self
            .audit_events
            .lock()
            .await
            .iter()
            .filter_map(|event| {
                let audit_event = AuditHistoryEventRecord {
                    audit_id: event.audit_id.clone(),
                    run_id: event.run_id.clone(),
                    actor_role: event.actor_role.clone(),
                    event_type: event.event_type.clone(),
                    event_status: event.event_status.clone(),
                    summary: event.summary.clone(),
                    payload: event.payload.clone(),
                    evidence_refs: evidence_values_to_strings(&event.evidence_refs),
                    created_at: None,
                };
                webhook_event_from_audit(Some(event.claim_id.as_str()), &audit_event)
            })
            .collect::<Vec<_>>();

        events.extend(
            self.pilot_audit_events
                .lock()
                .await
                .iter()
                .filter_map(|(claim_id, event)| webhook_event_from_audit(Some(claim_id), event)),
        );
        let attempts = self
            .webhook_delivery_attempts
            .lock()
            .await
            .values()
            .flat_map(|records| records.iter().cloned())
            .collect::<Vec<_>>();
        apply_webhook_delivery_state(&mut events, &attempts);
        sort_webhook_events(&mut events);
        Ok(events)
    }

    async fn save_webhook_delivery_attempt(
        &self,
        input: WebhookDeliveryAttemptInput,
    ) -> anyhow::Result<WebhookDeliveryAttemptRecord> {
        let attempted_at = chrono::Utc::now();
        let mut attempts = self.webhook_delivery_attempts.lock().await;
        let event_attempts = attempts.entry(input.event_id.clone()).or_default();
        let attempt_number = event_attempts.len() as u32 + 1;
        let record = WebhookDeliveryAttemptRecord {
            event_id: input.event_id,
            attempt_number,
            delivery_status: input.delivery_status.clone(),
            response_status_code: input.response_status_code,
            error_message: input.error_message,
            next_attempt_at: next_webhook_attempt_at(
                &input.delivery_status,
                attempt_number,
                attempted_at,
            )
            .map(|timestamp| timestamp.to_rfc3339()),
            attempted_at: Some(attempted_at.to_rfc3339()),
        };
        event_attempts.push(record.clone());
        Ok(record)
    }

    async fn register_feature_set(
        &self,
        input: RegisterFeatureSetInput,
    ) -> anyhow::Result<Option<FeatureSetRecord>> {
        if self.get_dataset(&input.dataset_id).await?.is_none() {
            return Ok(None);
        }
        let mut sequence = self.feature_set_sequence.lock().await;
        *sequence += 1;
        let feature_set_id = format!("feature_set_{}", *sequence);
        let record = FeatureSetRecord {
            feature_set_id: feature_set_id.clone(),
            business_domain: input.business_domain,
            feature_set_key: input.feature_set_key,
            version: input.version,
            dataset_id: input.dataset_id,
            features_uri: input.features_uri,
            feature_list_json: input.feature_list_json,
            row_count: input.row_count,
            label_column: input.label_column,
            status: input.status,
        };
        self.feature_sets
            .lock()
            .await
            .insert(feature_set_id, record.clone());
        Ok(Some(record))
    }

    async fn register_model_dataset(
        &self,
        input: RegisterModelDatasetInput,
    ) -> anyhow::Result<Option<ModelDatasetRecord>> {
        if !self
            .feature_sets
            .lock()
            .await
            .contains_key(&input.feature_set_id)
        {
            return Ok(None);
        }
        let mut sequence = self.model_dataset_sequence.lock().await;
        *sequence += 1;
        let model_dataset_id = format!("model_dataset_{}", *sequence);
        let record = ModelDatasetRecord {
            model_dataset_id: model_dataset_id.clone(),
            business_domain: input.business_domain,
            task_type: input.task_type,
            label_name: input.label_name,
            feature_set_id: input.feature_set_id,
            train_uri: input.train_uri,
            validation_uri: input.validation_uri,
            test_uri: input.test_uri,
            row_counts_json: input.row_counts_json,
            label_distribution_json: input.label_distribution_json,
            status: input.status,
        };
        self.model_datasets
            .lock()
            .await
            .insert(model_dataset_id, record.clone());
        Ok(Some(record))
    }

    async fn get_model_dataset_source_dataset(
        &self,
        model_dataset_id: &str,
    ) -> anyhow::Result<Option<DatasetRecord>> {
        let model_dataset = self
            .model_datasets
            .lock()
            .await
            .get(model_dataset_id)
            .cloned();
        let Some(model_dataset) = model_dataset else {
            return Ok(None);
        };
        let feature_set = self
            .feature_sets
            .lock()
            .await
            .get(&model_dataset.feature_set_id)
            .cloned();
        let Some(feature_set) = feature_set else {
            return Ok(None);
        };
        self.get_dataset(&feature_set.dataset_id).await
    }

    async fn register_model_evaluation(
        &self,
        input: RegisterModelEvaluationInput,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        if !self
            .model_datasets
            .lock()
            .await
            .contains_key(&input.model_dataset_id)
        {
            return Ok(None);
        }
        let record = ModelEvaluationRecord {
            evaluation_run_id: input.evaluation_run_id,
            model_key: input.model_key,
            model_version: input.model_version,
            model_dataset_id: input.model_dataset_id,
            scheme_family: input.scheme_family,
            auc: input.auc,
            ks: input.ks,
            precision: input.precision,
            recall: input.recall,
            f1: input.f1,
            accuracy: input.accuracy,
            threshold: input.threshold,
            confusion_matrix_json: input.confusion_matrix_json,
            feature_importance_uri: input.feature_importance_uri,
            metrics_json: input.metrics_json,
        };
        self.model_evaluations
            .lock()
            .await
            .insert(record.evaluation_run_id.clone(), record.clone());
        Ok(Some(record))
    }

    async fn get_model_evaluation(
        &self,
        evaluation_run_id: &str,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        Ok(self
            .model_evaluations
            .lock()
            .await
            .get(evaluation_run_id)
            .cloned())
    }

    async fn list_model_evaluations(&self) -> anyhow::Result<Vec<ModelEvaluationRecord>> {
        let mut evaluations = self
            .model_evaluations
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        evaluations.sort_by(|left, right| left.evaluation_run_id.cmp(&right.evaluation_run_id));
        Ok(evaluations)
    }

    async fn save_evidence_document(
        &self,
        input: CreateEvidenceDocumentInput,
    ) -> anyhow::Result<EvidenceDocumentRecord> {
        let record = EvidenceDocumentRecord {
            document_id: input.document_id,
            customer_scope_id: input.customer_scope_id,
            source_system: input.source_system,
            source_record_ref: input.source_record_ref,
            claim_id: input.claim_id,
            external_document_id: input.external_document_id,
            document_type: input.document_type,
            storage_uri: input.storage_uri,
            content_checksum: input.content_checksum,
            ingestion_status: input.ingestion_status,
            redaction_status: input.redaction_status,
            retention_policy_id: input.retention_policy_id,
            evidence_refs: input.evidence_refs,
            metadata_json: input.metadata_json,
            created_at: None,
            updated_at: None,
        };
        self.evidence_documents
            .lock()
            .await
            .insert(record.document_id.clone(), record.clone());
        Ok(record)
    }

    async fn list_evidence_documents(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentRecord>> {
        let mut records = self
            .evidence_documents
            .lock()
            .await
            .values()
            .filter(|record| {
                customer_scope_id.is_none_or(|scope| record.customer_scope_id == scope)
            })
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.document_id.cmp(&right.document_id));
        Ok(records)
    }

    async fn get_evidence_document(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentRecord>> {
        Ok(self
            .evidence_documents
            .lock()
            .await
            .get(document_id)
            .filter(|record| {
                customer_scope_id.is_none_or(|scope| record.customer_scope_id == scope)
            })
            .cloned())
    }

    async fn save_evidence_document_chunk(
        &self,
        input: CreateEvidenceDocumentChunkInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentChunkRecord>> {
        if self
            .get_evidence_document(&input.document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let record = EvidenceDocumentChunkRecord {
            chunk_id: input.chunk_id,
            document_id: input.document_id,
            chunk_index: input.chunk_index,
            chunking_version: input.chunking_version,
            redaction_status: input.redaction_status,
            text_checksum: input.text_checksum,
            token_count: input.token_count,
            storage_uri: input.storage_uri,
            source_offsets_json: input.source_offsets_json,
            evidence_refs: input.evidence_refs,
            created_at: None,
        };
        self.evidence_document_chunks
            .lock()
            .await
            .insert(record.chunk_id.clone(), record.clone());
        Ok(Some(record))
    }

    async fn list_evidence_document_chunks(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentChunkRecord>> {
        if self
            .get_evidence_document(document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(Vec::new());
        }
        let mut records = self
            .evidence_document_chunks
            .lock()
            .await
            .values()
            .filter(|record| record.document_id == document_id)
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by_key(|record| record.chunk_index);
        Ok(records)
    }

    async fn save_evidence_ocr_output(
        &self,
        input: CreateEvidenceOcrOutputInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceOcrOutputRecord>> {
        if self
            .get_evidence_document(&input.document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let record = EvidenceOcrOutputRecord {
            ocr_output_id: input.ocr_output_id,
            document_id: input.document_id,
            ocr_engine: input.ocr_engine,
            ocr_engine_version: input.ocr_engine_version,
            output_uri: input.output_uri,
            output_checksum: input.output_checksum,
            confidence_score: input.confidence_score,
            quality_status: input.quality_status,
            evidence_refs: input.evidence_refs,
            created_at: None,
        };
        self.evidence_ocr_outputs
            .lock()
            .await
            .insert(record.ocr_output_id.clone(), record.clone());
        Ok(Some(record))
    }

    async fn list_evidence_ocr_outputs(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceOcrOutputRecord>> {
        if self
            .get_evidence_document(document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(Vec::new());
        }
        let mut records = self
            .evidence_ocr_outputs
            .lock()
            .await
            .values()
            .filter(|record| record.document_id == document_id)
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.ocr_output_id.cmp(&right.ocr_output_id));
        Ok(records)
    }

    async fn save_evidence_embedding_job(
        &self,
        input: CreateEvidenceEmbeddingJobInput,
    ) -> anyhow::Result<EvidenceEmbeddingJobRecord> {
        let record = EvidenceEmbeddingJobRecord {
            embedding_job_id: input.embedding_job_id,
            customer_scope_id: input.customer_scope_id,
            target_kind: input.target_kind,
            target_ref: input.target_ref,
            embedding_model: input.embedding_model,
            embedding_model_version: input.embedding_model_version,
            chunking_version: input.chunking_version,
            redaction_status: input.redaction_status,
            vector_store_kind: input.vector_store_kind,
            vector_store_ref: input.vector_store_ref,
            embedding_checksum: input.embedding_checksum,
            status: input.status,
            evidence_refs: input.evidence_refs,
            created_at: None,
            completed_at: None,
        };
        self.evidence_embedding_jobs
            .lock()
            .await
            .insert(record.embedding_job_id.clone(), record.clone());
        Ok(record)
    }

    async fn list_evidence_embedding_jobs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceEmbeddingJobRecord>> {
        let mut records = self
            .evidence_embedding_jobs
            .lock()
            .await
            .values()
            .filter(|record| {
                customer_scope_id.is_none_or(|scope| record.customer_scope_id == scope)
            })
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.embedding_job_id.cmp(&right.embedding_job_id));
        Ok(records)
    }

    async fn save_evidence_retrieval_audit_event(
        &self,
        input: CreateEvidenceRetrievalAuditEventInput,
    ) -> anyhow::Result<EvidenceRetrievalAuditEventRecord> {
        let record = EvidenceRetrievalAuditEventRecord {
            retrieval_id: input.retrieval_id,
            customer_scope_id: input.customer_scope_id,
            actor_id: input.actor_id,
            actor_role: input.actor_role,
            query_kind: input.query_kind,
            query_checksum: input.query_checksum,
            retrieval_method: input.retrieval_method,
            embedding_model_version: input.embedding_model_version,
            top_k: input.top_k,
            source_refs: input.source_refs,
            result_refs: input.result_refs,
            redaction_status: input.redaction_status,
            evidence_refs: input.evidence_refs,
            created_at: None,
        };
        self.evidence_retrieval_audit_events
            .lock()
            .await
            .insert(record.retrieval_id.clone(), record.clone());
        Ok(record)
    }

    async fn list_evidence_retrieval_audit_events(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceRetrievalAuditEventRecord>> {
        let mut records = self
            .evidence_retrieval_audit_events
            .lock()
            .await
            .values()
            .filter(|record| {
                customer_scope_id.is_none_or(|scope| record.customer_scope_id == scope)
            })
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.retrieval_id.cmp(&right.retrieval_id));
        Ok(records)
    }
}

#[derive(Debug, Clone)]
pub struct PostgresScoringRepository {
    pool: PgPool,
}

impl PostgresScoringRepository {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    async fn load_agent_tool_calls(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<Vec<AgentToolCallRecord>> {
        let rows: Vec<(String, String, String, Value, Value)> = sqlx::query_as(
            "SELECT tool_call_id, tool_name, status, input_json, evidence_refs
             FROM tool_calls
             WHERE agent_run_id = $1
             ORDER BY created_at, id",
        )
        .bind(agent_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(tool_call_id, tool_name, status, input_json, evidence_refs)| {
                    AgentToolCallRecord {
                        tool_call_id,
                        tool_name,
                        status,
                        input_json,
                        evidence_refs: json_array_to_strings(evidence_refs),
                    }
                },
            )
            .collect())
    }

    async fn load_agent_context_snapshots(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<Vec<AgentContextSnapshotRecord>> {
        let rows: Vec<(String, String, Value, Value, String)> = sqlx::query_as(
            "SELECT snapshot_id, redaction_status, context_json, source_refs, checksum
             FROM agent_context_snapshots
             WHERE agent_run_id = $1
             ORDER BY created_at, id",
        )
        .bind(agent_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(snapshot_id, redaction_status, context_json, source_refs, checksum)| {
                    AgentContextSnapshotRecord {
                        snapshot_id,
                        redaction_status,
                        context_json,
                        source_refs: json_array_to_strings(source_refs),
                        checksum,
                    }
                },
            )
            .collect())
    }

    async fn load_agent_policy_checks(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<Vec<AgentPolicyCheckRecord>> {
        let rows: Vec<AgentPolicyCheckRow> = sqlx::query_as(
            "SELECT policy_check_id, tool_call_id, tool_name, policy_name, decision, reason, evidence_refs, created_at
             FROM agent_policy_checks
             WHERE agent_run_id = $1
             ORDER BY created_at, id",
        )
        .bind(agent_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| AgentPolicyCheckRecord {
                policy_check_id: row.policy_check_id,
                agent_run_id: agent_run_id.to_string(),
                tool_call_id: row.tool_call_id,
                tool_name: row.tool_name,
                policy_name: row.policy_name,
                decision: row.decision,
                reason: row.reason,
                evidence_refs: json_array_to_strings(row.evidence_refs),
                created_at: Some(row.created_at.to_rfc3339()),
            })
            .collect())
    }

    async fn load_agent_tool_results(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<Vec<AgentToolResultRecord>> {
        let rows: Vec<(String, String, String, String, Value, Value)> = sqlx::query_as(
            "SELECT tool_result_id, tool_call_id, tool_name, status, output_json, evidence_refs
             FROM tool_results
             WHERE agent_run_id = $1
             ORDER BY created_at, id",
        )
        .bind(agent_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(tool_result_id, tool_call_id, tool_name, status, output_json, evidence_refs)| {
                    AgentToolResultRecord {
                        tool_result_id,
                        tool_call_id,
                        tool_name,
                        status,
                        output_json,
                        evidence_refs: json_array_to_strings(evidence_refs),
                    }
                },
            )
            .collect())
    }

    async fn load_agent_approvals(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<Vec<AgentApprovalRecord>> {
        let rows: Vec<AgentApprovalRow> = sqlx::query_as(
            "SELECT approval_id, proposed_action, decision, approver, reason, evidence_refs, created_at
             FROM agent_approvals
             WHERE agent_run_id = $1
             ORDER BY created_at, id",
        )
        .bind(agent_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| AgentApprovalRecord {
                approval_id: row.approval_id,
                agent_run_id: agent_run_id.to_string(),
                proposed_action: row.proposed_action,
                decision: row.decision,
                approver: row.approver,
                reason: row.reason,
                evidence_refs: json_array_to_strings(row.evidence_refs),
                created_at: Some(row.created_at.to_rfc3339()),
            })
            .collect())
    }
}

#[async_trait]
impl ScoringRepository for PostgresScoringRepository {
    async fn upsert_claim_context(
        &self,
        context: ClaimContext,
        raw_payload: Value,
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;

        let member_row: (String,) = sqlx::query_as(
            "INSERT INTO members (external_member_id)
             VALUES ($1)
             ON CONFLICT (external_member_id) DO UPDATE SET updated_at = now()
             RETURNING id::text",
        )
        .bind(&context.member.external_member_id)
        .fetch_one(&mut *tx)
        .await?;

        let policy_row: (String,) = sqlx::query_as(
            "INSERT INTO policies
             (external_policy_id, member_id, product_code, coverage_start_date, coverage_end_date, coverage_limit_amount, currency)
             VALUES ($1, $2::uuid, $3, $4, $5, $6, $7)
             ON CONFLICT (external_policy_id) DO UPDATE SET updated_at = now()
             RETURNING id::text",
        )
        .bind(&context.policy.external_policy_id)
        .bind(&member_row.0)
        .bind(&context.policy.product_code)
        .bind(context.policy.coverage_start_date)
        .bind(context.policy.coverage_end_date)
        .bind(context.policy.coverage_limit.amount)
        .bind(&context.policy.coverage_limit.currency)
        .fetch_one(&mut *tx)
        .await?;

        let provider_row: (String,) = sqlx::query_as(
            "INSERT INTO providers (external_provider_id, name, provider_type, region, risk_tier)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (external_provider_id) DO UPDATE SET updated_at = now()
             RETURNING id::text",
        )
        .bind(&context.provider.external_provider_id)
        .bind(&context.provider.name)
        .bind(&context.provider.provider_type)
        .bind(&context.provider.region)
        .bind(format!("{:?}", context.provider.risk_tier))
        .fetch_one(&mut *tx)
        .await?;

        let claim_row: (String,) = sqlx::query_as(
            "INSERT INTO claims
             (external_claim_id, member_id, policy_id, provider_id, claim_type, diagnosis_code, service_date, claim_amount, currency, status, raw_payload)
             VALUES ($1, $2::uuid, $3::uuid, $4::uuid, 'medical', $5, $6, $7, $8, 'submitted', $9)
             ON CONFLICT (external_claim_id) DO UPDATE
             SET updated_at = now(), raw_payload = EXCLUDED.raw_payload, claim_amount = EXCLUDED.claim_amount
             RETURNING id::text",
        )
        .bind(&context.claim.external_claim_id)
        .bind(&member_row.0)
        .bind(&policy_row.0)
        .bind(&provider_row.0)
        .bind(&context.claim.diagnosis_code)
        .bind(context.claim.service_date)
        .bind(context.claim.amount.amount)
        .bind(&context.claim.amount.currency)
        .bind(raw_payload)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM claim_items WHERE claim_id = $1::uuid")
            .bind(&claim_row.0)
            .execute(&mut *tx)
            .await?;

        for item in &context.items {
            sqlx::query(
                "INSERT INTO claim_items
                 (claim_id, item_code, item_type, description, quantity, unit_amount, total_amount, currency)
                 VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(&claim_row.0)
            .bind(&item.item_code)
            .bind(&item.item_type)
            .bind(&item.description)
            .bind(item.quantity as i32)
            .bind(item.unit_amount.amount)
            .bind(item.total_amount.amount)
            .bind(&item.total_amount.currency)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn load_claim_context(
        &self,
        external_claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<ClaimContext>> {
        let row: Option<ClaimContextRow> = sqlx::query_as(
            "SELECT c.external_claim_id,
                    c.diagnosis_code,
                    c.service_date,
                    c.claim_amount,
                    c.currency AS claim_currency,
                    m.external_member_id,
                    m.dob,
                    m.gender,
                    p.external_policy_id,
                    p.product_code,
                    p.coverage_start_date,
                    p.coverage_end_date,
                    p.coverage_limit_amount,
                    p.currency AS policy_currency,
                    pr.external_provider_id,
                    pr.name AS provider_name,
                    pr.provider_type,
                    pr.region AS provider_region,
                    pr.risk_tier AS provider_risk_tier
             FROM claims c
             JOIN members m ON m.id = c.member_id
             JOIN policies p ON p.id = c.policy_id
             JOIN providers pr ON pr.id = c.provider_id
             WHERE c.external_claim_id = $1
               AND (
                 $2::text IS NULL OR EXISTS (
                   SELECT 1
                   FROM audit_events ae
                   WHERE ae.claim_id = c.id
                     AND ae.payload ->> 'customer_scope_id' = $2
                 )
               )",
        )
        .bind(external_claim_id)
        .bind(customer_scope_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let item_rows: Vec<ClaimItemRow> = sqlx::query_as(
            "SELECT ci.item_code, ci.item_type, ci.description, ci.quantity, ci.unit_amount, ci.total_amount, ci.currency
             FROM claim_items ci
             JOIN claims c ON c.id = ci.claim_id
             WHERE c.external_claim_id = $1
             ORDER BY ci.created_at, ci.item_code",
        )
        .bind(external_claim_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(Some(row.into_context(item_rows)))
    }

    async fn member_profile_summary(
        &self,
        member_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<MemberProfileSummaryRecord>> {
        let member_exists: Option<(String,)> = sqlx::query_as(
            "SELECT m.external_member_id
             FROM members m
             WHERE m.external_member_id = $1
               AND (
                 $2::text IS NULL OR EXISTS (
                   SELECT 1
                   FROM claims c
                   JOIN audit_events ae ON ae.claim_id = c.id
                   WHERE c.member_id = m.id
                     AND ae.payload ->> 'customer_scope_id' = $2
                 )
               )",
        )
        .bind(member_id)
        .bind(customer_scope_id)
        .fetch_optional(&self.pool)
        .await?;
        if member_exists.is_none() {
            return Ok(None);
        }

        let row: (i64, i64, Option<Decimal>, Option<String>) = sqlx::query_as(
            "SELECT COUNT(c.id)::bigint,
                    COUNT(DISTINCT p.id)::bigint,
                    SUM(c.claim_amount),
                    MIN(c.currency)
             FROM members m
             LEFT JOIN claims c ON c.member_id = m.id
             LEFT JOIN policies p ON p.id = c.policy_id
             WHERE m.external_member_id = $1
               AND (
                 $2::text IS NULL OR EXISTS (
                   SELECT 1
                   FROM audit_events ae
                   WHERE ae.claim_id = c.id
                     AND ae.payload ->> 'customer_scope_id' = $2
                 )
               )",
        )
        .bind(member_id)
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;
        let latest_claim: Option<(String,)> = sqlx::query_as(
            "SELECT c.external_claim_id
             FROM claims c
             JOIN members m ON m.id = c.member_id
             WHERE m.external_member_id = $1
               AND (
                 $2::text IS NULL OR EXISTS (
                   SELECT 1
                   FROM audit_events ae
                   WHERE ae.claim_id = c.id
                     AND ae.payload ->> 'customer_scope_id' = $2
                 )
               )
             ORDER BY c.service_date DESC, c.external_claim_id DESC
             LIMIT 1",
        )
        .bind(member_id)
        .bind(customer_scope_id)
        .fetch_optional(&self.pool)
        .await?;
        let high_risk: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT c.id)::bigint
             FROM members m
             JOIN claims c ON c.member_id = m.id
             JOIN scoring_runs sr ON sr.claim_id = c.id
             WHERE m.external_member_id = $1
               AND (
                 $2::text IS NULL OR EXISTS (
                   SELECT 1
                   FROM audit_events ae
                   WHERE ae.claim_id = c.id
                     AND ae.payload ->> 'customer_scope_id' = $2
                 )
               )
               AND sr.risk_score >= 70",
        )
        .bind(member_id)
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Some(member_profile_summary_record(
            MemberProfileSummaryInput {
                member_id: member_id.into(),
                claim_count: row.0 as u32,
                policy_count: row.1 as u32,
                total_claim_amount: row.2.unwrap_or(Decimal::ZERO),
                currency: row.3.unwrap_or_else(|| "UNKNOWN".into()),
                high_risk_claim_count: high_risk.0 as u32,
                latest_claim_id: latest_claim.map(|claim| claim.0),
                evidence_refs: BTreeSet::from([format!("members:{member_id}")]),
            },
        )))
    }

    async fn save_scoring_run(&self, run: PersistedScoringRun) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        let claim_row: Option<(String,)> =
            sqlx::query_as("SELECT id::text FROM claims WHERE external_claim_id = $1")
                .bind(&run.claim_id)
                .fetch_optional(&mut *tx)
                .await?;

        let claim_uuid = claim_row.map(|row| row.0);
        sqlx::query(
            "INSERT INTO scoring_runs
             (run_id, claim_id, source_system, actor_id, status, risk_score, rag, risk_level, recommended_action, confidence_score, confidence, routing_reason, routing_policy, score_breakdown, completed_at)
             VALUES ($1, $2::uuid, $3, $4, 'succeeded', $5, $6, $7, $8, $9, $10, $11, $12, $13, now())",
        )
        .bind(&run.run_id)
        .bind(claim_uuid.as_deref())
        .bind(&run.source_system)
        .bind(&run.actor_id)
        .bind(run.risk_score as i32)
        .bind(&run.rag)
        .bind(&run.risk_level)
        .bind(&run.recommended_action)
        .bind(run.confidence_score as i32)
        .bind(&run.confidence)
        .bind(&run.routing_reason)
        .bind(&run.routing_policy)
        .bind(&run.score_breakdown)
        .execute(&mut *tx)
        .await?;

        for feature in &run.feature_values {
            let feature_name = feature["name"].as_str().unwrap_or("unknown");
            let feature_version = feature["version"].as_i64().unwrap_or(1) as i32;
            sqlx::query(
                "INSERT INTO feature_values
                 (run_id, claim_id, feature_name, feature_version, value_json, evidence_json)
                 VALUES ($1, $2::uuid, $3, $4, $5, $6)",
            )
            .bind(&run.run_id)
            .bind(claim_uuid.as_deref())
            .bind(feature_name)
            .bind(feature_version)
            .bind(feature["value"].clone())
            .bind(feature["evidence_refs"].clone())
            .execute(&mut *tx)
            .await?;
        }

        for rule_run in &run.rule_runs {
            let rule_evidence = rule_run
                .get("evidence_refs")
                .filter(|evidence| evidence.is_array())
                .cloned()
                .unwrap_or_else(|| serde_json::json!([]));
            sqlx::query(
                "INSERT INTO rule_runs
                 (run_id, rule_id, rule_version_id, matched, score_contribution, alert_code, reason, evidence_json)
                 VALUES (
                   $1,
                   (SELECT id FROM rules WHERE rule_key = $2),
                   (
                     SELECT rv.id
                     FROM rule_versions rv
                     JOIN rules r ON r.id = rv.rule_id
                     WHERE r.rule_key = $2 AND rv.version = $3
                   ),
                   true,
                   $4,
                   $5,
                   $6,
                   $7
                 )",
            )
            .bind(&run.run_id)
            .bind(rule_run["rule_id"].as_str())
            .bind(rule_run["rule_version"].as_i64().unwrap_or(1) as i32)
            .bind(rule_run["score_contribution"].as_i64().unwrap_or(0) as i32)
            .bind(rule_run["alert_code"].as_str())
            .bind(rule_run["reason"].as_str())
            .bind(rule_evidence)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            "INSERT INTO model_scores
             (run_id, model_version_id, model_key, runtime_kind, execution_provider, score, label, explanation_json, latency_ms)
             VALUES (
               $1,
               (
                 SELECT id
                 FROM model_versions
                 WHERE model_key = $2 AND version = $3
               ),
               $2,
               $4,
               $5,
               $6,
               $7,
               $8,
               $9
             )",
        )
        .bind(&run.run_id)
        .bind(run.model_score["model_key"].as_str().unwrap_or("unknown"))
        .bind(run.model_score["model_version"].as_str().unwrap_or("unknown"))
        .bind(run.model_score["runtime_kind"].as_str().unwrap_or("unknown"))
        .bind(run.model_score["execution_provider"].as_str().unwrap_or("cpu"))
        .bind(run.model_score["score"].as_i64().unwrap_or(0) as i32)
        .bind(run.model_score["label"].as_str().unwrap_or("UNKNOWN"))
        .bind(run.model_score["explanations"].clone())
        .bind(run.model_score["latency_ms"].as_i64().unwrap_or(0) as i32)
        .execute(&mut *tx)
        .await?;

        if let Some(mut lead) = lead_from_scoring_run(&run, None) {
            if let Some((member_id, provider_id)) = sqlx::query_as::<_, (String, String)>(
                "SELECT m.external_member_id, pr.external_provider_id
                 FROM claims c
                 JOIN members m ON m.id = c.member_id
                 JOIN providers pr ON pr.id = c.provider_id
                 WHERE c.external_claim_id = $1",
            )
            .bind(&run.claim_id)
            .fetch_optional(&mut *tx)
            .await?
            {
                lead.member_id = member_id;
                lead.provider_id = provider_id;
            }
            sqlx::query(
                "INSERT INTO fwa_leads
                 (lead_id, run_id, claim_id, member_id, provider_id, source_system, review_mode, scheme_family, lead_source, status, disposition, risk_score, rag, reason, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                 ON CONFLICT (lead_id) DO UPDATE
                 SET run_id = EXCLUDED.run_id,
                     claim_id = EXCLUDED.claim_id,
                     member_id = EXCLUDED.member_id,
                     provider_id = EXCLUDED.provider_id,
                     source_system = EXCLUDED.source_system,
                     review_mode = EXCLUDED.review_mode,
                     scheme_family = EXCLUDED.scheme_family,
                     lead_source = EXCLUDED.lead_source,
                     status = EXCLUDED.status,
                     disposition = EXCLUDED.disposition,
                     risk_score = EXCLUDED.risk_score,
                     rag = EXCLUDED.rag,
                     reason = EXCLUDED.reason,
                     evidence_refs = EXCLUDED.evidence_refs,
                     updated_at = now()",
            )
            .bind(&lead.lead_id)
            .bind(&lead.run_id)
            .bind(&lead.claim_id)
            .bind(&lead.member_id)
            .bind(&lead.provider_id)
            .bind(&lead.source_system)
            .bind(&lead.review_mode)
            .bind(&lead.scheme_family)
            .bind(&lead.lead_source)
            .bind(&lead.status)
            .bind(&lead.disposition)
            .bind(lead.risk_score as i32)
            .bind(&lead.rag)
            .bind(&lead.reason)
            .bind(serde_json::json!(lead.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }

        insert_audit_event(
            &mut tx,
            &PersistedAuditEvent {
                audit_id: run.audit_id,
                run_id: run.run_id,
                claim_id: run.claim_id,
                source_system: run.source_system,
                actor_id: run.actor_id,
                actor_role: "tpa_system".into(),
                event_type: "scoring.completed".into(),
                event_status: "succeeded".into(),
                summary: "FWA scoring completed".into(),
                payload: run.audit_event,
                evidence_refs: run.evidence_refs,
            },
            claim_uuid.as_deref(),
        )
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn save_audit_event(&self, event: PersistedAuditEvent) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        let claim_row: Option<(String,)> =
            sqlx::query_as("SELECT id::text FROM claims WHERE external_claim_id = $1")
                .bind(&event.claim_id)
                .fetch_optional(&mut *tx)
                .await?;
        sqlx::query(
            "INSERT INTO scoring_runs
             (run_id, claim_id, source_system, actor_id, status, completed_at, error_code, error_message)
             VALUES ($1, $2::uuid, $3, $4, $5, now(), $6, $7)
             ON CONFLICT (run_id) DO NOTHING",
        )
        .bind(&event.run_id)
        .bind(claim_row.as_ref().map(|row| row.0.as_str()))
        .bind(&event.source_system)
        .bind(&event.actor_id)
        .bind(&event.event_status)
        .bind(&event.event_type)
        .bind(event.payload["error"].as_str())
        .execute(&mut *tx)
        .await?;
        insert_audit_event(
            &mut tx,
            &event,
            claim_row.as_ref().map(|row| row.0.as_str()),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn save_inbox_claim_run(&self, run: PersistedInboxClaimRun) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO inbox_claim_runs
             (run_id, audit_id, external_message_id, idempotency_key, external_message_fingerprint,
              raw_payload_checksum, raw_payload_ref, mapping_version, validation_result, scoring_ready,
              claim_id, source_system, customer_scope_id, canonical_claim_context, validation_errors,
              data_quality_signals, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
             ON CONFLICT (run_id) DO UPDATE
             SET audit_id = EXCLUDED.audit_id,
                 external_message_id = EXCLUDED.external_message_id,
                 idempotency_key = EXCLUDED.idempotency_key,
                 external_message_fingerprint = EXCLUDED.external_message_fingerprint,
                 raw_payload_checksum = EXCLUDED.raw_payload_checksum,
                 raw_payload_ref = EXCLUDED.raw_payload_ref,
                 mapping_version = EXCLUDED.mapping_version,
                 validation_result = EXCLUDED.validation_result,
                 scoring_ready = EXCLUDED.scoring_ready,
                 claim_id = EXCLUDED.claim_id,
                 source_system = EXCLUDED.source_system,
                 customer_scope_id = EXCLUDED.customer_scope_id,
                 canonical_claim_context = EXCLUDED.canonical_claim_context,
                 validation_errors = EXCLUDED.validation_errors,
                 data_quality_signals = EXCLUDED.data_quality_signals,
                 evidence_refs = EXCLUDED.evidence_refs,
                 updated_at = now()",
        )
        .bind(&run.run_id)
        .bind(&run.audit_id)
        .bind(&run.external_message_id)
        .bind(&run.idempotency_key)
        .bind(&run.external_message_fingerprint)
        .bind(&run.raw_payload_checksum)
        .bind(&run.raw_payload_ref)
        .bind(&run.mapping_version)
        .bind(&run.validation_result)
        .bind(run.scoring_ready)
        .bind(&run.claim_id)
        .bind(&run.source_system)
        .bind(&run.customer_scope_id)
        .bind(&run.canonical_claim_context)
        .bind(&run.validation_errors)
        .bind(&run.data_quality_signals)
        .bind(&run.evidence_refs)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_inbox_claim_run_by_idempotency_key(
        &self,
        idempotency_key: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
        let row = sqlx::query(
            "SELECT run_id, audit_id, external_message_id, idempotency_key,
                    external_message_fingerprint, raw_payload_checksum, raw_payload_ref,
                    mapping_version, validation_result, scoring_ready, claim_id,
                    source_system, customer_scope_id, canonical_claim_context,
                    validation_errors, data_quality_signals, evidence_refs
             FROM inbox_claim_runs
             WHERE idempotency_key = $1
               AND ($2::text IS NULL OR customer_scope_id = $2)",
        )
        .bind(idempotency_key)
        .bind(customer_scope_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(inbox_claim_run_from_row))
    }

    async fn get_inbox_claim_run_by_run_id(
        &self,
        run_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
        let row = sqlx::query(
            "SELECT run_id, audit_id, external_message_id, idempotency_key,
                    external_message_fingerprint, raw_payload_checksum, raw_payload_ref,
                    mapping_version, validation_result, scoring_ready, claim_id,
                    source_system, customer_scope_id, canonical_claim_context,
                    validation_errors, data_quality_signals, evidence_refs
             FROM inbox_claim_runs
             WHERE run_id = $1
               AND ($2::text IS NULL OR customer_scope_id = $2)",
        )
        .bind(run_id)
        .bind(customer_scope_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(inbox_claim_run_from_row))
    }

    async fn active_routing_policy(
        &self,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicy>> {
        ensure_default_routing_policies_seeded(&self.pool).await?;
        let row: Option<(Value,)> = sqlx::query_as(
            "SELECT policy_json
             FROM routing_policies
             WHERE status = 'active'
               AND review_mode IN ($1, 'both')
             ORDER BY CASE WHEN review_mode = $1 THEN 0 ELSE 1 END, version DESC
             LIMIT 1",
        )
        .bind(review_mode)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| serde_json::from_value(row.0))
            .transpose()
            .map_err(Into::into)
    }

    async fn list_routing_policies(&self) -> anyhow::Result<Vec<RoutingPolicyRecord>> {
        ensure_default_routing_policies_seeded(&self.pool).await?;
        let rows: Vec<(Value, String, String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT policy_json, status, owner, activated_at::text, created_at::text
             FROM routing_policies
             ORDER BY policy_key, review_mode, version DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(routing_policy_record_from_row)
            .collect()
    }

    async fn save_routing_policy_candidate(
        &self,
        policy: RoutingPolicy,
        owner: String,
    ) -> anyhow::Result<RoutingPolicyRecord> {
        ensure_default_routing_policies_seeded(&self.pool).await?;
        sqlx::query(
            "INSERT INTO routing_policies
             (policy_key, version, review_mode, status, owner, policy_json)
             VALUES ($1, $2, $3, 'draft', $4, $5)
             ON CONFLICT (policy_key, version, review_mode) DO UPDATE
             SET status = 'draft',
                 owner = EXCLUDED.owner,
                 policy_json = EXCLUDED.policy_json",
        )
        .bind(&policy.policy_id)
        .bind(policy.version as i32)
        .bind(&policy.review_mode)
        .bind(&owner)
        .bind(serde_json::to_value(&policy)?)
        .execute(&self.pool)
        .await?;

        Ok(routing_policy_record(policy, "draft", &owner, None, None))
    }

    async fn get_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        ensure_default_routing_policies_seeded(&self.pool).await?;
        let row: Option<(Value, String, String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT policy_json, status, owner, activated_at::text, created_at::text
                 FROM routing_policies
                 WHERE policy_key = $1 AND version = $2 AND review_mode = $3",
        )
        .bind(policy_id)
        .bind(version as i32)
        .bind(review_mode)
        .fetch_optional(&self.pool)
        .await?;

        row.map(routing_policy_record_from_row).transpose()
    }

    async fn update_routing_policy_status(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
        status: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        ensure_default_routing_policies_seeded(&self.pool).await?;
        let row: Option<(Value, String, String, Option<String>, Option<String>)> = sqlx::query_as(
            "UPDATE routing_policies
                 SET status = $4
                 WHERE policy_key = $1 AND version = $2 AND review_mode = $3
                 RETURNING policy_json, status, owner, activated_at::text, created_at::text",
        )
        .bind(policy_id)
        .bind(version as i32)
        .bind(review_mode)
        .bind(status)
        .fetch_optional(&self.pool)
        .await?;

        row.map(routing_policy_record_from_row).transpose()
    }

    async fn activate_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        ensure_default_routing_policies_seeded(&self.pool).await?;
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE routing_policies
             SET status = 'approved'
             WHERE review_mode = $1
               AND status = 'active'
               AND NOT (policy_key = $2 AND version = $3)",
        )
        .bind(review_mode)
        .bind(policy_id)
        .bind(version as i32)
        .execute(&mut *tx)
        .await?;

        let row: Option<(Value, String, String, Option<String>, Option<String>)> = sqlx::query_as(
            "UPDATE routing_policies
                 SET status = 'active', activated_at = now()
                 WHERE policy_key = $1 AND version = $2 AND review_mode = $3
                 RETURNING policy_json, status, owner, activated_at::text, created_at::text",
        )
        .bind(policy_id)
        .bind(version as i32)
        .bind(review_mode)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;

        row.map(routing_policy_record_from_row).transpose()
    }

    async fn list_rules(&self) -> anyhow::Result<Vec<RuleSummaryRecord>> {
        ensure_default_rules_seeded(&self.pool).await?;
        let rows: Vec<(String, String, String, String, i32, Value, i32, String)> = sqlx::query_as(
            "SELECT r.rule_key, r.name, r.status, r.owner, rv.version, rv.dsl, rv.score, rv.recommended_action
             FROM rules r
             JOIN LATERAL (
               SELECT version, dsl, score, recommended_action
               FROM rule_versions
               WHERE rule_id = r.id
               ORDER BY version DESC
               LIMIT 1
             ) rv ON true
             ORDER BY r.rule_key",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut summaries = rows
            .into_iter()
            .map(
                |(rule_id, name, status, owner, version, dsl, score, recommended_action)| {
                    let action = dsl.get("action").cloned().unwrap_or(Value::Null);
                    let review_mode = review_mode_from_dsl(&dsl);
                    let scheme_family = scheme_family_from_dsl(&dsl);
                    RuleSummaryRecord {
                        rule_id: rule_id.clone(),
                        name,
                        active_version: if status == "active" {
                            Some(version as u32)
                        } else {
                            None
                        },
                        latest_version: version as u32,
                        review_mode: review_mode.clone(),
                        scheme_family: scheme_family.clone(),
                        status,
                        owner,
                        score: score as u8,
                        alert_code: action["alert_code"]
                            .as_str()
                            .unwrap_or("UNKNOWN")
                            .to_string(),
                        recommended_action: parse_recommended_action(&recommended_action),
                        applicability_scope: rule_applicability_scope(&review_mode, &scheme_family),
                        backtest_result: default_rule_backtest_summary(),
                        estimated_saving: "0.00".into(),
                        false_positive_history: default_rule_false_positive_history(),
                        evidence_refs: rule_governance_evidence_refs(&rule_id, version as u32),
                    }
                },
            )
            .collect::<Vec<_>>();

        for summary in &mut summaries {
            let latest_backtest = self
                .latest_rule_backtest(&summary.rule_id, summary.latest_version)
                .await?;
            apply_rule_backtest_metadata(summary, latest_backtest.as_ref());
        }

        Ok(summaries)
    }

    async fn list_active_rules(&self) -> anyhow::Result<Vec<Rule>> {
        ensure_default_rules_seeded(&self.pool).await?;
        let rows: Vec<(String, String, i32, Value)> = sqlx::query_as(
            "SELECT r.rule_key, r.name, rv.version, rv.dsl
             FROM rules r
             JOIN LATERAL (
               SELECT version, dsl
               FROM rule_versions
               WHERE rule_id = r.id
               ORDER BY version DESC
               LIMIT 1
             ) rv ON true
             WHERE r.status = 'active'
             ORDER BY r.rule_key",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|(rule_id, name, version, dsl)| {
                runtime_rule_from_parts(rule_id, name, version as u32, dsl)
            })
            .collect()
    }

    async fn get_rule(&self, rule_id: &str) -> anyhow::Result<Option<RuleDetailRecord>> {
        ensure_default_rules_seeded(&self.pool).await?;
        let summary = self
            .list_rules()
            .await?
            .into_iter()
            .find(|rule| rule.rule_id == rule_id);
        let Some(summary) = summary else {
            return Ok(None);
        };

        let rows: Vec<(i32, Value, i32, String)> = sqlx::query_as(
            "SELECT rv.version, rv.dsl, rv.score, rv.recommended_action
             FROM rule_versions rv
             JOIN rules r ON r.id = rv.rule_id
             WHERE r.rule_key = $1
             ORDER BY rv.version DESC",
        )
        .bind(rule_id)
        .fetch_all(&self.pool)
        .await?;

        let versions = rows
            .into_iter()
            .map(|(version, dsl, score, recommended_action)| {
                let action = dsl.get("action").cloned().unwrap_or(Value::Null);
                RuleVersionRecord {
                    version: version as u32,
                    status: summary.status.clone(),
                    review_mode: review_mode_from_dsl(&dsl),
                    scheme_family: scheme_family_from_dsl(&dsl),
                    dsl,
                    score: score as u8,
                    alert_code: action["alert_code"]
                        .as_str()
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    recommended_action: parse_recommended_action(&recommended_action),
                    reason: action["reason"].as_str().unwrap_or("").to_string(),
                }
            })
            .collect();

        let audit_events = self.rule_audit_history(rule_id).await?;

        Ok(Some(RuleDetailRecord {
            summary,
            versions,
            audit_events,
        }))
    }

    async fn rule_audit_history(
        &self,
        rule_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        let rows: Vec<(String, String, String, String, String, String, Value, Value, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as(
                "SELECT audit_id, run_id, actor_role, event_type, event_status, summary, payload, evidence_refs, created_at
                 FROM audit_events
                 WHERE payload ->> 'rule_id' = $1
                 ORDER BY created_at, audit_id",
            )
            .bind(rule_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs,
                    created_at,
                )| AuditHistoryEventRecord {
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: Some(created_at.to_rfc3339()),
                },
            )
            .collect())
    }

    async fn save_rule_candidate(
        &self,
        rule: Rule,
        owner: String,
    ) -> anyhow::Result<RuleDetailRecord> {
        ensure_default_rules_seeded(&self.pool).await?;
        let detail = rule_detail_from_rule(rule, "draft", owner);
        let mut tx = self.pool.begin().await?;
        let row: (String,) = sqlx::query_as(
            "INSERT INTO rules (rule_key, name, status, owner)
             VALUES ($1, $2, 'draft', $3)
             ON CONFLICT (rule_key) DO UPDATE
             SET name = EXCLUDED.name,
                 status = 'draft',
                 owner = EXCLUDED.owner,
                 updated_at = now()
             RETURNING id::text",
        )
        .bind(&detail.summary.rule_id)
        .bind(&detail.summary.name)
        .bind(&detail.summary.owner)
        .fetch_one(&mut *tx)
        .await?;

        let version = &detail.versions[0];
        sqlx::query(
            "INSERT INTO rule_versions
             (rule_id, version, dsl, score, recommended_action, created_by)
             VALUES ($1::uuid, $2, $3, $4, $5, $6)
             ON CONFLICT (rule_id, version) DO UPDATE
             SET dsl = EXCLUDED.dsl,
                 score = EXCLUDED.score,
                 recommended_action = EXCLUDED.recommended_action",
        )
        .bind(&row.0)
        .bind(version.version as i32)
        .bind(&version.dsl)
        .bind(version.score as i32)
        .bind(format!("{:?}", version.recommended_action))
        .bind(&detail.summary.owner)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(detail)
    }

    async fn update_rule_status(
        &self,
        rule_id: &str,
        status: &str,
    ) -> anyhow::Result<Option<RuleSummaryRecord>> {
        ensure_default_rules_seeded(&self.pool).await?;
        let result =
            sqlx::query("UPDATE rules SET status = $1, updated_at = now() WHERE rule_key = $2")
                .bind(status)
                .bind(rule_id)
                .execute(&self.pool)
                .await?;
        if result.rows_affected() == 0 {
            return Ok(None);
        }
        Ok(self
            .list_rules()
            .await?
            .into_iter()
            .find(|rule| rule.rule_id == rule_id))
    }

    async fn rule_performance(&self) -> anyhow::Result<Vec<RulePerformanceRecord>> {
        ensure_default_rules_seeded(&self.pool).await?;
        let rules = self.list_rules().await?;
        let total_runs: (i64,) =
            sqlx::query_as("SELECT COUNT(*)::bigint FROM scoring_runs WHERE status = 'succeeded'")
                .fetch_one(&self.pool)
                .await?;

        let rule_run_rows: Vec<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT r.rule_key, rr.alert_code, c.external_claim_id
             FROM rule_runs rr
             JOIN scoring_runs sr ON sr.run_id = rr.run_id
             LEFT JOIN rules r ON r.id = rr.rule_id
             LEFT JOIN claims c ON c.id = sr.claim_id
             WHERE rr.matched = true",
        )
        .fetch_all(&self.pool)
        .await?;

        let outcome_rows: Vec<(String, bool, Option<Decimal>)> = sqlx::query_as(
            "SELECT claim_id, confirmed_fwa, saving_amount
             FROM investigation_results",
        )
        .fetch_all(&self.pool)
        .await?;
        let outcomes = outcome_rows
            .into_iter()
            .map(|(claim_id, confirmed_fwa, saving_amount)| {
                (
                    claim_id,
                    InvestigationOutcome {
                        confirmed_fwa,
                        saving_amount: saving_amount.unwrap_or(Decimal::ZERO),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        let alert_to_rule = rules
            .iter()
            .map(|rule| (rule.alert_code.clone(), rule.rule_id.clone()))
            .collect::<HashMap<_, _>>();
        let mut accumulators = rule_accumulators_from_rules(&rules);
        for (rule_id, alert_code, claim_id) in rule_run_rows {
            let rule_id = rule_id.or_else(|| {
                alert_code
                    .as_ref()
                    .and_then(|alert_code| alert_to_rule.get(alert_code).cloned())
            });
            let (Some(rule_id), Some(claim_id)) = (rule_id, claim_id) else {
                continue;
            };
            let Some(accumulator) = accumulators.get_mut(&rule_id) else {
                continue;
            };
            accumulator.trigger_count += 1;
            accumulator.triggered_claim_ids.insert(claim_id);
        }

        Ok(rule_performance_records(
            accumulators,
            &outcomes,
            total_runs.0.max(0) as u32,
        ))
    }

    async fn save_rule_backtest(
        &self,
        record: RuleBacktestRecord,
    ) -> anyhow::Result<RuleBacktestRecord> {
        let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
            "INSERT INTO rule_backtest_runs
             (rule_id, rule_version, sample_count, matched_count, reviewed_count,
              confirmed_fwa_count, false_positive_count, precision_value, recall_value,
              lift, false_positive_rate, estimated_saving, promotion_recommendation,
              blockers, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             RETURNING created_at",
        )
        .bind(&record.rule_id)
        .bind(record.rule_version as i32)
        .bind(record.sample_count as i32)
        .bind(record.matched_count as i32)
        .bind(record.reviewed_count as i32)
        .bind(record.confirmed_fwa_count as i32)
        .bind(record.false_positive_count as i32)
        .bind(record.precision)
        .bind(record.recall)
        .bind(record.lift)
        .bind(record.false_positive_rate)
        .bind(&record.estimated_saving)
        .bind(&record.promotion_recommendation)
        .bind(serde_json::json!(record.blockers))
        .bind(serde_json::json!(record.evidence_refs))
        .fetch_one(&self.pool)
        .await?;
        Ok(RuleBacktestRecord {
            created_at: Some(row.0.to_rfc3339()),
            ..record
        })
    }

    async fn latest_rule_backtest(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleBacktestRecord>> {
        let row: Option<(
            String,
            i32,
            i32,
            i32,
            i32,
            i32,
            i32,
            f64,
            f64,
            f64,
            f64,
            String,
            String,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT rule_id, rule_version, sample_count, matched_count, reviewed_count,
                    confirmed_fwa_count, false_positive_count, precision_value, recall_value,
                    lift, false_positive_rate, estimated_saving, promotion_recommendation,
                    blockers, evidence_refs, created_at
             FROM rule_backtest_runs
             WHERE rule_id = $1 AND rule_version = $2
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
        )
        .bind(rule_id)
        .bind(rule_version as i32)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                rule_id,
                rule_version,
                sample_count,
                matched_count,
                reviewed_count,
                confirmed_fwa_count,
                false_positive_count,
                precision,
                recall,
                lift,
                false_positive_rate,
                estimated_saving,
                promotion_recommendation,
                blockers,
                evidence_refs,
                created_at,
            )| RuleBacktestRecord {
                rule_id,
                rule_version: rule_version as u32,
                sample_count: sample_count.max(0) as u32,
                matched_count: matched_count.max(0) as u32,
                reviewed_count: reviewed_count.max(0) as u32,
                confirmed_fwa_count: confirmed_fwa_count.max(0) as u32,
                false_positive_count: false_positive_count.max(0) as u32,
                precision,
                recall,
                lift,
                false_positive_rate,
                estimated_saving,
                promotion_recommendation,
                blockers: json_array_to_strings(blockers),
                evidence_refs: json_array_to_strings(evidence_refs),
                created_at: Some(created_at.to_rfc3339()),
            },
        ))
    }

    async fn save_rule_promotion_review(
        &self,
        record: RulePromotionReviewRecord,
    ) -> anyhow::Result<RulePromotionReviewRecord> {
        let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
            "INSERT INTO rule_promotion_reviews
             (rule_id, rule_version, decision, reviewer, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING created_at",
        )
        .bind(&record.rule_id)
        .bind(record.rule_version as i32)
        .bind(&record.decision)
        .bind(&record.reviewer)
        .bind(&record.notes)
        .bind(serde_json::json!(record.evidence_refs.clone()))
        .fetch_one(&self.pool)
        .await?;
        Ok(RulePromotionReviewRecord {
            created_at: Some(row.0.to_rfc3339()),
            ..record
        })
    }

    async fn latest_rule_promotion_review(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RulePromotionReviewRecord>> {
        let row: Option<(
            String,
            i32,
            String,
            String,
            String,
            serde_json::Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT rule_id, rule_version, decision, reviewer, notes, evidence_refs, created_at
                 FROM rule_promotion_reviews
                 WHERE rule_id = $1 AND rule_version = $2
                 ORDER BY created_at DESC
                 LIMIT 1",
        )
        .bind(rule_id)
        .bind(rule_version as i32)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(rule_id, rule_version, decision, reviewer, notes, evidence_refs, created_at)| {
                RulePromotionReviewRecord {
                    rule_id,
                    rule_version: rule_version as u32,
                    decision,
                    reviewer,
                    notes,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: Some(created_at.to_rfc3339()),
                }
            },
        ))
    }

    async fn list_leads(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<LeadRecord>> {
        load_leads(&self.pool, customer_scope_id).await
    }

    async fn triage_lead(
        &self,
        lead_id: &str,
        input: TriageLeadInput,
    ) -> anyhow::Result<Option<TriageLeadRecord>> {
        let mut tx = self.pool.begin().await?;
        let lead = load_lead_in_tx(&mut tx, lead_id, input.customer_scope_id.as_deref()).await?;
        let Some(mut lead) = lead else {
            return Ok(None);
        };
        if input.decision == "merge_lead"
            && merge_target_lead_in_tx(&mut tx, &input).await?.is_none()
        {
            return Ok(None);
        }
        lead.status = triage_status_for_decision(&input.decision).into();
        lead.disposition = triage_disposition_for_decision(&input.decision).into();
        let case = (input.decision == "open_case").then(|| case_from_lead(&lead, &input));
        sqlx::query(
            "UPDATE fwa_leads
             SET status = $2, disposition = $3, updated_at = now()
             WHERE lead_id = $1",
        )
        .bind(&lead.lead_id)
        .bind(&lead.status)
        .bind(&lead.disposition)
        .execute(&mut *tx)
        .await?;
        if let Some(case) = &case {
            sqlx::query(
                "INSERT INTO investigation_cases
                 (case_id, lead_id, claim_id, member_id, provider_id, source_system, review_mode, scheme_family, lead_source, status, assignee, reviewer, priority, routing_reason, evidence_package_json)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                 ON CONFLICT (case_id) DO UPDATE
                 SET status = EXCLUDED.status,
                     review_mode = EXCLUDED.review_mode,
                     assignee = EXCLUDED.assignee,
                     reviewer = EXCLUDED.reviewer,
                     priority = EXCLUDED.priority,
                     routing_reason = EXCLUDED.routing_reason,
                     evidence_package_json = EXCLUDED.evidence_package_json,
                     updated_at = now()",
            )
            .bind(&case.case_id)
            .bind(&case.lead_id)
            .bind(&case.claim_id)
            .bind(&case.member_id)
            .bind(&case.provider_id)
            .bind(&case.source_system)
            .bind(&case.review_mode)
            .bind(&case.scheme_family)
            .bind(&case.lead_source)
            .bind(&case.status)
            .bind(&case.assignee)
            .bind(&case.reviewer)
            .bind(&case.priority)
            .bind(&case.routing_reason)
            .bind(&case.evidence_package)
            .execute(&mut *tx)
            .await?;
        }

        let audit_id = AuditEventId::new().to_string();
        insert_audit_event(
            &mut tx,
            &PersistedAuditEvent {
                audit_id: audit_id.clone(),
                run_id: lead.run_id.clone(),
                claim_id: lead.claim_id.clone(),
                source_system: lead.source_system.clone(),
                actor_id: input.assignee.clone(),
                actor_role: "fwa_operator".into(),
                event_type: "lead.triaged".into(),
                event_status: "succeeded".into(),
                summary: format!("Lead triaged: {}", input.decision),
                payload: triage_audit_payload(&lead, &input, case.as_ref()),
                evidence_refs: input
                    .evidence_refs
                    .iter()
                    .map(|value| Value::String(value.clone()))
                    .collect(),
            },
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(Some(TriageLeadRecord {
            lead,
            case,
            audit_id,
        }))
    }

    async fn list_cases(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<CaseRecord>> {
        load_cases(&self.pool, customer_scope_id).await
    }

    async fn update_case_status(
        &self,
        case_id: &str,
        input: UpdateCaseStatusInput,
    ) -> anyhow::Result<Option<UpdateCaseStatusRecord>> {
        let mut tx = self.pool.begin().await?;
        let case = load_case_in_tx(&mut tx, case_id, input.customer_scope_id.as_deref()).await?;
        let Some(mut case) = case else {
            return Ok(None);
        };
        let audit_run_id =
            load_lead_in_tx(&mut tx, &case.lead_id, input.customer_scope_id.as_deref())
                .await?
                .map(|lead| lead.run_id)
                .unwrap_or_else(|| format!("case_status_{}", case.case_id));
        let from_status = case.status.clone();
        case.status = input.status.clone();
        sqlx::query(
            "UPDATE investigation_cases
             SET status = $2, updated_at = now()
             WHERE case_id = $1",
        )
        .bind(&case.case_id)
        .bind(&case.status)
        .execute(&mut *tx)
        .await?;
        let case = load_case_in_tx(&mut tx, case_id, input.customer_scope_id.as_deref())
            .await?
            .expect("case should exist after status update");

        let audit_id = AuditEventId::new().to_string();
        insert_audit_event(
            &mut tx,
            &PersistedAuditEvent {
                audit_id: audit_id.clone(),
                run_id: audit_run_id,
                claim_id: case.claim_id.clone(),
                source_system: case.source_system.clone(),
                actor_id: input.actor_id.clone(),
                actor_role: "fwa_operator".into(),
                event_type: "case.status.updated".into(),
                event_status: "succeeded".into(),
                summary: format!("Case status updated: {} -> {}", from_status, case.status),
                payload: serde_json::json!({
                    "claim_id": case.claim_id,
                    "case_id": case.case_id,
                    "lead_id": case.lead_id,
                    "from_status": from_status,
                    "to_status": case.status,
                    "notes": input.notes,
                    "customer_scope_id": input.customer_scope_id
                }),
                evidence_refs: input
                    .evidence_refs
                    .iter()
                    .map(|value| Value::String(value.clone()))
                    .collect(),
            },
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(Some(UpdateCaseStatusRecord { case, audit_id }))
    }

    async fn create_audit_sample(
        &self,
        input: CreateAuditSampleInput,
    ) -> anyhow::Result<AuditSampleRecord> {
        let sample_id = format!("sample_{}", AuditEventId::new());
        let customer_scope_filter = input.customer_scope_id.clone();
        let customer_scope_id = customer_scope_filter.as_deref();
        let leads = if input.sample_mode == "random_control" {
            load_control_audit_population(&self.pool, customer_scope_id).await?
        } else {
            self.list_leads(customer_scope_id).await?
        };
        let strata_contexts = load_audit_sample_strata_contexts(&self.pool).await?;
        let existing_samples = self.list_audit_samples(customer_scope_id).await?;
        let reviewer_history =
            reviewer_lead_sample_counts(existing_samples.iter(), &input.reviewer);
        let sample = build_audit_sample(
            sample_id,
            input,
            leads,
            &strata_contexts,
            &reviewer_history,
            None,
        );
        sqlx::query(
            "INSERT INTO audit_samples
             (sample_id, customer_scope_id, sample_mode, population_definition, inclusion_criteria_json, deterministic_seed, selection_method, sample_size, reviewer, assignment_queue, selected_leads_json, outcome_distribution_json)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(&sample.sample_id)
        .bind(&sample.customer_scope_id)
        .bind(&sample.sample_mode)
        .bind(&sample.population_definition)
        .bind(&sample.inclusion_criteria)
        .bind(&sample.deterministic_seed)
        .bind(&sample.selection_method)
        .bind(sample.sample_size as i32)
        .bind(&sample.reviewer)
        .bind(&sample.assignment_queue)
        .bind(serde_json::to_value(&sample.selected_leads)?)
        .bind(&sample.outcome_distribution)
        .execute(&self.pool)
        .await?;
        self.list_audit_samples(customer_scope_id)
            .await?
            .into_iter()
            .find(|record| record.sample_id == sample.sample_id)
            .ok_or_else(|| anyhow::anyhow!("created audit sample was not found"))
    }

    async fn list_audit_samples(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditSampleRecord>> {
        let rows: Vec<(
            String,
            String,
            String,
            String,
            Value,
            Option<String>,
            String,
            i32,
            String,
            String,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT sample_id, customer_scope_id, sample_mode, population_definition, inclusion_criteria_json, deterministic_seed, selection_method, sample_size, reviewer, assignment_queue, selected_leads_json, outcome_distribution_json, created_at
             FROM audit_samples
             WHERE ($1::text IS NULL OR customer_scope_id = $1)
             ORDER BY created_at, sample_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        let samples = rows
            .into_iter()
            .map(
                |(
                    sample_id,
                    customer_scope_id,
                    sample_mode,
                    population_definition,
                    inclusion_criteria,
                    deterministic_seed,
                    selection_method,
                    sample_size,
                    reviewer,
                    assignment_queue,
                    selected_leads,
                    outcome_distribution,
                    created_at,
                )| AuditSampleRecord {
                    sample_id,
                    customer_scope_id,
                    sample_mode,
                    population_definition,
                    inclusion_criteria,
                    deterministic_seed,
                    selection_method,
                    sample_size: sample_size.max(0) as usize,
                    reviewer,
                    assignment_queue,
                    selected_leads: serde_json::from_value(selected_leads).unwrap_or_default(),
                    outcome_distribution,
                    created_at: Some(created_at.to_rfc3339()),
                },
            )
            .collect::<Vec<_>>();
        let reviews = self.list_qa_reviews(customer_scope_id).await?;
        Ok(with_sample_outcome_distributions(samples, &reviews))
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelVersionRecord>> {
        ensure_default_models_seeded(&self.pool).await?;
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT model_key, version, model_type, runtime_kind, execution_provider, status, COALESCE(metrics ->> 'review_mode', 'both'), artifact_uri, endpoint_url
             FROM model_versions
             ORDER BY model_key, version DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    model_key,
                    version,
                    model_type,
                    runtime_kind,
                    execution_provider,
                    status,
                    review_mode,
                    artifact_uri,
                    endpoint_url,
                )| ModelVersionRecord {
                    model_key,
                    version,
                    model_type,
                    runtime_kind,
                    execution_provider,
                    status,
                    review_mode: normalize_review_mode(&review_mode),
                    artifact_uri,
                    endpoint_url,
                },
            )
            .collect())
    }

    async fn save_model_version(
        &self,
        record: ModelVersionRecord,
    ) -> anyhow::Result<ModelVersionRecord> {
        sqlx::query(
            "INSERT INTO model_versions
             (model_key, version, model_type, runtime_kind, artifact_uri, endpoint_url, execution_provider, status, metrics, activated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, CASE WHEN $8 = 'active' THEN now() ELSE NULL END)
             ON CONFLICT (model_key, version) DO UPDATE
             SET model_type = EXCLUDED.model_type,
                 runtime_kind = EXCLUDED.runtime_kind,
                 artifact_uri = EXCLUDED.artifact_uri,
                 endpoint_url = EXCLUDED.endpoint_url,
                 execution_provider = EXCLUDED.execution_provider,
                 status = EXCLUDED.status,
                 metrics = model_versions.metrics || EXCLUDED.metrics,
                 activated_at = CASE WHEN EXCLUDED.status = 'active' THEN now() ELSE model_versions.activated_at END",
        )
        .bind(&record.model_key)
        .bind(&record.version)
        .bind(&record.model_type)
        .bind(&record.runtime_kind)
        .bind(&record.artifact_uri)
        .bind(&record.endpoint_url)
        .bind(&record.execution_provider)
        .bind(&record.status)
        .bind(serde_json::json!({ "review_mode": record.review_mode }))
        .execute(&self.pool)
        .await?;
        Ok(record)
    }

    async fn update_model_status(
        &self,
        model_key: &str,
        model_version: &str,
        status: &str,
    ) -> anyhow::Result<Option<ModelVersionRecord>> {
        ensure_default_models_seeded(&self.pool).await?;
        let row: Option<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "UPDATE model_versions
             SET status = $3,
                 activated_at = CASE WHEN $3 = 'active' THEN now() ELSE NULL END
             WHERE model_key = $1 AND version = $2
             RETURNING model_key, version, model_type, runtime_kind, execution_provider, status, COALESCE(metrics ->> 'review_mode', 'both'), artifact_uri, endpoint_url",
        )
        .bind(model_key)
        .bind(model_version)
        .bind(status)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(
                model_key,
                version,
                model_type,
                runtime_kind,
                execution_provider,
                status,
                review_mode,
                artifact_uri,
                endpoint_url,
            )| ModelVersionRecord {
                model_key,
                version,
                model_type,
                runtime_kind,
                execution_provider,
                status,
                review_mode: normalize_review_mode(&review_mode),
                artifact_uri,
                endpoint_url,
            },
        ))
    }

    async fn model_performance(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Option<ModelPerformanceRecord>> {
        ensure_default_models_seeded(&self.pool).await?;
        let known = self
            .list_models()
            .await?
            .into_iter()
            .any(|model| model.model_key == model_key);
        if !known {
            return Ok(None);
        }

        let row: (
            i64,
            Option<Decimal>,
            Option<i64>,
            Option<chrono::DateTime<chrono::Utc>>,
        ) = sqlx::query_as(
            "SELECT
                   COUNT(*)::bigint,
                   AVG(score),
                   SUM(CASE WHEN score >= 70 THEN 1 ELSE 0 END)::bigint,
                   MAX(created_at)
                 FROM model_scores
                 WHERE model_key = $1",
        )
        .bind(model_key)
        .fetch_one(&self.pool)
        .await?;
        let drift_metrics: Option<(Value,)> = sqlx::query_as(
            "SELECT metrics_json
             FROM model_evaluation_runs
             WHERE model_key = $1
             ORDER BY created_at DESC, evaluation_run_id DESC
             LIMIT 1",
        )
        .bind(model_key)
        .fetch_optional(&self.pool)
        .await?;
        let drift = drift_summary(
            drift_metrics
                .as_ref()
                .map(|row| &row.0)
                .unwrap_or(&Value::Null),
        );

        let scored_runs = row.0 as u32;
        if scored_runs == 0 {
            return Ok(Some(model_performance_with_drift(
                empty_model_performance(model_key),
                drift,
            )));
        }

        Ok(Some(model_performance_with_drift(
            ModelPerformanceRecord {
                model_key: model_key.to_string(),
                data_status: "ready".into(),
                scored_runs,
                average_score: row
                    .1
                    .map(|value| value.to_string().parse().unwrap_or(0.0))
                    .unwrap_or(0.0),
                high_risk_count: row.2.unwrap_or(0) as u32,
                score_psi: None,
                drift_status: "not_available".into(),
                latest_scored_at: row.3.map(|timestamp| timestamp.to_rfc3339()),
            },
            drift,
        )))
    }

    async fn save_model_promotion_review(
        &self,
        record: ModelPromotionReviewRecord,
    ) -> anyhow::Result<ModelPromotionReviewRecord> {
        let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
            "INSERT INTO model_promotion_reviews
             (model_key, model_version, decision, reviewer, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING created_at",
        )
        .bind(&record.model_key)
        .bind(&record.model_version)
        .bind(&record.decision)
        .bind(&record.reviewer)
        .bind(&record.notes)
        .bind(serde_json::json!(record.evidence_refs.clone()))
        .fetch_one(&self.pool)
        .await?;
        Ok(ModelPromotionReviewRecord {
            created_at: Some(row.0.to_rfc3339()),
            ..record
        })
    }

    async fn latest_model_promotion_review(
        &self,
        model_key: &str,
        model_version: &str,
    ) -> anyhow::Result<Option<ModelPromotionReviewRecord>> {
        let row: Option<(
            String,
            String,
            String,
            String,
            String,
            serde_json::Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT model_key, model_version, decision, reviewer, notes, evidence_refs, created_at
                 FROM model_promotion_reviews
                 WHERE model_key = $1 AND model_version = $2
                 ORDER BY created_at DESC
                 LIMIT 1",
        )
        .bind(model_key)
        .bind(model_version)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(model_key, model_version, decision, reviewer, notes, evidence_refs, created_at)| {
                ModelPromotionReviewRecord {
                    model_key,
                    model_version,
                    decision,
                    reviewer,
                    notes,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: Some(created_at.to_rfc3339()),
                }
            },
        ))
    }

    async fn save_model_retraining_job(
        &self,
        record: ModelRetrainingJobRecord,
    ) -> anyhow::Result<ModelRetrainingJobRecord> {
        let row: (
            String,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        ) = sqlx::query_as(
            "INSERT INTO model_retraining_jobs
                 (model_key, model_version, status, requested_by, request_notes, status_note,
                  updated_by, readiness_recommendation, latest_evaluation_id, source_dataset_id,
                  source_data_quality_score, source_data_quality_status, trigger_summary_json,
                  blocker_summary_json)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                 RETURNING id::text, created_at, updated_at",
        )
        .bind(&record.model_key)
        .bind(&record.model_version)
        .bind(&record.status)
        .bind(&record.requested_by)
        .bind(&record.request_notes)
        .bind(&record.status_note)
        .bind(&record.updated_by)
        .bind(&record.readiness_recommendation)
        .bind(&record.latest_evaluation_id)
        .bind(&record.source_dataset_id)
        .bind(record.source_data_quality_score)
        .bind(&record.source_data_quality_status)
        .bind(serde_json::json!(record.trigger_summary))
        .bind(serde_json::json!(record.blocker_summary))
        .fetch_one(&self.pool)
        .await?;
        Ok(ModelRetrainingJobRecord {
            job_id: row.0,
            created_at: Some(row.1.to_rfc3339()),
            updated_at: Some(row.2.to_rfc3339()),
            ..record
        })
    }

    async fn list_model_retraining_jobs(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Vec<ModelRetrainingJobRecord>> {
        let rows = sqlx::query(
            "SELECT id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                    status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                    source_dataset_id, source_data_quality_score, source_data_quality_status,
                    trigger_summary_json, blocker_summary_json, candidate_model_version,
                    candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                    output_evaluation_id, created_at, updated_at
             FROM model_retraining_jobs
             WHERE model_key = $1
             ORDER BY created_at DESC",
        )
        .bind(model_key)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(model_retraining_job_from_pg_row)
            .collect())
    }

    async fn get_model_retraining_job(
        &self,
        job_id: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let row = sqlx::query(
            "SELECT id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                    status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                    source_dataset_id, source_data_quality_score, source_data_quality_status,
                    trigger_summary_json, blocker_summary_json, candidate_model_version,
                    candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                    output_evaluation_id, created_at, updated_at
             FROM model_retraining_jobs
             WHERE id = $1::uuid",
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(model_retraining_job_from_pg_row))
    }

    async fn claim_next_model_retraining_job(
        &self,
        model_key: Option<&str>,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let row = sqlx::query(
            "WITH next_job AS (
                 SELECT id
                 FROM model_retraining_jobs
                 WHERE status = 'queued'
                   AND ($3::text IS NULL OR model_key = $3)
                 ORDER BY created_at ASC
                 LIMIT 1
                 FOR UPDATE SKIP LOCKED
             )
             UPDATE model_retraining_jobs
             SET status = 'running', updated_by = $1, status_note = $2, updated_at = now()
             WHERE id = (SELECT id FROM next_job)
             RETURNING id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                       status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                       source_dataset_id, source_data_quality_score, source_data_quality_status,
                       trigger_summary_json, blocker_summary_json, candidate_model_version,
                       candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                       output_evaluation_id, created_at, updated_at",
        )
        .bind(actor)
        .bind(status_note)
        .bind(model_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(model_retraining_job_from_pg_row))
    }

    async fn update_model_retraining_job_status(
        &self,
        job_id: &str,
        status: &str,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let row = sqlx::query(
            "UPDATE model_retraining_jobs
             SET status = $2, updated_by = $3, status_note = $4, updated_at = now()
             WHERE id = $1::uuid
             RETURNING id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                       status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                       source_dataset_id, source_data_quality_score, source_data_quality_status,
                       trigger_summary_json, blocker_summary_json, candidate_model_version,
                       candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                       output_evaluation_id, created_at, updated_at",
        )
        .bind(job_id)
        .bind(status)
        .bind(actor)
        .bind(status_note)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(model_retraining_job_from_pg_row))
    }

    async fn complete_model_retraining_job(
        &self,
        input: CompleteModelRetrainingJobInput<'_>,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let row = sqlx::query(
            "UPDATE model_retraining_jobs
             SET status = 'completed',
                 updated_by = $2,
                 status_note = $3,
                 candidate_model_version = $4,
                 candidate_artifact_uri = $5,
                 candidate_endpoint_url = $6,
                 validation_report_uri = $7,
                 output_evaluation_id = $8,
                 updated_at = now()
             WHERE id = $1::uuid
             RETURNING id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                       status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                       source_dataset_id, source_data_quality_score, source_data_quality_status,
                       trigger_summary_json, blocker_summary_json, candidate_model_version,
                       candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                       output_evaluation_id, created_at, updated_at",
        )
        .bind(input.job_id)
        .bind(input.actor)
        .bind(input.status_note)
        .bind(input.candidate_model_version)
        .bind(input.candidate_artifact_uri)
        .bind(input.candidate_endpoint_url)
        .bind(input.validation_report_uri)
        .bind(input.output_evaluation_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(model_retraining_job_from_pg_row))
    }

    async fn dashboard_summary(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<DashboardSummaryRecord> {
        let suspected: (i64, Option<Decimal>) = sqlx::query_as(
            "SELECT COUNT(*)::bigint, COALESCE(SUM(c.claim_amount), 0)
             FROM scoring_runs sr
             LEFT JOIN claims c ON c.id = sr.claim_id
             WHERE sr.risk_score >= 70
               AND ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))",
        )
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;

        let rag_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT COALESCE(rag, 'UNKNOWN'), COUNT(*)::bigint
             FROM scoring_runs sr
             WHERE rag IS NOT NULL
               AND ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))
             GROUP BY rag
             ORDER BY rag",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;

        let rule_hits: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint
             FROM rule_runs rr
             JOIN scoring_runs sr ON sr.run_id = rr.run_id
             WHERE rr.matched = true
               AND ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))",
        )
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;

        let model_rows: Vec<(String, i64, Option<Decimal>, Option<i64>)> = sqlx::query_as(
            "SELECT model_key,
                    COUNT(*)::bigint,
                    AVG(score),
                    SUM(CASE WHEN score >= 70 THEN 1 ELSE 0 END)::bigint
             FROM model_scores ms
             JOIN scoring_runs sr ON sr.run_id = ms.run_id
             WHERE ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))
             GROUP BY model_key
             ORDER BY model_key",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;

        let layer_payloads: Vec<(Value,)> = sqlx::query_as(
            "SELECT payload
             FROM audit_events
             WHERE event_type = 'scoring.completed'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        let audit_coverage_row: (i64, Option<i64>) = sqlx::query_as(
            "SELECT COUNT(*)::bigint,
                    SUM(
                        CASE
                            WHEN jsonb_typeof(payload->'canonical_claim_context_trace') = 'object'
                            THEN 1
                            ELSE 0
                        END
                    )::bigint
             FROM audit_events
             WHERE event_type = 'scoring.completed'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)",
        )
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;
        let audit_coverage = summarize_dashboard_audit_coverage(
            audit_coverage_row.0 as u32,
            audit_coverage_row.1.unwrap_or(0) as u32,
        );
        let mut layer_accumulators = BTreeMap::<String, (String, u32, u32, u32)>::new();
        for (payload,) in layer_payloads {
            for layer in payload
                .get("layers")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                let layer_id = layer["layer_id"].as_str().unwrap_or("UNKNOWN").to_string();
                let layer_name = layer["name"].as_str().unwrap_or("Unknown").to_string();
                let layer_score = layer["score"].as_u64().unwrap_or(0) as u32;
                let entry =
                    layer_accumulators
                        .entry(layer_id)
                        .or_insert((layer_name.clone(), 0, 0, 0));
                entry.0 = layer_name;
                entry.1 += 1;
                entry.2 += layer_score;
                if layer_score >= 70 {
                    entry.3 += 1;
                }
            }
        }

        let investigation: (i64, i64, Option<Decimal>) = sqlx::query_as(
            "SELECT COUNT(*)::bigint,
                    COALESCE(SUM(CASE WHEN confirmed_fwa THEN 1 ELSE 0 END), 0)::bigint,
                    COALESCE(SUM(saving_amount), 0)
             FROM investigation_results ir
             WHERE ($1::text IS NULL OR EXISTS (
               SELECT 1 FROM audit_events ae
               WHERE ae.event_type = 'investigation.result.received'
                 AND ae.event_status = 'succeeded'
                 AND ae.payload ->> 'investigation_id' = ir.investigation_id
                 AND ae.payload ->> 'customer_scope_id' = $1
             ))",
        )
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;

        let qa_reviews: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint
             FROM qa_reviews qr
             WHERE ($1::text IS NULL OR EXISTS (
               SELECT 1 FROM audit_events ae
               WHERE ae.event_type = 'qa.result.received'
                 AND ae.event_status = 'succeeded'
                 AND ae.payload ->> 'qa_case_id' = qr.qa_case_id
                 AND ae.payload ->> 'customer_scope_id' = $1
             ))",
        )
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;

        let scheme_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT scheme_family, COUNT(*)::bigint
             FROM fwa_leads l
             WHERE ($1::text IS NULL OR EXISTS (
               SELECT 1 FROM audit_events ae
               WHERE ae.run_id = l.run_id
                 AND ae.event_type = 'scoring.completed'
                 AND ae.event_status = 'succeeded'
                 AND ae.payload ->> 'customer_scope_id' = $1
             ))
             GROUP BY scheme_family
             ORDER BY scheme_family",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;

        let financial_impact_rows: Vec<(bool, Option<String>, Option<Decimal>, Option<String>)> =
            sqlx::query_as(
                "SELECT confirmed_fwa, financial_impact_type, saving_amount, currency
                 FROM investigation_results ir
                 WHERE ($1::text IS NULL OR EXISTS (
                   SELECT 1 FROM audit_events ae
                   WHERE ae.event_type = 'investigation.result.received'
                     AND ae.event_status = 'succeeded'
                     AND ae.payload ->> 'investigation_id' = ir.investigation_id
                     AND ae.payload ->> 'customer_scope_id' = $1
                 ))
                 ORDER BY created_at, investigation_id",
            )
            .bind(customer_scope_id)
            .fetch_all(&self.pool)
            .await?;
        let financial_impacts = financial_impact_rows
            .into_iter()
            .filter_map(
                |(confirmed_fwa, financial_impact_type, saving_amount, currency)| {
                    financial_impact_from_parts(
                        confirmed_fwa,
                        financial_impact_type.as_deref(),
                        saving_amount,
                        currency,
                    )
                },
            )
            .collect::<Vec<_>>();

        let saving_attributions: Vec<(
            String,
            String,
            String,
            String,
            Option<Decimal>,
            String,
            i64,
            Vec<String>,
        )> = sqlx::query_as(
            "SELECT source_type,
                        source_id,
                        financial_impact_type,
                        action,
                        COALESCE(SUM(saving_amount), 0),
                        currency,
                        COUNT(DISTINCT claim_id)::bigint,
                        ARRAY_REMOVE(ARRAY_AGG(DISTINCT ref.value ORDER BY ref.value), NULL)
                 FROM saving_attributions s
                 LEFT JOIN LATERAL jsonb_array_elements_text(s.evidence_refs) AS ref(value) ON TRUE
                 WHERE ($1::text IS NULL OR EXISTS (
                   SELECT 1 FROM audit_events ae
                   WHERE ae.event_type = 'investigation.result.received'
                     AND ae.event_status = 'succeeded'
                     AND ae.payload ->> 'investigation_id' = s.investigation_id
                     AND ae.payload ->> 'customer_scope_id' = $1
                 ))
                 GROUP BY source_type, source_id, financial_impact_type, action, currency
                 ORDER BY source_type, source_id, financial_impact_type, action, currency",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        let saving_segments: Vec<(String, String, Option<Decimal>, String, i64, i64)> =
            sqlx::query_as(
                "SELECT segment_type,
                        segment_id,
                        COALESCE(SUM(saving_amount), 0),
                        currency,
                        COUNT(DISTINCT claim_id)::bigint,
                        COUNT(*)::bigint
                 FROM (
                   SELECT 'provider'::text AS segment_type,
                          COALESCE(l.provider_id, 'unknown') AS segment_id,
                          s.saving_amount,
                          s.currency,
                          s.claim_id
                   FROM saving_attributions s
                   LEFT JOIN fwa_leads l ON l.claim_id = s.claim_id
                   WHERE ($1::text IS NULL OR EXISTS (
                     SELECT 1 FROM audit_events ae
                     WHERE ae.event_type = 'investigation.result.received'
                       AND ae.event_status = 'succeeded'
                       AND ae.payload ->> 'investigation_id' = s.investigation_id
                       AND ae.payload ->> 'customer_scope_id' = $1
                   ))
                   UNION ALL
                   SELECT 'scheme'::text AS segment_type,
                          COALESCE(l.scheme_family, 'unknown') AS segment_id,
                          s.saving_amount,
                          s.currency,
                          s.claim_id
                   FROM saving_attributions s
                   LEFT JOIN fwa_leads l ON l.claim_id = s.claim_id
                   WHERE ($1::text IS NULL OR EXISTS (
                     SELECT 1 FROM audit_events ae
                     WHERE ae.event_type = 'investigation.result.received'
                       AND ae.event_status = 'succeeded'
                       AND ae.payload ->> 'investigation_id' = s.investigation_id
                       AND ae.payload ->> 'customer_scope_id' = $1
                   ))
                   UNION ALL
                   SELECT 'campaign'::text AS segment_type,
                          COALESCE(NULLIF(regexp_replace(ref.value, '^campaigns?:', ''), ''), 'unknown') AS segment_id,
                          s.saving_amount,
                          s.currency,
                          s.claim_id
                   FROM saving_attributions s
                   CROSS JOIN LATERAL jsonb_array_elements_text(s.evidence_refs) AS ref(value)
                   WHERE (ref.value LIKE 'campaign:%'
                      OR ref.value LIKE 'campaigns:%')
                     AND ($1::text IS NULL OR EXISTS (
                       SELECT 1 FROM audit_events ae
                       WHERE ae.event_type = 'investigation.result.received'
                         AND ae.event_status = 'succeeded'
                         AND ae.payload ->> 'investigation_id' = s.investigation_id
                         AND ae.payload ->> 'customer_scope_id' = $1
                     ))
                 ) segments
                 GROUP BY segment_type, segment_id, currency
                 ORDER BY segment_type, segment_id, currency",
            )
            .bind(customer_scope_id)
            .fetch_all(&self.pool)
            .await?;
        let outcome_labels = self.list_outcome_labels(customer_scope_id).await?;
        let audit_samples = self.list_audit_samples(customer_scope_id).await?;
        let qa_review_records = self.list_qa_reviews(customer_scope_id).await?;
        let qa_feedback_items = self.list_qa_feedback_items(customer_scope_id).await?;
        let agent_runs = self.list_agent_runs(customer_scope_id).await?;
        let models = self.list_models().await?;
        let model_evaluations = self.list_model_evaluations().await?;
        let rules = self.list_rules().await?;
        let rule_performance = self.rule_performance().await?;

        Ok(DashboardSummaryRecord {
            suspected_claims: suspected.0 as u32,
            confirmed_fwa: investigation.1 as u32,
            risk_amount: suspected.1.unwrap_or(Decimal::ZERO).to_string(),
            saving_amount: investigation.2.unwrap_or(Decimal::ZERO).to_string(),
            rag_distribution: rag_rows
                .into_iter()
                .map(|(rag, count)| (rag, count as u32))
                .collect(),
            scheme_distribution: scheme_rows
                .into_iter()
                .map(|(scheme_family, count)| (scheme_family, count as u32))
                .collect(),
            rule_hits: rule_hits.0 as u32,
            model_scores: model_rows
                .into_iter()
                .map(|(model_key, scored_runs, average_score, high_risk_count)| {
                    (
                        model_key,
                        DashboardModelScoreRecord {
                            scored_runs: scored_runs as u32,
                            average_score: average_score
                                .map(|value| value.to_string().parse().unwrap_or(0.0))
                                .unwrap_or(0.0),
                            high_risk_count: high_risk_count.unwrap_or(0) as u32,
                        },
                    )
                })
                .collect(),
            layer_scores: layer_accumulators
                .into_iter()
                .map(
                    |(layer_id, (name, scored_runs, score_sum, high_risk_count))| {
                        let average_score = if scored_runs == 0 {
                            0.0
                        } else {
                            score_sum as f64 / scored_runs as f64
                        };
                        (
                            layer_id,
                            DashboardLayerScoreRecord {
                                name,
                                scored_runs,
                                average_score,
                                high_risk_count,
                            },
                        )
                    },
                )
                .collect(),
            saving_attributions: saving_attributions
                .into_iter()
                .map(
                    |(
                        source_type,
                        source_id,
                        financial_impact_type,
                        action,
                        saving_amount,
                        currency,
                        claim_count,
                        evidence_refs,
                    )| {
                        DashboardSavingAttributionRecord {
                            source_type,
                            source_id,
                            financial_impact_type,
                            action,
                            saving_amount: format_decimal_cents(
                                saving_amount.unwrap_or(Decimal::ZERO),
                            ),
                            currency,
                            claim_count: claim_count as u32,
                            evidence_refs,
                        }
                    },
                )
                .collect(),
            saving_segments: saving_segments
                .into_iter()
                .map(
                    |(
                        segment_type,
                        segment_id,
                        saving_amount,
                        currency,
                        claim_count,
                        attribution_count,
                    )| {
                        let saving_amount = saving_amount.unwrap_or(Decimal::ZERO);
                        let claim_count = claim_count as u32;
                        DashboardSavingSegmentRecord {
                            segment_type,
                            segment_id,
                            saving_amount: format_decimal_cents(saving_amount),
                            currency,
                            claim_count,
                            attribution_count: attribution_count as u32,
                            roi: segment_roi(saving_amount, claim_count),
                        }
                    },
                )
                .collect(),
            value_measurement: summarize_dashboard_value_measurement(
                &financial_impacts,
                rule_hits.0 as u32,
                rule_performance
                    .iter()
                    .map(|record| record.false_positive_count)
                    .sum::<u32>(),
            ),
            audit_coverage,
            label_pool: summarize_dashboard_label_pool(&outcome_labels),
            qa_queue: summarize_dashboard_qa_queue(
                &audit_samples,
                &qa_review_records,
                &qa_feedback_items,
            ),
            case_sla: summarize_dashboard_case_sla(&self.list_cases(customer_scope_id).await?),
            agent_governance: summarize_dashboard_agent_governance(&agent_runs),
            model_governance: summarize_dashboard_model_governance(&models, &model_evaluations),
            rule_governance: summarize_dashboard_rule_governance(&rules, &rule_performance),
            investigation_results: investigation.0 as u32,
            qa_reviews: qa_reviews.0 as u32,
        })
    }

    async fn provider_risk_summary(&self) -> anyhow::Result<ProviderRiskSummaryRecord> {
        let rows: Vec<(Value,)> = sqlx::query_as(
            "SELECT payload
             FROM audit_events
             WHERE event_type = 'scoring.completed'
               AND event_status = 'succeeded'",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(summarize_provider_risk_profiles(
            rows.iter().map(|(payload,)| payload),
        ))
    }

    async fn list_knowledge_cases(&self) -> anyhow::Result<Vec<KnowledgeCaseRecord>> {
        ensure_default_knowledge_cases_seeded(&self.pool).await?;
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            Value,
        )> = sqlx::query_as(
            "SELECT case_id, title, fwa_type, scheme_family, diagnosis_code, provider_region, provider_type, summary, outcome, tags, evidence_refs
             FROM knowledge_cases
             ORDER BY case_id",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    case_id,
                    title,
                    fwa_type,
                    scheme_family,
                    diagnosis_code,
                    provider_region,
                    provider_type,
                    summary,
                    outcome,
                    tags,
                    evidence_refs,
                )| KnowledgeCaseRecord {
                    case_id,
                    title,
                    fwa_type,
                    scheme_family,
                    diagnosis_code,
                    provider_region,
                    provider_type,
                    summary,
                    outcome,
                    tags: json_array_to_strings(tags),
                    evidence_refs: json_array_to_strings(evidence_refs),
                },
            )
            .collect())
    }

    async fn save_knowledge_case(
        &self,
        record: KnowledgeCaseRecord,
    ) -> anyhow::Result<KnowledgeCaseRecord> {
        sqlx::query(
            "INSERT INTO knowledge_cases
             (case_id, title, fwa_type, scheme_family, diagnosis_code, provider_region, provider_type, summary, outcome, tags, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (case_id) DO UPDATE
             SET title = EXCLUDED.title,
                 fwa_type = EXCLUDED.fwa_type,
                 scheme_family = EXCLUDED.scheme_family,
                 diagnosis_code = EXCLUDED.diagnosis_code,
                 provider_region = EXCLUDED.provider_region,
                 provider_type = EXCLUDED.provider_type,
                 summary = EXCLUDED.summary,
                 outcome = EXCLUDED.outcome,
                 tags = EXCLUDED.tags,
                 evidence_refs = EXCLUDED.evidence_refs,
                 updated_at = now()",
        )
        .bind(&record.case_id)
        .bind(&record.title)
        .bind(&record.fwa_type)
        .bind(&record.scheme_family)
        .bind(&record.diagnosis_code)
        .bind(&record.provider_region)
        .bind(&record.provider_type)
        .bind(&record.summary)
        .bind(&record.outcome)
        .bind(serde_json::json!(record.tags))
        .bind(serde_json::json!(record.evidence_refs))
        .execute(&self.pool)
        .await?;
        Ok(record)
    }

    async fn search_similar_cases(
        &self,
        query: SimilarCaseQuery,
    ) -> anyhow::Result<Vec<SimilarCaseRecord>> {
        let cases = self.list_knowledge_cases().await?;
        Ok(search_cases(cases, &query))
    }

    async fn save_agent_run(&self, run: PersistedAgentRun) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO agent_runs
             (agent_run_id, claim_id, status, decision_boundary, output_json, evidence_refs, completed_at)
             VALUES ($1, $2, $3, $4, $5, $6, now())
             ON CONFLICT (agent_run_id) DO UPDATE
             SET status = EXCLUDED.status,
                 decision_boundary = EXCLUDED.decision_boundary,
                 output_json = EXCLUDED.output_json,
                 evidence_refs = EXCLUDED.evidence_refs,
                 completed_at = EXCLUDED.completed_at",
        )
        .bind(&run.agent_run_id)
        .bind(&run.claim_id)
        .bind(&run.status)
        .bind(&run.decision_boundary)
        .bind(&run.output_json)
        .bind(Value::Array(run.evidence_refs.clone()))
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM agent_steps WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM agent_context_snapshots WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM tool_results WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM agent_policy_checks WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM tool_calls WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM agent_approvals WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;

        for step in &run.steps {
            sqlx::query(
                "INSERT INTO agent_steps
                 (agent_run_id, step_name, status, output_json, evidence_refs)
                 VALUES ($1, $2, 'succeeded', $3, $4)",
            )
            .bind(&run.agent_run_id)
            .bind(step["step_name"].as_str().unwrap_or("investigate"))
            .bind(step)
            .bind(step["evidence_refs"].clone())
            .execute(&mut *tx)
            .await?;
        }
        for snapshot in &run.context_snapshots {
            sqlx::query(
                "INSERT INTO agent_context_snapshots
                 (snapshot_id, agent_run_id, redaction_status, context_json, source_refs, checksum)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&snapshot.snapshot_id)
            .bind(&run.agent_run_id)
            .bind(&snapshot.redaction_status)
            .bind(&snapshot.context_json)
            .bind(string_values(&snapshot.source_refs))
            .bind(&snapshot.checksum)
            .execute(&mut *tx)
            .await?;
        }
        for call in &run.tool_calls {
            sqlx::query(
                "INSERT INTO tool_calls
                 (tool_call_id, agent_run_id, tool_name, status, input_json, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&call.tool_call_id)
            .bind(&run.agent_run_id)
            .bind(&call.tool_name)
            .bind(&call.status)
            .bind(&call.input_json)
            .bind(string_values(&call.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }
        for check in &run.policy_checks {
            sqlx::query(
                "INSERT INTO agent_policy_checks
                 (policy_check_id, agent_run_id, tool_call_id, tool_name, policy_name, decision, reason, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(&check.policy_check_id)
            .bind(&run.agent_run_id)
            .bind(&check.tool_call_id)
            .bind(&check.tool_name)
            .bind(&check.policy_name)
            .bind(&check.decision)
            .bind(&check.reason)
            .bind(string_values(&check.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }
        for result in &run.tool_results {
            sqlx::query(
                "INSERT INTO tool_results
                 (tool_result_id, tool_call_id, agent_run_id, tool_name, status, output_json, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&result.tool_result_id)
            .bind(&result.tool_call_id)
            .bind(&run.agent_run_id)
            .bind(&result.tool_name)
            .bind(&result.status)
            .bind(&result.output_json)
            .bind(string_values(&result.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }
        for approval in &run.approvals {
            sqlx::query(
                "INSERT INTO agent_approvals
                 (approval_id, agent_run_id, proposed_action, decision, approver, reason, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&approval.approval_id)
            .bind(&run.agent_run_id)
            .bind(&approval.proposed_action)
            .bind(&approval.decision)
            .bind(&approval.approver)
            .bind(&approval.reason)
            .bind(string_values(&approval.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn list_agent_runs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AgentRunLogRecord>> {
        let rows: Vec<(
            String,
            String,
            String,
            String,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
            Option<chrono::DateTime<chrono::Utc>>,
        )> = sqlx::query_as(
            "SELECT agent_run_id, claim_id, status, decision_boundary, output_json, evidence_refs, created_at, completed_at
             FROM agent_runs ar
             WHERE (
               $1::text IS NULL OR EXISTS (
                 SELECT 1
                 FROM audit_events ae
                 LEFT JOIN claims c ON c.id = ae.claim_id
                 WHERE ae.payload ->> 'customer_scope_id' = $1
                   AND (
                     ae.payload ->> 'claim_id' = ar.claim_id
                     OR c.external_claim_id = ar.claim_id
                     OR ae.claim_id::text = ar.claim_id
                   )
               )
             )
             ORDER BY created_at DESC, agent_run_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;

        let mut runs = Vec::with_capacity(rows.len());
        for (
            agent_run_id,
            claim_id,
            status,
            decision_boundary,
            output_json,
            evidence_refs,
            created_at,
            completed_at,
        ) in rows
        {
            let steps: Vec<(Value,)> = sqlx::query_as(
                "SELECT output_json
                 FROM agent_steps
                 WHERE agent_run_id = $1
                 ORDER BY created_at, id",
            )
            .bind(&agent_run_id)
            .fetch_all(&self.pool)
            .await?;
            let context_snapshots = self.load_agent_context_snapshots(&agent_run_id).await?;
            let policy_checks = self.load_agent_policy_checks(&agent_run_id).await?;
            let tool_calls = self.load_agent_tool_calls(&agent_run_id).await?;
            let tool_results = self.load_agent_tool_results(&agent_run_id).await?;
            let approvals = self.load_agent_approvals(&agent_run_id).await?;
            runs.push(AgentRunLogRecord {
                agent_run_id,
                claim_id,
                status,
                decision_boundary,
                output_json,
                evidence_refs: json_array_to_strings(evidence_refs),
                steps: steps.into_iter().map(|row| row.0).collect(),
                context_snapshots,
                policy_checks,
                tool_calls,
                tool_results,
                approvals,
                created_at: Some(created_at.to_rfc3339()),
                completed_at: completed_at.map(|value| value.to_rfc3339()),
            });
        }

        Ok(runs)
    }

    async fn save_agent_approval(
        &self,
        approval: AgentApprovalRecord,
    ) -> anyhow::Result<AgentApprovalRecord> {
        sqlx::query(
            "INSERT INTO agent_approvals
             (approval_id, agent_run_id, proposed_action, decision, approver, reason, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (approval_id) DO UPDATE
             SET decision = EXCLUDED.decision,
                 approver = EXCLUDED.approver,
                 reason = EXCLUDED.reason,
                 evidence_refs = EXCLUDED.evidence_refs",
        )
        .bind(&approval.approval_id)
        .bind(&approval.agent_run_id)
        .bind(&approval.proposed_action)
        .bind(&approval.decision)
        .bind(&approval.approver)
        .bind(&approval.reason)
        .bind(string_values(&approval.evidence_refs))
        .execute(&self.pool)
        .await?;
        Ok(approval)
    }

    async fn register_dataset(&self, input: RegisterDatasetInput) -> anyhow::Result<DatasetRecord> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO external_data_sources
             (source_key, display_name, business_domain, owner, description, status)
             VALUES ($1, $2, $3, $4, $5, 'active')
             ON CONFLICT (source_key) DO UPDATE
             SET display_name = EXCLUDED.display_name,
                 business_domain = EXCLUDED.business_domain,
                 owner = EXCLUDED.owner,
                 description = EXCLUDED.description,
                 updated_at = now()",
        )
        .bind(&input.source_key)
        .bind(&input.display_name)
        .bind(&input.business_domain)
        .bind(&input.owner)
        .bind(&input.description)
        .execute(&mut *tx)
        .await?;

        let dataset_row: (String,) = sqlx::query_as(
            "INSERT INTO external_dataset_versions
             (source_key, dataset_key, dataset_version, sample_grain, label_column, entity_keys, manifest_uri, schema_uri, profile_uri, storage_format, schema_hash, row_count, status)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (dataset_key, dataset_version) DO UPDATE
             SET manifest_uri = EXCLUDED.manifest_uri,
                 schema_uri = EXCLUDED.schema_uri,
                 profile_uri = EXCLUDED.profile_uri,
                 schema_hash = EXCLUDED.schema_hash,
                 row_count = EXCLUDED.row_count,
                 status = EXCLUDED.status
             RETURNING id::text",
        )
        .bind(&input.source_key)
        .bind(&input.dataset_key)
        .bind(&input.dataset_version)
        .bind(&input.sample_grain)
        .bind(&input.label_column)
        .bind(serde_json::json!(input.entity_keys))
        .bind(&input.manifest_uri)
        .bind(&input.schema_uri)
        .bind(&input.profile_uri)
        .bind(&input.storage_format)
        .bind(&input.schema_hash)
        .bind(input.row_count as i64)
        .bind(&input.status)
        .fetch_one(&mut *tx)
        .await?;

        for split in &input.splits {
            sqlx::query(
                "INSERT INTO external_dataset_splits
                 (dataset_id, split_name, data_uri, row_count, positive_count, negative_count, label_distribution_json)
                 VALUES ($1::uuid, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (dataset_id, split_name) DO UPDATE
                 SET data_uri = EXCLUDED.data_uri,
                     row_count = EXCLUDED.row_count,
                     positive_count = EXCLUDED.positive_count,
                     negative_count = EXCLUDED.negative_count,
                     label_distribution_json = EXCLUDED.label_distribution_json",
            )
            .bind(&dataset_row.0)
            .bind(&split.split_name)
            .bind(&split.data_uri)
            .bind(split.row_count as i64)
            .bind(split.positive_count.map(|value| value as i64))
            .bind(split.negative_count.map(|value| value as i64))
            .bind(&split.label_distribution_json)
            .execute(&mut *tx)
            .await?;
        }

        for field in &input.fields {
            sqlx::query(
                "INSERT INTO external_schema_fields
                 (dataset_id, field_name, logical_type, nullable, semantic_role, description, profile_json)
                 VALUES ($1::uuid, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (dataset_id, field_name) DO UPDATE
                 SET logical_type = EXCLUDED.logical_type,
                     nullable = EXCLUDED.nullable,
                     semantic_role = EXCLUDED.semantic_role,
                     description = EXCLUDED.description,
                     profile_json = EXCLUDED.profile_json",
            )
            .bind(&dataset_row.0)
            .bind(&field.field_name)
            .bind(&field.logical_type)
            .bind(field.nullable)
            .bind(&field.semantic_role)
            .bind(&field.description)
            .bind(&field.profile_json)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        load_dataset_record(&self.pool, &dataset_row.0)
            .await?
            .ok_or_else(|| anyhow::anyhow!("registered dataset was not found"))
    }

    async fn list_datasets(&self) -> anyhow::Result<Vec<DatasetRecord>> {
        let ids: Vec<(String,)> = sqlx::query_as(
            "SELECT id::text FROM external_dataset_versions ORDER BY dataset_key, dataset_version",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut datasets = Vec::new();
        for (id,) in ids {
            if let Some(dataset) = load_dataset_record(&self.pool, &id).await? {
                datasets.push(dataset);
            }
        }
        Ok(datasets)
    }

    async fn get_dataset(&self, dataset_id: &str) -> anyhow::Result<Option<DatasetRecord>> {
        load_dataset_record(&self.pool, dataset_id).await
    }

    async fn add_field_mapping(
        &self,
        dataset_id: &str,
        input: CreateFieldMappingInput,
    ) -> anyhow::Result<Option<FieldMappingRecord>> {
        if load_dataset_record(&self.pool, dataset_id).await?.is_none() {
            return Ok(None);
        }

        let row: (String,) = sqlx::query_as(
            "INSERT INTO external_field_mappings
             (dataset_id, external_field, canonical_target, feature_name, transform_kind, transform_json, status)
             VALUES ($1::uuid, $2, $3, $4, $5, $6, $7)
             RETURNING id::text",
        )
        .bind(dataset_id)
        .bind(&input.external_field)
        .bind(&input.canonical_target)
        .bind(&input.feature_name)
        .bind(&input.transform_kind)
        .bind(&input.transform_json)
        .bind(&input.status)
        .fetch_one(&self.pool)
        .await?;

        Ok(Some(FieldMappingRecord {
            mapping_id: row.0,
            dataset_id: dataset_id.to_string(),
            external_field: input.external_field,
            canonical_target: input.canonical_target,
            feature_name: input.feature_name,
            transform_kind: input.transform_kind,
            transform_json: input.transform_json,
            status: input.status,
        }))
    }

    async fn save_investigation_result(
        &self,
        record: InvestigationResultRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        let saving_attributions = derive_saving_attributions(&record);
        let mut tx = self.pool.begin().await?;
        let previous_case_id: Option<String> = sqlx::query_scalar(
            "SELECT case_id FROM investigation_results WHERE investigation_id = $1",
        )
        .bind(&record.investigation_id)
        .fetch_optional(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO investigation_results
             (investigation_id, case_id, claim_id, outcome, confirmed_fwa, financial_impact_type, saving_amount, currency, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (investigation_id) DO UPDATE
             SET case_id = EXCLUDED.case_id,
                 claim_id = EXCLUDED.claim_id,
                 outcome = EXCLUDED.outcome,
                 confirmed_fwa = EXCLUDED.confirmed_fwa,
                 financial_impact_type = EXCLUDED.financial_impact_type,
                 saving_amount = EXCLUDED.saving_amount,
                 currency = EXCLUDED.currency,
                 notes = EXCLUDED.notes,
                 evidence_refs = EXCLUDED.evidence_refs",
        )
        .bind(&record.investigation_id)
        .bind(&record.case_id)
        .bind(&record.claim_id)
        .bind(&record.outcome)
        .bind(record.confirmed_fwa)
        .bind(normalize_financial_impact_type(
            record.financial_impact_type.as_deref(),
        ))
        .bind(record.saving_amount)
        .bind(&record.currency)
        .bind(&record.notes)
        .bind(serde_json::json!(record.evidence_refs))
        .execute(&mut *tx)
        .await?;

        if previous_case_id.as_deref() != record.case_id.as_deref() {
            if let Some(case_id) = previous_case_id.as_deref() {
                sqlx::query(
                    "UPDATE investigation_cases
                     SET final_outcome = NULL,
                         reviewer_notes = NULL,
                         investigation_result_id = NULL,
                         updated_at = now()
                     WHERE case_id = $1
                       AND investigation_result_id = $2",
                )
                .bind(case_id)
                .bind(&record.investigation_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        if let Some(case_id) = record.case_id.as_deref() {
            let update = sqlx::query(
                "UPDATE investigation_cases
                 SET final_outcome = $1,
                     reviewer_notes = $2,
                     investigation_result_id = $3,
                     updated_at = now()
                 WHERE case_id = $4
                   AND claim_id = $5",
            )
            .bind(&record.outcome)
            .bind(&record.notes)
            .bind(&record.investigation_id)
            .bind(case_id)
            .bind(&record.claim_id)
            .execute(&mut *tx)
            .await?;
            if update.rows_affected() == 0 {
                anyhow::bail!("case not found for investigation result: {case_id}");
            }
        }

        sqlx::query("DELETE FROM saving_attributions WHERE investigation_id = $1")
            .bind(&record.investigation_id)
            .execute(&mut *tx)
            .await?;
        for attribution in saving_attributions {
            sqlx::query(
                "INSERT INTO saving_attributions
                 (attribution_id, claim_id, investigation_id, source_type, source_id, financial_impact_type, action, saving_amount, currency, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(&attribution.attribution_id)
            .bind(&attribution.claim_id)
            .bind(&attribution.investigation_id)
            .bind(&attribution.source_type)
            .bind(&attribution.source_id)
            .bind(&attribution.financial_impact_type)
            .bind(&attribution.action)
            .bind(attribution.saving_amount)
            .bind(&attribution.currency)
            .bind(serde_json::json!(attribution.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }

        let event = AuditHistoryEventRecord {
            audit_id: format!("audit_investigation_{}", record.investigation_id),
            run_id: format!("pilot_investigation_{}", record.investigation_id),
            actor_role: record
                .actor_role
                .clone()
                .unwrap_or_else(|| "tpa_system".into()),
            event_type: "investigation.result.received".into(),
            event_status: "succeeded".into(),
            summary: format!("Investigation result received: {}", record.outcome),
            payload: serde_json::to_value(&record)?,
            evidence_refs: record.evidence_refs.clone(),
            created_at: None,
        };
        insert_pilot_audit_event(&mut tx, &record.claim_id, &event).await?;
        tx.commit().await?;
        Ok(event)
    }

    async fn save_qa_review(
        &self,
        mut record: QaReviewRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        record.feedback_target = canonical_feedback_target(&record.feedback_target).into();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO qa_reviews
             (qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, feedback_status, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, 'open', $6, $7)
             ON CONFLICT (qa_case_id) DO UPDATE
             SET qa_conclusion = EXCLUDED.qa_conclusion,
                 issue_type = EXCLUDED.issue_type,
                 feedback_target = EXCLUDED.feedback_target,
                 feedback_status = EXCLUDED.feedback_status,
                 notes = EXCLUDED.notes,
                 evidence_refs = EXCLUDED.evidence_refs",
        )
        .bind(&record.qa_case_id)
        .bind(&record.claim_id)
        .bind(&record.qa_conclusion)
        .bind(&record.issue_type)
        .bind(&record.feedback_target)
        .bind(&record.notes)
        .bind(serde_json::json!(record.evidence_refs))
        .execute(&mut *tx)
        .await?;

        let event = AuditHistoryEventRecord {
            audit_id: format!("audit_qa_{}", record.qa_case_id),
            run_id: format!("pilot_qa_{}", record.qa_case_id),
            actor_role: record
                .actor_role
                .clone()
                .unwrap_or_else(|| "tpa_system".into()),
            event_type: "qa.result.received".into(),
            event_status: "succeeded".into(),
            summary: format!("QA result received: {}", record.qa_conclusion),
            payload: serde_json::to_value(&record)?,
            evidence_refs: record.evidence_refs.clone(),
            created_at: None,
        };
        insert_pilot_audit_event(&mut tx, &record.claim_id, &event).await?;
        tx.commit().await?;
        Ok(event)
    }

    async fn list_qa_feedback_items(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaFeedbackItemRecord>> {
        let allowed_qa_case_ids = if let Some(scope) = customer_scope_id {
            Some(
                self.list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("qa.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| event.payload["qa_case_id"].as_str().map(str::to_string))
                .collect::<BTreeSet<_>>(),
            )
        } else {
            None
        };
        let mut status_events = self
            .list_audit_events(AuditEventListFilter {
                limit: 10_000,
                event_type: Some("qa.feedback.status.updated".into()),
                customer_scope_id: customer_scope_id.map(str::to_string),
                ..Default::default()
            })
            .await?;
        status_events.reverse();
        let feedback_statuses = latest_qa_feedback_statuses(
            &status_events
                .into_iter()
                .map(|event| {
                    (
                        event.payload["claim_id"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        event,
                    )
                })
                .collect::<Vec<_>>(),
        );
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, feedback_status, notes, evidence_refs, created_at
             FROM qa_reviews
             WHERE qa_conclusion <> 'pass'
             ORDER BY created_at, qa_case_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut items = rows
            .into_iter()
            .filter(|(qa_case_id, _, _, _, _, _, _, _, _)| {
                allowed_qa_case_ids
                    .as_ref()
                    .is_none_or(|ids| ids.contains(qa_case_id))
            })
            .map(
                |(
                    qa_case_id,
                    claim_id,
                    qa_conclusion,
                    issue_type,
                    feedback_target,
                    feedback_status,
                    notes,
                    evidence_refs,
                    created_at,
                )| {
                    let feedback_id = qa_feedback_id(&qa_case_id);
                    let status_update = feedback_statuses.get(&feedback_id);
                    qa_review_to_feedback_item(
                        QaReviewRecord {
                            qa_case_id,
                            claim_id,
                            qa_conclusion,
                            issue_type,
                            feedback_target,
                            notes,
                            evidence_refs: json_array_to_strings(evidence_refs),
                            customer_scope_id: None,
                            actor_id: None,
                            actor_role: None,
                        },
                        Some(created_at.to_rfc3339()),
                        &feedback_status,
                        status_update,
                    )
                },
            )
            .collect::<Vec<_>>();
        sort_qa_feedback_items(&mut items);
        Ok(items)
    }

    async fn update_qa_feedback_status(
        &self,
        feedback_id: &str,
        input: UpdateQaFeedbackStatusInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<UpdateQaFeedbackStatusRecord>> {
        let Some(qa_case_id) = qa_case_id_from_feedback_id(feedback_id) else {
            return Ok(None);
        };
        if let Some(scope) = customer_scope_id {
            let is_in_scope = self
                .list_audit_events(AuditEventListFilter {
                    limit: 1,
                    event_type: Some("qa.result.received".into()),
                    qa_case_id: Some(qa_case_id.into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .next()
                .is_some();
            if !is_in_scope {
                return Ok(None);
            }
        }
        let mut tx = self.pool.begin().await?;
        let row: Option<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "WITH existing AS (
                 SELECT qa_case_id, feedback_status AS from_status
                 FROM qa_reviews
                 WHERE qa_case_id = $1 AND qa_conclusion <> 'pass'
             ),
             updated AS (
                 UPDATE qa_reviews
                 SET feedback_status = $2
                 FROM existing
                 WHERE qa_reviews.qa_case_id = existing.qa_case_id
                 RETURNING existing.from_status,
                           qa_reviews.qa_case_id,
                           qa_reviews.claim_id,
                           qa_reviews.qa_conclusion,
                           qa_reviews.issue_type,
                           qa_reviews.feedback_target,
                           qa_reviews.feedback_status,
                           qa_reviews.notes,
                           qa_reviews.evidence_refs,
                           qa_reviews.created_at
             )
             SELECT * FROM updated",
        )
        .bind(qa_case_id)
        .bind(&input.status)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((
            from_status,
            qa_case_id,
            claim_id,
            qa_conclusion,
            issue_type,
            feedback_target,
            feedback_status,
            notes,
            evidence_refs,
            created_at,
        )) = row
        else {
            return Ok(None);
        };
        let audit_id = AuditEventId::new().to_string();
        let item = qa_review_to_feedback_item(
            QaReviewRecord {
                qa_case_id,
                claim_id: claim_id.clone(),
                qa_conclusion,
                issue_type,
                feedback_target,
                notes,
                evidence_refs: json_array_to_strings(evidence_refs),
                customer_scope_id: None,
                actor_id: None,
                actor_role: None,
            },
            Some(created_at.to_rfc3339()),
            &feedback_status,
            Some(&QaFeedbackStatusUpdate {
                status: feedback_status.clone(),
                actor_id: Some(input.actor_id.clone()),
                audit_id: audit_id.clone(),
                updated_at: None,
                evidence_refs: input.evidence_refs.clone(),
            }),
        );
        insert_pilot_audit_event(
            &mut tx,
            &claim_id,
            &AuditHistoryEventRecord {
                audit_id: audit_id.clone(),
                run_id: format!("qa_feedback_status_{}", item.feedback_id),
                actor_role: "fwa_operator".into(),
                event_type: "qa.feedback.status.updated".into(),
                event_status: "succeeded".into(),
                summary: format!("QA feedback status updated: {}", item.status),
                payload: serde_json::json!({
                    "feedback_id": item.feedback_id,
                    "qa_case_id": item.qa_case_id,
                    "claim_id": item.claim_id,
                    "feedback_target": item.feedback_target,
                    "from_status": from_status,
                    "to_status": item.status,
                    "actor_id": input.actor_id,
                    "notes": input.notes,
                    "customer_scope_id": input.customer_scope_id
                }),
                evidence_refs: input.evidence_refs,
                created_at: None,
            },
        )
        .await?;
        tx.commit().await?;
        Ok(Some(UpdateQaFeedbackStatusRecord { item, audit_id }))
    }

    async fn list_qa_reviews(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaReviewRecord>> {
        let allowed_qa_case_ids = if let Some(scope) = customer_scope_id {
            Some(
                self.list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("qa.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| event.payload["qa_case_id"].as_str().map(str::to_string))
                .collect::<BTreeSet<_>>(),
            )
        } else {
            None
        };
        let rows: Vec<(String, String, String, String, String, String, Value)> = sqlx::query_as(
            "SELECT qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, notes, evidence_refs
             FROM qa_reviews
             ORDER BY qa_case_id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter(|(qa_case_id, _, _, _, _, _, _)| {
                allowed_qa_case_ids
                    .as_ref()
                    .is_none_or(|ids| ids.contains(qa_case_id))
            })
            .map(
                |(
                    qa_case_id,
                    claim_id,
                    qa_conclusion,
                    issue_type,
                    feedback_target,
                    notes,
                    evidence_refs,
                )| {
                    let feedback_target = canonical_feedback_target(&feedback_target).into();
                    QaReviewRecord {
                        qa_case_id,
                        claim_id,
                        qa_conclusion,
                        issue_type,
                        feedback_target,
                        notes,
                        evidence_refs: json_array_to_strings(evidence_refs),
                        customer_scope_id: None,
                        actor_id: None,
                        actor_role: None,
                    }
                },
            )
            .collect())
    }

    async fn list_outcome_labels(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<OutcomeLabelRecord>> {
        let allowed_investigation_ids = if let Some(scope) = customer_scope_id {
            Some(
                self.list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("investigation.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| {
                    event.payload["investigation_id"]
                        .as_str()
                        .map(str::to_string)
                })
                .collect::<BTreeSet<_>>(),
            )
        } else {
            None
        };
        let allowed_qa_case_ids = if let Some(scope) = customer_scope_id {
            Some(
                self.list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("qa.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| event.payload["qa_case_id"].as_str().map(str::to_string))
                .collect::<BTreeSet<_>>(),
            )
        } else {
            None
        };
        let investigation_rows: Vec<(
            String,
            String,
            String,
            bool,
            Option<String>,
            Option<Decimal>,
            Option<String>,
            String,
            Value,
        )> = sqlx::query_as(
            "SELECT investigation_id, claim_id, outcome, confirmed_fwa, financial_impact_type, saving_amount, currency, notes, evidence_refs
             FROM investigation_results
             ORDER BY created_at, investigation_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let qa_rows: Vec<(String, String, String, String, String, String, String, Value)> =
            sqlx::query_as(
                "SELECT qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, feedback_status, notes, evidence_refs
                 FROM qa_reviews
                 ORDER BY created_at, qa_case_id",
            )
            .fetch_all(&self.pool)
            .await?;
        let medical_review_rows: Vec<(String, String, Value, Value)> = sqlx::query_as(
            "SELECT audit_id, actor_role, payload, evidence_refs
             FROM audit_events
             WHERE event_type = 'medical.review.recorded'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)
             ORDER BY created_at, audit_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        let lead_triage_rows: Vec<(String, String, String, Value, Value)> = sqlx::query_as(
            "SELECT audit_id, run_id, actor_role, payload, evidence_refs
             FROM audit_events
             WHERE event_type = 'lead.triaged'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)
             ORDER BY created_at, audit_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        let label_bootstrap_rows: Vec<(String, String, String, Value, Value)> = sqlx::query_as(
            "SELECT audit_id, run_id, actor_role, payload, evidence_refs
             FROM audit_events
             WHERE event_type = 'label.bootstrap.reviewed'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)
             ORDER BY created_at, audit_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;

        let mut labels = investigation_rows
            .into_iter()
            .filter(|(investigation_id, _, _, _, _, _, _, _, _)| {
                allowed_investigation_ids
                    .as_ref()
                    .is_none_or(|ids| ids.contains(investigation_id))
            })
            .flat_map(
                |(
                    investigation_id,
                    claim_id,
                    outcome,
                    confirmed_fwa,
                    financial_impact_type,
                    saving_amount,
                    currency,
                    notes,
                    evidence_refs,
                )| {
                    labels_from_investigation_result(InvestigationResultRecord {
                        investigation_id,
                        case_id: None,
                        claim_id,
                        outcome,
                        confirmed_fwa,
                        financial_impact_type,
                        saving_amount,
                        currency,
                        notes,
                        evidence_refs: json_array_to_strings(evidence_refs),
                        customer_scope_id: None,
                        actor_id: None,
                        actor_role: None,
                    })
                },
            )
            .chain(
                qa_rows
                    .into_iter()
                    .filter(|(qa_case_id, _, _, _, _, _, _, _)| {
                        allowed_qa_case_ids
                            .as_ref()
                            .is_none_or(|ids| ids.contains(qa_case_id))
                    })
                    .map(
                        |(
                            qa_case_id,
                            claim_id,
                            qa_conclusion,
                            issue_type,
                            feedback_target,
                            feedback_status,
                            notes,
                            evidence_refs,
                        )| {
                            label_from_qa_review(
                                QaReviewRecord {
                                    qa_case_id,
                                    claim_id,
                                    qa_conclusion,
                                    issue_type,
                                    feedback_target,
                                    notes,
                                    evidence_refs: json_array_to_strings(evidence_refs),
                                    customer_scope_id: None,
                                    actor_id: None,
                                    actor_role: None,
                                },
                                &feedback_status,
                            )
                        },
                    ),
            )
            .chain(medical_review_rows.into_iter().flat_map(
                |(audit_id, actor_role, payload, evidence_refs)| {
                    labels_from_medical_review_event(&AuditHistoryEventRecord {
                        audit_id,
                        run_id: String::new(),
                        actor_role,
                        event_type: "medical.review.recorded".into(),
                        event_status: "succeeded".into(),
                        summary: String::new(),
                        payload,
                        evidence_refs: json_array_to_strings(evidence_refs),
                        created_at: None,
                    })
                },
            ))
            .chain(label_bootstrap_rows.into_iter().filter_map(
                |(audit_id, run_id, actor_role, payload, evidence_refs)| {
                    label_from_bootstrap_review_event(&AuditHistoryEventRecord {
                        audit_id,
                        run_id,
                        actor_role,
                        event_type: "label.bootstrap.reviewed".into(),
                        event_status: "succeeded".into(),
                        summary: String::new(),
                        payload,
                        evidence_refs: json_array_to_strings(evidence_refs),
                        created_at: None,
                    })
                },
            ))
            .collect::<Vec<_>>();
        labels.extend(labels_from_lead_triage_events(
            lead_triage_rows.into_iter().map(
                |(audit_id, run_id, actor_role, payload, evidence_refs)| AuditHistoryEventRecord {
                    audit_id,
                    run_id,
                    actor_role,
                    event_type: "lead.triaged".into(),
                    event_status: "succeeded".into(),
                    summary: String::new(),
                    payload,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: None,
                },
            ),
        ));
        labels.extend(
            self.list_cases(None)
                .await?
                .into_iter()
                .flat_map(labels_from_case_status),
        );
        sort_outcome_labels(&mut labels);
        Ok(labels)
    }

    async fn claim_audit_history(
        &self,
        claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        let rows: Vec<(String, String, String, String, String, String, Value, Value, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as(
                "SELECT ae.audit_id, ae.run_id, ae.actor_role, ae.event_type, ae.event_status, ae.summary, ae.payload, ae.evidence_refs, ae.created_at
                 FROM audit_events ae
                 LEFT JOIN claims c ON c.id = ae.claim_id
                 WHERE (payload ->> 'claim_id' = $1 OR c.external_claim_id = $1)
                   AND ($2::text IS NULL OR ae.payload ->> 'customer_scope_id' = $2)
                 ORDER BY ae.created_at, ae.audit_id",
            )
            .bind(claim_id)
            .bind(customer_scope_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs,
                    created_at,
                )| AuditHistoryEventRecord {
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: Some(created_at.to_rfc3339()),
                },
            )
            .collect())
    }

    async fn list_audit_events(
        &self,
        filter: AuditEventListFilter,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT ae.audit_id, ae.run_id, ae.actor_role, ae.event_type, ae.event_status, ae.summary, ae.payload, ae.evidence_refs, ae.created_at
             FROM audit_events ae
             LEFT JOIN claims c ON c.id = ae.claim_id
             WHERE ($2::text IS NULL OR ae.event_type = $2)
               AND ($3::text IS NULL OR ae.actor_id = $3)
               AND ($4::text IS NULL OR ae.run_id = $4)
               AND (
                 $5::text IS NULL
                 OR ae.payload ->> 'claim_id' = $5
                 OR c.external_claim_id = $5
                 OR ae.claim_id::text = $5
               )
               AND ($6::text IS NULL OR ae.payload ->> 'policy_id' = $6)
               AND ($7::text IS NULL OR ae.payload ->> 'version' = $7)
               AND ($8::text IS NULL OR ae.payload ->> 'review_mode' = $8)
               AND ($9::text IS NULL OR ae.payload ->> 'rule_id' = $9)
               AND ($10::text IS NULL OR ae.payload ->> 'rule_version' = $10)
               AND ($11::text IS NULL OR ae.payload ->> 'model_key' = $11)
               AND ($12::text IS NULL OR ae.payload ->> 'model_version' = $12)
               AND (
                 $13::text IS NULL
                 OR (
                   $13 = 'governance'
                   AND ae.event_type = ANY($14::text[])
                 )
               )
               AND ($15::text IS NULL OR ae.payload ->> 'sample_id' = $15)
               AND ($16::text IS NULL OR ae.payload ->> 'agent_run_id' = $16)
               AND ($17::text IS NULL OR ae.payload ->> 'dataset_id' = $17)
               AND ($18::text IS NULL OR ae.payload ->> 'feature_set_id' = $18)
               AND ($19::text IS NULL OR ae.payload ->> 'model_dataset_id' = $19)
               AND ($20::text IS NULL OR ae.payload ->> 'evaluation_run_id' = $20)
               AND ($21::bool IS NULL OR $21 = false OR ae.payload ? 'canonical_claim_context_trace')
               AND ($22::text IS NULL OR ae.payload ->> 'customer_scope_id' = $22)
             ORDER BY ae.created_at DESC, ae.audit_id DESC
             LIMIT $1",
        )
        .bind(filter.limit as i64)
        .bind(filter.event_type.as_deref())
        .bind(filter.actor_id.as_deref())
        .bind(filter.run_id.as_deref())
        .bind(filter.claim_id.as_deref())
        .bind(filter.routing_policy_id.as_deref())
        .bind(filter.routing_policy_version.as_deref())
        .bind(filter.review_mode.as_deref())
        .bind(filter.rule_id.as_deref())
        .bind(filter.rule_version.as_deref())
        .bind(filter.model_key.as_deref())
        .bind(filter.model_version.as_deref())
        .bind(filter.event_group.as_deref())
        .bind(GOVERNANCE_AUDIT_EVENT_TYPES)
        .bind(filter.sample_id.as_deref())
        .bind(filter.agent_run_id.as_deref())
        .bind(filter.dataset_id.as_deref())
        .bind(filter.feature_set_id.as_deref())
        .bind(filter.model_dataset_id.as_deref())
        .bind(filter.evaluation_run_id.as_deref())
        .bind(filter.has_canonical_trace)
        .bind(filter.customer_scope_id.as_deref())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs,
                    created_at,
                )| AuditHistoryEventRecord {
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: Some(created_at.to_rfc3339()),
                },
            )
            .collect())
    }

    async fn list_webhook_events(&self) -> anyhow::Result<Vec<WebhookEventRecord>> {
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT audit_id, run_id, actor_role, event_type, event_status, summary, payload, evidence_refs, created_at
             FROM audit_events
             ORDER BY created_at, audit_id",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut events = rows
            .into_iter()
            .filter_map(
                |(
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs,
                    created_at,
                )| {
                    webhook_event_from_audit(
                        None,
                        &AuditHistoryEventRecord {
                            audit_id,
                            run_id,
                            actor_role,
                            event_type,
                            event_status,
                            summary,
                            payload,
                            evidence_refs: json_array_to_strings(evidence_refs),
                            created_at: Some(created_at.to_rfc3339()),
                        },
                    )
                },
            )
            .collect::<Vec<_>>();
        let attempt_rows: Vec<(
            String,
            i32,
            String,
            Option<i32>,
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT event_id, attempt_number, delivery_status, response_status_code, error_message, next_attempt_at, attempted_at
             FROM webhook_delivery_attempts
             ORDER BY event_id, attempt_number",
        )
        .fetch_all(&self.pool)
        .await?;
        let attempts = attempt_rows
            .into_iter()
            .map(
                |(
                    event_id,
                    attempt_number,
                    delivery_status,
                    response_status_code,
                    error_message,
                    next_attempt_at,
                    attempted_at,
                )| WebhookDeliveryAttemptRecord {
                    event_id,
                    attempt_number: attempt_number.max(0) as u32,
                    delivery_status,
                    response_status_code: response_status_code.map(|value| value.max(0) as u16),
                    error_message,
                    next_attempt_at: next_attempt_at.map(|timestamp| timestamp.to_rfc3339()),
                    attempted_at: Some(attempted_at.to_rfc3339()),
                },
            )
            .collect::<Vec<_>>();
        apply_webhook_delivery_state(&mut events, &attempts);
        sort_webhook_events(&mut events);
        Ok(events)
    }

    async fn save_webhook_delivery_attempt(
        &self,
        input: WebhookDeliveryAttemptInput,
    ) -> anyhow::Result<WebhookDeliveryAttemptRecord> {
        let row: (Option<i32>,) = sqlx::query_as(
            "SELECT MAX(attempt_number)
             FROM webhook_delivery_attempts
             WHERE event_id = $1",
        )
        .bind(&input.event_id)
        .fetch_one(&self.pool)
        .await?;
        let attempt_number = row.0.unwrap_or(0) + 1;
        let attempted_at = chrono::Utc::now();
        let next_attempt_at =
            next_webhook_attempt_at(&input.delivery_status, attempt_number as u32, attempted_at);
        let inserted: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
            "INSERT INTO webhook_delivery_attempts
             (event_id, attempt_number, delivery_status, response_status_code, error_message, next_attempt_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING attempted_at",
        )
        .bind(&input.event_id)
        .bind(attempt_number)
        .bind(&input.delivery_status)
        .bind(input.response_status_code.map(i32::from))
        .bind(&input.error_message)
        .bind(next_attempt_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(WebhookDeliveryAttemptRecord {
            event_id: input.event_id,
            attempt_number: attempt_number as u32,
            delivery_status: input.delivery_status,
            response_status_code: input.response_status_code,
            error_message: input.error_message,
            next_attempt_at: next_attempt_at.map(|timestamp| timestamp.to_rfc3339()),
            attempted_at: Some(inserted.0.to_rfc3339()),
        })
    }

    async fn register_feature_set(
        &self,
        input: RegisterFeatureSetInput,
    ) -> anyhow::Result<Option<FeatureSetRecord>> {
        if load_dataset_record(&self.pool, &input.dataset_id)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let row: (String,) = sqlx::query_as(
            "INSERT INTO feature_set_versions
             (feature_set_key, business_domain, version, dataset_id, features_uri, feature_list_json, row_count, label_column, status)
             VALUES ($1, $2, $3, $4::uuid, $5, $6, $7, $8, $9)
             ON CONFLICT (feature_set_key, version) DO UPDATE
             SET features_uri = EXCLUDED.features_uri,
                 feature_list_json = EXCLUDED.feature_list_json,
                 row_count = EXCLUDED.row_count,
                 status = EXCLUDED.status
             RETURNING id::text",
        )
        .bind(&input.feature_set_key)
        .bind(&input.business_domain)
        .bind(&input.version)
        .bind(&input.dataset_id)
        .bind(&input.features_uri)
        .bind(&input.feature_list_json)
        .bind(input.row_count as i64)
        .bind(&input.label_column)
        .bind(&input.status)
        .fetch_one(&self.pool)
        .await?;
        Ok(Some(FeatureSetRecord {
            feature_set_id: row.0,
            business_domain: input.business_domain,
            feature_set_key: input.feature_set_key,
            version: input.version,
            dataset_id: input.dataset_id,
            features_uri: input.features_uri,
            feature_list_json: input.feature_list_json,
            row_count: input.row_count,
            label_column: input.label_column,
            status: input.status,
        }))
    }

    async fn register_model_dataset(
        &self,
        input: RegisterModelDatasetInput,
    ) -> anyhow::Result<Option<ModelDatasetRecord>> {
        let feature_set_known: Option<(String,)> =
            sqlx::query_as("SELECT id::text FROM feature_set_versions WHERE id = $1::uuid")
                .bind(&input.feature_set_id)
                .fetch_optional(&self.pool)
                .await?;
        if feature_set_known.is_none() {
            return Ok(None);
        }

        let row: (String,) = sqlx::query_as(
            "INSERT INTO model_dataset_versions
             (business_domain, task_type, label_name, feature_set_id, train_uri, validation_uri, test_uri, row_counts_json, label_distribution_json, status)
             VALUES ($1, $2, $3, $4::uuid, $5, $6, $7, $8, $9, $10)
             RETURNING id::text",
        )
        .bind(&input.business_domain)
        .bind(&input.task_type)
        .bind(&input.label_name)
        .bind(&input.feature_set_id)
        .bind(&input.train_uri)
        .bind(&input.validation_uri)
        .bind(&input.test_uri)
        .bind(&input.row_counts_json)
        .bind(&input.label_distribution_json)
        .bind(&input.status)
        .fetch_one(&self.pool)
        .await?;

        Ok(Some(ModelDatasetRecord {
            model_dataset_id: row.0,
            business_domain: input.business_domain,
            task_type: input.task_type,
            label_name: input.label_name,
            feature_set_id: input.feature_set_id,
            train_uri: input.train_uri,
            validation_uri: input.validation_uri,
            test_uri: input.test_uri,
            row_counts_json: input.row_counts_json,
            label_distribution_json: input.label_distribution_json,
            status: input.status,
        }))
    }

    async fn get_model_dataset_source_dataset(
        &self,
        model_dataset_id: &str,
    ) -> anyhow::Result<Option<DatasetRecord>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT fs.dataset_id::text
             FROM model_dataset_versions md
             JOIN feature_set_versions fs ON fs.id = md.feature_set_id
             WHERE md.id = $1::uuid",
        )
        .bind(model_dataset_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some((dataset_id,)) = row else {
            return Ok(None);
        };
        load_dataset_record(&self.pool, &dataset_id).await
    }

    async fn register_model_evaluation(
        &self,
        input: RegisterModelEvaluationInput,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        let model_dataset_known: Option<(String,)> =
            sqlx::query_as("SELECT id::text FROM model_dataset_versions WHERE id = $1::uuid")
                .bind(&input.model_dataset_id)
                .fetch_optional(&self.pool)
                .await?;
        if model_dataset_known.is_none() {
            return Ok(None);
        }

        sqlx::query(
            "INSERT INTO model_evaluation_runs
             (evaluation_run_id, model_key, model_version, model_dataset_id, scheme_family, auc, ks, precision_value, recall_value, f1, accuracy, threshold, confusion_matrix_json, feature_importance_uri, metrics_json)
             VALUES ($1, $2, $3, $4::uuid, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             ON CONFLICT (evaluation_run_id) DO UPDATE
             SET model_key = EXCLUDED.model_key,
                 model_version = EXCLUDED.model_version,
                 model_dataset_id = EXCLUDED.model_dataset_id,
                 scheme_family = EXCLUDED.scheme_family,
                 auc = EXCLUDED.auc,
                 ks = EXCLUDED.ks,
                 precision_value = EXCLUDED.precision_value,
                 recall_value = EXCLUDED.recall_value,
                 f1 = EXCLUDED.f1,
                 accuracy = EXCLUDED.accuracy,
                 threshold = EXCLUDED.threshold,
                 confusion_matrix_json = EXCLUDED.confusion_matrix_json,
                 feature_importance_uri = EXCLUDED.feature_importance_uri,
                 metrics_json = EXCLUDED.metrics_json",
        )
        .bind(&input.evaluation_run_id)
        .bind(&input.model_key)
        .bind(&input.model_version)
        .bind(&input.model_dataset_id)
        .bind(&input.scheme_family)
        .bind(input.auc)
        .bind(input.ks)
        .bind(input.precision)
        .bind(input.recall)
        .bind(input.f1)
        .bind(input.accuracy)
        .bind(input.threshold)
        .bind(&input.confusion_matrix_json)
        .bind(&input.feature_importance_uri)
        .bind(&input.metrics_json)
        .execute(&self.pool)
        .await?;

        self.get_model_evaluation(&input.evaluation_run_id).await
    }

    async fn get_model_evaluation(
        &self,
        evaluation_run_id: &str,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        let row: Option<(
            String,
            String,
            String,
            String,
            String,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Value,
            Option<String>,
            Value,
        )> = sqlx::query_as(
            "SELECT evaluation_run_id, model_key, model_version, model_dataset_id::text, scheme_family, auc, ks, precision_value, recall_value, f1, accuracy, threshold, confusion_matrix_json, feature_importance_uri, metrics_json
             FROM model_evaluation_runs
             WHERE evaluation_run_id = $1",
        )
        .bind(evaluation_run_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                evaluation_run_id,
                model_key,
                model_version,
                model_dataset_id,
                scheme_family,
                auc,
                ks,
                precision,
                recall,
                f1,
                accuracy,
                threshold,
                confusion_matrix_json,
                feature_importance_uri,
                metrics_json,
            )| ModelEvaluationRecord {
                evaluation_run_id,
                model_key,
                model_version,
                model_dataset_id,
                scheme_family,
                auc,
                ks,
                precision,
                recall,
                f1,
                accuracy,
                threshold,
                confusion_matrix_json,
                feature_importance_uri,
                metrics_json,
            },
        ))
    }

    async fn list_model_evaluations(&self) -> anyhow::Result<Vec<ModelEvaluationRecord>> {
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Value,
            Option<String>,
            Value,
        )> = sqlx::query_as(
            "SELECT evaluation_run_id, model_key, model_version, model_dataset_id::text, scheme_family, auc, ks, precision_value, recall_value, f1, accuracy, threshold, confusion_matrix_json, feature_importance_uri, metrics_json
             FROM model_evaluation_runs
             ORDER BY evaluation_run_id",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    evaluation_run_id,
                    model_key,
                    model_version,
                    model_dataset_id,
                    scheme_family,
                    auc,
                    ks,
                    precision,
                    recall,
                    f1,
                    accuracy,
                    threshold,
                    confusion_matrix_json,
                    feature_importance_uri,
                    metrics_json,
                )| ModelEvaluationRecord {
                    evaluation_run_id,
                    model_key,
                    model_version,
                    model_dataset_id,
                    scheme_family,
                    auc,
                    ks,
                    precision,
                    recall,
                    f1,
                    accuracy,
                    threshold,
                    confusion_matrix_json,
                    feature_importance_uri,
                    metrics_json,
                },
            )
            .collect())
    }

    async fn save_evidence_document(
        &self,
        input: CreateEvidenceDocumentInput,
    ) -> anyhow::Result<EvidenceDocumentRecord> {
        let row = sqlx::query(
            "WITH input_claim AS (
               SELECT id FROM claims WHERE external_claim_id = $5 LIMIT 1
             )
             INSERT INTO evidence_documents
             (document_id, customer_scope_id, source_system, source_record_ref, claim_id, external_document_id, document_type, storage_uri, content_checksum, ingestion_status, redaction_status, retention_policy_id, evidence_refs, metadata_json)
             VALUES ($1, $2, $3, $4, (SELECT id FROM input_claim), $6, $7, $8, $9, $10, $11, $12, $13, $14)
             ON CONFLICT (document_id) DO UPDATE SET
               customer_scope_id = EXCLUDED.customer_scope_id,
               source_system = EXCLUDED.source_system,
               source_record_ref = EXCLUDED.source_record_ref,
               claim_id = EXCLUDED.claim_id,
               external_document_id = EXCLUDED.external_document_id,
               document_type = EXCLUDED.document_type,
               storage_uri = EXCLUDED.storage_uri,
               content_checksum = EXCLUDED.content_checksum,
               ingestion_status = EXCLUDED.ingestion_status,
               redaction_status = EXCLUDED.redaction_status,
               retention_policy_id = EXCLUDED.retention_policy_id,
               evidence_refs = EXCLUDED.evidence_refs,
               metadata_json = EXCLUDED.metadata_json,
               updated_at = now()
             RETURNING document_id, customer_scope_id, source_system, source_record_ref,
               (SELECT external_claim_id FROM claims WHERE id = evidence_documents.claim_id) AS claim_id,
               external_document_id, document_type, storage_uri, content_checksum, ingestion_status,
               redaction_status, retention_policy_id, evidence_refs, metadata_json, created_at, updated_at",
        )
        .bind(&input.document_id)
        .bind(&input.customer_scope_id)
        .bind(&input.source_system)
        .bind(&input.source_record_ref)
        .bind(&input.claim_id)
        .bind(&input.external_document_id)
        .bind(&input.document_type)
        .bind(&input.storage_uri)
        .bind(&input.content_checksum)
        .bind(&input.ingestion_status)
        .bind(&input.redaction_status)
        .bind(&input.retention_policy_id)
        .bind(string_values(&input.evidence_refs))
        .bind(&input.metadata_json)
        .fetch_one(&self.pool)
        .await?;
        evidence_document_from_row(row)
    }

    async fn list_evidence_documents(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentRecord>> {
        let rows = sqlx::query(
            "SELECT d.document_id, d.customer_scope_id, d.source_system, d.source_record_ref,
                    c.external_claim_id AS claim_id, d.external_document_id, d.document_type,
                    d.storage_uri, d.content_checksum, d.ingestion_status, d.redaction_status,
                    d.retention_policy_id, d.evidence_refs, d.metadata_json, d.created_at, d.updated_at
             FROM evidence_documents d
             LEFT JOIN claims c ON c.id = d.claim_id
             WHERE ($1::text IS NULL OR d.customer_scope_id = $1)
             ORDER BY d.document_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(evidence_document_from_row).collect()
    }

    async fn get_evidence_document(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentRecord>> {
        let row = sqlx::query(
            "SELECT d.document_id, d.customer_scope_id, d.source_system, d.source_record_ref,
                    c.external_claim_id AS claim_id, d.external_document_id, d.document_type,
                    d.storage_uri, d.content_checksum, d.ingestion_status, d.redaction_status,
                    d.retention_policy_id, d.evidence_refs, d.metadata_json, d.created_at, d.updated_at
             FROM evidence_documents d
             LEFT JOIN claims c ON c.id = d.claim_id
             WHERE d.document_id = $1
               AND ($2::text IS NULL OR d.customer_scope_id = $2)",
        )
        .bind(document_id)
        .bind(customer_scope_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(evidence_document_from_row).transpose()
    }

    async fn save_evidence_document_chunk(
        &self,
        input: CreateEvidenceDocumentChunkInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentChunkRecord>> {
        if self
            .get_evidence_document(&input.document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let row = sqlx::query(
            "INSERT INTO evidence_document_chunks
             (chunk_id, document_id, chunk_index, chunking_version, redaction_status, text_checksum, token_count, storage_uri, source_offsets_json, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (document_id, chunk_index, chunking_version) DO UPDATE SET
               redaction_status = EXCLUDED.redaction_status,
               text_checksum = EXCLUDED.text_checksum,
               token_count = EXCLUDED.token_count,
               storage_uri = EXCLUDED.storage_uri,
               source_offsets_json = EXCLUDED.source_offsets_json,
               evidence_refs = EXCLUDED.evidence_refs
             RETURNING chunk_id, document_id, chunk_index, chunking_version, redaction_status, text_checksum, token_count, storage_uri, source_offsets_json, evidence_refs, created_at",
        )
        .bind(&input.chunk_id)
        .bind(&input.document_id)
        .bind(input.chunk_index)
        .bind(&input.chunking_version)
        .bind(&input.redaction_status)
        .bind(&input.text_checksum)
        .bind(input.token_count)
        .bind(&input.storage_uri)
        .bind(&input.source_offsets_json)
        .bind(string_values(&input.evidence_refs))
        .fetch_one(&self.pool)
        .await?;
        Ok(Some(evidence_document_chunk_from_row(row)?))
    }

    async fn list_evidence_document_chunks(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentChunkRecord>> {
        if self
            .get_evidence_document(document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(Vec::new());
        }
        let rows = sqlx::query(
            "SELECT chunk_id, document_id, chunk_index, chunking_version, redaction_status, text_checksum, token_count, storage_uri, source_offsets_json, evidence_refs, created_at
             FROM evidence_document_chunks
             WHERE document_id = $1
             ORDER BY chunk_index, chunk_id",
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(evidence_document_chunk_from_row)
            .collect()
    }

    async fn save_evidence_ocr_output(
        &self,
        input: CreateEvidenceOcrOutputInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceOcrOutputRecord>> {
        if self
            .get_evidence_document(&input.document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let row = sqlx::query(
            "INSERT INTO evidence_ocr_outputs
             (ocr_output_id, document_id, ocr_engine, ocr_engine_version, output_uri, output_checksum, confidence_score, quality_status, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (ocr_output_id) DO UPDATE SET
               ocr_engine = EXCLUDED.ocr_engine,
               ocr_engine_version = EXCLUDED.ocr_engine_version,
               output_uri = EXCLUDED.output_uri,
               output_checksum = EXCLUDED.output_checksum,
               confidence_score = EXCLUDED.confidence_score,
               quality_status = EXCLUDED.quality_status,
               evidence_refs = EXCLUDED.evidence_refs
             RETURNING ocr_output_id, document_id, ocr_engine, ocr_engine_version, output_uri, output_checksum, confidence_score, quality_status, evidence_refs, created_at",
        )
        .bind(&input.ocr_output_id)
        .bind(&input.document_id)
        .bind(&input.ocr_engine)
        .bind(&input.ocr_engine_version)
        .bind(&input.output_uri)
        .bind(&input.output_checksum)
        .bind(input.confidence_score)
        .bind(&input.quality_status)
        .bind(string_values(&input.evidence_refs))
        .fetch_one(&self.pool)
        .await?;
        Ok(Some(evidence_ocr_output_from_row(row)?))
    }

    async fn list_evidence_ocr_outputs(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceOcrOutputRecord>> {
        if self
            .get_evidence_document(document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(Vec::new());
        }
        let rows = sqlx::query(
            "SELECT ocr_output_id, document_id, ocr_engine, ocr_engine_version, output_uri, output_checksum, confidence_score, quality_status, evidence_refs, created_at
             FROM evidence_ocr_outputs
             WHERE document_id = $1
             ORDER BY ocr_output_id",
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(evidence_ocr_output_from_row).collect()
    }

    async fn save_evidence_embedding_job(
        &self,
        input: CreateEvidenceEmbeddingJobInput,
    ) -> anyhow::Result<EvidenceEmbeddingJobRecord> {
        let row = sqlx::query(
            "INSERT INTO evidence_embedding_jobs
             (embedding_job_id, customer_scope_id, target_kind, target_ref, embedding_model, embedding_model_version, chunking_version, redaction_status, vector_store_kind, vector_store_ref, embedding_checksum, status, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (embedding_job_id) DO UPDATE SET
               customer_scope_id = EXCLUDED.customer_scope_id,
               target_kind = EXCLUDED.target_kind,
               target_ref = EXCLUDED.target_ref,
               embedding_model = EXCLUDED.embedding_model,
               embedding_model_version = EXCLUDED.embedding_model_version,
               chunking_version = EXCLUDED.chunking_version,
               redaction_status = EXCLUDED.redaction_status,
               vector_store_kind = EXCLUDED.vector_store_kind,
               vector_store_ref = EXCLUDED.vector_store_ref,
               embedding_checksum = EXCLUDED.embedding_checksum,
               status = EXCLUDED.status,
               evidence_refs = EXCLUDED.evidence_refs
             RETURNING embedding_job_id, customer_scope_id, target_kind, target_ref, embedding_model, embedding_model_version, chunking_version, redaction_status, vector_store_kind, vector_store_ref, embedding_checksum, status, evidence_refs, created_at, completed_at",
        )
        .bind(&input.embedding_job_id)
        .bind(&input.customer_scope_id)
        .bind(&input.target_kind)
        .bind(&input.target_ref)
        .bind(&input.embedding_model)
        .bind(&input.embedding_model_version)
        .bind(&input.chunking_version)
        .bind(&input.redaction_status)
        .bind(&input.vector_store_kind)
        .bind(&input.vector_store_ref)
        .bind(&input.embedding_checksum)
        .bind(&input.status)
        .bind(string_values(&input.evidence_refs))
        .fetch_one(&self.pool)
        .await?;
        evidence_embedding_job_from_row(row)
    }

    async fn list_evidence_embedding_jobs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceEmbeddingJobRecord>> {
        let rows = sqlx::query(
            "SELECT embedding_job_id, customer_scope_id, target_kind, target_ref, embedding_model, embedding_model_version, chunking_version, redaction_status, vector_store_kind, vector_store_ref, embedding_checksum, status, evidence_refs, created_at, completed_at
             FROM evidence_embedding_jobs
             WHERE ($1::text IS NULL OR customer_scope_id = $1)
             ORDER BY embedding_job_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(evidence_embedding_job_from_row)
            .collect()
    }

    async fn save_evidence_retrieval_audit_event(
        &self,
        input: CreateEvidenceRetrievalAuditEventInput,
    ) -> anyhow::Result<EvidenceRetrievalAuditEventRecord> {
        let row = sqlx::query(
            "INSERT INTO evidence_retrieval_audit_events
             (retrieval_id, customer_scope_id, actor_id, actor_role, query_kind, query_checksum, retrieval_method, embedding_model_version, top_k, source_refs, result_refs, redaction_status, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (retrieval_id) DO UPDATE SET
               customer_scope_id = EXCLUDED.customer_scope_id,
               actor_id = EXCLUDED.actor_id,
               actor_role = EXCLUDED.actor_role,
               query_kind = EXCLUDED.query_kind,
               query_checksum = EXCLUDED.query_checksum,
               retrieval_method = EXCLUDED.retrieval_method,
               embedding_model_version = EXCLUDED.embedding_model_version,
               top_k = EXCLUDED.top_k,
               source_refs = EXCLUDED.source_refs,
               result_refs = EXCLUDED.result_refs,
               redaction_status = EXCLUDED.redaction_status,
               evidence_refs = EXCLUDED.evidence_refs
             RETURNING retrieval_id, customer_scope_id, actor_id, actor_role, query_kind, query_checksum, retrieval_method, embedding_model_version, top_k, source_refs, result_refs, redaction_status, evidence_refs, created_at",
        )
        .bind(&input.retrieval_id)
        .bind(&input.customer_scope_id)
        .bind(&input.actor_id)
        .bind(&input.actor_role)
        .bind(&input.query_kind)
        .bind(&input.query_checksum)
        .bind(&input.retrieval_method)
        .bind(&input.embedding_model_version)
        .bind(input.top_k)
        .bind(string_values(&input.source_refs))
        .bind(string_values(&input.result_refs))
        .bind(&input.redaction_status)
        .bind(string_values(&input.evidence_refs))
        .fetch_one(&self.pool)
        .await?;
        evidence_retrieval_audit_event_from_row(row)
    }

    async fn list_evidence_retrieval_audit_events(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceRetrievalAuditEventRecord>> {
        let rows = sqlx::query(
            "SELECT retrieval_id, customer_scope_id, actor_id, actor_role, query_kind, query_checksum, retrieval_method, embedding_model_version, top_k, source_refs, result_refs, redaction_status, evidence_refs, created_at
             FROM evidence_retrieval_audit_events
             WHERE ($1::text IS NULL OR customer_scope_id = $1)
             ORDER BY created_at DESC, retrieval_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(evidence_retrieval_audit_event_from_row)
            .collect()
    }
}

fn evidence_document_from_row(row: PgRow) -> anyhow::Result<EvidenceDocumentRecord> {
    Ok(EvidenceDocumentRecord {
        document_id: row.try_get("document_id")?,
        customer_scope_id: row.try_get("customer_scope_id")?,
        source_system: row.try_get("source_system")?,
        source_record_ref: row.try_get("source_record_ref")?,
        claim_id: row.try_get("claim_id")?,
        external_document_id: row.try_get("external_document_id")?,
        document_type: row.try_get("document_type")?,
        storage_uri: row.try_get("storage_uri")?,
        content_checksum: row.try_get("content_checksum")?,
        ingestion_status: row.try_get("ingestion_status")?,
        redaction_status: row.try_get("redaction_status")?,
        retention_policy_id: row.try_get("retention_policy_id")?,
        evidence_refs: json_array_to_strings(row.try_get("evidence_refs")?),
        metadata_json: row.try_get("metadata_json")?,
        created_at: timestamp_from_row(&row, "created_at")?,
        updated_at: timestamp_from_row(&row, "updated_at")?,
    })
}

fn evidence_document_chunk_from_row(row: PgRow) -> anyhow::Result<EvidenceDocumentChunkRecord> {
    Ok(EvidenceDocumentChunkRecord {
        chunk_id: row.try_get("chunk_id")?,
        document_id: row.try_get("document_id")?,
        chunk_index: row.try_get("chunk_index")?,
        chunking_version: row.try_get("chunking_version")?,
        redaction_status: row.try_get("redaction_status")?,
        text_checksum: row.try_get("text_checksum")?,
        token_count: row.try_get("token_count")?,
        storage_uri: row.try_get("storage_uri")?,
        source_offsets_json: row.try_get("source_offsets_json")?,
        evidence_refs: json_array_to_strings(row.try_get("evidence_refs")?),
        created_at: timestamp_from_row(&row, "created_at")?,
    })
}

fn evidence_ocr_output_from_row(row: PgRow) -> anyhow::Result<EvidenceOcrOutputRecord> {
    Ok(EvidenceOcrOutputRecord {
        ocr_output_id: row.try_get("ocr_output_id")?,
        document_id: row.try_get("document_id")?,
        ocr_engine: row.try_get("ocr_engine")?,
        ocr_engine_version: row.try_get("ocr_engine_version")?,
        output_uri: row.try_get("output_uri")?,
        output_checksum: row.try_get("output_checksum")?,
        confidence_score: row.try_get("confidence_score")?,
        quality_status: row.try_get("quality_status")?,
        evidence_refs: json_array_to_strings(row.try_get("evidence_refs")?),
        created_at: timestamp_from_row(&row, "created_at")?,
    })
}

fn evidence_embedding_job_from_row(row: PgRow) -> anyhow::Result<EvidenceEmbeddingJobRecord> {
    Ok(EvidenceEmbeddingJobRecord {
        embedding_job_id: row.try_get("embedding_job_id")?,
        customer_scope_id: row.try_get("customer_scope_id")?,
        target_kind: row.try_get("target_kind")?,
        target_ref: row.try_get("target_ref")?,
        embedding_model: row.try_get("embedding_model")?,
        embedding_model_version: row.try_get("embedding_model_version")?,
        chunking_version: row.try_get("chunking_version")?,
        redaction_status: row.try_get("redaction_status")?,
        vector_store_kind: row.try_get("vector_store_kind")?,
        vector_store_ref: row.try_get("vector_store_ref")?,
        embedding_checksum: row.try_get("embedding_checksum")?,
        status: row.try_get("status")?,
        evidence_refs: json_array_to_strings(row.try_get("evidence_refs")?),
        created_at: timestamp_from_row(&row, "created_at")?,
        completed_at: timestamp_from_row(&row, "completed_at")?,
    })
}

fn evidence_retrieval_audit_event_from_row(
    row: PgRow,
) -> anyhow::Result<EvidenceRetrievalAuditEventRecord> {
    Ok(EvidenceRetrievalAuditEventRecord {
        retrieval_id: row.try_get("retrieval_id")?,
        customer_scope_id: row.try_get("customer_scope_id")?,
        actor_id: row.try_get("actor_id")?,
        actor_role: row.try_get("actor_role")?,
        query_kind: row.try_get("query_kind")?,
        query_checksum: row.try_get("query_checksum")?,
        retrieval_method: row.try_get("retrieval_method")?,
        embedding_model_version: row.try_get("embedding_model_version")?,
        top_k: row.try_get("top_k")?,
        source_refs: json_array_to_strings(row.try_get("source_refs")?),
        result_refs: json_array_to_strings(row.try_get("result_refs")?),
        redaction_status: row.try_get("redaction_status")?,
        evidence_refs: json_array_to_strings(row.try_get("evidence_refs")?),
        created_at: timestamp_from_row(&row, "created_at")?,
    })
}

fn timestamp_from_row(row: &PgRow, column: &str) -> anyhow::Result<Option<String>> {
    let value: Option<chrono::DateTime<chrono::Utc>> = row.try_get(column)?;
    Ok(value.map(|timestamp| timestamp.to_rfc3339()))
}

fn _decimal_keeps_sqlx_feature_linked(_: Decimal) {}

const RULE_REVIEW_COST_AMOUNT: f64 = 100.0;

#[derive(Debug, Clone)]
struct InvestigationOutcome {
    confirmed_fwa: bool,
    saving_amount: Decimal,
}

#[derive(Debug, Clone)]
struct RulePerformanceAccumulator {
    rule_id: String,
    alert_code: String,
    trigger_count: u32,
    triggered_claim_ids: BTreeSet<String>,
}

fn rule_accumulators_from_rules(
    rules: &[RuleSummaryRecord],
) -> BTreeMap<String, RulePerformanceAccumulator> {
    rules
        .iter()
        .map(|rule| {
            (
                rule.rule_id.clone(),
                RulePerformanceAccumulator {
                    rule_id: rule.rule_id.clone(),
                    alert_code: rule.alert_code.clone(),
                    trigger_count: 0,
                    triggered_claim_ids: BTreeSet::new(),
                },
            )
        })
        .collect()
}

fn rule_performance_records(
    accumulators: BTreeMap<String, RulePerformanceAccumulator>,
    outcomes: &HashMap<String, InvestigationOutcome>,
    total_scoring_runs: u32,
) -> Vec<RulePerformanceRecord> {
    accumulators
        .into_values()
        .map(|accumulator| {
            let mut reviewed_count = 0_u32;
            let mut confirmed_fwa_count = 0_u32;
            let mut false_positive_count = 0_u32;
            let mut saving_amount = Decimal::ZERO;

            for claim_id in &accumulator.triggered_claim_ids {
                let Some(outcome) = outcomes.get(claim_id) else {
                    continue;
                };
                reviewed_count += 1;
                if outcome.confirmed_fwa {
                    confirmed_fwa_count += 1;
                    saving_amount += outcome.saving_amount;
                } else {
                    false_positive_count += 1;
                }
            }

            let trigger_count = accumulator.trigger_count;
            let mark_rate = ratio(trigger_count, total_scoring_runs);
            let precision = ratio(confirmed_fwa_count, reviewed_count);
            let false_positive_rate = ratio(false_positive_count, reviewed_count);
            let roi = if trigger_count == 0 {
                0.0
            } else {
                let saving = saving_amount.to_string().parse::<f64>().unwrap_or(0.0);
                saving / (trigger_count as f64 * RULE_REVIEW_COST_AMOUNT)
            };

            RulePerformanceRecord {
                rule_id: accumulator.rule_id,
                alert_code: accumulator.alert_code,
                trigger_count,
                reviewed_count,
                confirmed_fwa_count,
                false_positive_count,
                mark_rate,
                precision,
                false_positive_rate,
                saving_amount: format!("{:.2}", saving_amount.round_dp(2)),
                roi,
            }
        })
        .collect()
}

fn ratio(numerator: u32, denominator: u32) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn decimal_from_json(value: &Value) -> Decimal {
    if let Some(value) = value.as_str() {
        return value.parse::<Decimal>().unwrap_or(Decimal::ZERO);
    }
    if let Some(value) = value.as_f64() {
        return Decimal::from_f64_retain(value).unwrap_or(Decimal::ZERO);
    }
    Decimal::ZERO
}

fn lead_from_scoring_run(
    run: &PersistedScoringRun,
    context: Option<&ClaimContext>,
) -> Option<LeadRecord> {
    if run.risk_score < 70 {
        return None;
    }
    let evidence_refs = evidence_values_to_strings(&run.evidence_refs);
    Some(LeadRecord {
        lead_id: format!("lead_{}", run.claim_id),
        run_id: run.run_id.clone(),
        claim_id: run.claim_id.clone(),
        member_id: context
            .map(|context| context.member.external_member_id.clone())
            .unwrap_or_default(),
        provider_id: context
            .map(|context| context.provider.external_provider_id.clone())
            .unwrap_or_default(),
        source_system: run.source_system.clone(),
        review_mode: run
            .audit_event
            .get("review_mode")
            .and_then(Value::as_str)
            .unwrap_or("pre_payment")
            .to_string(),
        scheme_family: scheme_family_from_rule_runs(&run.rule_runs),
        lead_source: "scoring_run".into(),
        status: "new".into(),
        disposition: "pending_triage".into(),
        risk_score: run.risk_score,
        rag: run.rag.clone(),
        reason: run.routing_reason.clone(),
        evidence_refs,
    })
}

fn control_lead_from_scoring_run(
    run: &PersistedScoringRun,
    context: Option<&ClaimContext>,
) -> LeadRecord {
    let mut evidence_refs = evidence_values_to_strings(&run.evidence_refs);
    if evidence_refs.is_empty() {
        evidence_refs.push(format!("audit:{}", run.audit_id));
    }
    LeadRecord {
        lead_id: format!("control_lead_{}", run.claim_id),
        run_id: run.run_id.clone(),
        claim_id: run.claim_id.clone(),
        member_id: context
            .map(|context| context.member.external_member_id.clone())
            .unwrap_or_default(),
        provider_id: context
            .map(|context| context.provider.external_provider_id.clone())
            .unwrap_or_default(),
        source_system: run.source_system.clone(),
        review_mode: run
            .audit_event
            .get("review_mode")
            .and_then(Value::as_str)
            .unwrap_or("pre_payment")
            .to_string(),
        scheme_family: if run.risk_score >= 70 {
            scheme_family_from_rule_runs(&run.rule_runs)
        } else {
            "control_baseline".into()
        },
        lead_source: "random_control_scoring_run".into(),
        status: "new".into(),
        disposition: "pending_control_review".into(),
        risk_score: run.risk_score,
        rag: run.rag.clone(),
        reason: format!("Random control baseline sample: {}", run.routing_reason),
        evidence_refs,
    }
}

fn scheme_family_from_rule_runs(rule_runs: &[Value]) -> String {
    let alert_codes = rule_runs
        .iter()
        .filter_map(|run| run["alert_code"].as_str())
        .collect::<Vec<_>>();
    if alert_codes
        .iter()
        .any(|code| code.contains("DIAGNOSIS") || code.contains("MEDICAL"))
    {
        "diagnosis_procedure_mismatch".into()
    } else if alert_codes.iter().any(|code| code.contains("PROVIDER")) {
        "provider_peer_outlier".into()
    } else if alert_codes
        .iter()
        .any(|code| code.contains("EARLY") || code.contains("LIMIT"))
    {
        "early_high_value_claim".into()
    } else {
        "high_risk_claim".into()
    }
}

fn scheme_family_from_dsl(dsl: &Value) -> String {
    dsl.get("scheme_family")
        .and_then(Value::as_str)
        .or_else(|| dsl["action"]["scheme_family"].as_str())
        .map(normalize_scheme_family)
        .unwrap_or_else(|| {
            scheme_family_from_alert_code(dsl["action"]["alert_code"].as_str().unwrap_or(""))
        })
}

pub(crate) fn normalize_scheme_family(value: &str) -> String {
    canonical_scheme_family(value).unwrap_or_else(|| "high_risk_claim".into())
}

fn scheme_family_from_alert_code(alert_code: &str) -> String {
    let code = alert_code.to_ascii_uppercase();
    if code.contains("DUPLICATE") {
        "duplicate_billing".into()
    } else if code.contains("UPCOD") {
        "upcoding".into()
    } else if code.contains("UNBUND") {
        "unbundling".into()
    } else if code.contains("UNNECESSARY") {
        "medically_unnecessary_service".into()
    } else if code.contains("REPEATED")
        || code.contains("EXCESSIVE")
        || code.contains("UTILIZATION")
    {
        "excessive_utilization".into()
    } else if code.contains("DIAGNOSIS") || code.contains("MEDICAL") || code.contains("LOW_MEDICAL")
    {
        "diagnosis_procedure_mismatch".into()
    } else if code.contains("LAB") {
        "laboratory_testing_abuse".into()
    } else if code.contains("TELE") {
        "telehealth_abuse".into()
    } else if code.contains("GENETIC") {
        "genetic_testing_abuse".into()
    } else if code.contains("PHARMACY") || code.contains("OPIOID") || code.contains("CONTROLLED") {
        "pharmacy_controlled_substance_abuse".into()
    } else if code.contains("DME")
        || code.contains("HOME_HEALTH")
        || code.contains("HOSPICE")
        || code.contains("REHAB")
    {
        "dme_home_health_hospice_rehab_risk".into()
    } else if code.contains("PROVIDER") {
        "provider_peer_outlier".into()
    } else if code.contains("REFERRAL") || code.contains("OWNERSHIP") || code.contains("RELATION") {
        "relationship_concentration".into()
    } else if code.contains("EARLY") || code.contains("LIMIT") {
        "early_high_value_claim".into()
    } else if code.contains("MANY") || code.contains("HIGH_COST") || code.contains("PEER") {
        "excessive_utilization".into()
    } else {
        "high_risk_claim".into()
    }
}

pub(crate) fn scheme_family_from_knowledge_signals(fwa_type: &str, tags: &[String]) -> String {
    if let Some(scheme_family) = tags
        .iter()
        .find_map(|tag| tag.strip_prefix("scheme:").map(normalize_scheme_family))
    {
        return scheme_family;
    }

    if tags
        .iter()
        .any(|tag| tag.contains("medical_mismatch") || tag.contains("diagnosis"))
    {
        "diagnosis_procedure_mismatch".into()
    } else if tags
        .iter()
        .any(|tag| tag.contains("lab") || tag.contains("testing"))
    {
        "laboratory_testing_abuse".into()
    } else if tags.iter().any(|tag| tag.contains("provider")) {
        "provider_peer_outlier".into()
    } else if tags
        .iter()
        .any(|tag| tag.contains("early") || tag.contains("high_amount"))
    {
        "early_high_value_claim".into()
    } else {
        match fwa_type {
            "Waste" => "excessive_utilization".into(),
            "Abuse" => "high_risk_claim".into(),
            "Fraud" => "relationship_concentration".into(),
            _ => "high_risk_claim".into(),
        }
    }
}

fn case_from_lead(lead: &LeadRecord, input: &TriageLeadInput) -> CaseRecord {
    let sla_target_hours = sla_target_hours_for_priority(&input.priority);
    let evidence_sufficiency =
        assess_evidence_sufficiency(&lead.scheme_family, &case_evidence_text(lead, input));
    CaseRecord {
        case_id: format!("case_{}", lead.claim_id),
        lead_id: lead.lead_id.clone(),
        claim_id: lead.claim_id.clone(),
        member_id: lead.member_id.clone(),
        provider_id: lead.provider_id.clone(),
        source_system: lead.source_system.clone(),
        review_mode: lead.review_mode.clone(),
        scheme_family: lead.scheme_family.clone(),
        lead_source: lead.lead_source.clone(),
        status: "triage".into(),
        assignee: input.assignee.clone(),
        reviewer: input.reviewer.clone(),
        priority: input.priority.clone(),
        routing_reason: lead.reason.clone(),
        evidence_package: serde_json::json!({
            "lead_id": lead.lead_id.clone(),
            "claim_id": lead.claim_id.clone(),
            "review_mode": lead.review_mode.clone(),
            "risk_score": lead.risk_score,
            "rag": lead.rag.clone(),
            "reason": lead.reason.clone(),
            "triage_notes": input.notes.clone(),
            "evidence_sufficiency": evidence_sufficiency,
            "evidence_refs": triage_case_evidence_refs(lead, input),
            "evidence_refs_by_type": triage_case_evidence_refs_by_type(lead, input)
        }),
        sla_target_hours,
        sla_status: case_sla_status("triage", sla_target_hours, 0.0),
        time_to_triage_hours: 0.0,
        time_to_closure_hours: None,
        final_outcome: None,
        reviewer_notes: None,
        investigation_result_id: None,
    }
}

fn case_evidence_text(lead: &LeadRecord, input: &TriageLeadInput) -> String {
    let mut parts = vec![
        lead.claim_id.clone(),
        lead.member_id.clone(),
        lead.provider_id.clone(),
        lead.scheme_family.clone(),
        lead.reason.clone(),
        input.notes.clone(),
    ];
    parts.extend(lead.evidence_refs.clone());
    parts.extend(input.evidence_refs.clone());
    parts.join(" ")
}

fn triage_case_evidence_refs(lead: &LeadRecord, input: &TriageLeadInput) -> Vec<String> {
    let mut refs = lead.evidence_refs.clone();
    refs.extend(input.evidence_refs.clone());
    refs.push(format!("claims:{}", lead.claim_id));
    refs.push(format!("scoring_runs:{}:anomaly_score", lead.run_id));
    refs.sort();
    refs.dedup();
    refs
}

fn triage_case_evidence_refs_by_type(lead: &LeadRecord, input: &TriageLeadInput) -> Value {
    let mut claim = BTreeSet::from([format!("claims:{}", lead.claim_id)]);
    let mut rule = BTreeSet::new();
    let mut model = BTreeSet::new();
    let mut anomaly = BTreeSet::from([format!("scoring_runs:{}:anomaly_score", lead.run_id)]);
    let mut document = BTreeSet::new();
    let mut similar_case = BTreeSet::new();

    for reference in triage_case_evidence_refs(lead, input) {
        match evidence_ref_bucket(&reference) {
            Some("claim") => {
                claim.insert(reference);
            }
            Some("rule") => {
                rule.insert(reference);
            }
            Some("model") => {
                model.insert(reference);
            }
            Some("anomaly") => {
                anomaly.insert(reference);
            }
            Some("document") => {
                document.insert(reference);
            }
            Some("similar_case") => {
                similar_case.insert(reference);
            }
            _ => {}
        }
    }

    serde_json::json!({
        "claim": claim.into_iter().collect::<Vec<_>>(),
        "rule": rule.into_iter().collect::<Vec<_>>(),
        "model": model.into_iter().collect::<Vec<_>>(),
        "anomaly": anomaly.into_iter().collect::<Vec<_>>(),
        "document": document.into_iter().collect::<Vec<_>>(),
        "similar_case": similar_case.into_iter().collect::<Vec<_>>(),
    })
}

fn evidence_ref_bucket(reference: &str) -> Option<&'static str> {
    if let Ok(value) = serde_json::from_str::<Value>(reference) {
        if let Some(entity_type) = value.get("entity_type").and_then(Value::as_str) {
            return match entity_type {
                "claim" | "member" | "policy" | "provider" | "claim_item" => Some("claim"),
                "rule" | "rule_run" => Some("rule"),
                "model" | "model_score" | "model_version" => Some("model"),
                "document" | "document_chunk" | "ocr" => Some("document"),
                _ => None,
            };
        }
    }

    if reference.starts_with("knowledge_cases:")
        || reference.starts_with("retrieval:")
        || reference.starts_with("matched_signal:")
        || reference.starts_with("query_claim:")
    {
        Some("similar_case")
    } else if reference.starts_with("rule_runs:") || reference.starts_with("rules:") {
        Some("rule")
    } else if reference.starts_with("model_scores:") || reference.starts_with("model_versions:") {
        Some("model")
    } else if reference.starts_with("documents:")
        || reference.starts_with("document_chunks:")
        || reference.starts_with("ocr:")
    {
        Some("document")
    } else if reference.starts_with("claims:")
        || reference.starts_with("claim:")
        || reference.starts_with("members:")
        || reference.starts_with("policies:")
        || reference.starts_with("providers:")
        || reference.starts_with("claim_items:")
    {
        Some("claim")
    } else if reference.starts_with("anomaly:")
        || (reference.starts_with("scoring_runs:") && reference.contains("anomaly"))
    {
        Some("anomaly")
    } else {
        None
    }
}

fn triage_audit_payload(
    lead: &LeadRecord,
    input: &TriageLeadInput,
    case: Option<&CaseRecord>,
) -> Value {
    let evidence_sufficiency = case
        .and_then(|case| case.evidence_package.get("evidence_sufficiency"))
        .cloned();
    serde_json::json!({
        "claim_id": lead.claim_id.clone(),
        "lead_id": lead.lead_id.clone(),
        "case_id": case.map(|case| case.case_id.clone()),
        "review_mode": lead.review_mode.clone(),
        "decision": input.decision.clone(),
        "disposition": lead.disposition.clone(),
        "merge_target_lead_id": input.merge_target_lead_id.clone(),
        "notes": input.notes.clone(),
        "customer_scope_id": input.customer_scope_id.clone(),
        "evidence_sufficiency": evidence_sufficiency,
        "evidence_refs_by_type": case.and_then(|case| case.evidence_package.get("evidence_refs_by_type")).cloned(),
        "evidence_refs": input.evidence_refs.clone()
    })
}

fn triage_status_for_decision(decision: &str) -> &'static str {
    match decision {
        "open_case" => "triaged",
        "reject_lead" => "closed",
        "request_evidence" => "pending_evidence",
        "merge_lead" => "closed",
        _ => "triaged",
    }
}

fn triage_disposition_for_decision(decision: &str) -> &'static str {
    match decision {
        "open_case" => "open_case",
        "reject_lead" => "rejected",
        "request_evidence" => "pending_evidence",
        "merge_lead" => "merged",
        _ => "pending_triage",
    }
}

fn merge_target_lead_id(input: &TriageLeadInput) -> Option<&str> {
    input
        .merge_target_lead_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn merge_target_exists_in_memory(
    leads: &HashMap<String, LeadRecord>,
    input: &TriageLeadInput,
    visible_claim_ids: Option<&BTreeSet<String>>,
) -> bool {
    merge_target_lead_id(input).is_some_and(|target_lead_id| {
        leads.get(target_lead_id).is_some_and(|lead| {
            visible_claim_ids.is_none_or(|claim_ids| claim_ids.contains(&lead.claim_id))
        })
    })
}

async fn merge_target_lead_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: &TriageLeadInput,
) -> anyhow::Result<Option<LeadRecord>> {
    match merge_target_lead_id(input) {
        Some(target_lead_id) => {
            load_lead_in_tx(tx, target_lead_id, input.customer_scope_id.as_deref()).await
        }
        None => Ok(None),
    }
}

fn build_audit_sample(
    sample_id: String,
    input: CreateAuditSampleInput,
    leads: Vec<LeadRecord>,
    strata_contexts: &HashMap<String, AuditSampleStrataContext>,
    reviewer_history: &HashMap<String, u32>,
    created_at: Option<String>,
) -> AuditSampleRecord {
    let selection_method = selection_method_for_mode(&input.sample_mode).to_string();
    let mut candidates = leads
        .into_iter()
        .filter(|lead| lead_matches_inclusion(lead, &input.inclusion_criteria, strata_contexts))
        .collect::<Vec<_>>();
    if input.sample_mode == "post_payment_audit" {
        candidates.retain(|lead| lead.review_mode == "post_payment");
    }

    let selected_candidates = match selection_method.as_str() {
        "deterministic_hash" => {
            let seed = input
                .deterministic_seed
                .as_deref()
                .unwrap_or("default-seed");
            candidates.sort_by_key(|lead| deterministic_rank(seed, &lead.lead_id));
            candidates.into_iter().take(input.sample_size).collect()
        }
        "stratified_round_robin" => {
            select_stratified_candidates(candidates, strata_contexts, input.sample_size)
        }
        "reviewer_consistency_rotation" => {
            select_reviewer_rotation_candidates(candidates, reviewer_history, input.sample_size)
        }
        _ => {
            candidates.sort_by(|left, right| {
                right
                    .risk_score
                    .cmp(&left.risk_score)
                    .then_with(|| left.lead_id.cmp(&right.lead_id))
            });
            candidates.into_iter().take(input.sample_size).collect()
        }
    };

    let selected_leads = selected_candidates
        .into_iter()
        .map(|lead| audit_sample_lead_record(lead, strata_contexts, reviewer_history))
        .collect::<Vec<_>>();

    let mut sample = AuditSampleRecord {
        sample_id,
        customer_scope_id: input.customer_scope_id.unwrap_or_default(),
        sample_mode: input.sample_mode,
        population_definition: input.population_definition,
        inclusion_criteria: input.inclusion_criteria,
        deterministic_seed: input.deterministic_seed,
        selection_method,
        sample_size: selected_leads.len(),
        reviewer: input.reviewer,
        assignment_queue: input.assignment_queue,
        selected_leads,
        outcome_distribution: serde_json::json!({}),
        created_at,
    };
    sample.outcome_distribution = audit_sample_outcome_distribution(&sample, &[]);
    sample
}

fn select_stratified_candidates(
    candidates: Vec<LeadRecord>,
    strata_contexts: &HashMap<String, AuditSampleStrataContext>,
    sample_size: usize,
) -> Vec<LeadRecord> {
    let mut strata = BTreeMap::<String, Vec<LeadRecord>>::new();
    for lead in candidates {
        strata
            .entry(strata_key_for_lead(&lead, strata_contexts))
            .or_default()
            .push(lead);
    }

    let mut strata = strata
        .into_iter()
        .map(|(key, mut leads)| {
            leads.sort_by(|left, right| {
                right
                    .risk_score
                    .cmp(&left.risk_score)
                    .then_with(|| left.lead_id.cmp(&right.lead_id))
            });
            (key, VecDeque::from(leads))
        })
        .collect::<BTreeMap<_, _>>();

    let mut selected = Vec::new();
    while selected.len() < sample_size && strata.values().any(|leads| !leads.is_empty()) {
        for leads in strata.values_mut() {
            if selected.len() >= sample_size {
                break;
            }
            if let Some(lead) = leads.pop_front() {
                selected.push(lead);
            }
        }
    }
    selected
}

fn select_reviewer_rotation_candidates(
    mut candidates: Vec<LeadRecord>,
    reviewer_history: &HashMap<String, u32>,
    sample_size: usize,
) -> Vec<LeadRecord> {
    candidates.sort_by(|left, right| {
        reviewer_history
            .get(&left.lead_id)
            .unwrap_or(&0)
            .cmp(reviewer_history.get(&right.lead_id).unwrap_or(&0))
            .then_with(|| right.risk_score.cmp(&left.risk_score))
            .then_with(|| left.lead_id.cmp(&right.lead_id))
    });
    candidates.into_iter().take(sample_size).collect()
}

fn audit_sample_lead_record(
    lead: LeadRecord,
    strata_contexts: &HashMap<String, AuditSampleStrataContext>,
    reviewer_history: &HashMap<String, u32>,
) -> AuditSampleLeadRecord {
    let context = strata_context_for_lead(&lead, strata_contexts);
    let risk_band = risk_band_for_score(lead.risk_score).to_string();
    let strata_key = strata_key(
        &lead.scheme_family,
        &context.provider_type,
        &context.provider_region,
        &context.policy_type,
        &risk_band,
    );
    let prior_reviewer_sample_count = *reviewer_history.get(&lead.lead_id).unwrap_or(&0);
    AuditSampleLeadRecord {
        lead_id: lead.lead_id,
        claim_id: lead.claim_id,
        scheme_family: lead.scheme_family,
        review_mode: lead.review_mode,
        provider_id: lead.provider_id,
        provider_type: context.provider_type,
        provider_region: context.provider_region,
        policy_type: context.policy_type,
        risk_band,
        strata_key,
        prior_reviewer_sample_count,
        risk_score: lead.risk_score,
        rag: lead.rag,
        evidence_refs: lead.evidence_refs,
    }
}

fn strata_key_for_lead(
    lead: &LeadRecord,
    strata_contexts: &HashMap<String, AuditSampleStrataContext>,
) -> String {
    let context = strata_context_for_lead(lead, strata_contexts);
    strata_key(
        &lead.scheme_family,
        &context.provider_type,
        &context.provider_region,
        &context.policy_type,
        risk_band_for_score(lead.risk_score),
    )
}

fn strata_context_for_lead(
    lead: &LeadRecord,
    strata_contexts: &HashMap<String, AuditSampleStrataContext>,
) -> AuditSampleStrataContext {
    strata_contexts
        .get(&lead.claim_id)
        .cloned()
        .unwrap_or_else(|| AuditSampleStrataContext {
            provider_type: "unknown".into(),
            provider_region: "unknown".into(),
            policy_type: "unknown".into(),
        })
}

fn strata_key(
    scheme_family: &str,
    provider_type: &str,
    provider_region: &str,
    policy_type: &str,
    risk_band: &str,
) -> String {
    format!(
        "scheme={scheme_family}|provider_type={provider_type}|region={provider_region}|policy_type={policy_type}|risk_band={risk_band}"
    )
}

fn risk_band_for_score(risk_score: u8) -> &'static str {
    match risk_score {
        90..=100 => "critical",
        70..=89 => "high",
        40..=69 => "medium",
        _ => "low",
    }
}

fn default_review_mode() -> String {
    "pre_payment".into()
}

fn reviewer_lead_sample_counts<'a>(
    samples: impl IntoIterator<Item = &'a AuditSampleRecord>,
    reviewer: &str,
) -> HashMap<String, u32> {
    let mut counts = HashMap::<String, u32>::new();
    for sample in samples {
        if sample.reviewer != reviewer {
            continue;
        }
        for lead in &sample.selected_leads {
            *counts.entry(lead.lead_id.clone()).or_insert(0) += 1;
        }
    }
    counts
}

fn audit_sample_strata_contexts_from_claims(
    claims: &HashMap<String, ClaimContext>,
) -> HashMap<String, AuditSampleStrataContext> {
    claims
        .iter()
        .map(|(claim_id, context)| {
            (
                claim_id.clone(),
                AuditSampleStrataContext {
                    provider_type: context.provider.provider_type.clone(),
                    provider_region: context.provider.region.clone(),
                    policy_type: context.policy.product_code.clone(),
                },
            )
        })
        .collect()
}

fn with_sample_outcome_distributions(
    mut samples: Vec<AuditSampleRecord>,
    reviews: &[QaReviewRecord],
) -> Vec<AuditSampleRecord> {
    for sample in &mut samples {
        sample.outcome_distribution = audit_sample_outcome_distribution(sample, reviews);
    }
    samples
}

fn audit_sample_outcome_distribution(
    sample: &AuditSampleRecord,
    reviews: &[QaReviewRecord],
) -> Value {
    let reviews_by_case_id = reviews
        .iter()
        .map(|review| (review.qa_case_id.as_str(), review))
        .collect::<BTreeMap<_, _>>();
    let mut qa_conclusions = BTreeMap::<String, u32>::new();
    let mut issue_types = BTreeMap::<String, u32>::new();
    let mut feedback_targets = BTreeMap::<String, u32>::new();
    let mut strata_distribution = BTreeMap::<String, u32>::new();
    let mut review_mode_distribution = BTreeMap::<String, u32>::new();
    let mut reviewer_history_distribution = BTreeMap::<String, u32>::new();
    let mut reviewed_count = 0_u32;

    for lead in &sample.selected_leads {
        *strata_distribution
            .entry(lead.strata_key.clone())
            .or_insert(0) += 1;
        *review_mode_distribution
            .entry(lead.review_mode.clone())
            .or_insert(0) += 1;
        let history_bucket = if lead.prior_reviewer_sample_count == 0 {
            "new_to_reviewer"
        } else {
            "previously_sampled_by_reviewer"
        };
        *reviewer_history_distribution
            .entry(history_bucket.to_string())
            .or_insert(0) += 1;
    }

    for lead in &sample.selected_leads {
        let qa_case_id = format!("qa_{}_{}", sample.sample_id, lead.lead_id);
        let Some(review) = reviews_by_case_id.get(qa_case_id.as_str()) else {
            continue;
        };
        reviewed_count += 1;
        *qa_conclusions
            .entry(review.qa_conclusion.clone())
            .or_insert(0) += 1;
        *issue_types.entry(review.issue_type.clone()).or_insert(0) += 1;
        *feedback_targets
            .entry(canonical_feedback_target(&review.feedback_target).into())
            .or_insert(0) += 1;
    }

    let selected_count = sample.selected_leads.len() as u32;
    let mut distribution = serde_json::json!({
        "selected_count": selected_count,
        "reviewed_count": reviewed_count,
        "open_count": selected_count.saturating_sub(reviewed_count),
        "qa_conclusions": qa_conclusions,
        "issue_types": issue_types,
        "feedback_targets": feedback_targets,
        "strata_distribution": strata_distribution,
        "review_mode_distribution": review_mode_distribution,
        "reviewer_history_distribution": reviewer_history_distribution
    });
    if sample.sample_mode == "random_control" {
        let missed_risk_review_targets = sample
            .selected_leads
            .iter()
            .filter(|lead| matches!(lead.risk_band.as_str(), "low" | "medium"))
            .count() as u32;
        let false_positive_review_targets = sample
            .selected_leads
            .iter()
            .filter(|lead| matches!(lead.risk_band.as_str(), "high" | "critical"))
            .count() as u32;
        distribution["baseline_measurement"] = serde_json::json!({
            "control_cohort": true,
            "measurement_goal": "false_positive_and_missed_risk_baseline",
            "missed_risk_review_targets": missed_risk_review_targets,
            "false_positive_review_targets": false_positive_review_targets
        });
    }
    distribution
}

fn qa_review_to_feedback_item(
    review: QaReviewRecord,
    created_at: Option<String>,
    status: &str,
    status_update: Option<&QaFeedbackStatusUpdate>,
) -> QaFeedbackItemRecord {
    let priority = if review.qa_conclusion.contains("escalate") {
        "high"
    } else if review.qa_conclusion.contains("return") {
        "medium"
    } else {
        "low"
    };
    QaFeedbackItemRecord {
        feedback_id: qa_feedback_id(&review.qa_case_id),
        qa_case_id: review.qa_case_id.clone(),
        claim_id: review.claim_id.clone(),
        feedback_target: canonical_feedback_target(&review.feedback_target).into(),
        issue_type: review.issue_type.clone(),
        qa_conclusion: review.qa_conclusion.clone(),
        source: "qa_review".into(),
        status: status.into(),
        priority: priority.into(),
        summary: format!(
            "QA {} flagged {} feedback for claim {}",
            review.qa_case_id, review.feedback_target, review.claim_id
        ),
        note_present: !review.notes.trim().is_empty(),
        evidence_refs: review.evidence_refs,
        created_at,
        status_updated_by: status_update.and_then(|update| update.actor_id.clone()),
        status_audit_id: status_update.map(|update| update.audit_id.clone()),
        status_updated_at: status_update.and_then(|update| update.updated_at.clone()),
        status_evidence_refs: status_update
            .map(|update| update.evidence_refs.clone())
            .unwrap_or_default(),
    }
}

fn qa_feedback_id(qa_case_id: &str) -> String {
    format!("qa_feedback_{qa_case_id}")
}

fn qa_case_id_from_feedback_id(feedback_id: &str) -> Option<&str> {
    feedback_id.strip_prefix("qa_feedback_")
}

fn latest_qa_feedback_statuses(
    events: &[(String, AuditHistoryEventRecord)],
) -> HashMap<String, QaFeedbackStatusUpdate> {
    let mut statuses = HashMap::new();
    for (_, event) in events {
        if event.event_type == "qa.feedback.status.updated" {
            let Some(feedback_id) = event.payload["feedback_id"].as_str() else {
                continue;
            };
            let Some(status) = event.payload["to_status"].as_str() else {
                continue;
            };
            statuses.insert(
                feedback_id.to_string(),
                QaFeedbackStatusUpdate {
                    status: status.to_string(),
                    actor_id: event.payload["actor_id"].as_str().map(str::to_string),
                    audit_id: event.audit_id.clone(),
                    updated_at: event.created_at.clone(),
                    evidence_refs: event.evidence_refs.clone(),
                },
            );
        }
    }
    statuses
}

fn sla_target_hours_for_priority(priority: &str) -> u32 {
    match priority {
        "critical" => 8,
        "high" => 24,
        "medium" => 72,
        "low" => 168,
        _ => 72,
    }
}

fn is_terminal_case_status(status: &str) -> bool {
    matches!(status, "confirmed" | "rejected" | "closed")
}

fn case_sla_status(status: &str, sla_target_hours: u32, elapsed_hours: f64) -> String {
    if is_terminal_case_status(status) {
        if elapsed_hours > sla_target_hours as f64 {
            "closed_breached".into()
        } else {
            "closed_within_sla".into()
        }
    } else if elapsed_hours > sla_target_hours as f64 {
        "breached".into()
    } else {
        "on_track".into()
    }
}

fn persisted_audit_event_matches_filter(
    event: &PersistedAuditEvent,
    filter: &AuditEventListFilter,
) -> bool {
    if !audit_event_matches_group(&event.event_type, filter) {
        return false;
    }
    if filter
        .event_type
        .as_deref()
        .is_some_and(|event_type| event.event_type != event_type)
    {
        return false;
    }
    if filter.actor_id.as_deref().is_some_and(|actor_id| {
        event.actor_id != actor_id && !audit_event_payload_matches_actor(&event.payload, actor_id)
    }) {
        return false;
    }
    if filter
        .customer_scope_id
        .as_deref()
        .is_some_and(|scope| !audit_event_payload_matches_customer_scope(&event.payload, scope))
    {
        return false;
    }
    if filter
        .run_id
        .as_deref()
        .is_some_and(|run_id| event.run_id != run_id)
    {
        return false;
    }
    if let Some(claim_id) = filter.claim_id.as_deref() {
        let payload_claim_id = event.payload["claim_id"].as_str();
        if event.claim_id != claim_id && payload_claim_id != Some(claim_id) {
            return false;
        }
    }
    if !audit_event_payload_matches_routing_policy_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_rule_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_model_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_qa_feedback_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_audit_sample_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_agent_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_data_lineage_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_canonical_trace_filter(&event.payload, filter) {
        return false;
    }
    true
}

fn pilot_audit_event_matches_filter(
    claim_id: &str,
    event: &AuditHistoryEventRecord,
    filter: &AuditEventListFilter,
) -> bool {
    if !audit_event_matches_group(&event.event_type, filter) {
        return false;
    }
    if filter
        .event_type
        .as_deref()
        .is_some_and(|event_type| event.event_type != event_type)
    {
        return false;
    }
    if let Some(actor_id) = filter.actor_id.as_deref() {
        if !audit_event_payload_matches_actor(&event.payload, actor_id) {
            return false;
        }
    }
    if filter
        .customer_scope_id
        .as_deref()
        .is_some_and(|scope| !audit_event_payload_matches_customer_scope(&event.payload, scope))
    {
        return false;
    }
    if filter
        .run_id
        .as_deref()
        .is_some_and(|run_id| event.run_id != run_id)
    {
        return false;
    }
    if let Some(filter_claim_id) = filter.claim_id.as_deref() {
        let payload_claim_id = event.payload["claim_id"].as_str();
        if claim_id != filter_claim_id && payload_claim_id != Some(filter_claim_id) {
            return false;
        }
    }
    if !audit_event_payload_matches_routing_policy_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_rule_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_model_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_qa_feedback_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_audit_sample_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_agent_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_data_lineage_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_canonical_trace_filter(&event.payload, filter) {
        return false;
    }
    true
}

fn audit_event_payload_matches_actor(payload: &Value, actor_id: &str) -> bool {
    payload["actor_id"].as_str() == Some(actor_id)
        || payload["reviewer"].as_str() == Some(actor_id)
        || payload["owner"].as_str() == Some(actor_id)
        || payload["approver"].as_str() == Some(actor_id)
        || payload["requested_by"].as_str() == Some(actor_id)
}

fn audit_event_payload_matches_customer_scope(payload: &Value, customer_scope_id: &str) -> bool {
    payload["customer_scope_id"].as_str() == Some(customer_scope_id)
}

fn scoped_claim_ids_from_audit_events<'a>(
    events: impl Iterator<Item = &'a PersistedAuditEvent>,
    customer_scope_id: &str,
) -> BTreeSet<String> {
    events
        .filter(|event| {
            audit_event_payload_matches_customer_scope(&event.payload, customer_scope_id)
        })
        .map(|event| event.claim_id.clone())
        .collect()
}

fn audit_event_matches_group(event_type: &str, filter: &AuditEventListFilter) -> bool {
    match filter.event_group.as_deref() {
        None => true,
        Some("governance") => GOVERNANCE_AUDIT_EVENT_TYPES.contains(&event_type),
        Some(_) => false,
    }
}

fn audit_event_payload_matches_canonical_trace_filter(
    payload: &Value,
    filter: &AuditEventListFilter,
) -> bool {
    if filter.has_canonical_trace != Some(true) {
        return true;
    }
    payload
        .get("canonical_claim_context_trace")
        .and_then(Value::as_object)
        .is_some()
}

fn audit_event_payload_matches_rule_filter(payload: &Value, filter: &AuditEventListFilter) -> bool {
    if filter
        .rule_id
        .as_deref()
        .is_some_and(|rule_id| payload["rule_id"].as_str() != Some(rule_id))
    {
        return false;
    }
    if filter
        .rule_version
        .as_deref()
        .is_some_and(|version| !payload_field_matches_text(payload, "rule_version", version))
    {
        return false;
    }
    true
}

fn audit_event_payload_matches_model_filter(
    payload: &Value,
    filter: &AuditEventListFilter,
) -> bool {
    if filter
        .model_key
        .as_deref()
        .is_some_and(|model_key| payload["model_key"].as_str() != Some(model_key))
    {
        return false;
    }
    if filter
        .model_version
        .as_deref()
        .is_some_and(|version| payload["model_version"].as_str() != Some(version))
    {
        return false;
    }
    true
}

fn audit_event_payload_matches_routing_policy_filter(
    payload: &Value,
    filter: &AuditEventListFilter,
) -> bool {
    if filter
        .routing_policy_id
        .as_deref()
        .is_some_and(|policy_id| payload["policy_id"].as_str() != Some(policy_id))
    {
        return false;
    }
    if filter
        .routing_policy_version
        .as_deref()
        .is_some_and(|version| !payload_field_matches_text(payload, "version", version))
    {
        return false;
    }
    if filter
        .review_mode
        .as_deref()
        .is_some_and(|review_mode| payload["review_mode"].as_str() != Some(review_mode))
    {
        return false;
    }
    true
}

fn audit_event_payload_matches_qa_feedback_filter(
    payload: &Value,
    filter: &AuditEventListFilter,
) -> bool {
    if filter
        .feedback_id
        .as_deref()
        .is_some_and(|feedback_id| payload["feedback_id"].as_str() != Some(feedback_id))
    {
        return false;
    }
    if filter
        .qa_case_id
        .as_deref()
        .is_some_and(|qa_case_id| payload["qa_case_id"].as_str() != Some(qa_case_id))
    {
        return false;
    }
    true
}

fn audit_event_payload_matches_audit_sample_filter(
    payload: &Value,
    filter: &AuditEventListFilter,
) -> bool {
    if filter
        .sample_id
        .as_deref()
        .is_some_and(|sample_id| payload["sample_id"].as_str() != Some(sample_id))
    {
        return false;
    }
    true
}

fn audit_event_payload_matches_agent_filter(
    payload: &Value,
    filter: &AuditEventListFilter,
) -> bool {
    if filter
        .agent_run_id
        .as_deref()
        .is_some_and(|agent_run_id| payload["agent_run_id"].as_str() != Some(agent_run_id))
    {
        return false;
    }
    true
}

fn audit_event_payload_matches_data_lineage_filter(
    payload: &Value,
    filter: &AuditEventListFilter,
) -> bool {
    if filter
        .dataset_id
        .as_deref()
        .is_some_and(|dataset_id| payload["dataset_id"].as_str() != Some(dataset_id))
    {
        return false;
    }
    if filter
        .feature_set_id
        .as_deref()
        .is_some_and(|feature_set_id| payload["feature_set_id"].as_str() != Some(feature_set_id))
    {
        return false;
    }
    if filter
        .model_dataset_id
        .as_deref()
        .is_some_and(|model_dataset_id| {
            payload["model_dataset_id"].as_str() != Some(model_dataset_id)
        })
    {
        return false;
    }
    if filter
        .evaluation_run_id
        .as_deref()
        .is_some_and(|evaluation_run_id| {
            payload["evaluation_run_id"].as_str() != Some(evaluation_run_id)
        })
    {
        return false;
    }
    true
}

fn payload_field_matches_text(payload: &Value, field: &str, expected: &str) -> bool {
    payload[field].as_str() == Some(expected)
        || payload[field]
            .as_u64()
            .map(|value| value.to_string())
            .as_deref()
            == Some(expected)
}

fn webhook_event_from_audit(
    source_claim_id: Option<&str>,
    event: &AuditHistoryEventRecord,
) -> Option<WebhookEventRecord> {
    if event.event_status != "succeeded" {
        return None;
    }
    let event_type = match event.event_type.as_str() {
        "scoring.completed" => "fwa.score.completed",
        "lead.triaged"
            if event.payload["decision"].as_str() == Some("open_case")
                && event.payload["case_id"].as_str().is_some() =>
        {
            "fwa.case.routed"
        }
        "lead.triaged" => return None,
        "investigation.result.received" => "fwa.investigation.closed",
        "qa.result.received" => "fwa.qa.reviewed",
        "medical.review.recorded" => "fwa.medical.reviewed",
        "case.status.updated" => {
            let to_status = event.payload["to_status"].as_str().unwrap_or_default();
            if is_terminal_case_status(to_status) {
                "fwa.investigation.closed"
            } else {
                return None;
            }
        }
        _ => return None,
    };
    let claim_id = event.payload["claim_id"]
        .as_str()
        .or(source_claim_id)
        .unwrap_or_default()
        .to_string();
    if claim_id.is_empty() {
        return None;
    }
    let event_id = format!("webhook_{}", event.audit_id);
    let idempotency_key = format!("fwa-webhook:{}:{}", event_type, event.audit_id);
    let customer_scope_id = event
        .payload
        .get("customer_scope_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let signature_base_string = format!(
        "{}.{}.{}.{}",
        event_type, event.audit_id, event.run_id, claim_id
    );
    Some(WebhookEventRecord {
        event_id,
        event_type: event_type.into(),
        source_event_type: event.event_type.clone(),
        source_audit_id: event.audit_id.clone(),
        customer_scope_id,
        claim_id,
        run_id: event.run_id.clone(),
        delivery_status: "pending".into(),
        retry_count: 0,
        max_attempts: WEBHOOK_MAX_ATTEMPTS,
        next_attempt_at: event.created_at.clone(),
        last_attempt_at: None,
        last_response_status_code: None,
        last_error_message: None,
        idempotency_key,
        signature_key_id: "tpa-webhook-v1".into(),
        signature_algorithm: "hmac-sha256".into(),
        signature_base_string,
        payload: event.payload.clone(),
        evidence_refs: event.evidence_refs.clone(),
        occurred_at: event.created_at.clone(),
    })
}

const WEBHOOK_MAX_ATTEMPTS: u32 = 3;

fn apply_webhook_delivery_state(
    events: &mut [WebhookEventRecord],
    attempts: &[WebhookDeliveryAttemptRecord],
) {
    for event in events {
        let mut event_attempts = attempts
            .iter()
            .filter(|attempt| attempt.event_id == event.event_id)
            .collect::<Vec<_>>();
        event_attempts.sort_by_key(|attempt| attempt.attempt_number);
        let Some(latest) = event_attempts.last() else {
            continue;
        };
        event.retry_count = event_attempts.len() as u32;
        event.last_attempt_at = latest.attempted_at.clone();
        event.last_response_status_code = latest.response_status_code;
        event.last_error_message = latest.error_message.clone();
        event.next_attempt_at = latest.next_attempt_at.clone();
        event.delivery_status = if latest.delivery_status == "delivered" {
            "delivered".into()
        } else if event.retry_count >= event.max_attempts {
            "failed".into()
        } else {
            "retry_wait".into()
        };
    }
}

fn next_webhook_attempt_at(
    delivery_status: &str,
    attempt_number: u32,
    attempted_at: chrono::DateTime<chrono::Utc>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    if delivery_status != "failed" || attempt_number >= WEBHOOK_MAX_ATTEMPTS {
        return None;
    }
    let delay_minutes = match attempt_number {
        1 => 5,
        2 => 15,
        _ => 60,
    };
    Some(attempted_at + chrono::Duration::minutes(delay_minutes))
}

fn sort_webhook_events(events: &mut [WebhookEventRecord]) {
    events.sort_by(|left, right| {
        left.occurred_at
            .cmp(&right.occurred_at)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });
}

fn hours_between(start: chrono::DateTime<chrono::Utc>, end: chrono::DateTime<chrono::Utc>) -> f64 {
    end.signed_duration_since(start).num_seconds().max(0) as f64 / 3600.0
}

fn sort_qa_feedback_items(items: &mut [QaFeedbackItemRecord]) {
    items.sort_by(|left, right| {
        feedback_target_rank(&left.feedback_target)
            .cmp(&feedback_target_rank(&right.feedback_target))
            .then_with(|| left.qa_case_id.cmp(&right.qa_case_id))
    });
}

fn feedback_target_rank(target: &str) -> u8 {
    match canonical_feedback_target(target) {
        "rules" => 0,
        "model" => 1,
        _ => 2,
    }
}

#[derive(Debug, Clone)]
struct FinancialImpactRecord {
    impact_type: String,
    amount: Decimal,
    currency: Option<String>,
}

fn financial_impact_from_investigation(
    record: &InvestigationResultRecord,
) -> Option<FinancialImpactRecord> {
    financial_impact_from_parts(
        record.confirmed_fwa,
        record.financial_impact_type.as_deref(),
        record.saving_amount,
        record.currency.clone(),
    )
}

fn financial_impact_from_parts(
    confirmed_fwa: bool,
    financial_impact_type: Option<&str>,
    saving_amount: Option<Decimal>,
    currency: Option<String>,
) -> Option<FinancialImpactRecord> {
    if !confirmed_fwa {
        return None;
    }
    let amount = saving_amount?;
    if amount <= Decimal::ZERO {
        return None;
    }
    Some(FinancialImpactRecord {
        impact_type: normalize_financial_impact_type(financial_impact_type).into(),
        amount,
        currency,
    })
}

fn normalize_financial_impact_type(value: Option<&str>) -> &'static str {
    match value.unwrap_or("prevented_payment") {
        "recovered_amount" => "recovered_amount",
        "avoided_future_exposure" => "avoided_future_exposure",
        "deterrence_estimate" => "deterrence_estimate",
        "estimated_impact" => "estimated_impact",
        _ => "prevented_payment",
    }
}

fn labels_from_investigation_result(record: InvestigationResultRecord) -> Vec<OutcomeLabelRecord> {
    let mut labels = vec![OutcomeLabelRecord {
        label_id: format!(
            "label_investigation_{}_confirmed_fwa",
            record.investigation_id
        ),
        claim_id: record.claim_id.clone(),
        label_name: "confirmed_fwa".into(),
        label_value: record.confirmed_fwa.to_string(),
        source_type: "investigation_result".into(),
        source_id: record.investigation_id.clone(),
        governance_status: if record.confirmed_fwa {
            "approved_for_training".into()
        } else {
            "needs_review".into()
        },
        feedback_target: "model".into(),
        currency: None,
        evidence_refs: record.evidence_refs.clone(),
    }];

    if !record.confirmed_fwa {
        labels.push(OutcomeLabelRecord {
            label_id: format!(
                "label_investigation_{}_false_positive",
                record.investigation_id
            ),
            claim_id: record.claim_id.clone(),
            label_name: "false_positive".into(),
            label_value: "true".into(),
            source_type: "investigation_result".into(),
            source_id: record.investigation_id.clone(),
            governance_status: "needs_review".into(),
            feedback_target: "rules".into(),
            currency: None,
            evidence_refs: record.evidence_refs.clone(),
        });
    }

    if let Some(saving_amount) = record.saving_amount {
        let impact_type = normalize_financial_impact_type(record.financial_impact_type.as_deref());
        let label_name = match impact_type {
            "recovered_amount" => "amount_recovered",
            "avoided_future_exposure" => "avoided_future_exposure",
            "deterrence_estimate" => "deterrence_estimate",
            "estimated_impact" => "estimated_impact",
            _ => "amount_prevented",
        };
        labels.push(OutcomeLabelRecord {
            label_id: format!(
                "label_investigation_{}_{}",
                record.investigation_id, label_name
            ),
            claim_id: record.claim_id,
            label_name: label_name.into(),
            label_value: saving_amount.to_string(),
            source_type: "investigation_result".into(),
            source_id: record.investigation_id,
            governance_status: "approved_for_training".into(),
            feedback_target: "workflow".into(),
            currency: record.currency,
            evidence_refs: record.evidence_refs,
        });
    }

    labels
}

fn label_from_qa_review(record: QaReviewRecord, feedback_status: &str) -> OutcomeLabelRecord {
    OutcomeLabelRecord {
        label_id: format!("label_qa_{}_{}", record.qa_case_id, record.issue_type),
        claim_id: record.claim_id,
        label_name: record.issue_type,
        label_value: "true".into(),
        source_type: "qa_review".into(),
        source_id: record.qa_case_id,
        governance_status: qa_label_governance_status(feedback_status).into(),
        feedback_target: canonical_feedback_target(&record.feedback_target).into(),
        currency: None,
        evidence_refs: record.evidence_refs,
    }
}

fn qa_label_governance_status(feedback_status: &str) -> &'static str {
    if feedback_status == "resolved" {
        "approved_for_training"
    } else {
        "needs_review"
    }
}

fn labels_from_medical_review_event(event: &AuditHistoryEventRecord) -> Vec<OutcomeLabelRecord> {
    let Some(claim_id) = event.payload["claim_id"].as_str() else {
        return Vec::new();
    };
    medical_review_outcome_labels(event)
        .into_iter()
        .map(|label_name| {
            let (label_value, governance_status, feedback_target) =
                medical_review_label_fields(&label_name);
            OutcomeLabelRecord {
                label_id: format!("label_medical_review_{}_{}", event.audit_id, label_name),
                claim_id: claim_id.to_string(),
                label_name,
                label_value: label_value.into(),
                source_type: "medical_review".into(),
                source_id: event.audit_id.clone(),
                governance_status: governance_status.into(),
                feedback_target: feedback_target.into(),
                currency: None,
                evidence_refs: event.evidence_refs.clone(),
            }
        })
        .collect()
}

fn medical_review_outcome_labels(event: &AuditHistoryEventRecord) -> Vec<String> {
    let outcomes = event
        .payload
        .get("clinical_outcomes")
        .and_then(Value::as_array)
        .map(|outcomes| {
            outcomes
                .iter()
                .filter_map(Value::as_str)
                .filter(|outcome| is_allowed_medical_review_label(outcome))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !outcomes.is_empty() {
        return unique_strings(outcomes);
    }
    event.payload["decision"]
        .as_str()
        .map(|decision| vec![medical_review_label_from_decision(decision).to_string()])
        .unwrap_or_default()
}

fn medical_review_label_from_decision(decision: &str) -> &'static str {
    match decision {
        "request_more_evidence" => "insufficient_evidence",
        "medical_necessity_issue" => "medical_necessity_issue",
        "no_medical_issue" => "false_positive",
        _ => "clinical_evidence_sufficient",
    }
}

fn medical_review_label_fields(label_name: &str) -> (&'static str, &'static str, &'static str) {
    match label_name {
        "insufficient_evidence" | "medical_necessity_review_required" => {
            ("true", "needs_review", "workflow")
        }
        "documentation_issue" => ("true", "approved_for_training", "workflow"),
        "medical_necessity_issue" | "false_positive" => ("true", "approved_for_training", "model"),
        _ => ("true", "approved_for_training", "workflow"),
    }
}

fn is_allowed_medical_review_label(label_name: &str) -> bool {
    matches!(
        label_name,
        "documentation_issue"
            | "medical_necessity_review_required"
            | "insufficient_evidence"
            | "medical_necessity_issue"
            | "clinical_evidence_sufficient"
            | "false_positive"
    )
}

fn unique_strings(values: Vec<String>) -> Vec<String> {
    values.into_iter().fold(Vec::new(), |mut unique, value| {
        if !unique.contains(&value) {
            unique.push(value);
        }
        unique
    })
}

fn labels_from_lead_triage_events(
    events: impl IntoIterator<Item = AuditHistoryEventRecord>,
) -> Vec<OutcomeLabelRecord> {
    let mut labels_by_lead = BTreeMap::new();
    for event in events {
        if let Some(label) = label_from_lead_triage_event(&event) {
            labels_by_lead.insert(label.source_id.clone(), label);
        }
    }
    labels_by_lead.into_values().collect()
}

fn label_from_lead_triage_event(event: &AuditHistoryEventRecord) -> Option<OutcomeLabelRecord> {
    let claim_id = event.payload["claim_id"].as_str()?.to_string();
    let lead_id = event.payload["lead_id"].as_str()?.to_string();
    let disposition = lead_disposition_label_value(
        event.payload["decision"].as_str(),
        event.payload["disposition"].as_str(),
    )?;
    if event.evidence_refs.is_empty() {
        return None;
    }
    Some(OutcomeLabelRecord {
        label_id: format!("label_lead_{}_lead_disposition", lead_id),
        claim_id,
        label_name: "lead_disposition".into(),
        label_value: disposition.into(),
        source_type: "lead_triage".into(),
        source_id: lead_id,
        governance_status: "needs_review".into(),
        feedback_target: "workflow".into(),
        currency: None,
        evidence_refs: event.evidence_refs.clone(),
    })
}

fn label_from_bootstrap_review_event(
    event: &AuditHistoryEventRecord,
) -> Option<OutcomeLabelRecord> {
    let item_id = event.payload["item_id"].as_str()?.to_string();
    Some(OutcomeLabelRecord {
        label_id: format!(
            "label_bootstrap_{}_{}",
            item_id,
            event.payload["label_name"].as_str()?
        ),
        claim_id: event.payload["claim_id"].as_str()?.to_string(),
        label_name: event.payload["label_name"].as_str()?.to_string(),
        label_value: event.payload["label_value"].as_str()?.to_string(),
        source_type: "label_bootstrap".into(),
        source_id: item_id,
        governance_status: event.payload["governance_status"].as_str()?.to_string(),
        feedback_target: event.payload["feedback_target"]
            .as_str()
            .unwrap_or("workflow")
            .to_string(),
        currency: None,
        evidence_refs: event.evidence_refs.clone(),
    })
}

fn lead_disposition_label_value(
    decision: Option<&str>,
    disposition: Option<&str>,
) -> Option<&'static str> {
    match decision.or(disposition)? {
        "open_case" => Some("promoted"),
        "reject_lead" | "rejected" => Some("rejected"),
        "request_evidence" | "pending_evidence" => Some("requested_more_evidence"),
        "merge_lead" | "merged" => Some("merged"),
        _ => None,
    }
}

fn labels_from_case_status(record: CaseRecord) -> Vec<OutcomeLabelRecord> {
    let confirmed_fwa = match record.status.as_str() {
        "confirmed" => true,
        "rejected" => false,
        _ => return Vec::new(),
    };
    let evidence_refs = case_label_evidence_refs(&record);
    let mut labels = vec![OutcomeLabelRecord {
        label_id: format!("label_case_{}_confirmed_fwa", record.case_id),
        claim_id: record.claim_id.clone(),
        label_name: "confirmed_fwa".into(),
        label_value: confirmed_fwa.to_string(),
        source_type: "case_status".into(),
        source_id: record.case_id.clone(),
        governance_status: if confirmed_fwa {
            "approved_for_training".into()
        } else {
            "needs_review".into()
        },
        feedback_target: "model".into(),
        currency: None,
        evidence_refs: evidence_refs.clone(),
    }];

    if !confirmed_fwa {
        labels.push(OutcomeLabelRecord {
            label_id: format!("label_case_{}_false_positive", record.case_id),
            claim_id: record.claim_id,
            label_name: "false_positive".into(),
            label_value: "true".into(),
            source_type: "case_status".into(),
            source_id: record.case_id,
            governance_status: "needs_review".into(),
            feedback_target: "rules".into(),
            currency: None,
            evidence_refs,
        });
    }

    labels
}

fn audit_history_from_persisted(event: &PersistedAuditEvent) -> AuditHistoryEventRecord {
    AuditHistoryEventRecord {
        audit_id: event.audit_id.clone(),
        run_id: event.run_id.clone(),
        actor_role: event.actor_role.clone(),
        event_type: event.event_type.clone(),
        event_status: event.event_status.clone(),
        summary: event.summary.clone(),
        payload: event.payload.clone(),
        evidence_refs: evidence_values_to_strings(&event.evidence_refs),
        created_at: None,
    }
}

fn case_label_evidence_refs(record: &CaseRecord) -> Vec<String> {
    let mut refs = json_array_to_strings(record.evidence_package["evidence_refs"].clone());
    refs.push(format!("investigation_cases:{}", record.case_id));
    refs
}

fn sort_outcome_labels(labels: &mut [OutcomeLabelRecord]) {
    labels.sort_by(|left, right| {
        left.claim_id
            .cmp(&right.claim_id)
            .then_with(|| left.source_type.cmp(&right.source_type))
            .then_with(|| left.source_id.cmp(&right.source_id))
            .then_with(|| left.label_name.cmp(&right.label_name))
    });
}

fn model_retraining_job_from_pg_row(row: PgRow) -> ModelRetrainingJobRecord {
    let trigger_summary_json: Value = row.get("trigger_summary_json");
    let blocker_summary_json: Value = row.get("blocker_summary_json");
    let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
    let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");

    ModelRetrainingJobRecord {
        job_id: row.get("job_id"),
        model_key: row.get("model_key"),
        model_version: row.get("model_version"),
        status: row.get("status"),
        requested_by: row.get("requested_by"),
        request_notes: row.get("request_notes"),
        status_note: row.get("status_note"),
        updated_by: row.get("updated_by"),
        readiness_recommendation: row.get("readiness_recommendation"),
        latest_evaluation_id: row.get("latest_evaluation_id"),
        source_dataset_id: row.get("source_dataset_id"),
        source_data_quality_score: row.get("source_data_quality_score"),
        source_data_quality_status: row.get("source_data_quality_status"),
        trigger_summary: json_string_array(trigger_summary_json),
        blocker_summary: json_string_array(blocker_summary_json),
        candidate_model_version: row.get("candidate_model_version"),
        candidate_artifact_uri: row.get("candidate_artifact_uri"),
        candidate_endpoint_url: row.get("candidate_endpoint_url"),
        validation_report_uri: row.get("validation_report_uri"),
        output_evaluation_id: row.get("output_evaluation_id"),
        created_at: Some(created_at.to_rfc3339()),
        updated_at: Some(updated_at.to_rfc3339()),
    }
}

fn json_string_array(value: Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn summarize_dashboard_audit_coverage(
    scoring_runs: u32,
    canonical_trace_runs: u32,
) -> DashboardAuditCoverageRecord {
    DashboardAuditCoverageRecord {
        scoring_runs,
        canonical_trace_runs,
        canonical_trace_coverage: if scoring_runs == 0 {
            0.0
        } else {
            canonical_trace_runs as f64 / scoring_runs as f64
        },
    }
}

fn summarize_dashboard_label_pool(labels: &[OutcomeLabelRecord]) -> DashboardLabelPoolRecord {
    DashboardLabelPoolRecord {
        total_labels: labels.len() as u32,
        approved_for_training: labels
            .iter()
            .filter(|label| label.governance_status == "approved_for_training")
            .count() as u32,
        needs_review: labels
            .iter()
            .filter(|label| label.governance_status == "needs_review")
            .count() as u32,
        rule_feedback: labels
            .iter()
            .filter(|label| label.feedback_target == "rules")
            .count() as u32,
        model_feedback: labels
            .iter()
            .filter(|label| canonical_feedback_target(&label.feedback_target) == "model")
            .count() as u32,
        features_feedback: labels
            .iter()
            .filter(|label| label.feedback_target == "features")
            .count() as u32,
        provider_profile_feedback: labels
            .iter()
            .filter(|label| label.feedback_target == "provider_profile")
            .count() as u32,
        workflow_feedback: labels
            .iter()
            .filter(|label| label.feedback_target == "workflow")
            .count() as u32,
        case_status_labels: labels
            .iter()
            .filter(|label| label.source_type == "case_status")
            .count() as u32,
        medical_review_labels: labels
            .iter()
            .filter(|label| label.source_type == "medical_review")
            .count() as u32,
        false_positive_labels: labels
            .iter()
            .filter(|label| label.label_name == "false_positive")
            .count() as u32,
        evidence_backed_labels: labels
            .iter()
            .filter(|label| !label.evidence_refs.is_empty())
            .count() as u32,
    }
}

fn summarize_dashboard_qa_queue(
    samples: &[AuditSampleRecord],
    reviews: &[QaReviewRecord],
    feedback_items: &[QaFeedbackItemRecord],
) -> DashboardQaQueueRecord {
    let reviewed_case_ids = reviews
        .iter()
        .map(|review| review.qa_case_id.as_str())
        .collect::<BTreeSet<_>>();
    let disagreement_case_ids = reviews
        .iter()
        .filter(|review| review.qa_conclusion != "pass")
        .map(|review| review.qa_case_id.as_str())
        .collect::<BTreeSet<_>>();
    let sampled_cases = samples
        .iter()
        .map(|sample| sample.selected_leads.len() as u32)
        .sum::<u32>();
    let sampled_qa_case_ids = samples
        .iter()
        .flat_map(|sample| {
            sample.selected_leads.iter().map(move |lead| {
                format!("qa_{}_{}", sample.sample_id.as_str(), lead.lead_id.as_str())
            })
        })
        .collect::<Vec<_>>();
    let reviewed_cases = sampled_qa_case_ids
        .iter()
        .filter(|qa_case_id| reviewed_case_ids.contains(qa_case_id.as_str()))
        .count() as u32;
    let disagreement_cases = sampled_qa_case_ids
        .iter()
        .filter(|qa_case_id| disagreement_case_ids.contains(qa_case_id.as_str()))
        .count() as u32;
    let disagreement_rate = if reviewed_cases == 0 {
        0.0
    } else {
        disagreement_cases as f64 / reviewed_cases as f64
    };
    let feedback_open_count = count_feedback_status(feedback_items, "open");
    let feedback_in_progress_count = count_feedback_status(feedback_items, "in_progress");
    let unresolved_feedback_items = feedback_items
        .iter()
        .filter(|item| matches!(item.status.as_str(), "open" | "in_progress"))
        .collect::<Vec<_>>();

    DashboardQaQueueRecord {
        sampled_cases,
        open_cases: sampled_cases.saturating_sub(reviewed_cases),
        reviewed_cases,
        disagreement_cases,
        disagreement_rate,
        feedback_open_count,
        feedback_in_progress_count,
        feedback_resolved_count: count_feedback_status(feedback_items, "resolved"),
        feedback_dismissed_count: count_feedback_status(feedback_items, "dismissed"),
        unresolved_feedback_count: feedback_open_count + feedback_in_progress_count,
        rules_unresolved_feedback_count: count_feedback_target(&unresolved_feedback_items, "rules"),
        models_unresolved_feedback_count: count_feedback_target(
            &unresolved_feedback_items,
            "model",
        ),
        features_unresolved_feedback_count: count_feedback_target(
            &unresolved_feedback_items,
            "features",
        ),
        provider_profile_unresolved_feedback_count: count_feedback_target(
            &unresolved_feedback_items,
            "provider_profile",
        ),
        workflow_unresolved_feedback_count: count_feedback_target(
            &unresolved_feedback_items,
            "workflow",
        ),
        tpa_unresolved_feedback_count: count_feedback_target(&unresolved_feedback_items, "tpa"),
    }
}

fn count_feedback_status(items: &[QaFeedbackItemRecord], status: &str) -> u32 {
    items.iter().filter(|item| item.status == status).count() as u32
}

fn count_feedback_target(items: &[&QaFeedbackItemRecord], feedback_target: &str) -> u32 {
    items
        .iter()
        .filter(|item| {
            canonical_feedback_target(&item.feedback_target)
                == canonical_feedback_target(feedback_target)
        })
        .count() as u32
}

fn summarize_dashboard_case_sla(cases: &[CaseRecord]) -> DashboardCaseSlaRecord {
    let total_cases = cases.len() as u32;
    let closed_cases = cases
        .iter()
        .filter(|case| is_terminal_case_status(&case.status))
        .count() as u32;
    let breached_cases = cases
        .iter()
        .filter(|case| case.sla_status == "breached" || case.sla_status == "closed_breached")
        .count() as u32;
    let closure_times = cases
        .iter()
        .filter_map(|case| case.time_to_closure_hours)
        .collect::<Vec<_>>();
    DashboardCaseSlaRecord {
        total_cases,
        open_cases: total_cases.saturating_sub(closed_cases),
        closed_cases,
        breached_cases,
        sla_breach_rate: if total_cases == 0 {
            0.0
        } else {
            breached_cases as f64 / total_cases as f64
        },
        average_time_to_triage_hours: average_hours(
            cases.iter().map(|case| case.time_to_triage_hours),
        ),
        average_time_to_closure_hours: average_hours(closure_times.into_iter()),
    }
}

fn average_hours(values: impl Iterator<Item = f64>) -> f64 {
    let mut count = 0_u32;
    let mut sum = 0.0;
    for value in values {
        count += 1;
        sum += value;
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

fn summarize_dashboard_agent_governance(
    runs: &[AgentRunLogRecord],
) -> DashboardAgentGovernanceRecord {
    let mut pending_approvals = 0_u32;
    let mut approved_approvals = 0_u32;
    let mut rejected_approvals = 0_u32;

    for approval in runs.iter().flat_map(|run| run.approvals.iter()) {
        match approval.decision.as_str() {
            "pending" => pending_approvals += 1,
            "approved" => approved_approvals += 1,
            "rejected" => rejected_approvals += 1,
            _ => {}
        }
    }

    DashboardAgentGovernanceRecord {
        total_runs: runs.len() as u32,
        successful_runs: runs.iter().filter(|run| run.status == "succeeded").count() as u32,
        evidence_backed_runs: runs
            .iter()
            .filter(|run| !run.evidence_refs.is_empty())
            .count() as u32,
        tool_call_count: runs.iter().map(|run| run.tool_calls.len() as u32).sum(),
        policy_check_count: runs.iter().map(|run| run.policy_checks.len() as u32).sum(),
        denied_policy_check_count: runs
            .iter()
            .flat_map(|run| run.policy_checks.iter())
            .filter(|check| check.decision == "denied")
            .count() as u32,
        failed_tool_call_count: runs
            .iter()
            .flat_map(|run| run.tool_calls.iter())
            .filter(|call| call.status == "failed")
            .count() as u32,
        pending_approvals,
        approved_approvals,
        rejected_approvals,
    }
}

fn summarize_dashboard_model_governance(
    models: &[ModelVersionRecord],
    evaluations: &[ModelEvaluationRecord],
) -> DashboardModelGovernanceRecord {
    let known_models = models
        .iter()
        .map(|model| (model.model_key.as_str(), model.version.as_str()))
        .collect::<BTreeSet<_>>();
    let mut latest_evaluations = BTreeMap::<(&str, &str), &ModelEvaluationRecord>::new();
    for evaluation in evaluations {
        let key = (
            evaluation.model_key.as_str(),
            evaluation.model_version.as_str(),
        );
        if !known_models.contains(&key) {
            continue;
        }
        latest_evaluations
            .entry(key)
            .and_modify(|existing| {
                if evaluation.evaluation_run_id > existing.evaluation_run_id {
                    *existing = evaluation;
                }
            })
            .or_insert(evaluation);
    }

    let mut drift_watch_count = 0_u32;
    let mut drift_detected_count = 0_u32;
    let mut precision_values = Vec::new();
    let mut recall_values = Vec::new();

    for evaluation in latest_evaluations.values() {
        match drift_summary(&evaluation.metrics_json).1.as_str() {
            "watch" => drift_watch_count += 1,
            "drift" => drift_detected_count += 1,
            _ => {}
        }
        if let Some(precision) = evaluation.precision.as_ref() {
            precision_values.push(decimal_to_f64(precision));
        }
        if let Some(recall) = evaluation.recall.as_ref() {
            recall_values.push(decimal_to_f64(recall));
        }
    }

    DashboardModelGovernanceRecord {
        total_models: models.len() as u32,
        evaluated_models: latest_evaluations.len() as u32,
        drift_watch_count,
        drift_detected_count,
        average_precision: average_f64(&precision_values),
        average_recall: average_f64(&recall_values),
    }
}

fn summarize_dashboard_rule_governance(
    rules: &[RuleSummaryRecord],
    performance: &[RulePerformanceRecord],
) -> DashboardRuleGovernanceRecord {
    let total_trigger_count = performance
        .iter()
        .map(|record| record.trigger_count)
        .sum::<u32>();
    let reviewed_count = performance
        .iter()
        .map(|record| record.reviewed_count)
        .sum::<u32>();
    let confirmed_fwa_count = performance
        .iter()
        .map(|record| record.confirmed_fwa_count)
        .sum::<u32>();
    let false_positive_count = performance
        .iter()
        .map(|record| record.false_positive_count)
        .sum::<u32>();
    let saving_amount = performance
        .iter()
        .map(|record| {
            record
                .saving_amount
                .parse::<Decimal>()
                .unwrap_or(Decimal::ZERO)
        })
        .sum::<Decimal>();
    let saving = decimal_to_f64(&saving_amount);
    let review_cost = total_trigger_count as f64 * RULE_REVIEW_COST_AMOUNT;

    DashboardRuleGovernanceRecord {
        total_rules: rules.len() as u32,
        active_rules: rules.iter().filter(|rule| rule.status == "active").count() as u32,
        triggered_rules: performance
            .iter()
            .filter(|record| record.trigger_count > 0)
            .count() as u32,
        total_trigger_count,
        reviewed_count,
        confirmed_fwa_count,
        false_positive_count,
        precision: ratio(confirmed_fwa_count, reviewed_count),
        false_positive_rate: ratio(false_positive_count, reviewed_count),
        saving_amount: format_decimal_cents(saving_amount),
        roi: if review_cost == 0.0 {
            0.0
        } else {
            saving / review_cost
        },
    }
}

fn summarize_dashboard_value_measurement(
    impacts: &[FinancialImpactRecord],
    review_events: u32,
    false_positive_events: u32,
) -> DashboardValueMeasurementRecord {
    let mut prevented_payment = Decimal::ZERO;
    let mut recovered_amount = Decimal::ZERO;
    let mut avoided_future_exposure = Decimal::ZERO;
    let mut deterrence_estimate = Decimal::ZERO;
    let mut other_estimated_impact = Decimal::ZERO;
    let mut currency = None;

    for impact in impacts {
        if currency.is_none() {
            currency = impact.currency.clone();
        }
        match impact.impact_type.as_str() {
            "recovered_amount" => recovered_amount += impact.amount,
            "avoided_future_exposure" => avoided_future_exposure += impact.amount,
            "deterrence_estimate" => deterrence_estimate += impact.amount,
            "estimated_impact" => other_estimated_impact += impact.amount,
            _ => prevented_payment += impact.amount,
        }
    }

    let review_cost = Decimal::from(review_events) * Decimal::from(RULE_REVIEW_COST_AMOUNT as u32);
    let false_positive_operational_cost =
        Decimal::from(false_positive_events) * Decimal::from(RULE_REVIEW_COST_AMOUNT as u32);
    let reviewer_capacity_hours = Decimal::from(review_events) * Decimal::new(25, 2);
    let estimated_impact = avoided_future_exposure + deterrence_estimate + other_estimated_impact;
    let net_value = prevented_payment + recovered_amount + estimated_impact - review_cost;

    DashboardValueMeasurementRecord {
        prevented_payment: format_decimal_cents(prevented_payment),
        recovered_amount: format_decimal_cents(recovered_amount),
        avoided_future_exposure: format_decimal_cents(avoided_future_exposure),
        deterrence_estimate: format_decimal_cents(deterrence_estimate),
        estimated_impact: format_decimal_cents(estimated_impact),
        review_cost: format_decimal_cents(review_cost),
        false_positive_operational_cost: format_decimal_cents(false_positive_operational_cost),
        reviewer_capacity_hours: format_decimal_cents(reviewer_capacity_hours),
        net_value: format_decimal_cents(net_value),
        currency: currency.unwrap_or_else(|| "CNY".into()),
        evidence_caveat:
            "Observed values come from confirmed investigation outcomes; avoided exposure and deterrence remain estimated until validated."
                .into(),
    }
}

fn decimal_to_f64(value: &Decimal) -> f64 {
    value.to_string().parse().unwrap_or(0.0)
}

fn average_f64(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

#[derive(Debug, Default)]
struct ProviderRiskAccumulator {
    provider_id: String,
    risk_score: u8,
    risk_tier: String,
    review_required: bool,
    review_route: String,
    claim_count: u32,
    specialty: Option<String>,
    network_status: Option<String>,
    review_failure_count: u32,
    confirmed_fwa_count: u32,
    false_positive_count: u32,
    network_risk_score: Option<u8>,
    latest_claim_id: Option<String>,
    outlier_flags: BTreeSet<String>,
    graph_reasons: BTreeSet<String>,
    evidence_refs: BTreeSet<String>,
}

fn summarize_provider_risk_profiles<'a>(
    payloads: impl Iterator<Item = &'a Value>,
) -> ProviderRiskSummaryRecord {
    let mut providers = BTreeMap::<String, ProviderRiskAccumulator>::new();

    for payload in payloads {
        let mut counted_provider_id = None::<String>;

        if let Some(profile) = payload.get("provider_profile") {
            if let Some(provider_id) = profile.get("provider_id").and_then(Value::as_str) {
                let risk_score = profile
                    .get("risk_score")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    .min(100) as u8;
                let entry = provider_accumulator_entry(&mut providers, provider_id);

                touch_provider_accumulator(entry, payload);
                counted_provider_id = Some(provider_id.to_string());
                entry.review_required |= profile
                    .get("review_required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);

                if risk_score >= entry.risk_score {
                    entry.risk_score = risk_score;
                    entry.risk_tier = profile
                        .get("risk_tier")
                        .and_then(Value::as_str)
                        .unwrap_or("low")
                        .to_string();
                    entry.review_route = profile
                        .get("review_route")
                        .and_then(Value::as_str)
                        .unwrap_or("none")
                        .to_string();
                    entry.specialty = profile
                        .get("specialty")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    entry.network_status = profile
                        .get("network_status")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                }
                entry.review_failure_count = entry.review_failure_count.max(
                    profile
                        .get("review_failure_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        .min(u32::MAX as u64) as u32,
                );
                entry.confirmed_fwa_count = entry.confirmed_fwa_count.max(
                    profile
                        .get("confirmed_fwa_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        .min(u32::MAX as u64) as u32,
                );
                entry.false_positive_count = entry.false_positive_count.max(
                    profile
                        .get("false_positive_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        .min(u32::MAX as u64) as u32,
                );

                extend_string_set(&mut entry.outlier_flags, profile.get("outlier_flags"));
                extend_string_set(&mut entry.evidence_refs, profile.get("evidence_refs"));
            }
        }

        if let Some(graph) = payload.get("provider_relationships") {
            if let Some(provider_id) = graph.get("provider_id").and_then(Value::as_str) {
                let risk_score = graph
                    .get("risk_score")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    .min(100) as u8;
                let entry = provider_accumulator_entry(&mut providers, provider_id);

                if counted_provider_id.as_deref() != Some(provider_id) {
                    touch_provider_accumulator(entry, payload);
                }
                entry.review_required |= graph
                    .get("review_required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                entry.network_risk_score =
                    Some(entry.network_risk_score.unwrap_or(0).max(risk_score));

                if risk_score >= entry.risk_score {
                    entry.risk_score = risk_score;
                    entry.risk_tier = graph
                        .get("risk_tier")
                        .and_then(Value::as_str)
                        .unwrap_or("low")
                        .to_string();
                    entry.review_route = graph
                        .get("review_route")
                        .and_then(Value::as_str)
                        .unwrap_or("none")
                        .to_string();
                }

                extend_string_set(&mut entry.graph_reasons, graph.get("graph_reasons"));
                extend_string_set(&mut entry.evidence_refs, graph.get("evidence_refs"));
            }
        }
    }

    let mut providers = providers
        .into_values()
        .map(|provider| ProviderRiskSummaryItemRecord {
            provider_id: provider.provider_id,
            risk_score: provider.risk_score,
            risk_tier: provider.risk_tier,
            review_required: provider.review_required,
            review_route: provider.review_route,
            claim_count: provider.claim_count,
            specialty: provider.specialty,
            network_status: provider.network_status,
            review_failure_count: provider.review_failure_count,
            confirmed_fwa_count: provider.confirmed_fwa_count,
            false_positive_count: provider.false_positive_count,
            network_risk_score: provider.network_risk_score,
            latest_claim_id: provider.latest_claim_id,
            outlier_flags: provider.outlier_flags.into_iter().collect(),
            graph_reasons: provider.graph_reasons.into_iter().collect(),
            evidence_refs: provider.evidence_refs.into_iter().collect(),
        })
        .collect::<Vec<_>>();
    providers.sort_by(|left, right| {
        right
            .risk_score
            .cmp(&left.risk_score)
            .then_with(|| left.provider_id.cmp(&right.provider_id))
    });

    ProviderRiskSummaryRecord {
        provider_count: providers.len() as u32,
        review_required_count: providers
            .iter()
            .filter(|provider| provider.review_required)
            .count() as u32,
        high_risk_count: providers
            .iter()
            .filter(|provider| provider.risk_score >= 70)
            .count() as u32,
        providers,
    }
}

fn provider_accumulator_entry<'a>(
    providers: &'a mut BTreeMap<String, ProviderRiskAccumulator>,
    provider_id: &str,
) -> &'a mut ProviderRiskAccumulator {
    providers
        .entry(provider_id.to_string())
        .or_insert_with(|| ProviderRiskAccumulator {
            provider_id: provider_id.to_string(),
            risk_tier: "low".into(),
            review_route: "none".into(),
            ..ProviderRiskAccumulator::default()
        })
}

fn touch_provider_accumulator(entry: &mut ProviderRiskAccumulator, payload: &Value) {
    entry.claim_count += 1;
    entry.latest_claim_id = payload
        .get("claim_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| entry.latest_claim_id.clone());
}

fn extend_string_set(target: &mut BTreeSet<String>, value: Option<&Value>) {
    if let Some(items) = value.and_then(Value::as_array) {
        target.extend(items.iter().filter_map(Value::as_str).map(str::to_string));
    }
}

fn selection_method_for_mode(sample_mode: &str) -> &'static str {
    match sample_mode {
        "random_control" => "deterministic_hash",
        "stratified" => "stratified_round_robin",
        "qa_calibration" => "reviewer_consistency_rotation",
        "post_payment_audit" => "risk_score_desc_post_payment",
        _ => "risk_score_desc",
    }
}

fn lead_matches_inclusion(
    lead: &LeadRecord,
    criteria: &Value,
    strata_contexts: &HashMap<String, AuditSampleStrataContext>,
) -> bool {
    if let Some(min_risk_score) = criteria["min_risk_score"].as_u64() {
        if lead.risk_score < min_risk_score as u8 {
            return false;
        }
    }
    if let Some(scheme_family) = criteria["scheme_family"].as_str() {
        if lead.scheme_family != scheme_family {
            return false;
        }
    }
    if let Some(rag) = criteria["rag"].as_str() {
        if !lead.rag.eq_ignore_ascii_case(rag) {
            return false;
        }
    }
    if let Some(review_mode) = criteria["review_mode"].as_str() {
        if lead.review_mode != review_mode {
            return false;
        }
    }
    let context = strata_context_for_lead(lead, strata_contexts);
    if let Some(provider_type) = criteria["provider_type"].as_str() {
        if context.provider_type != provider_type {
            return false;
        }
    }
    if let Some(provider_region) = criteria["provider_region"].as_str() {
        if context.provider_region != provider_region {
            return false;
        }
    }
    if let Some(policy_type) = criteria["policy_type"].as_str() {
        if context.policy_type != policy_type {
            return false;
        }
    }
    if let Some(risk_band) = criteria["risk_band"].as_str() {
        if risk_band_for_score(lead.risk_score) != risk_band {
            return false;
        }
    }
    true
}

fn deterministic_rank(seed: &str, lead_id: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed.hash(&mut hasher);
    lead_id.hash(&mut hasher);
    hasher.finish()
}

async fn load_leads(
    pool: &PgPool,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<LeadRecord>> {
    let rows: Vec<LeadRow> = sqlx::query_as(
        "SELECT lead_id, run_id, claim_id, member_id, provider_id, source_system, COALESCE(review_mode, 'pre_payment'), scheme_family, lead_source, status, disposition, risk_score, rag, reason, evidence_refs
         FROM fwa_leads
         WHERE (
           $1::text IS NULL OR EXISTS (
             SELECT 1
             FROM audit_events ae
             JOIN claims scoped_claim ON scoped_claim.id = ae.claim_id
             WHERE scoped_claim.external_claim_id = fwa_leads.claim_id
               AND ae.payload ->> 'customer_scope_id' = $1
           )
         )
         ORDER BY created_at, lead_id",
    )
    .bind(customer_scope_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(lead_from_row).collect())
}

async fn load_control_audit_population(
    pool: &PgPool,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<LeadRecord>> {
    let rows: Vec<LeadRow> = sqlx::query_as(
        "SELECT 'control_lead_' || c.external_claim_id,
                sr.run_id,
                c.external_claim_id,
                COALESCE(m.external_member_id, ''),
                COALESCE(pr.external_provider_id, ''),
                sr.source_system,
                COALESCE(scoring_event.payload->>'review_mode', 'pre_payment'),
                CASE
                  WHEN sr.risk_score >= 70 THEN 'high_risk_claim'
                  ELSE 'control_baseline'
                END,
                'random_control_scoring_run',
                'new',
                'pending_control_review',
                sr.risk_score,
                COALESCE(sr.rag, 'GREEN'),
                'Random control baseline sample: ' || COALESCE(sr.routing_reason, 'scored claim'),
                jsonb_build_array('scoring_runs:' || sr.run_id)
         FROM scoring_runs sr
         JOIN claims c ON c.id = sr.claim_id
         LEFT JOIN members m ON m.id = c.member_id
         LEFT JOIN providers pr ON pr.id = c.provider_id
         LEFT JOIN LATERAL (
           SELECT payload
           FROM audit_events ae
           WHERE ae.run_id = sr.run_id
             AND ae.event_type = 'scoring.completed'
           ORDER BY ae.created_at DESC
           LIMIT 1
         ) scoring_event ON TRUE
         WHERE sr.status = 'succeeded'
           AND sr.risk_score IS NOT NULL
           AND ($1::text IS NULL OR scoring_event.payload->>'customer_scope_id' = $1)
         ORDER BY sr.completed_at, sr.run_id",
    )
    .bind(customer_scope_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(lead_from_row).collect())
}

async fn load_audit_sample_strata_contexts(
    pool: &PgPool,
) -> anyhow::Result<HashMap<String, AuditSampleStrataContext>> {
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT c.external_claim_id,
                COALESCE(pr.provider_type, 'unknown'),
                COALESCE(pr.region, 'unknown'),
                COALESCE(p.product_code, 'unknown')
         FROM claims c
         LEFT JOIN providers pr ON pr.id = c.provider_id
         LEFT JOIN policies p ON p.id = c.policy_id",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(claim_id, provider_type, provider_region, policy_type)| {
            (
                claim_id,
                AuditSampleStrataContext {
                    provider_type,
                    provider_region,
                    policy_type,
                },
            )
        })
        .collect())
}

async fn load_lead_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    lead_id: &str,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Option<LeadRecord>> {
    let row: Option<LeadRow> = sqlx::query_as(
        "SELECT lead_id, run_id, claim_id, member_id, provider_id, source_system, COALESCE(review_mode, 'pre_payment'), scheme_family, lead_source, status, disposition, risk_score, rag, reason, evidence_refs
         FROM fwa_leads
         WHERE lead_id = $1
           AND (
             $2::text IS NULL OR EXISTS (
               SELECT 1
               FROM audit_events ae
               JOIN claims scoped_claim ON scoped_claim.id = ae.claim_id
               WHERE scoped_claim.external_claim_id = fwa_leads.claim_id
                 AND ae.payload ->> 'customer_scope_id' = $2
             )
           )",
    )
    .bind(lead_id)
    .bind(customer_scope_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(lead_from_row))
}

fn lead_from_row(row: LeadRow) -> LeadRecord {
    let (
        lead_id,
        run_id,
        claim_id,
        member_id,
        provider_id,
        source_system,
        review_mode,
        scheme_family,
        lead_source,
        status,
        disposition,
        risk_score,
        rag,
        reason,
        evidence_refs,
    ) = row;
    LeadRecord {
        lead_id,
        run_id,
        claim_id,
        member_id,
        provider_id,
        source_system,
        review_mode,
        scheme_family,
        lead_source,
        status,
        disposition,
        risk_score: risk_score.clamp(0, 100) as u8,
        rag,
        reason,
        evidence_refs: json_array_to_strings(evidence_refs),
    }
}

async fn load_cases(
    pool: &PgPool,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<CaseRecord>> {
    let rows: Vec<CaseRow> = sqlx::query_as(
        "SELECT c.case_id, c.lead_id, c.claim_id, c.member_id, c.provider_id, c.source_system, COALESCE(c.review_mode, l.review_mode, 'pre_payment') AS review_mode, c.scheme_family, c.lead_source, c.status, c.assignee, c.reviewer, c.priority, c.routing_reason, c.evidence_package_json, c.final_outcome, c.reviewer_notes, c.investigation_result_id, l.created_at AS lead_created_at, c.created_at AS case_created_at, c.updated_at AS case_updated_at
         FROM investigation_cases c
         JOIN fwa_leads l ON l.lead_id = c.lead_id
         WHERE (
           $1::text IS NULL OR EXISTS (
             SELECT 1
             FROM audit_events ae
             JOIN claims scoped_claim ON scoped_claim.id = ae.claim_id
             WHERE scoped_claim.external_claim_id = c.claim_id
               AND ae.payload ->> 'customer_scope_id' = $1
           )
         )
         ORDER BY c.created_at, c.case_id",
    )
    .bind(customer_scope_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(case_from_row).collect())
}

async fn load_case_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    case_id: &str,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Option<CaseRecord>> {
    let row: Option<CaseRow> = sqlx::query_as(
        "SELECT c.case_id, c.lead_id, c.claim_id, c.member_id, c.provider_id, c.source_system, COALESCE(c.review_mode, l.review_mode, 'pre_payment') AS review_mode, c.scheme_family, c.lead_source, c.status, c.assignee, c.reviewer, c.priority, c.routing_reason, c.evidence_package_json, c.final_outcome, c.reviewer_notes, c.investigation_result_id, l.created_at AS lead_created_at, c.created_at AS case_created_at, c.updated_at AS case_updated_at
         FROM investigation_cases c
         JOIN fwa_leads l ON l.lead_id = c.lead_id
         WHERE c.case_id = $1
           AND (
             $2::text IS NULL OR EXISTS (
               SELECT 1
               FROM audit_events ae
               JOIN claims scoped_claim ON scoped_claim.id = ae.claim_id
               WHERE scoped_claim.external_claim_id = c.claim_id
                 AND ae.payload ->> 'customer_scope_id' = $2
             )
           )",
    )
    .bind(case_id)
    .bind(customer_scope_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(case_from_row))
}

fn case_from_row(row: CaseRow) -> CaseRecord {
    let sla_target_hours = sla_target_hours_for_priority(&row.priority);
    let time_to_closure_hours = is_terminal_case_status(&row.status)
        .then(|| hours_between(row.case_created_at, row.case_updated_at));
    let elapsed_hours = time_to_closure_hours
        .unwrap_or_else(|| hours_between(row.case_created_at, chrono::Utc::now()));
    let sla_status = case_sla_status(&row.status, sla_target_hours, elapsed_hours);
    let time_to_triage_hours = hours_between(row.lead_created_at, row.case_created_at);
    CaseRecord {
        case_id: row.case_id,
        lead_id: row.lead_id,
        claim_id: row.claim_id,
        member_id: row.member_id,
        provider_id: row.provider_id,
        source_system: row.source_system,
        review_mode: row.review_mode,
        scheme_family: row.scheme_family,
        lead_source: row.lead_source,
        status: row.status,
        assignee: row.assignee,
        reviewer: row.reviewer,
        priority: row.priority,
        routing_reason: row.routing_reason,
        evidence_package: row.evidence_package_json,
        sla_target_hours,
        sla_status,
        time_to_triage_hours,
        time_to_closure_hours,
        final_outcome: row.final_outcome,
        reviewer_notes: row.reviewer_notes,
        investigation_result_id: row.investigation_result_id,
    }
}

pub fn default_runtime_rules() -> Vec<Rule> {
    vec![
        Rule {
            rule_id: "rule_early_claim".into(),
            version: 1,
            name: "Early claim".into(),
            review_mode: "both".into(),
            scheme_family: Some("early_high_value_claim".into()),
            conditions: vec![Condition {
                field: "days_since_policy_start".into(),
                operator: "<=".into(),
                value: serde_json::json!(7),
            }],
            action: RuleAction {
                score: 75,
                alert_code: "EARLY_CLAIM".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                reason: "保单生效后 7 天内发生理赔".into(),
            },
        },
        Rule {
            rule_id: "rule_early_high_amount".into(),
            version: 1,
            name: "Early high amount".into(),
            review_mode: "both".into(),
            scheme_family: Some("early_high_value_claim".into()),
            conditions: vec![
                Condition {
                    field: "days_since_policy_start".into(),
                    operator: "<=".into(),
                    value: serde_json::json!(10),
                },
                Condition {
                    field: "claim_amount_to_limit_ratio".into(),
                    operator: ">=".into(),
                    value: serde_json::json!(0.7),
                },
            ],
            action: RuleAction {
                score: 45,
                alert_code: "EARLY_HIGH_AMOUNT".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                reason: "保单生效早期发生高额理赔".into(),
            },
        },
        Rule {
            rule_id: "rule_high_cost_single_item".into(),
            version: 1,
            name: "High cost single item".into(),
            review_mode: "both".into(),
            scheme_family: Some("high_risk_claim".into()),
            conditions: vec![Condition {
                field: "high_cost_item_ratio".into(),
                operator: ">=".into(),
                value: serde_json::json!(0.5),
            }],
            action: RuleAction {
                score: 25,
                alert_code: "HIGH_COST_SINGLE_ITEM".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                reason: "单个高价项目占理赔金额比例偏高".into(),
            },
        },
        Rule {
            rule_id: "rule_large_limit_usage".into(),
            version: 1,
            name: "Large limit usage".into(),
            review_mode: "both".into(),
            scheme_family: Some("early_high_value_claim".into()),
            conditions: vec![Condition {
                field: "claim_amount_to_limit_ratio".into(),
                operator: ">=".into(),
                value: serde_json::json!(0.8),
            }],
            action: RuleAction {
                score: 35,
                alert_code: "LARGE_LIMIT_USAGE".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                reason: "理赔金额接近保障额度".into(),
            },
        },
        Rule {
            rule_id: "rule_low_medical_match".into(),
            version: 1,
            name: "Low medical match".into(),
            review_mode: "both".into(),
            scheme_family: Some("diagnosis_procedure_mismatch".into()),
            conditions: vec![Condition {
                field: "diagnosis_procedure_match_score".into(),
                operator: "<=".into(),
                value: serde_json::json!(0.4),
            }],
            action: RuleAction {
                score: 30,
                alert_code: "LOW_MEDICAL_MATCH".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                reason: "诊断与项目匹配度偏低".into(),
            },
        },
        Rule {
            rule_id: "rule_duplicate_claim".into(),
            version: 1,
            name: "Duplicate claim".into(),
            review_mode: "both".into(),
            scheme_family: Some("duplicate_billing".into()),
            conditions: vec![Condition {
                field: "duplicate_claim_similarity_score".into(),
                operator: ">=".into(),
                value: serde_json::json!(0.95),
            }],
            action: RuleAction {
                score: 35,
                alert_code: "DUPLICATE_CLAIM".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                reason: "同一投保人、Provider、服务日期、项目和金额疑似重复理赔".into(),
            },
        },
        Rule {
            rule_id: "rule_upcoding_complexity".into(),
            version: 1,
            name: "Upcoding complexity".into(),
            review_mode: "both".into(),
            scheme_family: Some("upcoding".into()),
            conditions: vec![
                Condition {
                    field: "diagnosis_procedure_match_score".into(),
                    operator: "<=".into(),
                    value: serde_json::json!(0.45),
                },
                Condition {
                    field: "high_cost_item_ratio".into(),
                    operator: ">=".into(),
                    value: serde_json::json!(0.5),
                },
            ],
            action: RuleAction {
                score: 35,
                alert_code: "UPCODING_COMPLEXITY".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                reason: "高复杂度或高价项目与诊断支持度偏低，疑似 upcoding".into(),
            },
        },
        Rule {
            rule_id: "rule_unbundling_component_pattern".into(),
            version: 1,
            name: "Unbundling component pattern".into(),
            review_mode: "both".into(),
            scheme_family: Some("unbundling".into()),
            conditions: vec![Condition {
                field: "claim_item_count".into(),
                operator: ">=".into(),
                value: serde_json::json!(6),
            }],
            action: RuleAction {
                score: 25,
                alert_code: "UNBUNDLING_COMPONENT_PATTERN".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                reason: "同一案件明细项目数量异常偏多，需核查是否存在拆分计费".into(),
            },
        },
        Rule {
            rule_id: "rule_medically_unnecessary_service".into(),
            version: 1,
            name: "Medically unnecessary service".into(),
            review_mode: "both".into(),
            scheme_family: Some("medically_unnecessary_service".into()),
            conditions: vec![Condition {
                field: "clinical_review_required".into(),
                operator: "==".into(),
                value: serde_json::json!(1),
            }],
            action: RuleAction {
                score: 30,
                alert_code: "MEDICALLY_UNNECESSARY_SERVICE".into(),
                recommended_action: RecommendedAction::RequestEvidence,
                action_class: RuleActionClass::PendingEvidence,
                required_evidence: vec![RequiredEvidence {
                    evidence_type: "clinical_missing_evidence".into(),
                    evidence_request_type: Some("clinical_document_request".into()),
                    blocking: true,
                    policy_authority_ref: Some("policy:clinical-evidence:v1".into()),
                    exception_check: Some("required_clinical_documents_not_present".into()),
                }],
                reason: "临床证据不足或存在缺失，需复核医疗必要性".into(),
            },
        },
        Rule {
            rule_id: "rule_same_member_repeated_service".into(),
            version: 1,
            name: "Same member repeated service".into(),
            review_mode: "both".into(),
            scheme_family: Some("excessive_utilization".into()),
            conditions: vec![Condition {
                field: "same_member_service_count_30d".into(),
                operator: ">=".into(),
                value: serde_json::json!(3),
            }],
            action: RuleAction {
                score: 25,
                alert_code: "SAME_MEMBER_REPEATED_SERVICE".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                reason: "同一投保人短期内同类服务重复出现，需核查过度使用".into(),
            },
        },
        Rule {
            rule_id: "rule_relationship_concentration".into(),
            version: 1,
            name: "Relationship concentration".into(),
            review_mode: "both".into(),
            scheme_family: Some("relationship_concentration".into()),
            conditions: vec![Condition {
                field: "provider_high_risk_neighbor_signal".into(),
                operator: "==".into(),
                value: serde_json::json!(true),
            }],
            action: RuleAction {
                score: 35,
                alert_code: "RELATIONSHIP_CONCENTRATION".into(),
                recommended_action: RecommendedAction::EscalateInvestigation,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                reason: "Provider 关系网络存在高风险邻居或集中关联信号".into(),
            },
        },
        Rule {
            rule_id: "rule_many_claim_items".into(),
            version: 1,
            name: "Many claim items".into(),
            review_mode: "both".into(),
            scheme_family: Some("excessive_utilization".into()),
            conditions: vec![Condition {
                field: "claim_item_count".into(),
                operator: ">=".into(),
                value: serde_json::json!(5),
            }],
            action: RuleAction {
                score: 20,
                alert_code: "MANY_CLAIM_ITEMS".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                reason: "理赔明细项目数量偏多".into(),
            },
        },
        Rule {
            rule_id: "rule_peer_p95_amount".into(),
            version: 1,
            name: "Peer P95 amount".into(),
            review_mode: "both".into(),
            scheme_family: Some("provider_peer_outlier".into()),
            conditions: vec![Condition {
                field: "claim_amount_peer_percentile".into(),
                operator: ">=".into(),
                value: serde_json::json!(95),
            }],
            action: RuleAction {
                score: 25,
                alert_code: "PEER_P95_AMOUNT".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                reason: "理赔金额高于同类样本 P95".into(),
            },
        },
        Rule {
            rule_id: "rule_peer_p99_amount".into(),
            version: 1,
            name: "Peer P99 amount".into(),
            review_mode: "both".into(),
            scheme_family: Some("provider_peer_outlier".into()),
            conditions: vec![Condition {
                field: "claim_amount_peer_percentile".into(),
                operator: ">=".into(),
                value: serde_json::json!(99),
            }],
            action: RuleAction {
                score: 40,
                alert_code: "PEER_P99_AMOUNT".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                reason: "理赔金额高于同类样本 P99".into(),
            },
        },
        Rule {
            rule_id: "rule_provider_high_risk_tier".into(),
            version: 1,
            name: "Provider high risk tier".into(),
            review_mode: "both".into(),
            scheme_family: Some("provider_peer_outlier".into()),
            conditions: vec![Condition {
                field: "provider_risk_tier".into(),
                operator: "==".into(),
                value: serde_json::json!("HIGH"),
            }],
            action: RuleAction {
                score: 30,
                alert_code: "PROVIDER_HIGH_RISK_TIER".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                reason: "Provider 风险等级较高".into(),
            },
        },
        Rule {
            rule_id: "rule_provider_profile_high".into(),
            version: 1,
            name: "Provider profile high".into(),
            review_mode: "both".into(),
            scheme_family: Some("provider_peer_outlier".into()),
            conditions: vec![Condition {
                field: "provider_profile_score".into(),
                operator: ">=".into(),
                value: serde_json::json!(70),
            }],
            action: RuleAction {
                score: 30,
                alert_code: "PROVIDER_PROFILE_HIGH".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                reason: "Provider 风险画像分偏高".into(),
            },
        },
    ]
}

fn default_rule_details() -> Vec<RuleDetailRecord> {
    default_runtime_rules()
        .into_iter()
        .map(|rule| rule_detail_from_rule(rule, "active", "rules-ops".into()))
        .collect()
}

fn rule_detail_from_rule(rule: Rule, status: &str, owner: String) -> RuleDetailRecord {
    let active_version = (status == "active").then_some(rule.version);
    let review_mode = normalize_review_mode(&rule.review_mode);
    let scheme_family = rule
        .scheme_family
        .as_deref()
        .map(normalize_scheme_family)
        .unwrap_or_else(|| scheme_family_from_alert_code(&rule.action.alert_code));
    let dsl = serde_json::json!({
        "review_mode": review_mode,
        "scheme_family": scheme_family,
        "conditions": rule.conditions,
        "action": rule.action
    });
    let summary = RuleSummaryRecord {
        rule_id: rule.rule_id.clone(),
        name: rule.name.clone(),
        status: status.into(),
        owner,
        active_version,
        latest_version: rule.version,
        review_mode: review_mode.clone(),
        scheme_family: scheme_family.clone(),
        score: rule.action.score,
        alert_code: rule.action.alert_code.clone(),
        recommended_action: rule.action.recommended_action,
        applicability_scope: rule_applicability_scope(&review_mode, &scheme_family),
        backtest_result: default_rule_backtest_summary(),
        estimated_saving: "0.00".into(),
        false_positive_history: default_rule_false_positive_history(),
        evidence_refs: rule_governance_evidence_refs(&rule.rule_id, rule.version),
    };
    let version = RuleVersionRecord {
        version: rule.version,
        status: status.into(),
        dsl,
        review_mode,
        scheme_family,
        score: rule.action.score,
        alert_code: rule.action.alert_code,
        recommended_action: rule.action.recommended_action,
        reason: rule.action.reason,
    };
    RuleDetailRecord {
        summary,
        versions: vec![version],
        audit_events: vec![],
    }
}

fn rule_applicability_scope(
    review_mode: &str,
    scheme_family: &str,
) -> RuleApplicabilityScopeRecord {
    RuleApplicabilityScopeRecord {
        review_mode: review_mode.into(),
        scheme_family: scheme_family.into(),
        source: "rule_dsl".into(),
    }
}

fn rule_governance_evidence_refs(rule_id: &str, version: u32) -> Vec<String> {
    vec![format!("rules:{rule_id}:v{version}")]
}

fn default_rule_backtest_summary() -> RuleBacktestSummaryRecord {
    RuleBacktestSummaryRecord {
        status: "not_run".into(),
        sample_count: 0,
        matched_count: 0,
        precision: 0.0,
        recall: 0.0,
        lift: 0.0,
        false_positive_rate: 0.0,
        estimated_saving: "0.00".into(),
        evidence_refs: vec![],
        created_at: None,
    }
}

fn default_rule_false_positive_history() -> RuleFalsePositiveHistoryRecord {
    RuleFalsePositiveHistoryRecord {
        status: "not_observed".into(),
        false_positive_count: 0,
        false_positive_rate: 0.0,
        evidence_refs: vec![],
    }
}

fn rule_backtest_summary(backtest: &RuleBacktestRecord) -> RuleBacktestSummaryRecord {
    RuleBacktestSummaryRecord {
        status: "completed".into(),
        sample_count: backtest.sample_count,
        matched_count: backtest.matched_count,
        precision: backtest.precision,
        recall: backtest.recall,
        lift: backtest.lift,
        false_positive_rate: backtest.false_positive_rate,
        estimated_saving: backtest.estimated_saving.clone(),
        evidence_refs: backtest.evidence_refs.clone(),
        created_at: backtest.created_at.clone(),
    }
}

fn rule_false_positive_history(backtest: &RuleBacktestRecord) -> RuleFalsePositiveHistoryRecord {
    RuleFalsePositiveHistoryRecord {
        status: if backtest.reviewed_count == 0 {
            "not_observed"
        } else {
            "observed"
        }
        .into(),
        false_positive_count: backtest.false_positive_count,
        false_positive_rate: backtest.false_positive_rate,
        evidence_refs: backtest.evidence_refs.clone(),
    }
}

fn apply_rule_backtest_metadata(
    summary: &mut RuleSummaryRecord,
    backtest: Option<&RuleBacktestRecord>,
) {
    if let Some(backtest) = backtest {
        summary.estimated_saving = backtest.estimated_saving.clone();
        summary.backtest_result = rule_backtest_summary(backtest);
        summary.false_positive_history = rule_false_positive_history(backtest);
        for reference in &backtest.evidence_refs {
            if !summary.evidence_refs.contains(reference) {
                summary.evidence_refs.push(reference.clone());
            }
        }
    }
}

fn latest_rule_backtest_for<'a>(
    backtests: &'a [RuleBacktestRecord],
    rule_id: &str,
    rule_version: u32,
) -> Option<&'a RuleBacktestRecord> {
    backtests
        .iter()
        .rev()
        .find(|record| record.rule_id == rule_id && record.rule_version == rule_version)
}

fn apply_rule_status(detail: &mut RuleDetailRecord, statuses: &HashMap<String, String>) {
    if let Some(status) = statuses.get(&detail.summary.rule_id) {
        detail.summary.status = status.clone();
        detail.summary.active_version =
            (status == "active").then_some(detail.summary.latest_version);
        for version in &mut detail.versions {
            version.status = status.clone();
        }
    }
}

fn parse_recommended_action(value: &str) -> RecommendedAction {
    match value {
        "AutoApprove" | "StandardProcessing" => RecommendedAction::StandardProcessing,
        "QaSample" => RecommendedAction::QaSample,
        "RequestEvidence" => RecommendedAction::RequestEvidence,
        "EscalateInvestigation" => RecommendedAction::EscalateInvestigation,
        "PostPaymentAudit" => RecommendedAction::PostPaymentAudit,
        "ProviderReview" => RecommendedAction::ProviderReview,
        "RecoveryReview" => RecommendedAction::RecoveryReview,
        _ => RecommendedAction::ManualReview,
    }
}

fn review_mode_from_dsl(dsl: &Value) -> String {
    dsl.get("review_mode")
        .and_then(Value::as_str)
        .map(normalize_review_mode)
        .unwrap_or_else(|| "both".into())
}

fn normalize_review_mode(value: &str) -> String {
    match value {
        "pre_payment" | "post_payment" | "both" => value.into(),
        _ => "both".into(),
    }
}

fn routing_policy_review_mode_applies(
    policy_review_mode: &str,
    requested_review_mode: &str,
) -> bool {
    policy_review_mode == "both" || policy_review_mode == requested_review_mode
}

fn default_routing_policies() -> Vec<RoutingPolicy> {
    ["pre_payment", "post_payment", "both"]
        .into_iter()
        .map(fwa_scoring::default_routing_policy)
        .collect()
}

fn seed_default_routing_policy_records(policies: &mut Vec<RoutingPolicyRecord>) {
    if policies.is_empty() {
        policies.extend(
            default_routing_policies()
                .into_iter()
                .map(|policy| routing_policy_record(policy, "active", "system", None, None)),
        );
    }
}

fn routing_policy_record(
    policy: RoutingPolicy,
    status: &str,
    owner: &str,
    activated_at: Option<String>,
    created_at: Option<String>,
) -> RoutingPolicyRecord {
    RoutingPolicyRecord {
        policy_id: policy.policy_id,
        version: policy.version,
        review_mode: policy.review_mode,
        status: status.into(),
        owner: owner.into(),
        risk_thresholds: policy.risk_thresholds,
        confidence_thresholds: policy.confidence_thresholds,
        provider_review_threshold: policy.provider_review_threshold,
        activated_at,
        created_at,
    }
}

fn routing_policy_record_from_row(
    row: (Value, String, String, Option<String>, Option<String>),
) -> anyhow::Result<RoutingPolicyRecord> {
    let (policy_json, status, owner, activated_at, created_at) = row;
    let policy: RoutingPolicy = serde_json::from_value(policy_json)?;
    Ok(routing_policy_record(
        policy,
        &status,
        &owner,
        activated_at,
        created_at,
    ))
}

fn routing_policy_from_record(record: &RoutingPolicyRecord) -> RoutingPolicy {
    RoutingPolicy {
        policy_id: record.policy_id.clone(),
        version: record.version,
        review_mode: record.review_mode.clone(),
        risk_thresholds: record.risk_thresholds.clone(),
        confidence_thresholds: record.confidence_thresholds.clone(),
        provider_review_threshold: record.provider_review_threshold,
    }
}

fn runtime_rule_from_detail(detail: RuleDetailRecord) -> anyhow::Result<Rule> {
    let version = detail
        .versions
        .into_iter()
        .find(|version| Some(version.version) == detail.summary.active_version)
        .ok_or_else(|| {
            anyhow::anyhow!("active version missing for rule {}", detail.summary.rule_id)
        })?;
    runtime_rule_from_parts(
        detail.summary.rule_id,
        detail.summary.name,
        version.version,
        version.dsl,
    )
}

fn runtime_rule_from_parts(
    rule_id: String,
    name: String,
    version: u32,
    dsl: Value,
) -> anyhow::Result<Rule> {
    Ok(Rule {
        rule_id,
        version,
        name,
        review_mode: review_mode_from_dsl(&dsl),
        scheme_family: dsl["scheme_family"].as_str().map(normalize_scheme_family),
        conditions: serde_json::from_value(dsl["conditions"].clone())?,
        action: serde_json::from_value(dsl["action"].clone())?,
    })
}

async fn ensure_default_rules_seeded(pool: &PgPool) -> anyhow::Result<()> {
    for detail in default_rule_details() {
        let mut tx = pool.begin().await?;
        let row: (String,) = sqlx::query_as(
            "INSERT INTO rules (rule_key, name, status, owner)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (rule_key) DO UPDATE SET updated_at = now()
             RETURNING id::text",
        )
        .bind(&detail.summary.rule_id)
        .bind(&detail.summary.name)
        .bind(&detail.summary.status)
        .bind(&detail.summary.owner)
        .fetch_one(&mut *tx)
        .await?;

        for version in detail.versions {
            sqlx::query(
                "INSERT INTO rule_versions
                 (rule_id, version, dsl, score, recommended_action, created_by, approved_by, published_at)
                 VALUES ($1::uuid, $2, $3, $4, $5, 'system', 'system', now())
                 ON CONFLICT (rule_id, version) DO NOTHING",
            )
            .bind(&row.0)
            .bind(version.version as i32)
            .bind(&version.dsl)
            .bind(version.score as i32)
            .bind(format!("{:?}", version.recommended_action))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
    }
    Ok(())
}

fn default_model_versions() -> Vec<ModelVersionRecord> {
    vec![ModelVersionRecord {
        model_key: "baseline_fwa".into(),
        version: "0.1.0".into(),
        model_type: "baseline_classifier".into(),
        runtime_kind: "python_http".into(),
        execution_provider: "cpu".into(),
        status: "active".into(),
        review_mode: "both".into(),
        artifact_uri: None,
        endpoint_url: Some("http://127.0.0.1:8001/score".into()),
    }]
}

fn model_version_key(model_key: &str, model_version: &str) -> String {
    format!("{model_key}:{model_version}")
}

fn empty_model_performance(model_key: &str) -> ModelPerformanceRecord {
    ModelPerformanceRecord {
        model_key: model_key.to_string(),
        data_status: "empty".into(),
        scored_runs: 0,
        average_score: 0.0,
        high_risk_count: 0,
        score_psi: None,
        drift_status: "not_available".into(),
        latest_scored_at: None,
    }
}

fn model_performance_with_drift(
    mut performance: ModelPerformanceRecord,
    drift: (Option<f64>, String),
) -> ModelPerformanceRecord {
    performance.score_psi = drift.0;
    performance.drift_status = drift.1;
    performance
}

fn drift_summary(metrics: &Value) -> (Option<f64>, String) {
    let score_psi = metrics
        .get("score_psi")
        .or_else(|| metrics.get("psi"))
        .and_then(Value::as_f64);
    let status = match score_psi {
        Some(value) if value < 0.10 => "stable",
        Some(value) if value < 0.25 => "watch",
        Some(_) => "drift",
        None => "not_available",
    };
    (score_psi, status.into())
}

fn default_knowledge_cases() -> Vec<KnowledgeCaseRecord> {
    vec![
        KnowledgeCaseRecord {
            case_id: "KC-1001".into(),
            title: "Early high-amount respiratory claim".into(),
            fwa_type: "Abuse".into(),
            scheme_family: "diagnosis_procedure_mismatch".into(),
            diagnosis_code: "J10".into(),
            provider_region: "Shanghai".into(),
            provider_type: "hospital".into(),
            summary: "保单生效早期发生高额呼吸系统相关理赔，项目组合与相似已确认案例接近。".into(),
            outcome: "Manual review confirmed over-treatment pattern".into(),
            tags: vec![
                "early_claim".into(),
                "high_amount".into(),
                "medical_mismatch".into(),
            ],
            evidence_refs: vec![
                "knowledge_cases:KC-1001".into(),
                "rule_runs:EARLY_CLAIM".into(),
            ],
        },
        KnowledgeCaseRecord {
            case_id: "KC-1002".into(),
            title: "Provider repeated high-cost package pattern".into(),
            fwa_type: "Waste".into(),
            scheme_family: "provider_peer_outlier".into(),
            diagnosis_code: "M54".into(),
            provider_region: "Beijing".into(),
            provider_type: "clinic".into(),
            summary: "同一 provider 在短期内重复出现高价项目组合，金额分布显著偏离同地区 peer。"
                .into(),
            outcome: "Provider education and pre-payment review added".into(),
            tags: vec!["provider_pattern".into(), "high_amount".into()],
            evidence_refs: vec![
                "knowledge_cases:KC-1002".into(),
                "feature_values:provider_high_cost_item_ratio_30d".into(),
            ],
        },
    ]
}

fn search_cases(
    cases: Vec<KnowledgeCaseRecord>,
    query: &SimilarCaseQuery,
) -> Vec<SimilarCaseRecord> {
    let mut results = cases
        .into_iter()
        .filter_map(|case| {
            let mut score: f64 = 0.0;
            let mut matched_signals = Vec::new();

            if case.diagnosis_code == query.diagnosis_code {
                score += 0.45;
                matched_signals.push(format!("diagnosis:{}", query.diagnosis_code));
            }
            if case.provider_region == query.provider_region {
                score += 0.25;
                matched_signals.push(format!("region:{}", query.provider_region));
            }
            for tag in &query.tags {
                if case.tags.iter().any(|case_tag| case_tag == tag) {
                    score += 0.15;
                    matched_signals.push(format!("tag:{tag}"));
                }
            }

            if score <= 0.0 {
                None
            } else {
                let mut provenance_refs = vec![
                    format!("knowledge_cases:{}", case.case_id),
                    "retrieval:structured_signal_overlap".into(),
                ];
                if let Some(claim_id) = &query.claim_id {
                    provenance_refs.push(format!("query_claim:{claim_id}"));
                }
                provenance_refs.extend(
                    matched_signals
                        .iter()
                        .map(|signal| format!("matched_signal:{signal}")),
                );

                Some(SimilarCaseRecord {
                    case_id: case.case_id,
                    title: case.title,
                    scheme_family: case.scheme_family,
                    similarity_score: score.min(1.0),
                    matched_signals,
                    retrieval_method: "structured_signal_overlap".into(),
                    provenance_refs,
                    summary: case.summary,
                    outcome: case.outcome,
                    evidence_refs: case.evidence_refs,
                })
            }
        })
        .collect::<Vec<_>>();

    results.sort_by(|left, right| {
        right
            .similarity_score
            .partial_cmp(&left.similarity_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

fn agent_run_log_from_persisted(run: &PersistedAgentRun) -> AgentRunLogRecord {
    AgentRunLogRecord {
        agent_run_id: run.agent_run_id.clone(),
        claim_id: run.claim_id.clone(),
        status: run.status.clone(),
        decision_boundary: run.decision_boundary.clone(),
        output_json: run.output_json.clone(),
        evidence_refs: evidence_values_to_strings(&run.evidence_refs),
        steps: run.steps.clone(),
        context_snapshots: run.context_snapshots.clone(),
        policy_checks: run.policy_checks.clone(),
        tool_calls: run.tool_calls.clone(),
        tool_results: run.tool_results.clone(),
        approvals: run.approvals.clone(),
        created_at: None,
        completed_at: None,
    }
}

fn json_array_to_strings(value: Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn string_values(values: &[String]) -> Value {
    Value::Array(values.iter().cloned().map(Value::String).collect())
}

fn evidence_values_to_strings(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .map(|value| match value {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .collect()
}

type DatasetRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Value,
    String,
    String,
    String,
    String,
    String,
    i64,
    String,
);
type DatasetSplitRow = (String, String, i64, Option<i64>, Option<i64>, Value);
type DatasetMappingRow = (
    String,
    String,
    String,
    Option<String>,
    String,
    Value,
    String,
);

async fn load_dataset_record(
    pool: &PgPool,
    dataset_id: &str,
) -> anyhow::Result<Option<DatasetRecord>> {
    let row: Option<DatasetRow> = sqlx::query_as(
        "SELECT d.id::text,
                d.source_key,
                s.display_name,
                s.business_domain,
                d.dataset_key,
                d.dataset_version,
                d.sample_grain,
                d.label_column,
                d.entity_keys,
                d.manifest_uri,
                d.schema_uri,
                d.profile_uri,
                d.storage_format,
                d.schema_hash,
                d.row_count,
                d.status
         FROM external_dataset_versions d
         JOIN external_data_sources s ON s.source_key = d.source_key
         WHERE d.id = $1::uuid",
    )
    .bind(dataset_id)
    .fetch_optional(pool)
    .await?;

    let Some((
        dataset_id,
        source_key,
        display_name,
        business_domain,
        dataset_key,
        dataset_version,
        sample_grain,
        label_column,
        entity_keys,
        manifest_uri,
        schema_uri,
        profile_uri,
        storage_format,
        schema_hash,
        row_count,
        status,
    )) = row
    else {
        return Ok(None);
    };

    let split_rows: Vec<DatasetSplitRow> = sqlx::query_as(
        "SELECT split_name, data_uri, row_count, positive_count, negative_count, label_distribution_json
         FROM external_dataset_splits
         WHERE dataset_id = $1::uuid
         ORDER BY split_name",
    )
    .bind(&dataset_id)
    .fetch_all(pool)
    .await?;

    let field_rows: Vec<(String, String, bool, String, String, Value)> = sqlx::query_as(
        "SELECT field_name, logical_type, nullable, semantic_role, description, profile_json
         FROM external_schema_fields
         WHERE dataset_id = $1::uuid
         ORDER BY field_name",
    )
    .bind(&dataset_id)
    .fetch_all(pool)
    .await?;

    let mapping_rows: Vec<DatasetMappingRow> = sqlx::query_as(
        "SELECT id::text, external_field, canonical_target, feature_name, transform_kind, transform_json, status
             FROM external_field_mappings
             WHERE dataset_id = $1::uuid
             ORDER BY created_at, external_field",
    )
    .bind(&dataset_id)
    .fetch_all(pool)
    .await?;

    Ok(Some(DatasetRecord {
        dataset_id: dataset_id.clone(),
        source_key,
        display_name,
        business_domain,
        dataset_key,
        dataset_version,
        sample_grain,
        label_column,
        entity_keys: json_array_to_strings(entity_keys),
        manifest_uri,
        schema_uri,
        profile_uri,
        storage_format,
        schema_hash,
        row_count: row_count as u64,
        status,
        splits: split_rows
            .into_iter()
            .map(
                |(
                    split_name,
                    data_uri,
                    row_count,
                    positive_count,
                    negative_count,
                    label_distribution_json,
                )| DatasetSplitRecord {
                    split_name,
                    data_uri,
                    row_count: row_count as u64,
                    positive_count: positive_count.map(|value| value as u64),
                    negative_count: negative_count.map(|value| value as u64),
                    label_distribution_json,
                },
            )
            .collect(),
        fields: field_rows
            .into_iter()
            .map(
                |(field_name, logical_type, nullable, semantic_role, description, profile_json)| {
                    SchemaFieldRecord {
                        field_name,
                        logical_type,
                        nullable,
                        semantic_role,
                        description,
                        profile_json,
                    }
                },
            )
            .collect(),
        mappings: mapping_rows
            .into_iter()
            .map(
                |(
                    mapping_id,
                    external_field,
                    canonical_target,
                    feature_name,
                    transform_kind,
                    transform_json,
                    status,
                )| FieldMappingRecord {
                    mapping_id,
                    dataset_id: dataset_id.clone(),
                    external_field,
                    canonical_target,
                    feature_name,
                    transform_kind,
                    transform_json,
                    status,
                },
            )
            .collect(),
    }))
}

async fn ensure_default_models_seeded(pool: &PgPool) -> anyhow::Result<()> {
    for model in default_model_versions() {
        sqlx::query(
            "INSERT INTO model_versions
             (model_key, version, model_type, runtime_kind, artifact_uri, endpoint_url, execution_provider, status, metrics, activated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now())
             ON CONFLICT (model_key, version) DO UPDATE SET
               metrics = model_versions.metrics || EXCLUDED.metrics",
        )
        .bind(&model.model_key)
        .bind(&model.version)
        .bind(&model.model_type)
        .bind(&model.runtime_kind)
        .bind(&model.artifact_uri)
        .bind(&model.endpoint_url)
        .bind(&model.execution_provider)
        .bind(&model.status)
        .bind(serde_json::json!({ "review_mode": model.review_mode }))
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn ensure_default_routing_policies_seeded(pool: &PgPool) -> anyhow::Result<()> {
    for policy in default_routing_policies() {
        sqlx::query(
            "INSERT INTO routing_policies
             (policy_key, version, review_mode, status, owner, policy_json, activated_at)
             VALUES ($1, $2, $3, 'active', 'system', $4, now())
             ON CONFLICT (policy_key, version, review_mode) DO UPDATE SET
               policy_json = EXCLUDED.policy_json",
        )
        .bind(&policy.policy_id)
        .bind(policy.version as i32)
        .bind(&policy.review_mode)
        .bind(serde_json::to_value(&policy)?)
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn ensure_default_knowledge_cases_seeded(pool: &PgPool) -> anyhow::Result<()> {
    for case in default_knowledge_cases() {
        sqlx::query(
            "INSERT INTO knowledge_cases
             (case_id, title, fwa_type, scheme_family, diagnosis_code, provider_region, provider_type, summary, outcome, tags, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (case_id) DO UPDATE SET
               scheme_family = EXCLUDED.scheme_family,
               updated_at = now()",
        )
        .bind(&case.case_id)
        .bind(&case.title)
        .bind(&case.fwa_type)
        .bind(&case.scheme_family)
        .bind(&case.diagnosis_code)
        .bind(&case.provider_region)
        .bind(&case.provider_type)
        .bind(&case.summary)
        .bind(&case.outcome)
        .bind(serde_json::json!(case.tags))
        .bind(serde_json::json!(case.evidence_refs))
        .execute(pool)
        .await?;
    }
    Ok(())
}

fn derive_saving_attributions(record: &InvestigationResultRecord) -> Vec<SavingAttributionRecord> {
    if !record.confirmed_fwa {
        return Vec::new();
    }
    let Some(total_saving) = record.saving_amount else {
        return Vec::new();
    };
    if total_saving <= Decimal::ZERO {
        return Vec::new();
    }

    let sources = record
        .evidence_refs
        .iter()
        .filter_map(|reference| recognized_attribution_source(reference))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if sources.is_empty() {
        return Vec::new();
    }

    let share = (total_saving / Decimal::from(sources.len() as u32)).round_dp(2);
    let currency = record.currency.clone().unwrap_or_else(|| "UNKNOWN".into());
    let financial_impact_type =
        normalize_financial_impact_type(record.financial_impact_type.as_deref()).to_string();

    sources
        .into_iter()
        .map(|(source_type, source_id)| SavingAttributionRecord {
            attribution_id: format!(
                "saving_{}_{}_{}",
                sanitize_identifier(&record.investigation_id),
                source_type,
                sanitize_identifier(&source_id)
            ),
            claim_id: record.claim_id.clone(),
            investigation_id: record.investigation_id.clone(),
            source_type,
            source_id,
            financial_impact_type: financial_impact_type.clone(),
            action: "investigation_confirmed".into(),
            saving_amount: share,
            currency: currency.clone(),
            evidence_refs: record.evidence_refs.clone(),
        })
        .collect()
}

fn recognized_attribution_source(reference: &str) -> Option<(String, String)> {
    if let Some(source_id) = reference.strip_prefix("agent_run:") {
        return Some(("agent".into(), source_id.to_string()));
    }
    if let Some(source_id) = reference.strip_prefix("rule_runs:") {
        return Some(("rule".into(), source_id.to_string()));
    }
    if let Some(source_id) = reference.strip_prefix("rules:") {
        return non_empty_prefix_before_version(source_id)
            .map(|source_id| ("rule".into(), source_id.to_string()));
    }
    if let Some(source_id) = reference.strip_prefix("model_scores:") {
        return Some(("model".into(), source_id.to_string()));
    }
    if let Some(source_id) = reference.strip_prefix("model_versions:") {
        return non_empty_prefix_before_version(source_id)
            .map(|source_id| ("model".into(), source_id.to_string()));
    }
    None
}

fn non_empty_prefix_before_version(reference_body: &str) -> Option<&str> {
    reference_body
        .split(':')
        .next()
        .map(str::trim)
        .filter(|source_id| !source_id.is_empty())
}

fn summarize_saving_attributions(
    records: &[SavingAttributionRecord],
) -> Vec<DashboardSavingAttributionRecord> {
    let mut accumulators = BTreeMap::<
        (String, String, String, String, String),
        (Decimal, u32, BTreeSet<String>),
    >::new();
    for record in records {
        let key = (
            record.source_type.clone(),
            record.source_id.clone(),
            record.financial_impact_type.clone(),
            record.action.clone(),
            record.currency.clone(),
        );
        let entry = accumulators
            .entry(key)
            .or_insert((Decimal::ZERO, 0, BTreeSet::new()));
        entry.0 += record.saving_amount;
        entry.1 += 1;
        entry.2.extend(record.evidence_refs.iter().cloned());
    }

    accumulators
        .into_iter()
        .map(
            |(
                (source_type, source_id, financial_impact_type, action, currency),
                (saving_amount, claim_count, evidence_refs),
            )| {
                DashboardSavingAttributionRecord {
                    source_type,
                    source_id,
                    financial_impact_type,
                    action,
                    saving_amount: format_decimal_cents(saving_amount),
                    currency,
                    claim_count,
                    evidence_refs: evidence_refs.into_iter().collect(),
                }
            },
        )
        .collect()
}

fn summarize_saving_segments(
    records: &[SavingAttributionRecord],
    leads: &[LeadRecord],
) -> Vec<DashboardSavingSegmentRecord> {
    let claim_segments = leads
        .iter()
        .map(|lead| {
            (
                lead.claim_id.as_str(),
                (lead.provider_id.as_str(), lead.scheme_family.as_str()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut accumulators =
        BTreeMap::<(String, String, String), (Decimal, BTreeSet<String>, u32)>::new();

    for record in records {
        let (provider_id, scheme_family) = claim_segments
            .get(record.claim_id.as_str())
            .copied()
            .unwrap_or(("unknown", "unknown"));
        let mut segments = vec![
            ("provider", provider_id.to_string()),
            ("scheme", scheme_family.to_string()),
        ];
        segments.extend(
            campaign_ids_from_evidence_refs(&record.evidence_refs)
                .into_iter()
                .map(|campaign_id| ("campaign", campaign_id)),
        );
        for (segment_type, segment_id) in segments {
            let key = (
                segment_type.to_string(),
                segment_id,
                record.currency.clone(),
            );
            let entry = accumulators
                .entry(key)
                .or_insert((Decimal::ZERO, BTreeSet::new(), 0));
            entry.0 += record.saving_amount;
            entry.1.insert(record.claim_id.clone());
            entry.2 += 1;
        }
    }

    accumulators
        .into_iter()
        .map(
            |((segment_type, segment_id, currency), (saving_amount, claims, attribution_count))| {
                let claim_count = claims.len() as u32;
                DashboardSavingSegmentRecord {
                    segment_type,
                    segment_id,
                    saving_amount: format_decimal_cents(saving_amount),
                    currency,
                    claim_count,
                    attribution_count,
                    roi: segment_roi(saving_amount, claim_count),
                }
            },
        )
        .collect()
}

fn campaign_ids_from_evidence_refs(evidence_refs: &[String]) -> BTreeSet<String> {
    evidence_refs
        .iter()
        .filter_map(|reference| {
            reference
                .strip_prefix("campaigns:")
                .or_else(|| reference.strip_prefix("campaign:"))
        })
        .map(str::trim)
        .filter(|campaign_id| !campaign_id.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn segment_roi(saving_amount: Decimal, claim_count: u32) -> f64 {
    if claim_count == 0 {
        return 0.0;
    }
    let review_cost = claim_count as f64 * RULE_REVIEW_COST_AMOUNT;
    if review_cost == 0.0 {
        0.0
    } else {
        decimal_to_f64(&saving_amount) / review_cost
    }
}

fn format_decimal_cents(value: Decimal) -> String {
    format!("{:.2}", value.round_dp(2))
}

fn member_profile_from_contexts(
    member_id: &str,
    contexts: &[ClaimContext],
    runs: &[PersistedScoringRun],
) -> Option<MemberProfileSummaryRecord> {
    if contexts.is_empty() {
        return None;
    }

    let claim_ids = contexts
        .iter()
        .map(|context| context.claim.external_claim_id.clone())
        .collect::<BTreeSet<_>>();
    let policy_count = contexts
        .iter()
        .map(|context| context.policy.external_policy_id.clone())
        .collect::<BTreeSet<_>>()
        .len() as u32;
    let total_claim_amount = contexts
        .iter()
        .map(|context| context.claim.amount.amount)
        .sum::<Decimal>();
    let currency = contexts
        .first()
        .map(|context| context.claim.amount.currency.clone())
        .unwrap_or_else(|| "UNKNOWN".into());
    let high_risk_claim_count = runs
        .iter()
        .filter(|run| claim_ids.contains(&run.claim_id) && run.risk_score >= 70)
        .map(|run| run.claim_id.clone())
        .collect::<BTreeSet<_>>()
        .len() as u32;
    let latest_claim_id = contexts
        .iter()
        .max_by(|left, right| {
            left.claim
                .service_date
                .cmp(&right.claim.service_date)
                .then_with(|| {
                    left.claim
                        .external_claim_id
                        .cmp(&right.claim.external_claim_id)
                })
        })
        .map(|context| context.claim.external_claim_id.clone());
    let evidence_refs = std::iter::once(format!("members:{member_id}"))
        .chain(
            claim_ids
                .iter()
                .map(|claim_id| format!("claims:{claim_id}")),
        )
        .collect::<BTreeSet<_>>();

    Some(member_profile_summary_record(MemberProfileSummaryInput {
        member_id: member_id.into(),
        claim_count: contexts.len() as u32,
        policy_count,
        total_claim_amount,
        currency,
        high_risk_claim_count,
        latest_claim_id,
        evidence_refs,
    }))
}

fn member_profile_summary_record(input: MemberProfileSummaryInput) -> MemberProfileSummaryRecord {
    let risk_level_summary = if input.high_risk_claim_count > 0 {
        "has_high_risk_history"
    } else {
        "no_high_risk_history"
    };
    let profile_summary = format!(
        "投保人共有 {} 张保单、{} 笔历史理赔，累计理赔金额 {} {}，其中 {} 笔为高风险评分记录。",
        input.policy_count,
        input.claim_count,
        format_decimal_cents(input.total_claim_amount),
        input.currency,
        input.high_risk_claim_count
    );

    MemberProfileSummaryRecord {
        member_id: input.member_id,
        claim_count: input.claim_count,
        policy_count: input.policy_count,
        total_claim_amount: input.total_claim_amount,
        currency: input.currency,
        high_risk_claim_count: input.high_risk_claim_count,
        latest_claim_id: input.latest_claim_id,
        risk_level_summary: risk_level_summary.into(),
        profile_summary,
        evidence_refs: input.evidence_refs.into_iter().collect(),
    }
}

fn sanitize_identifier(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

async fn insert_audit_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event: &PersistedAuditEvent,
    claim_uuid: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO audit_events
         (audit_id, run_id, claim_id, actor_id, actor_role, source_system, event_type, event_status, summary, payload, evidence_refs)
         VALUES ($1, $2, $3::uuid, $4, $5, $6, $7, $8, $9, $10, $11)
         ON CONFLICT (audit_id) DO UPDATE
         SET run_id = EXCLUDED.run_id,
             claim_id = EXCLUDED.claim_id,
             actor_id = EXCLUDED.actor_id,
             actor_role = EXCLUDED.actor_role,
             source_system = EXCLUDED.source_system,
             event_type = EXCLUDED.event_type,
             event_status = EXCLUDED.event_status,
             summary = EXCLUDED.summary,
             payload = EXCLUDED.payload,
             evidence_refs = EXCLUDED.evidence_refs",
    )
    .bind(&event.audit_id)
    .bind(&event.run_id)
    .bind(claim_uuid)
    .bind(&event.actor_id)
    .bind(&event.actor_role)
    .bind(&event.source_system)
    .bind(&event.event_type)
    .bind(&event.event_status)
    .bind(&event.summary)
    .bind(&event.payload)
    .bind(serde_json::Value::Array(event.evidence_refs.clone()))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_pilot_audit_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    claim_id: &str,
    event: &AuditHistoryEventRecord,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO scoring_runs
         (run_id, source_system, actor_id, status, completed_at)
         VALUES ($1, 'pilot-loop', $2, 'succeeded', now())
         ON CONFLICT (run_id) DO NOTHING",
    )
    .bind(&event.run_id)
    .bind(&event.actor_role)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "INSERT INTO audit_events
         (audit_id, run_id, claim_id, actor_id, actor_role, source_system, event_type, event_status, summary, payload, evidence_refs)
         VALUES ($1, $2, NULL, $3, $4, 'pilot-loop', $5, $6, $7, $8, $9)
         ON CONFLICT (audit_id) DO UPDATE
         SET event_status = EXCLUDED.event_status,
             summary = EXCLUDED.summary,
             payload = EXCLUDED.payload,
             evidence_refs = EXCLUDED.evidence_refs",
    )
    .bind(&event.audit_id)
    .bind(&event.run_id)
    .bind(claim_id)
    .bind(&event.actor_role)
    .bind(&event.event_type)
    .bind(&event.event_status)
    .bind(&event.summary)
    .bind(&event.payload)
    .bind(serde_json::json!(event.evidence_refs))
    .execute(&mut **tx)
    .await?;
    Ok(())
}
