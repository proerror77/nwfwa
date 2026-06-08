use serde::Deserialize;
use serde_json::Value;

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
