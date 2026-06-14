use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DashboardSummary {
    pub(crate) suspected_claims: u32,
    pub(crate) confirmed_fwa: u32,
    pub(crate) risk_amount: String,
    pub(crate) saving_amount: String,
    pub(crate) rag_distribution: BTreeMap<String, u32>,
    pub(crate) scheme_distribution: BTreeMap<String, u32>,
    pub(crate) rule_hits: u32,
    pub(crate) model_scores: BTreeMap<String, DashboardModelScore>,
    pub(crate) layer_scores: BTreeMap<String, DashboardLayerScore>,
    pub(crate) saving_attributions: Vec<DashboardSavingAttribution>,
    pub(crate) saving_segments: Vec<DashboardSavingSegment>,
    pub(crate) value_measurement: DashboardValueMeasurement,
    pub(crate) audit_coverage: DashboardAuditCoverage,
    pub(crate) label_pool: DashboardLabelPool,
    pub(crate) qa_queue: DashboardQaQueue,
    pub(crate) case_sla: DashboardCaseSla,
    pub(crate) agent_governance: DashboardAgentGovernance,
    pub(crate) model_governance: DashboardModelGovernance,
    pub(crate) rule_governance: DashboardRuleGovernance,
    pub(crate) investigation_results: u32,
    pub(crate) qa_reviews: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DashboardModelScore {
    pub(crate) scored_runs: u32,
    pub(crate) average_score: f64,
    pub(crate) high_risk_count: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DashboardLayerScore {
    pub(crate) name: String,
    pub(crate) scored_runs: u32,
    pub(crate) average_score: f64,
    pub(crate) high_risk_count: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DashboardSavingAttribution {
    pub(crate) source_type: String,
    pub(crate) source_id: String,
    pub(crate) financial_impact_type: String,
    pub(crate) action: String,
    pub(crate) saving_amount: String,
    pub(crate) currency: String,
    pub(crate) claim_count: u32,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DashboardSavingSegment {
    pub(crate) segment_type: String,
    pub(crate) segment_id: String,
    pub(crate) saving_amount: String,
    pub(crate) currency: String,
    pub(crate) claim_count: u32,
    pub(crate) attribution_count: u32,
    pub(crate) roi: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DashboardValueMeasurement {
    pub(crate) prevented_payment: String,
    pub(crate) recovered_amount: String,
    pub(crate) avoided_future_exposure: String,
    pub(crate) deterrence_estimate: String,
    pub(crate) estimated_impact: String,
    pub(crate) review_cost: String,
    pub(crate) false_positive_operational_cost: String,
    pub(crate) reviewer_capacity_hours: String,
    pub(crate) net_value: String,
    pub(crate) currency: String,
    pub(crate) evidence_caveat: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DashboardAuditCoverage {
    pub(crate) scoring_runs: u32,
    pub(crate) canonical_trace_runs: u32,
    pub(crate) canonical_trace_coverage: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DashboardLabelPool {
    pub(crate) total_labels: u32,
    pub(crate) approved_for_training: u32,
    pub(crate) needs_review: u32,
    pub(crate) rule_feedback: u32,
    pub(crate) model_feedback: u32,
    pub(crate) features_feedback: u32,
    pub(crate) provider_profile_feedback: u32,
    pub(crate) workflow_feedback: u32,
    pub(crate) case_status_labels: u32,
    pub(crate) medical_review_labels: u32,
    pub(crate) false_positive_labels: u32,
    pub(crate) evidence_backed_labels: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DashboardQaQueue {
    pub(crate) sampled_cases: u32,
    pub(crate) open_cases: u32,
    pub(crate) reviewed_cases: u32,
    pub(crate) disagreement_cases: u32,
    pub(crate) disagreement_rate: f64,
    pub(crate) feedback_open_count: u32,
    pub(crate) feedback_in_progress_count: u32,
    pub(crate) feedback_resolved_count: u32,
    pub(crate) feedback_dismissed_count: u32,
    pub(crate) unresolved_feedback_count: u32,
    pub(crate) rules_unresolved_feedback_count: u32,
    pub(crate) models_unresolved_feedback_count: u32,
    pub(crate) features_unresolved_feedback_count: u32,
    pub(crate) provider_profile_unresolved_feedback_count: u32,
    pub(crate) workflow_unresolved_feedback_count: u32,
    pub(crate) tpa_unresolved_feedback_count: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DashboardCaseSla {
    pub(crate) total_cases: u32,
    pub(crate) open_cases: u32,
    pub(crate) closed_cases: u32,
    pub(crate) breached_cases: u32,
    pub(crate) sla_breach_rate: f64,
    pub(crate) average_time_to_triage_hours: f64,
    pub(crate) average_time_to_closure_hours: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DashboardAgentGovernance {
    pub(crate) total_runs: u32,
    pub(crate) successful_runs: u32,
    pub(crate) evidence_backed_runs: u32,
    pub(crate) tool_call_count: u32,
    pub(crate) policy_check_count: u32,
    pub(crate) denied_policy_check_count: u32,
    pub(crate) failed_tool_call_count: u32,
    pub(crate) pending_approvals: u32,
    pub(crate) approved_approvals: u32,
    pub(crate) rejected_approvals: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DashboardModelGovernance {
    pub(crate) total_models: u32,
    pub(crate) evaluated_models: u32,
    pub(crate) drift_watch_count: u32,
    pub(crate) drift_detected_count: u32,
    pub(crate) average_precision: Option<f64>,
    pub(crate) average_recall: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct DashboardRuleGovernance {
    pub(crate) total_rules: u32,
    pub(crate) active_rules: u32,
    pub(crate) triggered_rules: u32,
    pub(crate) total_trigger_count: u32,
    pub(crate) reviewed_count: u32,
    pub(crate) confirmed_fwa_count: u32,
    pub(crate) false_positive_count: u32,
    pub(crate) precision: f64,
    pub(crate) false_positive_rate: f64,
    pub(crate) saving_amount: String,
    pub(crate) roi: f64,
}
