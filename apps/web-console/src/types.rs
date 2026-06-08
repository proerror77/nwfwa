mod data_sources;
mod dashboard;
mod evidence;
mod factor;
mod models;
mod runtime;
mod routing;

pub(crate) use data_sources::*;
pub(crate) use dashboard::*;
pub(crate) use evidence::*;
pub(crate) use factor::*;
pub(crate) use models::*;
pub(crate) use runtime::*;
pub(crate) use routing::*;

use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CorrectionHint {
    pub(crate) field_path: String,
    pub(crate) severity: String,
    pub(crate) blocks_scoring: bool,
    pub(crate) next_action: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct MemberProfileSummary {
    pub(crate) member_id: String,
    pub(crate) claim_count: u32,
    pub(crate) policy_count: u32,
    pub(crate) total_claim_amount: Value,
    pub(crate) currency: String,
    pub(crate) high_risk_claim_count: u32,
    pub(crate) latest_claim_id: Option<String>,
    pub(crate) risk_level_summary: String,
    pub(crate) profile_summary: String,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ProviderRiskSummary {
    pub(crate) provider_count: u32,
    pub(crate) review_required_count: u32,
    pub(crate) high_risk_count: u32,
    pub(crate) providers: Vec<ProviderRiskItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ProviderRiskItem {
    pub(crate) provider_id: String,
    pub(crate) risk_score: u8,
    pub(crate) risk_tier: String,
    pub(crate) review_required: bool,
    pub(crate) review_route: String,
    pub(crate) claim_count: u32,
    pub(crate) specialty: Option<String>,
    pub(crate) network_status: Option<String>,
    pub(crate) review_failure_count: u32,
    pub(crate) confirmed_fwa_count: u32,
    pub(crate) false_positive_count: u32,
    pub(crate) network_risk_score: Option<u8>,
    pub(crate) latest_claim_id: Option<String>,
    pub(crate) outlier_flags: Vec<String>,
    pub(crate) graph_reasons: Vec<String>,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AuditSampleListResponse {
    pub(crate) samples: Vec<AuditSampleRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AuditSampleRecord {
    pub(crate) sample_id: String,
    pub(crate) sample_mode: String,
    pub(crate) population_definition: String,
    pub(crate) inclusion_criteria: Value,
    pub(crate) deterministic_seed: Option<String>,
    pub(crate) selection_method: String,
    pub(crate) sample_size: usize,
    pub(crate) reviewer: String,
    pub(crate) assignment_queue: String,
    pub(crate) selected_leads: Vec<AuditSampleLead>,
    pub(crate) outcome_distribution: Value,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AuditSampleLead {
    pub(crate) lead_id: String,
    pub(crate) claim_id: String,
    pub(crate) scheme_family: String,
    pub(crate) review_mode: String,
    pub(crate) provider_id: String,
    pub(crate) provider_type: String,
    pub(crate) provider_region: String,
    pub(crate) policy_type: String,
    pub(crate) risk_band: String,
    pub(crate) strata_key: String,
    pub(crate) prior_reviewer_sample_count: u32,
    pub(crate) risk_score: u8,
    pub(crate) rag: String,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct QaQueueListResponse {
    pub(crate) items: Vec<QaQueueItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct QaQueueItem {
    pub(crate) qa_case_id: String,
    pub(crate) sample_id: String,
    pub(crate) lead_id: String,
    pub(crate) claim_id: String,
    pub(crate) scheme_family: String,
    pub(crate) rag: String,
    pub(crate) risk_score: u8,
    pub(crate) reviewer: String,
    pub(crate) assignment_queue: String,
    pub(crate) status: String,
    pub(crate) qa_conclusion: Option<String>,
    pub(crate) issue_type: Option<String>,
    pub(crate) feedback_target: Option<String>,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) canonical_source_refs: Vec<String>,
    pub(crate) canonical_evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct QaQueueSummary {
    pub(crate) open_count: u32,
    pub(crate) in_progress_count: u32,
    pub(crate) resolved_count: u32,
    pub(crate) dismissed_count: u32,
    pub(crate) unresolved_count: u32,
    pub(crate) rules_feedback_count: u32,
    pub(crate) models_feedback_count: u32,
    pub(crate) features_feedback_count: u32,
    pub(crate) provider_profile_feedback_count: u32,
    pub(crate) workflow_feedback_count: u32,
    pub(crate) tpa_feedback_count: u32,
    pub(crate) high_priority_count: u32,
    pub(crate) evidence_backed_count: u32,
    pub(crate) highest_priority: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct QaFeedbackItemListResponse {
    pub(crate) items: Vec<QaFeedbackItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct QaFeedbackItem {
    pub(crate) feedback_id: String,
    pub(crate) qa_case_id: String,
    pub(crate) claim_id: String,
    pub(crate) feedback_target: String,
    pub(crate) issue_type: String,
    pub(crate) qa_conclusion: String,
    pub(crate) source: String,
    pub(crate) status: String,
    pub(crate) priority: String,
    pub(crate) summary: String,
    pub(crate) note_present: bool,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
    pub(crate) status_updated_by: Option<String>,
    pub(crate) status_audit_id: Option<String>,
    pub(crate) status_updated_at: Option<String>,
    pub(crate) status_evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct QaReviewSnapshot {
    pub(crate) queue: Vec<QaQueueItem>,
    pub(crate) summary: QaQueueSummary,
    pub(crate) feedback_items: Vec<QaFeedbackItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct HistoricalBackfillListResponse {
    pub(crate) jobs: Vec<HistoricalBackfillJob>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct HistoricalBackfillResponse {
    pub(crate) job: HistoricalBackfillJob,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct HistoricalBackfillJob {
    pub(crate) job_id: String,
    pub(crate) status: String,
    pub(crate) dataset_refs: Vec<String>,
    pub(crate) rule_refs: Vec<String>,
    pub(crate) candidate_count: u32,
    pub(crate) leads: Vec<HistoricalBackfillLead>,
    pub(crate) reviewer: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct HistoricalBackfillLead {
    pub(crate) lead_id: String,
    pub(crate) claim_id: String,
    pub(crate) scheme_family: String,
    pub(crate) risk_score: u8,
    pub(crate) rag: String,
    pub(crate) status: String,
    pub(crate) reason: String,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceRequestListResponse {
    pub(crate) requests: Vec<EvidenceRequestRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceRequestGenerateResponse {
    pub(crate) requests: Vec<EvidenceRequestRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceRequestRecord {
    pub(crate) request_id: String,
    pub(crate) claim_id: String,
    pub(crate) scoring_audit_id: String,
    pub(crate) status: String,
    pub(crate) request_reason: String,
    pub(crate) missing_evidence: Vec<String>,
    pub(crate) items: Vec<EvidenceRequestItem>,
    pub(crate) reviewer_queue: String,
    pub(crate) requested_by: String,
    pub(crate) notes: Option<String>,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
    pub(crate) updated_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceRequestItem {
    pub(crate) item_id: String,
    pub(crate) document_type: String,
    pub(crate) status: String,
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) blocking: bool,
    pub(crate) policy_authority_ref: Option<String>,
    pub(crate) exception_check: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct LabelBootstrapQueueResponse {
    pub(crate) items: Vec<LabelBootstrapItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct LabelBootstrapReviewResponse {
    pub(crate) item: LabelBootstrapItem,
    pub(crate) audit_id: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct LabelBootstrapItem {
    pub(crate) item_id: String,
    pub(crate) claim_id: String,
    pub(crate) source_type: String,
    pub(crate) source_id: String,
    pub(crate) suggested_label_name: String,
    pub(crate) suggested_label_value: String,
    pub(crate) governance_status: String,
    pub(crate) training_eligible: bool,
    pub(crate) review_status: String,
    pub(crate) review_audit_id: Option<String>,
    pub(crate) reviewer: Option<String>,
    pub(crate) feedback_target: String,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BootstrapOpsSnapshot {
    pub(crate) backfills: Vec<HistoricalBackfillJob>,
    pub(crate) evidence_requests: Vec<EvidenceRequestRecord>,
    pub(crate) label_items: Vec<LabelBootstrapItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct KnowledgeCaseListResponse {
    pub(crate) cases: Vec<KnowledgeCase>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct KnowledgeCase {
    pub(crate) case_id: String,
    pub(crate) title: String,
    pub(crate) fwa_type: String,
    pub(crate) scheme_family: String,
    pub(crate) diagnosis_code: String,
    pub(crate) provider_region: String,
    pub(crate) provider_type: String,
    pub(crate) summary: String,
    pub(crate) outcome: String,
    pub(crate) tags: Vec<String>,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct SimilarCaseSearchResponse {
    pub(crate) results: Vec<SimilarCase>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct SimilarCase {
    pub(crate) case_id: String,
    pub(crate) title: String,
    pub(crate) scheme_family: String,
    pub(crate) similarity_score: f64,
    pub(crate) matched_signals: Vec<String>,
    pub(crate) retrieval_method: String,
    pub(crate) provenance_refs: Vec<String>,
    pub(crate) summary: String,
    pub(crate) outcome: String,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct KnowledgeSnapshot {
    pub(crate) cases: Vec<KnowledgeCase>,
    pub(crate) results: Vec<SimilarCase>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AuditEventListResponse {
    pub(crate) events: Vec<AuditEventRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AuditEventRecord {
    pub(crate) audit_id: String,
    pub(crate) run_id: String,
    pub(crate) event_type: String,
    pub(crate) event_status: String,
    pub(crate) summary: String,
    pub(crate) payload: Value,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ApiCallListResponse {
    pub(crate) calls: Vec<ApiCallRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ApiCallRecord {
    pub(crate) call_id: String,
    pub(crate) endpoint: String,
    pub(crate) method: String,
    pub(crate) status_code: u16,
    pub(crate) result: String,
    pub(crate) source_system: String,
    pub(crate) claim_id: String,
    pub(crate) run_id: String,
    pub(crate) audit_id: String,
    pub(crate) event_type: String,
    pub(crate) idempotency_key: Option<String>,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) observed_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentRunListResponse {
    pub(crate) runs: Vec<AgentRunRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentRunRecord {
    pub(crate) agent_run_id: String,
    pub(crate) claim_id: String,
    pub(crate) status: String,
    pub(crate) decision_boundary: String,
    pub(crate) output_json: Value,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) steps: Vec<Value>,
    pub(crate) context_snapshots: Vec<Value>,
    pub(crate) policy_checks: Vec<Value>,
    pub(crate) tool_calls: Vec<Value>,
    pub(crate) tool_results: Vec<Value>,
    pub(crate) approvals: Vec<AgentApprovalView>,
    pub(crate) created_at: Option<String>,
    pub(crate) completed_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentApprovalView {
    pub(crate) approval_id: String,
    pub(crate) proposed_action: String,
    pub(crate) decision: String,
    pub(crate) approver: String,
    pub(crate) reason: String,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentInvestigationResponse {
    pub(crate) agent_run_id: String,
    pub(crate) decision_boundary: String,
    pub(crate) risk_summary: String,
    pub(crate) findings: Vec<AgentInvestigationFinding>,
    pub(crate) investigation_checklist: Vec<String>,
    pub(crate) similar_cases: Vec<AgentInvestigationSimilarCase>,
    pub(crate) qa_opinion_draft: String,
    pub(crate) evidence_sufficiency: AgentEvidenceSufficiency,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) evidence_refs_by_type: AgentEvidenceBuckets,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentInvestigationFinding {
    pub(crate) finding: String,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentInvestigationSimilarCase {
    pub(crate) case_id: String,
    pub(crate) similarity_score: f64,
    pub(crate) matched_signals: Vec<String>,
    pub(crate) provenance_refs: Vec<String>,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentEvidenceSufficiency {
    pub(crate) scheme_family: String,
    pub(crate) status: String,
    pub(crate) minimum_evidence: Vec<String>,
    pub(crate) present_evidence: Vec<String>,
    pub(crate) missing_evidence: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentEvidenceBuckets {
    pub(crate) claim: Vec<String>,
    pub(crate) rule: Vec<String>,
    pub(crate) model: Vec<String>,
    pub(crate) anomaly: Vec<String>,
    pub(crate) document: Vec<String>,
    pub(crate) similar_case: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GovernanceSnapshot {
    pub(crate) health: HealthResponse,
    pub(crate) audit_events: Vec<AuditEventRecord>,
    pub(crate) api_calls: Vec<ApiCallRecord>,
    pub(crate) agent_runs: Vec<AgentRunRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct HealthResponse {
    pub(crate) status: String,
    pub(crate) service: String,
    pub(crate) version: String,
    pub(crate) pilot_readiness: PilotReadiness,
    pub(crate) checks: Vec<HealthCheck>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct PilotReadiness {
    pub(crate) status: String,
    pub(crate) ready_for_customer_pilot: bool,
    pub(crate) required_check_names: Vec<String>,
    pub(crate) required_check_count: usize,
    pub(crate) ready_check_count: usize,
    pub(crate) blocking_check_count: usize,
    pub(crate) blocking_check_names: Vec<String>,
    pub(crate) remediation_summary: Vec<String>,
    pub(crate) ready_checks: Vec<HealthCheck>,
    pub(crate) blocking_checks: Vec<HealthCheck>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct HealthCheck {
    pub(crate) name: String,
    pub(crate) status: String,
    pub(crate) runtime_kind: Option<String>,
    pub(crate) remediation: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RuleListResponse {
    pub(crate) rules: Vec<RuleSummary>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RuleSummary {
    pub(crate) rule_id: String,
    pub(crate) name: String,
    pub(crate) status: String,
    pub(crate) owner: String,
    pub(crate) active_version: Option<u32>,
    pub(crate) latest_version: u32,
    pub(crate) review_mode: String,
    pub(crate) scheme_family: String,
    pub(crate) score: u8,
    pub(crate) alert_code: String,
    pub(crate) recommended_action: String,
    pub(crate) applicability_scope: RuleApplicabilityScope,
    pub(crate) backtest_result: RuleBacktestSummary,
    pub(crate) estimated_saving: String,
    pub(crate) false_positive_history: RuleFalsePositiveHistory,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RuleApplicabilityScope {
    pub(crate) review_mode: String,
    pub(crate) scheme_family: String,
    pub(crate) source: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RuleBacktestSummary {
    pub(crate) status: String,
    pub(crate) sample_count: u32,
    pub(crate) matched_count: u32,
    pub(crate) precision: f64,
    pub(crate) recall: f64,
    pub(crate) lift: f64,
    pub(crate) false_positive_rate: f64,
    pub(crate) estimated_saving: String,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RuleFalsePositiveHistory {
    pub(crate) status: String,
    pub(crate) false_positive_count: u32,
    pub(crate) false_positive_rate: f64,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RulePerformanceResponse {
    pub(crate) rules: Vec<RulePerformance>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RulePerformance {
    pub(crate) rule_id: String,
    pub(crate) alert_code: String,
    pub(crate) trigger_count: u32,
    pub(crate) reviewed_count: u32,
    pub(crate) confirmed_fwa_count: u32,
    pub(crate) false_positive_count: u32,
    pub(crate) mark_rate: f64,
    pub(crate) precision: f64,
    pub(crate) false_positive_rate: f64,
    pub(crate) saving_amount: String,
    pub(crate) roi: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RulePromotionGates {
    pub(crate) rule_id: String,
    pub(crate) rule_version: u32,
    pub(crate) review_mode: String,
    pub(crate) decision: String,
    pub(crate) status: String,
    pub(crate) passed_count: usize,
    pub(crate) total_count: usize,
    pub(crate) trigger_count: u32,
    pub(crate) reviewed_count: u32,
    pub(crate) false_positive_rate: f64,
    pub(crate) saving_amount: String,
    pub(crate) open_rule_feedback_count: usize,
    pub(crate) unresolved_rule_feedback_count: usize,
    pub(crate) approved_label_count: usize,
    pub(crate) needs_review_label_count: usize,
    pub(crate) gates: Vec<RulePromotionGate>,
    pub(crate) blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RulePromotionGate {
    pub(crate) label: String,
    pub(crate) passed: bool,
    pub(crate) blocker: String,
    pub(crate) evidence_source: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RuleOpsSnapshot {
    pub(crate) rules: Vec<RuleSummary>,
    pub(crate) performance: Vec<RulePerformance>,
    pub(crate) gates: RulePromotionGates,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RuleDiscoveryResponse {
    pub(crate) sample_count: usize,
    pub(crate) positive_count: usize,
    pub(crate) candidates: Vec<RuleDiscoveryCandidate>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RuleDiscoveryCandidate {
    pub(crate) rule: Value,
    pub(crate) support: usize,
    pub(crate) precision: f64,
    pub(crate) recall: f64,
    pub(crate) lift: f64,
    pub(crate) estimated_saving: String,
    pub(crate) false_positive_rate: f64,
    pub(crate) matched_claim_ids: Vec<String>,
    pub(crate) explanation: String,
    #[serde(default)]
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RuleBacktestResponse {
    pub(crate) sample_count: usize,
    pub(crate) matched_count: usize,
    pub(crate) reviewed_count: usize,
    pub(crate) confirmed_fwa_count: usize,
    pub(crate) false_positive_count: usize,
    pub(crate) match_rate: f64,
    pub(crate) precision: f64,
    pub(crate) recall: f64,
    pub(crate) lift: f64,
    pub(crate) false_positive_rate: f64,
    pub(crate) average_score_contribution: f64,
    pub(crate) estimated_saving: String,
    pub(crate) promotion_recommendation: String,
    pub(crate) blockers: Vec<String>,
    pub(crate) matched_claim_ids: Vec<String>,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct LeadListResponse {
    pub(crate) leads: Vec<LeadRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct LeadRecord {
    pub(crate) lead_id: String,
    pub(crate) run_id: String,
    pub(crate) claim_id: String,
    pub(crate) member_id: String,
    pub(crate) provider_id: String,
    pub(crate) source_system: String,
    pub(crate) review_mode: String,
    pub(crate) scheme_family: String,
    pub(crate) lead_source: String,
    pub(crate) status: String,
    pub(crate) disposition: String,
    pub(crate) risk_score: u8,
    pub(crate) rag: String,
    pub(crate) reason: String,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct CaseListResponse {
    pub(crate) cases: Vec<CaseRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct CaseRecord {
    pub(crate) case_id: String,
    pub(crate) lead_id: String,
    pub(crate) claim_id: String,
    pub(crate) member_id: String,
    pub(crate) provider_id: String,
    pub(crate) source_system: String,
    pub(crate) review_mode: String,
    pub(crate) scheme_family: String,
    pub(crate) lead_source: String,
    pub(crate) status: String,
    pub(crate) assignee: String,
    pub(crate) reviewer: String,
    pub(crate) priority: String,
    pub(crate) routing_reason: String,
    pub(crate) evidence_package: Value,
    pub(crate) sla_target_hours: u32,
    pub(crate) sla_status: String,
    pub(crate) time_to_triage_hours: f64,
    pub(crate) time_to_closure_hours: Option<f64>,
    pub(crate) final_outcome: Option<String>,
    pub(crate) reviewer_notes: Option<String>,
    pub(crate) investigation_result_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct TriageLeadRecord {
    pub(crate) lead: LeadRecord,
    pub(crate) case: Option<CaseRecord>,
    pub(crate) audit_id: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct UpdateCaseStatusRecord {
    pub(crate) case: CaseRecord,
    pub(crate) audit_id: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct PilotWritebackResponse {
    pub(crate) claim_id: String,
    pub(crate) event_type: String,
    pub(crate) event_status: String,
    pub(crate) audit_id: String,
    pub(crate) run_id: String,
    pub(crate) idempotency_key: String,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LeadsCasesSnapshot {
    pub(crate) leads: Vec<LeadRecord>,
    pub(crate) cases: Vec<CaseRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct MedicalReviewQueueResponse {
    pub(crate) items: Vec<MedicalReviewQueueItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct MedicalReviewQueueItem {
    pub(crate) claim_id: String,
    pub(crate) run_id: String,
    pub(crate) audit_id: String,
    pub(crate) medical_reasonableness_score: u8,
    pub(crate) review_route: String,
    pub(crate) evidence_status: String,
    pub(crate) missing_evidence: Vec<String>,
    pub(crate) item_finding_count: u32,
    pub(crate) first_item_code: Option<String>,
    pub(crate) first_issue_type: Option<String>,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) canonical_source_refs: Vec<String>,
    pub(crate) canonical_evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
    pub(crate) review_status: String,
    pub(crate) review_audit_id: Option<String>,
    pub(crate) review_decision: Option<String>,
    pub(crate) reviewer: Option<String>,
    pub(crate) reviewed_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct MedicalReviewResultResponse {
    pub(crate) claim_id: String,
    pub(crate) event_type: String,
    pub(crate) event_status: String,
    pub(crate) audit_id: String,
    pub(crate) run_id: String,
    pub(crate) review_status: String,
    pub(crate) clinical_outcomes: Vec<String>,
    pub(crate) evidence_refs: Vec<String>,
}
