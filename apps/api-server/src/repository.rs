use async_trait::async_trait;
use chrono::NaiveDate;
use fwa_core::{
    AuditEventId, Claim, ClaimContext, ClaimId, ClaimItem, Member, MemberId, Money, Policy,
    PolicyId, Provider, ProviderId, ProviderRiskTier, RecommendedAction,
};
use fwa_rules::{Condition, Rule, RuleAction};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Transaction};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
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
    pub scheme_family: String,
    pub lead_source: String,
    pub status: String,
    pub assignee: String,
    pub reviewer: String,
    pub priority: String,
    pub routing_reason: String,
    pub evidence_package: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageLeadInput {
    pub decision: String,
    pub assignee: String,
    pub reviewer: String,
    pub priority: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageLeadRecord {
    pub case: CaseRecord,
    pub audit_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCaseStatusInput {
    pub status: String,
    pub actor_id: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSampleRecord {
    pub sample_id: String,
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
    pub created_at: Option<String>,
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
    pub action: String,
    pub saving_amount: String,
    pub currency: String,
    pub claim_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRiskSummaryItemRecord {
    pub provider_id: String,
    pub risk_score: u8,
    pub risk_tier: String,
    pub review_required: bool,
    pub review_route: String,
    pub claim_count: u32,
    pub latest_claim_id: Option<String>,
    pub outlier_flags: Vec<String>,
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
    pub workflow_feedback: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardQaQueueRecord {
    pub sampled_cases: u32,
    pub open_cases: u32,
    pub reviewed_cases: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardAgentGovernanceRecord {
    pub total_runs: u32,
    pub successful_runs: u32,
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
pub struct DashboardSummaryRecord {
    pub suspected_claims: u32,
    pub confirmed_fwa: u32,
    pub risk_amount: String,
    pub saving_amount: String,
    pub rag_distribution: BTreeMap<String, u32>,
    pub rule_hits: u32,
    pub model_scores: BTreeMap<String, DashboardModelScoreRecord>,
    pub layer_scores: BTreeMap<String, DashboardLayerScoreRecord>,
    pub saving_attributions: Vec<DashboardSavingAttributionRecord>,
    pub label_pool: DashboardLabelPoolRecord,
    pub qa_queue: DashboardQaQueueRecord,
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
    pub claim_id: String,
    pub outcome: String,
    pub confirmed_fwa: bool,
    pub saving_amount: Option<Decimal>,
    pub currency: Option<String>,
    pub notes: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone)]
struct SavingAttributionRecord {
    attribution_id: String,
    claim_id: String,
    investigation_id: String,
    source_type: String,
    source_id: String,
    action: String,
    saving_amount: Decimal,
    currency: String,
    evidence_refs: Vec<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditHistoryEventRecord {
    pub audit_id: String,
    pub run_id: String,
    pub event_type: String,
    pub event_status: String,
    pub summary: String,
    pub payload: Value,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
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
    i32,
    String,
    String,
    Value,
);
type CaseRow = (
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
    String,
    String,
    Value,
);

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
    ) -> anyhow::Result<Option<ClaimContext>>;

    async fn member_profile_summary(
        &self,
        member_id: &str,
    ) -> anyhow::Result<Option<MemberProfileSummaryRecord>>;

    async fn save_scoring_run(&self, run: PersistedScoringRun) -> anyhow::Result<()>;

    async fn save_audit_event(&self, event: PersistedAuditEvent) -> anyhow::Result<()>;

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

    async fn list_leads(&self) -> anyhow::Result<Vec<LeadRecord>>;

    async fn triage_lead(
        &self,
        lead_id: &str,
        input: TriageLeadInput,
    ) -> anyhow::Result<Option<TriageLeadRecord>>;

    async fn list_cases(&self) -> anyhow::Result<Vec<CaseRecord>>;

    async fn update_case_status(
        &self,
        case_id: &str,
        input: UpdateCaseStatusInput,
    ) -> anyhow::Result<Option<UpdateCaseStatusRecord>>;

    async fn create_audit_sample(
        &self,
        input: CreateAuditSampleInput,
    ) -> anyhow::Result<AuditSampleRecord>;

    async fn list_audit_samples(&self) -> anyhow::Result<Vec<AuditSampleRecord>>;

    async fn list_models(&self) -> anyhow::Result<Vec<ModelVersionRecord>>;

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

    async fn dashboard_summary(&self) -> anyhow::Result<DashboardSummaryRecord>;

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

    async fn list_agent_runs(&self) -> anyhow::Result<Vec<AgentRunLogRecord>>;

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

    async fn list_qa_feedback_items(&self) -> anyhow::Result<Vec<QaFeedbackItemRecord>>;

    async fn list_qa_reviews(&self) -> anyhow::Result<Vec<QaReviewRecord>>;

    async fn list_outcome_labels(&self) -> anyhow::Result<Vec<OutcomeLabelRecord>>;

    async fn claim_audit_history(
        &self,
        claim_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>>;

    async fn register_feature_set(
        &self,
        input: RegisterFeatureSetInput,
    ) -> anyhow::Result<Option<FeatureSetRecord>>;

    async fn register_model_dataset(
        &self,
        input: RegisterModelDatasetInput,
    ) -> anyhow::Result<Option<ModelDatasetRecord>>;

    async fn register_model_evaluation(
        &self,
        input: RegisterModelEvaluationInput,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>>;

    async fn get_model_evaluation(
        &self,
        evaluation_run_id: &str,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>>;

    async fn list_model_evaluations(&self) -> anyhow::Result<Vec<ModelEvaluationRecord>>;
}

pub type SharedRepository = Arc<dyn ScoringRepository>;

#[derive(Debug, Default)]
pub struct InMemoryScoringRepository {
    claims: Mutex<HashMap<String, ClaimContext>>,
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
    model_evaluations: Mutex<HashMap<String, ModelEvaluationRecord>>,
    model_promotion_reviews: Mutex<Vec<ModelPromotionReviewRecord>>,
    saving_attributions: Mutex<Vec<SavingAttributionRecord>>,
}

impl InMemoryScoringRepository {
    pub fn shared() -> SharedRepository {
        Arc::new(Self::default())
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
    ) -> anyhow::Result<Option<ClaimContext>> {
        Ok(self.claims.lock().await.get(external_claim_id).cloned())
    }

    async fn member_profile_summary(
        &self,
        member_id: &str,
    ) -> anyhow::Result<Option<MemberProfileSummaryRecord>> {
        let member_claims = self
            .claims
            .lock()
            .await
            .values()
            .filter(|context| context.member.external_member_id == member_id)
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
        self.audit_events.lock().await.push(event);
        Ok(())
    }

    async fn list_rules(&self) -> anyhow::Result<Vec<RuleSummaryRecord>> {
        let statuses = self.rule_statuses.lock().await;
        let mut details = default_rule_details();
        details.extend(self.candidate_rules.lock().await.values().cloned());
        let mut rules = details
            .into_iter()
            .map(|mut detail| {
                apply_rule_status(&mut detail, &statuses);
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
        let mut details = default_rule_details();
        details.extend(self.candidate_rules.lock().await.values().cloned());
        let audit_events = self.rule_audit_history(rule_id).await?;
        Ok(details
            .into_iter()
            .find(|detail| detail.summary.rule_id == rule_id)
            .map(|mut detail| {
                apply_rule_status(&mut detail, &statuses);
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

    async fn list_leads(&self) -> anyhow::Result<Vec<LeadRecord>> {
        let mut leads = self
            .leads
            .lock()
            .await
            .values()
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
        let Some(lead) = leads.get_mut(lead_id) else {
            return Ok(None);
        };
        lead.status = "triaged".into();
        lead.disposition = input.decision.clone();
        let case = case_from_lead(lead, &input);
        self.cases
            .lock()
            .await
            .insert(case.case_id.clone(), case.clone());
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
            payload: serde_json::json!({
                "lead_id": lead.lead_id.clone(),
                "case_id": case.case_id.clone(),
                "decision": input.decision.clone(),
                "notes": input.notes.clone()
            }),
            evidence_refs: lead
                .evidence_refs
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        });
        Ok(Some(TriageLeadRecord { case, audit_id }))
    }

    async fn list_cases(&self) -> anyhow::Result<Vec<CaseRecord>> {
        let mut cases = self
            .cases
            .lock()
            .await
            .values()
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
        let from_status = case.status.clone();
        case.status = input.status.clone();
        let case = case.clone();
        let audit_id = AuditEventId::new().to_string();
        self.audit_events.lock().await.push(PersistedAuditEvent {
            audit_id: audit_id.clone(),
            run_id: format!("case_status_{}", case.case_id),
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
                "notes": input.notes
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
        let leads = self.list_leads().await?;
        let sample = build_audit_sample(sample_id, input, leads, None);
        self.audit_samples
            .lock()
            .await
            .insert(sample.sample_id.clone(), sample.clone());
        Ok(sample)
    }

    async fn list_audit_samples(&self) -> anyhow::Result<Vec<AuditSampleRecord>> {
        let mut samples = self
            .audit_samples
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        samples.sort_by(|left, right| left.sample_id.cmp(&right.sample_id));
        Ok(samples)
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelVersionRecord>> {
        Ok(default_model_versions())
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

    async fn dashboard_summary(&self) -> anyhow::Result<DashboardSummaryRecord> {
        let runs = self.runs.lock().await;
        let claims = self.claims.lock().await;
        let pilot_events = self.pilot_audit_events.lock().await;
        let saving_attribution_records = self.saving_attributions.lock().await;

        let mut risk_amount = Decimal::ZERO;
        let mut rag_distribution = BTreeMap::new();
        let mut model_accumulators = BTreeMap::<String, (u32, u32, u32)>::new();
        let mut layer_accumulators = BTreeMap::<String, (String, u32, u32, u32)>::new();
        let mut rule_hits = 0_u32;

        for run in runs.iter() {
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

        for (_, event) in pilot_events.iter() {
            match event.event_type.as_str() {
                "investigation.result.received" => {
                    investigation_results += 1;
                    if let Ok(record) =
                        serde_json::from_value::<InvestigationResultRecord>(event.payload.clone())
                    {
                        outcome_labels.extend(labels_from_investigation_result(record));
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
                        outcome_labels.push(label_from_qa_review(record));
                    }
                }
                _ => {}
            }
        }
        let suspected_claims = runs.iter().filter(|run| run.risk_score >= 70).count() as u32;
        let saving_attributions = summarize_saving_attributions(&saving_attribution_records);
        drop(saving_attribution_records);
        drop(pilot_events);
        drop(claims);
        drop(runs);

        let audit_samples = self.list_audit_samples().await?;
        let qa_review_records = self.list_qa_reviews().await?;
        let agent_runs = self.list_agent_runs().await?;
        let models = self.list_models().await?;
        let model_evaluations = self.list_model_evaluations().await?;
        let rules = self.list_rules().await?;
        let rule_performance = self.rule_performance().await?;

        Ok(DashboardSummaryRecord {
            suspected_claims,
            confirmed_fwa,
            risk_amount: risk_amount.to_string(),
            saving_amount: saving_amount.to_string(),
            rag_distribution,
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
            label_pool: summarize_dashboard_label_pool(&outcome_labels),
            qa_queue: summarize_dashboard_qa_queue(&audit_samples, &qa_review_records),
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

    async fn list_agent_runs(&self) -> anyhow::Result<Vec<AgentRunLogRecord>> {
        let mut runs = self
            .agent_runs
            .lock()
            .await
            .iter()
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
        let event = AuditHistoryEventRecord {
            audit_id: format!("audit_investigation_{}", record.investigation_id),
            run_id: format!("pilot_investigation_{}", record.investigation_id),
            event_type: "investigation.result.received".into(),
            event_status: "succeeded".into(),
            summary: format!("Investigation result received: {}", record.outcome),
            payload: serde_json::to_value(&record)?,
            evidence_refs: record.evidence_refs.clone(),
            created_at: None,
        };
        self.pilot_audit_events
            .lock()
            .await
            .push((record.claim_id.clone(), event.clone()));
        let mut stored_attributions = self.saving_attributions.lock().await;
        stored_attributions
            .retain(|attribution| attribution.investigation_id != record.investigation_id);
        stored_attributions.extend(saving_attributions);
        Ok(event)
    }

    async fn save_qa_review(
        &self,
        record: QaReviewRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        let event = AuditHistoryEventRecord {
            audit_id: format!("audit_qa_{}", record.qa_case_id),
            run_id: format!("pilot_qa_{}", record.qa_case_id),
            event_type: "qa.result.received".into(),
            event_status: "succeeded".into(),
            summary: format!("QA result received: {}", record.qa_conclusion),
            payload: serde_json::to_value(&record)?,
            evidence_refs: record.evidence_refs.clone(),
            created_at: None,
        };
        self.pilot_audit_events
            .lock()
            .await
            .push((record.claim_id, event.clone()));
        Ok(event)
    }

    async fn list_qa_feedback_items(&self) -> anyhow::Result<Vec<QaFeedbackItemRecord>> {
        let mut items = self
            .pilot_audit_events
            .lock()
            .await
            .iter()
            .filter_map(|(_, event)| {
                (event.event_type == "qa.result.received")
                    .then(|| serde_json::from_value::<QaReviewRecord>(event.payload.clone()).ok())
                    .flatten()
            })
            .filter(|review| review.qa_conclusion != "pass")
            .map(|review| qa_review_to_feedback_item(review, None))
            .collect::<Vec<_>>();
        sort_qa_feedback_items(&mut items);
        Ok(items)
    }

    async fn list_qa_reviews(&self) -> anyhow::Result<Vec<QaReviewRecord>> {
        let mut reviews = self
            .pilot_audit_events
            .lock()
            .await
            .iter()
            .filter_map(|(_, event)| {
                (event.event_type == "qa.result.received")
                    .then(|| serde_json::from_value::<QaReviewRecord>(event.payload.clone()).ok())
                    .flatten()
            })
            .collect::<Vec<_>>();
        reviews.sort_by(|left, right| left.qa_case_id.cmp(&right.qa_case_id));
        Ok(reviews)
    }

    async fn list_outcome_labels(&self) -> anyhow::Result<Vec<OutcomeLabelRecord>> {
        let mut labels = self
            .pilot_audit_events
            .lock()
            .await
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
                        .map(|review| vec![label_from_qa_review(review)])
                }
                _ => None,
            })
            .flatten()
            .collect::<Vec<_>>();
        sort_outcome_labels(&mut labels);
        Ok(labels)
    }

    async fn claim_audit_history(
        &self,
        claim_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        let mut events = self
            .audit_events
            .lock()
            .await
            .iter()
            .filter(|event| event.claim_id == claim_id)
            .map(|event| AuditHistoryEventRecord {
                audit_id: event.audit_id.clone(),
                run_id: event.run_id.clone(),
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
                .filter(|(event_claim_id, _)| event_claim_id == claim_id)
                .map(|(_, event)| event.clone()),
        );
        Ok(events)
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
             WHERE c.external_claim_id = $1",
        )
        .bind(external_claim_id)
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
    ) -> anyhow::Result<Option<MemberProfileSummaryRecord>> {
        let member_exists: Option<(String,)> =
            sqlx::query_as("SELECT external_member_id FROM members WHERE external_member_id = $1")
                .bind(member_id)
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
             WHERE m.external_member_id = $1",
        )
        .bind(member_id)
        .fetch_one(&self.pool)
        .await?;
        let latest_claim: Option<(String,)> = sqlx::query_as(
            "SELECT c.external_claim_id
             FROM claims c
             JOIN members m ON m.id = c.member_id
             WHERE m.external_member_id = $1
             ORDER BY c.service_date DESC, c.external_claim_id DESC
             LIMIT 1",
        )
        .bind(member_id)
        .fetch_optional(&self.pool)
        .await?;
        let high_risk: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT c.id)::bigint
             FROM members m
             JOIN claims c ON c.member_id = m.id
             JOIN scoring_runs sr ON sr.claim_id = c.id
             WHERE m.external_member_id = $1
               AND sr.risk_score >= 70",
        )
        .bind(member_id)
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
             (run_id, claim_id, source_system, actor_id, status, risk_score, rag, risk_level, recommended_action, confidence_score, confidence, routing_reason, score_breakdown, completed_at)
             VALUES ($1, $2::uuid, $3, $4, 'succeeded', $5, $6, $7, $8, $9, $10, $11, $12, now())",
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
                   '[]'::jsonb
                 )",
            )
            .bind(&run.run_id)
            .bind(rule_run["rule_id"].as_str())
            .bind(rule_run["rule_version"].as_i64().unwrap_or(1) as i32)
            .bind(rule_run["score_contribution"].as_i64().unwrap_or(0) as i32)
            .bind(rule_run["alert_code"].as_str())
            .bind(rule_run["reason"].as_str())
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            "INSERT INTO model_scores
             (run_id, model_key, runtime_kind, execution_provider, score, label, explanation_json, latency_ms)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&run.run_id)
        .bind(run.model_score["model_key"].as_str().unwrap_or("unknown"))
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
                 (lead_id, run_id, claim_id, member_id, provider_id, source_system, scheme_family, lead_source, status, disposition, risk_score, rag, reason, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                 ON CONFLICT (lead_id) DO UPDATE
                 SET status = EXCLUDED.status,
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

        Ok(rows
            .into_iter()
            .map(
                |(rule_id, name, status, owner, version, dsl, score, recommended_action)| {
                    let action = dsl.get("action").cloned().unwrap_or(Value::Null);
                    RuleSummaryRecord {
                        rule_id,
                        name,
                        active_version: if status == "active" {
                            Some(version as u32)
                        } else {
                            None
                        },
                        latest_version: version as u32,
                        review_mode: review_mode_from_dsl(&dsl),
                        scheme_family: scheme_family_from_dsl(&dsl),
                        status,
                        owner,
                        score: score as u8,
                        alert_code: action["alert_code"]
                            .as_str()
                            .unwrap_or("UNKNOWN")
                            .to_string(),
                        recommended_action: parse_recommended_action(&recommended_action),
                    }
                },
            )
            .collect())
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
        let rows: Vec<(String, String, String, String, String, Value, Value, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as(
                "SELECT audit_id, run_id, event_type, event_status, summary, payload, evidence_refs, created_at
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
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs,
                    created_at,
                )| AuditHistoryEventRecord {
                    audit_id,
                    run_id,
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
             (rule_id, rule_version, decision, reviewer, notes)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING created_at",
        )
        .bind(&record.rule_id)
        .bind(record.rule_version as i32)
        .bind(&record.decision)
        .bind(&record.reviewer)
        .bind(&record.notes)
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
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT rule_id, rule_version, decision, reviewer, notes, created_at
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
            |(rule_id, rule_version, decision, reviewer, notes, created_at)| {
                RulePromotionReviewRecord {
                    rule_id,
                    rule_version: rule_version as u32,
                    decision,
                    reviewer,
                    notes,
                    created_at: Some(created_at.to_rfc3339()),
                }
            },
        ))
    }

    async fn list_leads(&self) -> anyhow::Result<Vec<LeadRecord>> {
        load_leads(&self.pool).await
    }

    async fn triage_lead(
        &self,
        lead_id: &str,
        input: TriageLeadInput,
    ) -> anyhow::Result<Option<TriageLeadRecord>> {
        let mut tx = self.pool.begin().await?;
        let lead = load_lead_in_tx(&mut tx, lead_id).await?;
        let Some(mut lead) = lead else {
            return Ok(None);
        };
        lead.status = "triaged".into();
        lead.disposition = input.decision.clone();
        let case = case_from_lead(&lead, &input);
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
        sqlx::query(
            "INSERT INTO investigation_cases
             (case_id, lead_id, claim_id, member_id, provider_id, source_system, scheme_family, lead_source, status, assignee, reviewer, priority, routing_reason, evidence_package_json)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
             ON CONFLICT (case_id) DO UPDATE
             SET status = EXCLUDED.status,
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
                payload: serde_json::json!({
                    "lead_id": lead.lead_id.clone(),
                    "case_id": case.case_id.clone(),
                    "decision": input.decision.clone(),
                    "notes": input.notes.clone()
                }),
                evidence_refs: lead
                    .evidence_refs
                    .iter()
                    .map(|value| Value::String(value.clone()))
                    .collect(),
            },
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(Some(TriageLeadRecord { case, audit_id }))
    }

    async fn list_cases(&self) -> anyhow::Result<Vec<CaseRecord>> {
        load_cases(&self.pool).await
    }

    async fn update_case_status(
        &self,
        case_id: &str,
        input: UpdateCaseStatusInput,
    ) -> anyhow::Result<Option<UpdateCaseStatusRecord>> {
        let mut tx = self.pool.begin().await?;
        let case = load_case_in_tx(&mut tx, case_id).await?;
        let Some(mut case) = case else {
            return Ok(None);
        };
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

        let audit_id = AuditEventId::new().to_string();
        insert_audit_event(
            &mut tx,
            &PersistedAuditEvent {
                audit_id: audit_id.clone(),
                run_id: format!("case_status_{}", case.case_id),
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
                    "notes": input.notes
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
        let leads = self.list_leads().await?;
        let sample = build_audit_sample(sample_id, input, leads, None);
        sqlx::query(
            "INSERT INTO audit_samples
             (sample_id, sample_mode, population_definition, inclusion_criteria_json, deterministic_seed, selection_method, sample_size, reviewer, assignment_queue, selected_leads_json, outcome_distribution_json)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(&sample.sample_id)
        .bind(&sample.sample_mode)
        .bind(&sample.population_definition)
        .bind(&sample.inclusion_criteria)
        .bind(&sample.deterministic_seed)
        .bind(&sample.selection_method)
        .bind(sample.sample_size as i64)
        .bind(&sample.reviewer)
        .bind(&sample.assignment_queue)
        .bind(serde_json::to_value(&sample.selected_leads)?)
        .bind(&sample.outcome_distribution)
        .execute(&self.pool)
        .await?;
        self.list_audit_samples()
            .await?
            .into_iter()
            .find(|record| record.sample_id == sample.sample_id)
            .ok_or_else(|| anyhow::anyhow!("created audit sample was not found"))
    }

    async fn list_audit_samples(&self) -> anyhow::Result<Vec<AuditSampleRecord>> {
        let rows: Vec<(
            String,
            String,
            String,
            Value,
            Option<String>,
            String,
            i64,
            String,
            String,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT sample_id, sample_mode, population_definition, inclusion_criteria_json, deterministic_seed, selection_method, sample_size, reviewer, assignment_queue, selected_leads_json, outcome_distribution_json, created_at
             FROM audit_samples
             ORDER BY created_at, sample_id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    sample_id,
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
            .collect())
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
             (model_key, model_version, decision, reviewer, notes)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING created_at",
        )
        .bind(&record.model_key)
        .bind(&record.model_version)
        .bind(&record.decision)
        .bind(&record.reviewer)
        .bind(&record.notes)
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
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT model_key, model_version, decision, reviewer, notes, created_at
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
            |(model_key, model_version, decision, reviewer, notes, created_at)| {
                ModelPromotionReviewRecord {
                    model_key,
                    model_version,
                    decision,
                    reviewer,
                    notes,
                    created_at: Some(created_at.to_rfc3339()),
                }
            },
        ))
    }

    async fn dashboard_summary(&self) -> anyhow::Result<DashboardSummaryRecord> {
        let suspected: (i64, Option<Decimal>) = sqlx::query_as(
            "SELECT COUNT(*)::bigint, COALESCE(SUM(c.claim_amount), 0)
             FROM scoring_runs sr
             LEFT JOIN claims c ON c.id = sr.claim_id
             WHERE sr.risk_score >= 70",
        )
        .fetch_one(&self.pool)
        .await?;

        let rag_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT COALESCE(rag, 'UNKNOWN'), COUNT(*)::bigint
             FROM scoring_runs
             WHERE rag IS NOT NULL
             GROUP BY rag
             ORDER BY rag",
        )
        .fetch_all(&self.pool)
        .await?;

        let rule_hits: (i64,) =
            sqlx::query_as("SELECT COUNT(*)::bigint FROM rule_runs WHERE matched = true")
                .fetch_one(&self.pool)
                .await?;

        let model_rows: Vec<(String, i64, Option<Decimal>, Option<i64>)> = sqlx::query_as(
            "SELECT model_key,
                    COUNT(*)::bigint,
                    AVG(score),
                    SUM(CASE WHEN score >= 70 THEN 1 ELSE 0 END)::bigint
             FROM model_scores
             GROUP BY model_key
             ORDER BY model_key",
        )
        .fetch_all(&self.pool)
        .await?;

        let layer_payloads: Vec<(Value,)> = sqlx::query_as(
            "SELECT payload
             FROM audit_events
             WHERE event_type = 'scoring.completed'
               AND event_status = 'succeeded'",
        )
        .fetch_all(&self.pool)
        .await?;
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
             FROM investigation_results",
        )
        .fetch_one(&self.pool)
        .await?;

        let qa_reviews: (i64,) = sqlx::query_as("SELECT COUNT(*)::bigint FROM qa_reviews")
            .fetch_one(&self.pool)
            .await?;

        let saving_attributions: Vec<(String, String, String, Option<Decimal>, String, i64)> =
            sqlx::query_as(
                "SELECT source_type,
                        source_id,
                        action,
                        COALESCE(SUM(saving_amount), 0),
                        currency,
                        COUNT(DISTINCT claim_id)::bigint
                 FROM saving_attributions
                 GROUP BY source_type, source_id, action, currency
                 ORDER BY source_type, source_id, action, currency",
            )
            .fetch_all(&self.pool)
            .await?;
        let outcome_labels = self.list_outcome_labels().await?;
        let audit_samples = self.list_audit_samples().await?;
        let qa_review_records = self.list_qa_reviews().await?;
        let agent_runs = self.list_agent_runs().await?;
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
                    |(source_type, source_id, action, saving_amount, currency, claim_count)| {
                        DashboardSavingAttributionRecord {
                            source_type,
                            source_id,
                            action,
                            saving_amount: format_decimal_cents(
                                saving_amount.unwrap_or(Decimal::ZERO),
                            ),
                            currency,
                            claim_count: claim_count as u32,
                        }
                    },
                )
                .collect(),
            label_pool: summarize_dashboard_label_pool(&outcome_labels),
            qa_queue: summarize_dashboard_qa_queue(&audit_samples, &qa_review_records),
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
            Value,
            Value,
        )> = sqlx::query_as(
            "SELECT case_id, title, fwa_type, diagnosis_code, provider_region, provider_type, summary, outcome, tags, evidence_refs
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
             (case_id, title, fwa_type, diagnosis_code, provider_region, provider_type, summary, outcome, tags, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (case_id) DO UPDATE
             SET title = EXCLUDED.title,
                 fwa_type = EXCLUDED.fwa_type,
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

    async fn list_agent_runs(&self) -> anyhow::Result<Vec<AgentRunLogRecord>> {
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
             FROM agent_runs
             ORDER BY created_at DESC, agent_run_id",
        )
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
        sqlx::query(
            "INSERT INTO investigation_results
             (investigation_id, claim_id, outcome, confirmed_fwa, saving_amount, currency, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (investigation_id) DO UPDATE
             SET outcome = EXCLUDED.outcome,
                 confirmed_fwa = EXCLUDED.confirmed_fwa,
                 saving_amount = EXCLUDED.saving_amount,
                 currency = EXCLUDED.currency,
                 notes = EXCLUDED.notes,
                 evidence_refs = EXCLUDED.evidence_refs",
        )
        .bind(&record.investigation_id)
        .bind(&record.claim_id)
        .bind(&record.outcome)
        .bind(record.confirmed_fwa)
        .bind(record.saving_amount)
        .bind(&record.currency)
        .bind(&record.notes)
        .bind(serde_json::json!(record.evidence_refs))
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM saving_attributions WHERE investigation_id = $1")
            .bind(&record.investigation_id)
            .execute(&mut *tx)
            .await?;
        for attribution in saving_attributions {
            sqlx::query(
                "INSERT INTO saving_attributions
                 (attribution_id, claim_id, investigation_id, source_type, source_id, action, saving_amount, currency, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(&attribution.attribution_id)
            .bind(&attribution.claim_id)
            .bind(&attribution.investigation_id)
            .bind(&attribution.source_type)
            .bind(&attribution.source_id)
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
            event_type: "investigation.result.received".into(),
            event_status: "succeeded".into(),
            summary: format!("Investigation result received: {}", record.outcome),
            payload: serde_json::to_value(&record)?,
            evidence_refs: record.evidence_refs.clone(),
            created_at: None,
        };
        insert_pilot_audit_event(&mut tx, &record.claim_id, &event, "tpa_system").await?;
        tx.commit().await?;
        Ok(event)
    }

    async fn save_qa_review(
        &self,
        record: QaReviewRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO qa_reviews
             (qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (qa_case_id) DO UPDATE
             SET qa_conclusion = EXCLUDED.qa_conclusion,
                 issue_type = EXCLUDED.issue_type,
                 feedback_target = EXCLUDED.feedback_target,
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
            event_type: "qa.result.received".into(),
            event_status: "succeeded".into(),
            summary: format!("QA result received: {}", record.qa_conclusion),
            payload: serde_json::to_value(&record)?,
            evidence_refs: record.evidence_refs.clone(),
            created_at: None,
        };
        insert_pilot_audit_event(&mut tx, &record.claim_id, &event, "qa_reviewer").await?;
        tx.commit().await?;
        Ok(event)
    }

    async fn list_qa_feedback_items(&self) -> anyhow::Result<Vec<QaFeedbackItemRecord>> {
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, notes, evidence_refs, created_at
             FROM qa_reviews
             WHERE qa_conclusion <> 'pass'
             ORDER BY created_at, qa_case_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut items = rows
            .into_iter()
            .map(
                |(
                    qa_case_id,
                    claim_id,
                    qa_conclusion,
                    issue_type,
                    feedback_target,
                    notes,
                    evidence_refs,
                    created_at,
                )| {
                    qa_review_to_feedback_item(
                        QaReviewRecord {
                            qa_case_id,
                            claim_id,
                            qa_conclusion,
                            issue_type,
                            feedback_target,
                            notes,
                            evidence_refs: json_array_to_strings(evidence_refs),
                        },
                        Some(created_at.to_rfc3339()),
                    )
                },
            )
            .collect::<Vec<_>>();
        sort_qa_feedback_items(&mut items);
        Ok(items)
    }

    async fn list_qa_reviews(&self) -> anyhow::Result<Vec<QaReviewRecord>> {
        let rows: Vec<(String, String, String, String, String, String, Value)> = sqlx::query_as(
            "SELECT qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, notes, evidence_refs
             FROM qa_reviews
             ORDER BY qa_case_id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    qa_case_id,
                    claim_id,
                    qa_conclusion,
                    issue_type,
                    feedback_target,
                    notes,
                    evidence_refs,
                )| QaReviewRecord {
                    qa_case_id,
                    claim_id,
                    qa_conclusion,
                    issue_type,
                    feedback_target,
                    notes,
                    evidence_refs: json_array_to_strings(evidence_refs),
                },
            )
            .collect())
    }

    async fn list_outcome_labels(&self) -> anyhow::Result<Vec<OutcomeLabelRecord>> {
        let investigation_rows: Vec<(
            String,
            String,
            String,
            bool,
            Option<Decimal>,
            Option<String>,
            String,
            Value,
        )> = sqlx::query_as(
            "SELECT investigation_id, claim_id, outcome, confirmed_fwa, saving_amount, currency, notes, evidence_refs
             FROM investigation_results
             ORDER BY created_at, investigation_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let qa_rows: Vec<(String, String, String, String, String, String, Value)> =
            sqlx::query_as(
                "SELECT qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, notes, evidence_refs
                 FROM qa_reviews
                 ORDER BY created_at, qa_case_id",
            )
            .fetch_all(&self.pool)
            .await?;

        let mut labels = investigation_rows
            .into_iter()
            .flat_map(
                |(
                    investigation_id,
                    claim_id,
                    outcome,
                    confirmed_fwa,
                    saving_amount,
                    currency,
                    notes,
                    evidence_refs,
                )| {
                    labels_from_investigation_result(InvestigationResultRecord {
                        investigation_id,
                        claim_id,
                        outcome,
                        confirmed_fwa,
                        saving_amount,
                        currency,
                        notes,
                        evidence_refs: json_array_to_strings(evidence_refs),
                    })
                },
            )
            .chain(qa_rows.into_iter().map(
                |(
                    qa_case_id,
                    claim_id,
                    qa_conclusion,
                    issue_type,
                    feedback_target,
                    notes,
                    evidence_refs,
                )| {
                    label_from_qa_review(QaReviewRecord {
                        qa_case_id,
                        claim_id,
                        qa_conclusion,
                        issue_type,
                        feedback_target,
                        notes,
                        evidence_refs: json_array_to_strings(evidence_refs),
                    })
                },
            ))
            .collect::<Vec<_>>();
        sort_outcome_labels(&mut labels);
        Ok(labels)
    }

    async fn claim_audit_history(
        &self,
        claim_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        let rows: Vec<(String, String, String, String, String, Value, Value, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as(
                "SELECT ae.audit_id, ae.run_id, ae.event_type, ae.event_status, ae.summary, ae.payload, ae.evidence_refs, ae.created_at
                 FROM audit_events ae
                 LEFT JOIN claims c ON c.id = ae.claim_id
                 WHERE payload ->> 'claim_id' = $1 OR c.external_claim_id = $1
                 ORDER BY ae.created_at, ae.audit_id",
            )
            .bind(claim_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    audit_id,
                    run_id,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs,
                    created_at,
                )| AuditHistoryEventRecord {
                    audit_id,
                    run_id,
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
             (evaluation_run_id, model_key, model_version, model_dataset_id, auc, ks, precision_value, recall_value, f1, accuracy, threshold, confusion_matrix_json, feature_importance_uri, metrics_json)
             VALUES ($1, $2, $3, $4::uuid, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
             ON CONFLICT (evaluation_run_id) DO UPDATE
             SET model_key = EXCLUDED.model_key,
                 model_version = EXCLUDED.model_version,
                 model_dataset_id = EXCLUDED.model_dataset_id,
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
            "SELECT evaluation_run_id, model_key, model_version, model_dataset_id::text, auc, ks, precision_value, recall_value, f1, accuracy, threshold, confusion_matrix_json, feature_importance_uri, metrics_json
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
            "SELECT evaluation_run_id, model_key, model_version, model_dataset_id::text, auc, ks, precision_value, recall_value, f1, accuracy, threshold, confusion_matrix_json, feature_importance_uri, metrics_json
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

fn normalize_scheme_family(value: &str) -> String {
    match value {
        "duplicate_billing"
        | "upcoding"
        | "unbundling"
        | "medically_unnecessary_service"
        | "excessive_utilization"
        | "diagnosis_procedure_mismatch"
        | "laboratory_testing_abuse"
        | "telehealth_abuse"
        | "genetic_testing_abuse"
        | "pharmacy_controlled_substance_abuse"
        | "dme_home_health_hospice_rehab_risk"
        | "provider_peer_outlier"
        | "relationship_concentration"
        | "early_high_value_claim"
        | "high_risk_claim" => value.to_string(),
        _ => "high_risk_claim".into(),
    }
}

fn scheme_family_from_alert_code(alert_code: &str) -> String {
    let code = alert_code.to_ascii_uppercase();
    if code.contains("DUPLICATE") {
        "duplicate_billing".into()
    } else if code.contains("UPCOD") {
        "upcoding".into()
    } else if code.contains("UNBUND") {
        "unbundling".into()
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

fn case_from_lead(lead: &LeadRecord, input: &TriageLeadInput) -> CaseRecord {
    CaseRecord {
        case_id: format!("case_{}", lead.claim_id),
        lead_id: lead.lead_id.clone(),
        claim_id: lead.claim_id.clone(),
        member_id: lead.member_id.clone(),
        provider_id: lead.provider_id.clone(),
        source_system: lead.source_system.clone(),
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
            "risk_score": lead.risk_score,
            "rag": lead.rag.clone(),
            "reason": lead.reason.clone(),
            "triage_notes": input.notes.clone(),
            "evidence_refs": lead.evidence_refs.clone()
        }),
    }
}

fn build_audit_sample(
    sample_id: String,
    input: CreateAuditSampleInput,
    leads: Vec<LeadRecord>,
    created_at: Option<String>,
) -> AuditSampleRecord {
    let selection_method = selection_method_for_mode(&input.sample_mode).to_string();
    let mut candidates = leads
        .into_iter()
        .filter(|lead| lead_matches_inclusion(lead, &input.inclusion_criteria))
        .collect::<Vec<_>>();

    match selection_method.as_str() {
        "deterministic_hash" => {
            let seed = input
                .deterministic_seed
                .as_deref()
                .unwrap_or("default-seed");
            candidates.sort_by_key(|lead| deterministic_rank(seed, &lead.lead_id));
        }
        "scheme_family_then_risk_score" => candidates.sort_by(|left, right| {
            left.scheme_family
                .cmp(&right.scheme_family)
                .then_with(|| right.risk_score.cmp(&left.risk_score))
                .then_with(|| left.lead_id.cmp(&right.lead_id))
        }),
        _ => candidates.sort_by(|left, right| {
            right
                .risk_score
                .cmp(&left.risk_score)
                .then_with(|| left.lead_id.cmp(&right.lead_id))
        }),
    }

    let selected_leads = candidates
        .into_iter()
        .take(input.sample_size)
        .map(|lead| AuditSampleLeadRecord {
            lead_id: lead.lead_id,
            claim_id: lead.claim_id,
            scheme_family: lead.scheme_family,
            risk_score: lead.risk_score,
            rag: lead.rag,
            evidence_refs: lead.evidence_refs,
        })
        .collect::<Vec<_>>();

    AuditSampleRecord {
        sample_id,
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
    }
}

fn qa_review_to_feedback_item(
    review: QaReviewRecord,
    created_at: Option<String>,
) -> QaFeedbackItemRecord {
    let priority = if review.qa_conclusion.contains("escalate") {
        "high"
    } else if review.qa_conclusion.contains("return") {
        "medium"
    } else {
        "low"
    };
    QaFeedbackItemRecord {
        feedback_id: format!("qa_feedback_{}", review.qa_case_id),
        qa_case_id: review.qa_case_id.clone(),
        claim_id: review.claim_id.clone(),
        feedback_target: review.feedback_target.clone(),
        issue_type: review.issue_type.clone(),
        qa_conclusion: review.qa_conclusion.clone(),
        source: "qa_review".into(),
        status: "open".into(),
        priority: priority.into(),
        summary: format!(
            "QA {} flagged {} feedback for claim {}",
            review.qa_case_id, review.feedback_target, review.claim_id
        ),
        note_present: !review.notes.trim().is_empty(),
        evidence_refs: review.evidence_refs,
        created_at,
    }
}

fn sort_qa_feedback_items(items: &mut [QaFeedbackItemRecord]) {
    items.sort_by(|left, right| {
        feedback_target_rank(&left.feedback_target)
            .cmp(&feedback_target_rank(&right.feedback_target))
            .then_with(|| left.qa_case_id.cmp(&right.qa_case_id))
    });
}

fn feedback_target_rank(target: &str) -> u8 {
    match target {
        "rules" => 0,
        "models" => 1,
        _ => 2,
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
        feedback_target: "models".into(),
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
        labels.push(OutcomeLabelRecord {
            label_id: format!(
                "label_investigation_{}_amount_prevented",
                record.investigation_id
            ),
            claim_id: record.claim_id,
            label_name: "amount_prevented".into(),
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

fn label_from_qa_review(record: QaReviewRecord) -> OutcomeLabelRecord {
    OutcomeLabelRecord {
        label_id: format!("label_qa_{}_{}", record.qa_case_id, record.issue_type),
        claim_id: record.claim_id,
        label_name: record.issue_type,
        label_value: "true".into(),
        source_type: "qa_review".into(),
        source_id: record.qa_case_id,
        governance_status: "needs_review".into(),
        feedback_target: record.feedback_target,
        currency: None,
        evidence_refs: record.evidence_refs,
    }
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
            .filter(|label| label.feedback_target == "models")
            .count() as u32,
        workflow_feedback: labels
            .iter()
            .filter(|label| label.feedback_target == "workflow")
            .count() as u32,
    }
}

fn summarize_dashboard_qa_queue(
    samples: &[AuditSampleRecord],
    reviews: &[QaReviewRecord],
) -> DashboardQaQueueRecord {
    let reviewed_case_ids = reviews
        .iter()
        .map(|review| review.qa_case_id.as_str())
        .collect::<BTreeSet<_>>();
    let sampled_cases = samples
        .iter()
        .map(|sample| sample.selected_leads.len() as u32)
        .sum::<u32>();
    let reviewed_cases = samples
        .iter()
        .flat_map(|sample| {
            sample.selected_leads.iter().map(move |lead| {
                format!("qa_{}_{}", sample.sample_id.as_str(), lead.lead_id.as_str())
            })
        })
        .filter(|qa_case_id| reviewed_case_ids.contains(qa_case_id.as_str()))
        .count() as u32;

    DashboardQaQueueRecord {
        sampled_cases,
        open_cases: sampled_cases.saturating_sub(reviewed_cases),
        reviewed_cases,
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
    latest_claim_id: Option<String>,
    outlier_flags: BTreeSet<String>,
    evidence_refs: BTreeSet<String>,
}

fn summarize_provider_risk_profiles<'a>(
    payloads: impl Iterator<Item = &'a Value>,
) -> ProviderRiskSummaryRecord {
    let mut providers = BTreeMap::<String, ProviderRiskAccumulator>::new();

    for payload in payloads {
        let Some(profile) = payload.get("provider_profile") else {
            continue;
        };
        let Some(provider_id) = profile.get("provider_id").and_then(Value::as_str) else {
            continue;
        };
        let risk_score = profile
            .get("risk_score")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(100) as u8;
        let entry =
            providers
                .entry(provider_id.to_string())
                .or_insert_with(|| ProviderRiskAccumulator {
                    provider_id: provider_id.to_string(),
                    risk_tier: "low".into(),
                    review_route: "none".into(),
                    ..ProviderRiskAccumulator::default()
                });

        entry.claim_count += 1;
        entry.latest_claim_id = payload
            .get("claim_id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| entry.latest_claim_id.clone());
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
        }

        extend_string_set(&mut entry.outlier_flags, profile.get("outlier_flags"));
        extend_string_set(&mut entry.evidence_refs, profile.get("evidence_refs"));
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
            latest_claim_id: provider.latest_claim_id,
            outlier_flags: provider.outlier_flags.into_iter().collect(),
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

fn extend_string_set(target: &mut BTreeSet<String>, value: Option<&Value>) {
    if let Some(items) = value.and_then(Value::as_array) {
        target.extend(items.iter().filter_map(Value::as_str).map(str::to_string));
    }
}

fn selection_method_for_mode(sample_mode: &str) -> &'static str {
    match sample_mode {
        "random_control" => "deterministic_hash",
        "stratified" => "scheme_family_then_risk_score",
        "qa_calibration" => "reviewer_consistency_rotation",
        "post_payment_audit" => "risk_score_desc_post_payment",
        _ => "risk_score_desc",
    }
}

fn lead_matches_inclusion(lead: &LeadRecord, criteria: &Value) -> bool {
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
    true
}

fn deterministic_rank(seed: &str, lead_id: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed.hash(&mut hasher);
    lead_id.hash(&mut hasher);
    hasher.finish()
}

async fn load_leads(pool: &PgPool) -> anyhow::Result<Vec<LeadRecord>> {
    let rows: Vec<LeadRow> = sqlx::query_as(
        "SELECT lead_id, run_id, claim_id, member_id, provider_id, source_system, scheme_family, lead_source, status, disposition, risk_score, rag, reason, evidence_refs
         FROM fwa_leads
         ORDER BY created_at, lead_id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(lead_from_row).collect())
}

async fn load_lead_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    lead_id: &str,
) -> anyhow::Result<Option<LeadRecord>> {
    let row: Option<LeadRow> = sqlx::query_as(
        "SELECT lead_id, run_id, claim_id, member_id, provider_id, source_system, scheme_family, lead_source, status, disposition, risk_score, rag, reason, evidence_refs
         FROM fwa_leads
         WHERE lead_id = $1",
    )
    .bind(lead_id)
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

async fn load_cases(pool: &PgPool) -> anyhow::Result<Vec<CaseRecord>> {
    let rows: Vec<CaseRow> = sqlx::query_as(
        "SELECT case_id, lead_id, claim_id, member_id, provider_id, source_system, scheme_family, lead_source, status, assignee, reviewer, priority, routing_reason, evidence_package_json
         FROM investigation_cases
         ORDER BY created_at, case_id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(case_from_row).collect())
}

async fn load_case_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    case_id: &str,
) -> anyhow::Result<Option<CaseRecord>> {
    let row: Option<CaseRow> = sqlx::query_as(
        "SELECT case_id, lead_id, claim_id, member_id, provider_id, source_system, scheme_family, lead_source, status, assignee, reviewer, priority, routing_reason, evidence_package_json
         FROM investigation_cases
         WHERE case_id = $1",
    )
    .bind(case_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(case_from_row))
}

fn case_from_row(row: CaseRow) -> CaseRecord {
    let (
        case_id,
        lead_id,
        claim_id,
        member_id,
        provider_id,
        source_system,
        scheme_family,
        lead_source,
        status,
        assignee,
        reviewer,
        priority,
        routing_reason,
        evidence_package,
    ) = row;
    CaseRecord {
        case_id,
        lead_id,
        claim_id,
        member_id,
        provider_id,
        source_system,
        scheme_family,
        lead_source,
        status,
        assignee,
        reviewer,
        priority,
        routing_reason,
        evidence_package,
    }
}

pub fn default_runtime_rules() -> Vec<Rule> {
    vec![
        Rule {
            rule_id: "rule_early_claim".into(),
            version: 1,
            name: "Early claim".into(),
            conditions: vec![Condition {
                field: "days_since_policy_start".into(),
                operator: "<=".into(),
                value: serde_json::json!(7),
            }],
            action: RuleAction {
                score: 75,
                alert_code: "EARLY_CLAIM".into(),
                recommended_action: RecommendedAction::ManualReview,
                reason: "保单生效后 7 天内发生理赔".into(),
            },
        },
        Rule {
            rule_id: "rule_early_high_amount".into(),
            version: 1,
            name: "Early high amount".into(),
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
                reason: "保单生效早期发生高额理赔".into(),
            },
        },
        Rule {
            rule_id: "rule_high_cost_single_item".into(),
            version: 1,
            name: "High cost single item".into(),
            conditions: vec![Condition {
                field: "high_cost_item_ratio".into(),
                operator: ">=".into(),
                value: serde_json::json!(0.5),
            }],
            action: RuleAction {
                score: 25,
                alert_code: "HIGH_COST_SINGLE_ITEM".into(),
                recommended_action: RecommendedAction::ManualReview,
                reason: "单个高价项目占理赔金额比例偏高".into(),
            },
        },
        Rule {
            rule_id: "rule_large_limit_usage".into(),
            version: 1,
            name: "Large limit usage".into(),
            conditions: vec![Condition {
                field: "claim_amount_to_limit_ratio".into(),
                operator: ">=".into(),
                value: serde_json::json!(0.8),
            }],
            action: RuleAction {
                score: 35,
                alert_code: "LARGE_LIMIT_USAGE".into(),
                recommended_action: RecommendedAction::ManualReview,
                reason: "理赔金额接近保障额度".into(),
            },
        },
        Rule {
            rule_id: "rule_low_medical_match".into(),
            version: 1,
            name: "Low medical match".into(),
            conditions: vec![Condition {
                field: "diagnosis_procedure_match_score".into(),
                operator: "<=".into(),
                value: serde_json::json!(0.4),
            }],
            action: RuleAction {
                score: 30,
                alert_code: "LOW_MEDICAL_MATCH".into(),
                recommended_action: RecommendedAction::ManualReview,
                reason: "诊断与项目匹配度偏低".into(),
            },
        },
        Rule {
            rule_id: "rule_many_claim_items".into(),
            version: 1,
            name: "Many claim items".into(),
            conditions: vec![Condition {
                field: "claim_item_count".into(),
                operator: ">=".into(),
                value: serde_json::json!(5),
            }],
            action: RuleAction {
                score: 20,
                alert_code: "MANY_CLAIM_ITEMS".into(),
                recommended_action: RecommendedAction::ManualReview,
                reason: "理赔明细项目数量偏多".into(),
            },
        },
        Rule {
            rule_id: "rule_peer_p95_amount".into(),
            version: 1,
            name: "Peer P95 amount".into(),
            conditions: vec![Condition {
                field: "claim_amount_peer_percentile".into(),
                operator: ">=".into(),
                value: serde_json::json!(95),
            }],
            action: RuleAction {
                score: 25,
                alert_code: "PEER_P95_AMOUNT".into(),
                recommended_action: RecommendedAction::ManualReview,
                reason: "理赔金额高于同类样本 P95".into(),
            },
        },
        Rule {
            rule_id: "rule_peer_p99_amount".into(),
            version: 1,
            name: "Peer P99 amount".into(),
            conditions: vec![Condition {
                field: "claim_amount_peer_percentile".into(),
                operator: ">=".into(),
                value: serde_json::json!(99),
            }],
            action: RuleAction {
                score: 40,
                alert_code: "PEER_P99_AMOUNT".into(),
                recommended_action: RecommendedAction::ManualReview,
                reason: "理赔金额高于同类样本 P99".into(),
            },
        },
        Rule {
            rule_id: "rule_provider_high_risk_tier".into(),
            version: 1,
            name: "Provider high risk tier".into(),
            conditions: vec![Condition {
                field: "provider_risk_tier".into(),
                operator: "==".into(),
                value: serde_json::json!("HIGH"),
            }],
            action: RuleAction {
                score: 30,
                alert_code: "PROVIDER_HIGH_RISK_TIER".into(),
                recommended_action: RecommendedAction::ManualReview,
                reason: "Provider 风险等级较高".into(),
            },
        },
        Rule {
            rule_id: "rule_provider_profile_high".into(),
            version: 1,
            name: "Provider profile high".into(),
            conditions: vec![Condition {
                field: "provider_profile_score".into(),
                operator: ">=".into(),
                value: serde_json::json!(70),
            }],
            action: RuleAction {
                score: 30,
                alert_code: "PROVIDER_PROFILE_HIGH".into(),
                recommended_action: RecommendedAction::ManualReview,
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
    let review_mode = "both".to_string();
    let scheme_family = scheme_family_from_alert_code(&rule.action.alert_code);
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
        "AutoApprove" => RecommendedAction::AutoApprove,
        "EscalateInvestigation" => RecommendedAction::EscalateInvestigation,
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
               status = EXCLUDED.status,
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

async fn ensure_default_knowledge_cases_seeded(pool: &PgPool) -> anyhow::Result<()> {
    for case in default_knowledge_cases() {
        sqlx::query(
            "INSERT INTO knowledge_cases
             (case_id, title, fwa_type, diagnosis_code, provider_region, provider_type, summary, outcome, tags, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (case_id) DO UPDATE SET updated_at = now()",
        )
        .bind(&case.case_id)
        .bind(&case.title)
        .bind(&case.fwa_type)
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
    if let Some(source_id) = reference.strip_prefix("model_scores:") {
        return Some(("model".into(), source_id.to_string()));
    }
    None
}

fn summarize_saving_attributions(
    records: &[SavingAttributionRecord],
) -> Vec<DashboardSavingAttributionRecord> {
    let mut accumulators = BTreeMap::<(String, String, String, String), (Decimal, u32)>::new();
    for record in records {
        let key = (
            record.source_type.clone(),
            record.source_id.clone(),
            record.action.clone(),
            record.currency.clone(),
        );
        let entry = accumulators.entry(key).or_insert((Decimal::ZERO, 0));
        entry.0 += record.saving_amount;
        entry.1 += 1;
    }

    accumulators
        .into_iter()
        .map(
            |((source_type, source_id, action, currency), (saving_amount, claim_count))| {
                DashboardSavingAttributionRecord {
                    source_type,
                    source_id,
                    action,
                    saving_amount: format_decimal_cents(saving_amount),
                    currency,
                    claim_count,
                }
            },
        )
        .collect()
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
    actor_role: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO scoring_runs
         (run_id, source_system, actor_id, status, completed_at)
         VALUES ($1, 'pilot-loop', $2, 'succeeded', now())
         ON CONFLICT (run_id) DO NOTHING",
    )
    .bind(&event.run_id)
    .bind(actor_role)
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
    .bind(actor_role)
    .bind(&event.event_type)
    .bind(&event.event_status)
    .bind(&event.summary)
    .bind(&event.payload)
    .bind(serde_json::json!(event.evidence_refs))
    .execute(&mut **tx)
    .await?;
    Ok(())
}
