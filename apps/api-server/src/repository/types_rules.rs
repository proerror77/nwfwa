use super::types::AuditHistoryEventRecord;
use fwa_core::RecommendedAction;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub submitted_by_actor_id: Option<String>,
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
pub struct RuleShadowRunRecord {
    pub rule_id: String,
    pub rule_version: u32,
    pub report_uri: String,
    pub decision: String,
    pub reviewer: String,
    pub notes: String,
    pub reviewed_count: u32,
    pub matched_count: u32,
    pub false_positive_count: u32,
    pub false_positive_rate: f64,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConditionLibraryRecord {
    pub condition_key: String,
    pub source_rule_key: String,
    pub source_rule_version: u32,
    pub condition_index: u32,
    pub field: String,
    pub operator: String,
    pub value: Value,
    pub review_mode: String,
    pub scheme_family: String,
    pub status: String,
    pub owner: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
