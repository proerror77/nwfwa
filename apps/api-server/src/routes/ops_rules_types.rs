use crate::repository::RuleConditionLibraryRecord;
use chrono::NaiveDate;
use fwa_rules::Rule;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct RuleListResponse {
    pub rules: Vec<crate::repository::RuleSummaryRecord>,
}

#[derive(Debug, Serialize)]
pub struct RuleConditionLibraryResponse {
    pub conditions: Vec<RuleConditionLibraryRecord>,
}

#[derive(Debug, Serialize)]
pub struct RulePerformanceResponse {
    pub rules: Vec<crate::repository::RulePerformanceRecord>,
}

#[derive(Debug, Serialize)]
pub struct RulePromotionGate {
    pub label: String,
    pub passed: bool,
    pub blocker: String,
    pub evidence_source: String,
}

#[derive(Debug, Serialize)]
pub struct RulePromotionGatesResponse {
    pub rule_id: String,
    pub rule_version: u32,
    pub review_mode: String,
    pub decision: String,
    pub status: String,
    pub passed_count: usize,
    pub total_count: usize,
    pub trigger_count: u32,
    pub reviewed_count: u32,
    pub false_positive_rate: f64,
    pub saving_amount: String,
    pub open_rule_feedback_count: usize,
    pub unresolved_rule_feedback_count: usize,
    pub approved_label_count: usize,
    pub needs_review_label_count: usize,
    pub gates: Vec<RulePromotionGate>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitRulePromotionReviewRequest {
    pub decision: String,
    pub reviewer: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitRuleShadowRunRequest {
    pub rule_version: u32,
    pub reviewed_count: u32,
    pub matched_count: u32,
    pub false_positive_count: u32,
    pub false_positive_rate: f64,
    pub report_uri: String,
    pub decision: String,
    pub reviewer: String,
    pub notes: String,
    #[serde(default)]
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RuleLifecycleRequest {
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RuleBacktestRequest {
    pub rule: Rule,
    #[serde(default)]
    pub samples: Vec<RuleBacktestSample>,
    pub expected_review_capacity: Option<usize>,
    pub dataset_uri: Option<String>,
    pub label_column: Option<String>,
    pub claim_id_column: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RuleBacktestSample {
    pub external_claim_id: String,
    pub claim_amount: Decimal,
    pub currency: String,
    pub service_date: NaiveDate,
    pub confirmed_fwa: Option<bool>,
    pub policy: RuleBacktestPolicy,
}

#[derive(Debug, Deserialize)]
pub struct RuleBacktestPolicy {
    pub external_policy_id: String,
    pub coverage_start_date: NaiveDate,
    pub coverage_end_date: NaiveDate,
    pub coverage_limit: Decimal,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleBacktestResponse {
    pub sample_count: usize,
    pub matched_count: usize,
    pub reviewed_count: usize,
    pub confirmed_fwa_count: usize,
    pub false_positive_count: usize,
    pub match_rate: f64,
    pub precision: f64,
    pub recall: f64,
    pub lift: f64,
    pub false_positive_rate: f64,
    pub average_score_contribution: f64,
    pub estimated_saving: String,
    pub promotion_recommendation: String,
    pub blockers: Vec<String>,
    pub matched_claim_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RuleDiscoveryRequest {
    pub min_support: Option<usize>,
    #[serde(default)]
    pub samples: Vec<RuleDiscoverySample>,
    #[serde(default)]
    pub model_explanations: Vec<RuleDiscoveryModelExplanation>,
    pub source_model_key: Option<String>,
    pub source_model_version: Option<String>,
    pub feature_importance_uri: Option<String>,
    pub min_abs_contribution: Option<f64>,
    pub dataset_uri: Option<String>,
    pub label_column: Option<String>,
    pub claim_id_column: Option<String>,
    pub candidate_feature_fields: Option<Vec<String>>,
    pub max_candidates: Option<usize>,
    pub max_tree_depth: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct RuleDiscoverySample {
    #[serde(flatten)]
    pub sample: RuleBacktestSample,
    pub confirmed_fwa: bool,
}

#[derive(Debug, Deserialize)]
pub struct RuleDiscoveryModelExplanation {
    pub feature: String,
    pub direction: String,
    pub contribution: f64,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ReviewRuleCandidateRequest {
    pub rule: Rule,
    pub decision: String,
    pub reviewer: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ReviewRuleCandidateResponse {
    pub rule_id: String,
    pub decision: String,
    pub entered_rule_library: bool,
    pub accepted_for_governance_review: bool,
    pub saved_draft_rule_id: Option<String>,
    pub active_rule_writeback: bool,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RuleDiscoveryResponse {
    pub sample_count: usize,
    pub positive_count: usize,
    pub candidates: Vec<RuleDiscoveryCandidate>,
}

#[derive(Debug, Serialize)]
pub struct RuleDiscoveryCandidate {
    pub rule: Rule,
    pub support: usize,
    pub precision: f64,
    pub recall: f64,
    pub lift: f64,
    pub estimated_saving: String,
    pub false_positive_rate: f64,
    pub matched_claim_ids: Vec<String>,
    pub explanation: String,
    pub condition_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveRuleCandidateRequest {
    pub rule: Rule,
    pub owner: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RuleLifecycleResponse {
    pub rule_id: String,
    pub status: String,
    pub active_version: Option<u32>,
    pub latest_version: u32,
}
