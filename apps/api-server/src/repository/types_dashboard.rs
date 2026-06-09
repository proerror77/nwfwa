use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
