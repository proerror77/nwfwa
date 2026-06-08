use crate::{
    app::AppState,
    auth::{AuthenticatedActor, AuthenticatedApiPrincipal},
    error::ApiError,
    repository::{
        PersistedAuditEvent, QaFeedbackItemRecord, RuleBacktestRecord, RuleConditionLibraryRecord,
        RulePerformanceRecord, RulePromotionReviewRecord, RuleShadowRunRecord, RuleSummaryRecord,
    },
    routes::pii,
};
use anyhow::{bail, Context};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use fwa_audit::ActorContext;
use fwa_auth::AuthenticatedPrincipal;
use fwa_core::{
    canonical_scheme_family, AuditEventId, Claim, ClaimContext, ClaimId, Member, MemberId, Money,
    Policy, PolicyId, Provider, ProviderId, ProviderRiskTier, RecommendedAction, RuleActionClass,
    ScoringRunId,
};
use fwa_features::{calculate_features, FeatureMap, FeatureValue};
use fwa_rules::{evaluate_rules, Condition, Rule, RuleAction};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::File, path::PathBuf};

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

impl RuleBacktestRecord {
    fn from_response(rule_id: &str, rule_version: u32, response: &RuleBacktestResponse) -> Self {
        Self {
            rule_id: rule_id.into(),
            rule_version,
            sample_count: response.sample_count as u32,
            matched_count: response.matched_count as u32,
            reviewed_count: response.reviewed_count as u32,
            confirmed_fwa_count: response.confirmed_fwa_count as u32,
            false_positive_count: response.false_positive_count as u32,
            precision: response.precision,
            recall: response.recall,
            lift: response.lift,
            false_positive_rate: response.false_positive_rate,
            estimated_saving: response.estimated_saving.clone(),
            promotion_recommendation: response.promotion_recommendation.clone(),
            blockers: response.blockers.clone(),
            evidence_refs: response.evidence_refs.clone(),
            created_at: None,
        }
    }
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

#[derive(Debug, PartialEq)]
struct CandidateReviewOutcome {
    accepted_for_governance_review: bool,
    saved_draft_rule_id: Option<String>,
    active_rule_writeback: bool,
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

pub async fn list_rules(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
) -> Result<Json<RuleListResponse>, ApiError> {
    let rules = state
        .repository
        .list_rules()
        .await
        .map_err(internal_error("RULE_LIST_FAILED"))?;
    Ok(Json(RuleListResponse { rules }))
}

pub async fn rule_performance(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
) -> Result<Json<RulePerformanceResponse>, ApiError> {
    let rules = state
        .repository
        .rule_performance()
        .await
        .map_err(internal_error("RULE_PERFORMANCE_FAILED"))?;
    Ok(Json(RulePerformanceResponse { rules }))
}

pub async fn list_rule_conditions(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
) -> Result<Json<RuleConditionLibraryResponse>, ApiError> {
    let conditions = state
        .repository
        .list_rule_conditions()
        .await
        .map_err(internal_error("RULE_CONDITION_LIST_FAILED"))?;
    Ok(Json(RuleConditionLibraryResponse { conditions }))
}

pub async fn rule_promotion_gates(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
    Path(rule_id): Path<String>,
) -> Result<Json<RulePromotionGatesResponse>, ApiError> {
    Ok(Json(load_rule_promotion_gates(&state, &rule_id).await?))
}

async fn load_rule_promotion_gates(
    state: &AppState,
    rule_id: &str,
) -> Result<RulePromotionGatesResponse, ApiError> {
    let detail = state
        .repository
        .get_rule(rule_id)
        .await
        .map_err(internal_error("RULE_LOAD_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?;
    let latest_action = latest_rule_action(&detail);
    let rule = detail.summary;
    let performance = state
        .repository
        .rule_performance()
        .await
        .map_err(internal_error("RULE_PERFORMANCE_FAILED"))?
        .into_iter()
        .find(|record| record.rule_id == rule_id)
        .unwrap_or_else(|| empty_rule_performance(&rule));
    let latest_review = state
        .repository
        .latest_rule_promotion_review(&rule.rule_id, rule.latest_version)
        .await
        .map_err(internal_error("RULE_PROMOTION_REVIEW_LOAD_FAILED"))?;
    let latest_backtest = state
        .repository
        .latest_rule_backtest(&rule.rule_id, rule.latest_version)
        .await
        .map_err(internal_error("RULE_BACKTEST_LOAD_FAILED"))?;
    let latest_shadow_run = state
        .repository
        .latest_rule_shadow_run(&rule.rule_id, rule.latest_version)
        .await
        .map_err(internal_error("RULE_SHADOW_RUN_LOAD_FAILED"))?;
    let outcome_labels = state
        .repository
        .list_outcome_labels(None)
        .await
        .map_err(internal_error("OUTCOME_LABEL_LIST_FAILED"))?;
    let feedback_items = state
        .repository
        .list_qa_feedback_items(None)
        .await
        .map_err(internal_error("QA_FEEDBACK_LIST_FAILED"))?;

    Ok(build_rule_promotion_gates(
        &rule,
        &performance,
        latest_backtest.as_ref(),
        latest_shadow_run.as_ref(),
        &outcome_labels,
        &feedback_items,
        latest_review.as_ref(),
        latest_action.as_ref(),
    ))
}

pub async fn submit_rule_promotion_review(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(rule_id): Path<String>,
    Json(request): Json<SubmitRulePromotionReviewRequest>,
) -> Result<Json<RulePromotionReviewRecord>, ApiError> {
    let actor = require_permission(principal, "ops:rules:review")?;
    if !matches!(request.decision.as_str(), "approved" | "rejected") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROMOTION_DECISION",
            "decision must be approved or rejected",
        ));
    }
    if request.reviewer.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REVIEWER",
            "reviewer is required",
        ));
    }
    if request.notes.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROMOTION_REVIEW_NOTES",
            "promotion review notes are required",
        ));
    }
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_PROMOTION_REVIEW_EVIDENCE",
            "promotion review evidence_refs are required",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.notes.as_str())
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_PROMOTION_REVIEW",
            "promotion review notes and evidence_refs must not contain PII",
        ));
    }
    let rule = state
        .repository
        .get_rule(&rule_id)
        .await
        .map_err(internal_error("RULE_LOAD_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?
        .summary;
    let review = state
        .repository
        .save_rule_promotion_review(RulePromotionReviewRecord {
            rule_id: rule.rule_id.clone(),
            rule_version: rule.latest_version,
            decision: request.decision,
            reviewer: request.reviewer,
            notes: request.notes,
            evidence_refs: request.evidence_refs,
            created_at: None,
        })
        .await
        .map_err(internal_error("RULE_PROMOTION_REVIEW_SAVE_FAILED"))?;
    record_rule_promotion_audit(&state, &actor, &review)
        .await
        .map_err(internal_error("RULE_PROMOTION_AUDIT_SAVE_FAILED"))?;
    Ok(Json(review))
}

pub async fn submit_rule_shadow_run(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(rule_id): Path<String>,
    Json(request): Json<SubmitRuleShadowRunRequest>,
) -> Result<Json<RuleShadowRunRecord>, ApiError> {
    let actor = require_permission(principal, "ops:rules:review")?;
    validate_rule_shadow_run_request(&rule_id, &request)?;
    let rule = state
        .repository
        .get_rule(&rule_id)
        .await
        .map_err(internal_error("RULE_LOAD_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?
        .summary;
    if request.rule_version != rule.latest_version {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RULE_SHADOW_VERSION_MISMATCH",
            "shadow run rule_version must match the latest rule version",
        ));
    }
    let record = state
        .repository
        .save_rule_shadow_run(RuleShadowRunRecord {
            rule_id: rule.rule_id.clone(),
            rule_version: request.rule_version,
            report_uri: request.report_uri,
            decision: request.decision,
            reviewer: request.reviewer,
            notes: request.notes,
            reviewed_count: request.reviewed_count,
            matched_count: request.matched_count,
            false_positive_count: request.false_positive_count,
            false_positive_rate: request.false_positive_rate,
            blockers: request.blockers,
            evidence_refs: request.evidence_refs,
            created_at: None,
        })
        .await
        .map_err(internal_error("RULE_SHADOW_RUN_SAVE_FAILED"))?;
    record_rule_shadow_run_audit(&state, &actor, &record)
        .await
        .map_err(internal_error("RULE_SHADOW_RUN_AUDIT_SAVE_FAILED"))?;
    Ok(Json(record))
}

pub async fn get_rule(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
    Path(rule_id): Path<String>,
) -> Result<Json<crate::repository::RuleDetailRecord>, ApiError> {
    let rule = state
        .repository
        .get_rule(&rule_id)
        .await
        .map_err(internal_error("RULE_LOAD_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?;
    Ok(Json(rule))
}

fn build_rule_promotion_gates(
    rule: &RuleSummaryRecord,
    performance: &RulePerformanceRecord,
    latest_backtest: Option<&RuleBacktestRecord>,
    latest_shadow_run: Option<&RuleShadowRunRecord>,
    outcome_labels: &[crate::repository::OutcomeLabelRecord],
    feedback_items: &[QaFeedbackItemRecord],
    latest_review: Option<&RulePromotionReviewRecord>,
    latest_action: Option<&RuleAction>,
) -> RulePromotionGatesResponse {
    let effective_reviewed_count = performance.reviewed_count.max(
        latest_backtest
            .map(|backtest| backtest.reviewed_count)
            .unwrap_or(0),
    );
    let effective_false_positive_rate = if performance.reviewed_count > 0 {
        performance.false_positive_rate
    } else {
        latest_backtest
            .map(|backtest| backtest.false_positive_rate)
            .unwrap_or(0.0)
    };
    let performance_saving = decimal_from_string(&performance.saving_amount);
    let backtest_saving = latest_backtest
        .map(|backtest| decimal_from_string(&backtest.estimated_saving))
        .unwrap_or(Decimal::ZERO);
    let effective_saving = if performance_saving > Decimal::ZERO {
        performance_saving
    } else {
        backtest_saving
    };
    let has_review_evidence = effective_reviewed_count > 0;
    let backtest_blockers_clear = latest_backtest
        .map(|backtest| backtest.blockers.is_empty())
        .unwrap_or(true);
    let has_saving_evidence = effective_saving > Decimal::ZERO;
    let review_evidence_source = review_evidence_source(performance, latest_backtest);
    let saving_evidence_source = saving_evidence_source(performance, latest_backtest);
    let approved = latest_review
        .map(|review| review.decision == "approved")
        .unwrap_or_else(|| matches!(rule.status.as_str(), "approved" | "active"));
    let passed_shadow_run =
        latest_shadow_run.filter(|run| run.decision == "shadow_passed" && run.blockers.is_empty());
    let runtime_shadow_rollout = performance.trigger_count > 0 && performance.reviewed_count > 0;
    let shadow_rollout = passed_shadow_run.is_some() || runtime_shadow_rollout;
    let shadow_evidence_source = if passed_shadow_run.is_some() {
        "shadow"
    } else if runtime_shadow_rollout {
        "runtime"
    } else {
        "missing"
    };
    let rule_feedback_items = feedback_items
        .iter()
        .filter(|item| {
            item.feedback_target == "rules" && feedback_targets_rule(&item.evidence_refs, rule)
        })
        .collect::<Vec<_>>();
    let open_rule_feedback_count = rule_feedback_items
        .iter()
        .filter(|item| item.status == "open")
        .count();
    let unresolved_rule_feedback_count = rule_feedback_items
        .iter()
        .filter(|item| is_unresolved_feedback_status(&item.status))
        .count();
    let rule_feedback_labels = outcome_labels
        .iter()
        .filter(|label| {
            label.feedback_target == "rules" && feedback_targets_rule(&label.evidence_refs, rule)
        })
        .collect::<Vec<_>>();
    let approved_rule_feedback = rule_feedback_labels
        .iter()
        .filter(|label| label.governance_status == "approved_for_training")
        .count();
    let needs_review_rule_feedback = rule_feedback_labels
        .iter()
        .filter(|label| label.governance_status == "needs_review")
        .count();
    let unresolved_rule_feedback = rule_feedback_labels
        .iter()
        .any(|label| label.governance_status == "needs_review");
    let rule_feedback_governance = !unresolved_rule_feedback;
    let mut gates = vec![
        rule_gate(
            "Named owner",
            !rule.owner.trim().is_empty(),
            "owner missing",
            if rule.owner.trim().is_empty() {
                "missing"
            } else {
                "metadata"
            },
        ),
        rule_gate(
            "Deterministic backtest evidence",
            has_review_evidence && backtest_blockers_clear,
            backtest_evidence_blocker(has_review_evidence, backtest_blockers_clear),
            review_evidence_source,
        ),
        rule_gate(
            "Estimated saving",
            has_saving_evidence,
            "estimated saving missing",
            saving_evidence_source,
        ),
        rule_gate(
            "False-positive burden",
            has_review_evidence && effective_false_positive_rate <= 0.30,
            "false-positive burden missing",
            if has_review_evidence && effective_false_positive_rate <= 0.30 {
                review_evidence_source
            } else {
                "missing"
            },
        ),
        rule_gate(
            "Approval before routing",
            approved,
            "approval missing",
            if approved { "approval" } else { "missing" },
        ),
        rule_gate(
            "Rule QA feedback closure",
            unresolved_rule_feedback_count == 0,
            "unresolved rule QA feedback",
            "qa_feedback",
        ),
        rule_gate(
            "Rule feedback governance",
            rule_feedback_governance,
            "rule feedback labels need review",
            if rule_feedback_labels.is_empty() {
                "missing"
            } else {
                "labels"
            },
        ),
        rule_gate(
            "Shadow or limited rollout",
            shadow_rollout,
            "shadow rollout missing",
            shadow_evidence_source,
        ),
        rule_gate(
            "Rollback path",
            rule.latest_version > 0,
            "rollback path missing",
            if rule.latest_version > 0 {
                "metadata"
            } else {
                "missing"
            },
        ),
    ];
    if let Some(action) = latest_action.filter(|action| deterministic_adjudication_action(action)) {
        gates.extend(adjudication_policy_gates(action, shadow_rollout));
    }
    let blockers = gates
        .iter()
        .filter(|gate| !gate.passed)
        .map(|gate| gate.blocker.clone())
        .collect::<Vec<_>>();

    RulePromotionGatesResponse {
        rule_id: rule.rule_id.clone(),
        rule_version: rule.latest_version,
        review_mode: rule.review_mode.clone(),
        decision: if blockers.is_empty() {
            "routing_allowed".into()
        } else {
            "routing_blocked".into()
        },
        status: rule.status.clone(),
        passed_count: gates.len() - blockers.len(),
        total_count: gates.len(),
        trigger_count: performance.trigger_count,
        reviewed_count: effective_reviewed_count,
        false_positive_rate: effective_false_positive_rate,
        saving_amount: format!("{:.2}", effective_saving.round_dp(2)),
        open_rule_feedback_count,
        unresolved_rule_feedback_count,
        approved_label_count: approved_rule_feedback,
        needs_review_label_count: needs_review_rule_feedback,
        gates,
        blockers,
    }
}

fn latest_rule_action(detail: &crate::repository::RuleDetailRecord) -> Option<RuleAction> {
    detail
        .versions
        .iter()
        .find(|version| version.version == detail.summary.latest_version)
        .and_then(|version| serde_json::from_value(version.dsl["action"].clone()).ok())
}

fn deterministic_adjudication_action(action: &RuleAction) -> bool {
    matches!(
        action.action_class,
        RuleActionClass::HardDeny | RuleActionClass::StraightThrough
    )
}

fn adjudication_policy_gates(action: &RuleAction, shadow_rollout: bool) -> Vec<RulePromotionGate> {
    let policy = action.adjudication_policy.as_ref();
    let has_customer_approval = policy
        .map(|policy| non_empty(&policy.customer_approval_ref))
        .unwrap_or(false);
    let has_authority_and_exception = action.required_evidence.iter().any(|evidence| {
        evidence
            .policy_authority_ref
            .as_deref()
            .is_some_and(non_empty)
            && evidence.exception_check.as_deref().is_some_and(non_empty)
    });
    let has_appeal_or_override = policy
        .map(|policy| non_empty(&policy.appeal_or_override_route))
        .unwrap_or(false);
    let has_effective_date_and_rollback = policy
        .map(|policy| non_empty(&policy.effective_date) && non_empty(&policy.rollback_plan_ref))
        .unwrap_or(false);
    let has_production_threshold = policy
        .map(|policy| non_empty(&policy.production_threshold_ref))
        .unwrap_or(false);
    let has_routing_impact = policy
        .map(|policy| non_empty(&policy.routing_impact_ref))
        .unwrap_or(false)
        && shadow_rollout;
    vec![
        rule_gate(
            "Customer-approved adjudication rule list",
            has_customer_approval,
            "customer-approved rule list missing",
            if has_customer_approval {
                "approval"
            } else {
                "missing"
            },
        ),
        rule_gate(
            "Policy authority and exception check",
            has_authority_and_exception,
            "policy authority or exception check missing",
            if has_authority_and_exception {
                "metadata"
            } else {
                "missing"
            },
        ),
        rule_gate(
            "Appeal or override route",
            has_appeal_or_override,
            "appeal or override route missing",
            if has_appeal_or_override {
                "metadata"
            } else {
                "missing"
            },
        ),
        rule_gate(
            "Effective date and rollback plan",
            has_effective_date_and_rollback,
            "effective date or rollback plan missing",
            if has_effective_date_and_rollback {
                "metadata"
            } else {
                "missing"
            },
        ),
        rule_gate(
            "Production thresholds",
            has_production_threshold,
            "production thresholds missing",
            if has_production_threshold {
                "metadata"
            } else {
                "missing"
            },
        ),
        rule_gate(
            "Routing impact promotion",
            has_routing_impact,
            "routing impact evidence missing",
            if has_routing_impact {
                "runtime"
            } else {
                "missing"
            },
        ),
    ]
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn decimal_from_string(value: &str) -> Decimal {
    value.parse::<Decimal>().unwrap_or(Decimal::ZERO)
}

fn is_unresolved_feedback_status(status: &str) -> bool {
    matches!(status, "open" | "in_progress")
}

fn review_evidence_source(
    performance: &RulePerformanceRecord,
    latest_backtest: Option<&RuleBacktestRecord>,
) -> &'static str {
    if performance.reviewed_count > 0 {
        "runtime"
    } else if latest_backtest
        .map(|backtest| backtest.reviewed_count > 0)
        .unwrap_or(false)
    {
        "backtest"
    } else {
        "missing"
    }
}

fn saving_evidence_source(
    performance: &RulePerformanceRecord,
    latest_backtest: Option<&RuleBacktestRecord>,
) -> &'static str {
    if decimal_from_string(&performance.saving_amount) > Decimal::ZERO {
        "runtime"
    } else if latest_backtest
        .map(|backtest| decimal_from_string(&backtest.estimated_saving) > Decimal::ZERO)
        .unwrap_or(false)
    {
        "backtest"
    } else {
        "missing"
    }
}

fn backtest_evidence_blocker(
    has_review_evidence: bool,
    backtest_blockers_clear: bool,
) -> &'static str {
    if !has_review_evidence {
        "backtest evidence missing"
    } else if !backtest_blockers_clear {
        "backtest blockers unresolved"
    } else {
        "none"
    }
}

fn rule_gate(label: &str, passed: bool, blocker: &str, evidence_source: &str) -> RulePromotionGate {
    RulePromotionGate {
        label: label.into(),
        passed,
        blocker: blocker.into(),
        evidence_source: evidence_source.into(),
    }
}

fn feedback_targets_rule(evidence_refs: &[String], rule: &RuleSummaryRecord) -> bool {
    let rule_run_ref = format!("rule_runs:{}", rule.alert_code);
    evidence_refs.iter().any(|reference| {
        reference == &rule_run_ref
            || reference
                .strip_prefix("rules:")
                .and_then(|source_id| source_id.split(":v").next())
                == Some(rule.rule_id.as_str())
    })
}

fn empty_rule_performance(rule: &RuleSummaryRecord) -> RulePerformanceRecord {
    RulePerformanceRecord {
        rule_id: rule.rule_id.clone(),
        alert_code: rule.alert_code.clone(),
        trigger_count: 0,
        reviewed_count: 0,
        confirmed_fwa_count: 0,
        false_positive_count: 0,
        mark_rate: 0.0,
        precision: 0.0,
        false_positive_rate: 0.0,
        saving_amount: "0.00".into(),
        roi: 0.0,
    }
}

pub async fn backtest_rule(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Json(request): Json<RuleBacktestRequest>,
) -> Result<Json<RuleBacktestResponse>, ApiError> {
    let mining_samples =
        backtest_mining_samples(&request).map_err(bad_request("RULE_BACKTEST_DATASET_FAILED"))?;
    let mut matched_claim_ids = Vec::new();
    let mut score_sum = 0_u32;
    let mut saving = Decimal::ZERO;
    let mut true_positive_count = 0_usize;
    let mut false_positive_count = 0_usize;
    let positive_count = mining_samples
        .iter()
        .filter(|sample| sample.confirmed_fwa == Some(true))
        .count();
    let reviewed_count = mining_samples
        .iter()
        .filter(|sample| sample.confirmed_fwa.is_some())
        .count();
    let labeled_backtest = reviewed_count > 0;

    for sample in &mining_samples {
        let features = feature_map_from_mining_sample(sample);
        let matches = evaluate_rules(std::slice::from_ref(&request.rule), &features)
            .map_err(internal_error("RULE_BACKTEST_FAILED"))?;
        if !matches.is_empty() {
            matched_claim_ids.push(sample.claim_id.clone());
            score_sum += matches
                .iter()
                .map(|rule_match| rule_match.score_contribution as u32)
                .sum::<u32>();
            match sample.confirmed_fwa {
                Some(true) => {
                    true_positive_count += 1;
                    saving += sample.claim_amount * Decimal::new(10, 2);
                }
                Some(false) => {
                    false_positive_count += 1;
                }
                None => {
                    saving += sample.claim_amount * Decimal::new(10, 2);
                }
            }
        }
    }

    let sample_count = mining_samples.len();
    let matched_count = matched_claim_ids.len();
    let match_rate = if sample_count == 0 {
        0.0
    } else {
        matched_count as f64 / sample_count as f64
    };
    let average_score_contribution = if matched_count == 0 {
        0.0
    } else {
        score_sum as f64 / matched_count as f64
    };
    let precision = if !labeled_backtest || matched_count == 0 {
        0.0
    } else {
        true_positive_count as f64 / matched_count as f64
    };
    let recall = if !labeled_backtest || positive_count == 0 {
        0.0
    } else {
        true_positive_count as f64 / positive_count as f64
    };
    let false_positive_rate = if !labeled_backtest || matched_count == 0 {
        0.0
    } else {
        false_positive_count as f64 / matched_count as f64
    };
    let baseline_rate = if sample_count == 0 {
        0.0
    } else {
        positive_count as f64 / sample_count as f64
    };
    let lift = if !labeled_backtest || baseline_rate == 0.0 {
        0.0
    } else {
        precision / baseline_rate
    };
    let mut blockers = Vec::new();
    if !labeled_backtest {
        blockers.push("labeled outcomes missing".into());
    }
    if reviewed_count < 2 {
        blockers.push("reviewed sample count below 2".into());
    }
    if precision < 0.70 {
        blockers.push("precision below 0.70".into());
    }
    if recall < 0.60 {
        blockers.push("recall below 0.60".into());
    }
    if false_positive_rate > 0.30 {
        blockers.push("false-positive rate above 0.30".into());
    }
    if request
        .expected_review_capacity
        .map(|capacity| matched_count > capacity)
        .unwrap_or(false)
    {
        blockers.push("review capacity exceeded".into());
    }
    let promotion_recommendation = if blockers.is_empty() {
        "eligible_for_review"
    } else {
        "needs_more_evidence"
    };

    let response = RuleBacktestResponse {
        sample_count,
        matched_count,
        reviewed_count,
        confirmed_fwa_count: positive_count,
        false_positive_count,
        match_rate,
        precision,
        recall,
        lift,
        false_positive_rate,
        average_score_contribution,
        estimated_saving: format!("{:.2}", saving.round_dp(2)),
        promotion_recommendation: promotion_recommendation.into(),
        blockers,
        matched_claim_ids,
        evidence_refs: backtest_evidence_refs(&request),
    };
    let record = state
        .repository
        .save_rule_backtest(RuleBacktestRecord::from_response(
            &request.rule.rule_id,
            request.rule.version,
            &response,
        ))
        .await
        .map_err(internal_error("RULE_BACKTEST_SAVE_FAILED"))?;
    record_rule_backtest_audit(&state, &actor, &record)
        .await
        .map_err(internal_error("RULE_BACKTEST_AUDIT_SAVE_FAILED"))?;

    Ok(Json(response))
}

fn backtest_evidence_refs(request: &RuleBacktestRequest) -> Vec<String> {
    let mut refs = vec![format!(
        "rules:{}:v{}",
        request.rule.rule_id, request.rule.version
    )];
    refs.push(format!(
        "rule_backtests:{}:v{}",
        request.rule.rule_id, request.rule.version
    ));
    if let Some(dataset_uri) = normalized_optional_str(request.dataset_uri.as_deref()) {
        refs.push(format!("dataset:{dataset_uri}"));
    }
    refs
}

pub async fn discover_rules(
    State(_state): State<AppState>,
    _actor: AuthenticatedActor,
    Json(request): Json<RuleDiscoveryRequest>,
) -> Result<Json<RuleDiscoveryResponse>, ApiError> {
    let min_support = request.min_support.unwrap_or(1);
    let mining_samples =
        discovery_mining_samples(&request).map_err(bad_request("RULE_DISCOVERY_DATASET_FAILED"))?;
    let sample_count = mining_samples.len();
    let positive_count = mining_samples
        .iter()
        .filter(|sample| sample.confirmed_fwa == Some(true))
        .count();
    if sample_count == 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RULE_DISCOVERY_EMPTY_DATASET",
            "rule discovery requires labeled samples or a parquet dataset_uri",
        ));
    }
    if positive_count == 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RULE_DISCOVERY_NO_POSITIVE_LABELS",
            "rule discovery requires at least one positive label",
        ));
    }
    let baseline_rate = if sample_count == 0 {
        0.0
    } else {
        positive_count as f64 / sample_count as f64
    };

    let discovery_evidence_refs = rule_discovery_evidence_refs(&request);
    let mut candidates = mine_statistical_rule_candidates(
        &request,
        &mining_samples,
        min_support,
        positive_count,
        baseline_rate,
        &discovery_evidence_refs,
    );
    candidates.truncate(request.max_candidates.unwrap_or(8));

    Ok(Json(RuleDiscoveryResponse {
        sample_count,
        positive_count,
        candidates,
    }))
}

pub async fn save_rule_candidate(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Json(mut request): Json<SaveRuleCandidateRequest>,
) -> Result<Json<crate::repository::RuleDetailRecord>, ApiError> {
    request.rule.scheme_family = Some(validate_rule_candidate(&request.rule)?);
    let owner = request.owner.unwrap_or_else(|| "rule-discovery".into());
    let detail = state
        .repository
        .save_rule_candidate(request.rule, owner)
        .await
        .map_err(internal_error("RULE_CANDIDATE_SAVE_FAILED"))?;
    record_rule_audit(
        &state,
        &actor,
        RuleAuditInput {
            rule: &detail.summary,
            event_type: "rule.candidate.saved",
            from_status: None,
            to_status: &detail.summary.status,
            summary: "Rule candidate saved",
            evidence_refs: default_rule_evidence_refs(&detail.summary),
        },
    )
    .await
    .map_err(internal_error("RULE_AUDIT_SAVE_FAILED"))?;
    Ok(Json(detail))
}

pub async fn review_rule_candidate(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Json(request): Json<ReviewRuleCandidateRequest>,
) -> Result<Json<ReviewRuleCandidateResponse>, ApiError> {
    let actor = require_permission(principal, "ops:rules:review")?;
    if !matches!(request.decision.as_str(), "accepted" | "rejected") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_CANDIDATE_REVIEW_DECISION",
            "decision must be accepted or rejected",
        ));
    }
    if request.reviewer.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REVIEWER",
            "reviewer is required",
        ));
    }
    if request.notes.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_CANDIDATE_REVIEW_NOTES",
            "candidate review notes are required",
        ));
    }
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_CANDIDATE_REVIEW_EVIDENCE",
            "candidate review evidence_refs are required",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.notes.as_str())
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_CANDIDATE_REVIEW",
            "candidate review notes and evidence_refs must not contain PII",
        ));
    }

    validate_candidate_review_backtest_evidence(&request.decision, &request.evidence_refs)?;
    let mut saved_draft_rule_id = None;
    if request.decision == "accepted" {
        let backtest = state
            .repository
            .latest_rule_backtest(&request.rule.rule_id, request.rule.version)
            .await
            .map_err(internal_error("RULE_CANDIDATE_BACKTEST_LOAD_FAILED"))?;
        let Some(backtest) = backtest else {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "RULE_CANDIDATE_BACKTEST_REQUIRED",
                "accepted rule candidates require a completed backtest",
            ));
        };
        if !backtest.blockers.is_empty() {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "RULE_CANDIDATE_BACKTEST_BLOCKED",
                format!(
                    "accepted rule candidate backtest blockers must be resolved: {}",
                    backtest.blockers.join(", ")
                ),
            ));
        }
        let shadow_run = state
            .repository
            .latest_rule_shadow_run(&request.rule.rule_id, request.rule.version)
            .await
            .map_err(internal_error("RULE_CANDIDATE_SHADOW_LOAD_FAILED"))?;
        let Some(shadow_run) = shadow_run else {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "RULE_CANDIDATE_SHADOW_REQUIRED",
                "accepted rule candidates require passed shadow evidence",
            ));
        };
        if shadow_run.decision != "shadow_passed" {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "RULE_CANDIDATE_SHADOW_BLOCKED",
                format!(
                    "accepted rule candidate shadow evidence is not passed: {}",
                    shadow_run.decision
                ),
            ));
        }
        if !shadow_run.blockers.is_empty() {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "RULE_CANDIDATE_SHADOW_BLOCKED",
                format!(
                    "accepted rule candidate shadow blockers must be resolved: {}",
                    shadow_run.blockers.join(", ")
                ),
            ));
        }
        validate_candidate_review_shadow_evidence(&request.evidence_refs, &shadow_run.report_uri)?;
        let mut rule = request.rule.clone();
        rule.scheme_family = Some(validate_rule_candidate(&rule)?);
        let detail = state
            .repository
            .save_rule_candidate(rule, "rule-discovery-review".into())
            .await
            .map_err(internal_error("RULE_CANDIDATE_ACCEPT_SAVE_FAILED"))?;
        saved_draft_rule_id = Some(detail.summary.rule_id);
    }
    let outcome = candidate_review_outcome(&request.decision, saved_draft_rule_id.clone());
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: format!(
                "rule_candidate_review_{}",
                rule_id_slug(&request.rule.rule_id)
            ),
            claim_id: request.rule.rule_id.clone(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "rule.candidate.reviewed".into(),
            event_status: "succeeded".into(),
            summary: format!(
                "Rule candidate {}: {}",
                request.rule.rule_id, request.decision
            ),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "rule_id": request.rule.rule_id,
                "rule_version": request.rule.version,
                "decision": request.decision,
                "reviewer": request.reviewer,
                "entered_rule_library": false,
                "accepted_for_governance_review": outcome.accepted_for_governance_review,
                "saved_draft_rule_id": outcome.saved_draft_rule_id,
                "active_rule_writeback": outcome.active_rule_writeback,
                "condition_count": request.rule.conditions.len(),
                "note_present": !request.notes.trim().is_empty(),
            }),
            evidence_refs: request
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
        .map_err(internal_error("RULE_CANDIDATE_REVIEW_AUDIT_SAVE_FAILED"))?;

    Ok(Json(ReviewRuleCandidateResponse {
        rule_id: request.rule.rule_id,
        decision: request.decision,
        entered_rule_library: false,
        accepted_for_governance_review: outcome.accepted_for_governance_review,
        saved_draft_rule_id: outcome.saved_draft_rule_id,
        active_rule_writeback: outcome.active_rule_writeback,
        evidence_refs: request.evidence_refs,
    }))
}

pub async fn submit_rule(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(rule_id): Path<String>,
    Json(request): Json<RuleLifecycleRequest>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    validate_rule_lifecycle_request(&request)?;
    update_status(state, actor, rule_id, "submitted", request.evidence_refs).await
}

pub async fn approve_rule(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(rule_id): Path<String>,
    Json(request): Json<RuleLifecycleRequest>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    validate_rule_lifecycle_request(&request)?;
    let actor = require_permission(principal, "ops:rules:approve")?;
    update_status_with_required_previous(
        state,
        actor,
        rule_id,
        "approved",
        Some("submitted"),
        request.evidence_refs,
    )
    .await
}

pub async fn publish_rule(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(rule_id): Path<String>,
    Json(request): Json<RuleLifecycleRequest>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    validate_rule_lifecycle_request(&request)?;
    let actor = require_permission(principal, "ops:rules:publish")?;
    let previous = state
        .repository
        .get_rule(&rule_id)
        .await
        .map_err(internal_error("RULE_LOAD_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?
        .summary;
    if previous.status != "approved" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "RULE_APPROVAL_REQUIRED",
            "rule must be approved before publish",
        ));
    }
    let gates = load_rule_promotion_gates(&state, &rule_id).await?;
    if gates.decision != "routing_allowed" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "RULE_PROMOTION_GATES_BLOCKED",
            format!(
                "rule promotion gates blocked: {}",
                gates.blockers.join(", ")
            ),
        ));
    }
    let rule = state
        .repository
        .update_rule_status(&rule_id, "active")
        .await
        .map_err(internal_error("RULE_STATUS_UPDATE_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?;
    record_rule_audit(
        &state,
        &actor,
        RuleAuditInput {
            rule: &rule,
            event_type: "rule.status.changed",
            from_status: Some(&previous.status),
            to_status: &rule.status,
            summary: "Rule status changed",
            evidence_refs: request.evidence_refs,
        },
    )
    .await
    .map_err(internal_error("RULE_AUDIT_SAVE_FAILED"))?;
    Ok(Json(RuleLifecycleResponse {
        rule_id: rule.rule_id,
        status: rule.status,
        active_version: rule.active_version,
        latest_version: rule.latest_version,
    }))
}

pub async fn rollback_rule(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(rule_id): Path<String>,
    Json(request): Json<RuleLifecycleRequest>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    validate_rule_lifecycle_request(&request)?;
    let actor = require_permission(principal, "ops:rules:rollback")?;
    let previous = state
        .repository
        .get_rule(&rule_id)
        .await
        .map_err(internal_error("RULE_LOAD_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?
        .summary;
    if previous.status != "active" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "RULE_ROLLBACK_REQUIRES_ACTIVE",
            "only active rules can be rolled back",
        ));
    }
    let rule = state
        .repository
        .update_rule_status(&rule_id, "approved")
        .await
        .map_err(internal_error("RULE_STATUS_UPDATE_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?;
    record_rule_audit(
        &state,
        &actor,
        RuleAuditInput {
            rule: &rule,
            event_type: "rule.rollback.completed",
            from_status: Some(&previous.status),
            to_status: &rule.status,
            summary: "Rule rollback completed",
            evidence_refs: request.evidence_refs,
        },
    )
    .await
    .map_err(internal_error("RULE_AUDIT_SAVE_FAILED"))?;
    Ok(Json(RuleLifecycleResponse {
        rule_id: rule.rule_id,
        status: rule.status,
        active_version: rule.active_version,
        latest_version: rule.latest_version,
    }))
}

async fn update_status(
    state: AppState,
    actor: ActorContext,
    rule_id: String,
    status: &'static str,
    evidence_refs: Vec<String>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    update_status_with_required_previous(state, actor, rule_id, status, None, evidence_refs).await
}

async fn update_status_with_required_previous(
    state: AppState,
    actor: ActorContext,
    rule_id: String,
    status: &'static str,
    required_previous_status: Option<&'static str>,
    evidence_refs: Vec<String>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    let previous = state
        .repository
        .get_rule(&rule_id)
        .await
        .map_err(internal_error("RULE_LOAD_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?
        .summary;
    if let Some(required_status) = required_previous_status {
        if previous.status != required_status {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "RULE_STATUS_REQUIRED",
                format!("rule must be {required_status} before {status}"),
            ));
        }
    }
    let rule = state
        .repository
        .update_rule_status(&rule_id, status)
        .await
        .map_err(internal_error("RULE_STATUS_UPDATE_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?;
    record_rule_audit(
        &state,
        &actor,
        RuleAuditInput {
            rule: &rule,
            event_type: "rule.status.changed",
            from_status: Some(&previous.status),
            to_status: &rule.status,
            summary: "Rule status changed",
            evidence_refs,
        },
    )
    .await
    .map_err(internal_error("RULE_AUDIT_SAVE_FAILED"))?;
    Ok(Json(RuleLifecycleResponse {
        rule_id: rule.rule_id,
        status: rule.status,
        active_version: rule.active_version,
        latest_version: rule.latest_version,
    }))
}

fn validate_rule_candidate(rule: &Rule) -> Result<String, ApiError> {
    let Some(scheme_family) = rule.scheme_family.as_deref() else {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_CANDIDATE",
            "scheme_family is required for rule candidates",
        ));
    };
    let Some(canonical) = canonical_scheme_family(scheme_family) else {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_CANDIDATE",
            "scheme_family must map to a known FWA scheme family",
        ));
    };
    Ok(canonical)
}

fn validate_rule_lifecycle_request(request: &RuleLifecycleRequest) -> Result<(), ApiError> {
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_RULE_LIFECYCLE_EVIDENCE",
            "rule lifecycle evidence_refs are required",
        ));
    }
    if pii::contains_pii(request.evidence_refs.iter().map(String::as_str)) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_RULE_LIFECYCLE",
            "rule lifecycle evidence_refs must not contain PII",
        ));
    }
    Ok(())
}

fn require_permission(
    principal: AuthenticatedPrincipal,
    permission: &str,
) -> Result<ActorContext, ApiError> {
    if !principal.has_permission(permission) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "PERMISSION_DENIED",
            format!("missing permission: {permission}"),
        ));
    }
    Ok(principal.actor)
}

struct RuleAuditInput<'a> {
    rule: &'a RuleSummaryRecord,
    event_type: &'static str,
    from_status: Option<&'a str>,
    to_status: &'a str,
    summary: &'static str,
    evidence_refs: Vec<String>,
}

async fn record_rule_audit(
    state: &AppState,
    actor: &ActorContext,
    input: RuleAuditInput<'_>,
) -> anyhow::Result<()> {
    let payload = serde_json::json!({
        "customer_scope_id": actor.customer_scope_id,
        "rule_id": input.rule.rule_id,
        "rule_version": input.rule.latest_version,
        "from_status": input.from_status,
        "to_status": input.to_status,
        "owner": input.rule.owner,
        "alert_code": input.rule.alert_code,
        "recommended_action": input.rule.recommended_action,
    });
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: input.event_type.to_string(),
            event_status: "succeeded".into(),
            summary: input.summary.into(),
            payload,
            evidence_refs: input
                .evidence_refs
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

fn default_rule_evidence_refs(rule: &RuleSummaryRecord) -> Vec<String> {
    vec![format!("rules:{}:v{}", rule.rule_id, rule.latest_version)]
}

async fn record_rule_backtest_audit(
    state: &AppState,
    actor: &ActorContext,
    record: &RuleBacktestRecord,
) -> anyhow::Result<()> {
    let mut payload = serde_json::to_value(record)?;
    if let Some(payload) = payload.as_object_mut() {
        payload.insert(
            "customer_scope_id".into(),
            serde_json::json!(actor.customer_scope_id),
        );
    }
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "rule.backtest.completed".into(),
            event_status: "succeeded".into(),
            summary: "Rule backtest completed".into(),
            payload,
            evidence_refs: record
                .evidence_refs
                .iter()
                .map(|reference| serde_json::json!(reference))
                .collect(),
        })
        .await
}

async fn record_rule_promotion_audit(
    state: &AppState,
    actor: &ActorContext,
    review: &RulePromotionReviewRecord,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "rule.promotion.reviewed".into(),
            event_status: "succeeded".into(),
            summary: format!("Rule promotion review: {}", review.decision),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "rule_id": review.rule_id,
                "rule_version": review.rule_version,
                "decision": review.decision,
                "reviewer": review.reviewer,
                "note_present": !review.notes.trim().is_empty(),
            }),
            evidence_refs: review
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

async fn record_rule_shadow_run_audit(
    state: &AppState,
    actor: &ActorContext,
    record: &RuleShadowRunRecord,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "rule.shadow_run.reviewed".into(),
            event_status: "succeeded".into(),
            summary: format!("Rule shadow run reviewed: {}", record.decision),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "rule_id": record.rule_id,
                "rule_version": record.rule_version,
                "decision": record.decision,
                "reviewer": record.reviewer,
                "report_uri": record.report_uri,
                "reviewed_count": record.reviewed_count,
                "matched_count": record.matched_count,
                "false_positive_count": record.false_positive_count,
                "false_positive_rate": record.false_positive_rate,
                "blocker_count": record.blockers.len(),
                "active_rule_writeback": false,
            }),
            evidence_refs: record
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

#[derive(Debug, Clone)]
struct MiningSample {
    claim_id: String,
    claim_amount: Decimal,
    confirmed_fwa: Option<bool>,
    features: BTreeMap<String, f64>,
}

#[derive(Debug)]
struct FeatureSplitCandidate {
    feature: String,
    operator: &'static str,
    threshold: f64,
    support: usize,
    true_positive_count: usize,
    false_positive_count: usize,
    precision: f64,
    recall: f64,
    lift: f64,
    false_positive_rate: f64,
    saving: Decimal,
    matched_claim_ids: Vec<String>,
    positive_mean: f64,
    negative_mean: f64,
    negative_stddev: f64,
    statistical_threshold: f64,
    model_reason: Option<String>,
}

#[derive(Debug, Clone)]
struct TreePathCondition {
    feature: String,
    operator: &'static str,
    threshold: f64,
}

#[derive(Debug)]
struct TreeRuleCandidate {
    conditions: Vec<TreePathCondition>,
    support: usize,
    true_positive_count: usize,
    false_positive_count: usize,
    precision: f64,
    recall: f64,
    lift: f64,
    false_positive_rate: f64,
    saving: Decimal,
    matched_claim_ids: Vec<String>,
    depth: usize,
    gini: f64,
}

#[derive(Debug)]
struct TreeSplit {
    feature: String,
    threshold: f64,
    gain: f64,
    left_indices: Vec<usize>,
    right_indices: Vec<usize>,
}

fn mine_statistical_rule_candidates(
    request: &RuleDiscoveryRequest,
    samples: &[MiningSample],
    min_support: usize,
    positive_count: usize,
    baseline_rate: f64,
    evidence_refs: &[String],
) -> Vec<RuleDiscoveryCandidate> {
    let mut features = samples
        .iter()
        .flat_map(|sample| sample.features.keys().cloned())
        .collect::<Vec<_>>();
    features.sort();
    features.dedup();
    if let Some(requested_features) = &request.candidate_feature_fields {
        if !requested_features.is_empty() {
            features.retain(|feature| {
                requested_features
                    .iter()
                    .any(|requested| requested == feature)
            });
        }
    }

    let mut candidates = mine_tree_rule_candidates(
        request,
        samples,
        &features,
        min_support,
        positive_count,
        baseline_rate,
        evidence_refs,
    );
    candidates.extend(
        features
            .into_iter()
            .filter_map(|feature| {
                best_feature_split(
                    &feature,
                    request,
                    samples,
                    min_support,
                    positive_count,
                    baseline_rate,
                )
            })
            .map(|split| split.into_response(evidence_refs)),
    );
    candidates.sort_by(|left, right| {
        right
            .precision
            .partial_cmp(&left.precision)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .lift
                    .partial_cmp(&left.lift)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| right.support.cmp(&left.support))
            .then_with(|| left.rule.rule_id.cmp(&right.rule.rule_id))
    });
    candidates
}

fn mine_tree_rule_candidates(
    request: &RuleDiscoveryRequest,
    samples: &[MiningSample],
    features: &[String],
    min_support: usize,
    positive_count: usize,
    baseline_rate: f64,
    evidence_refs: &[String],
) -> Vec<RuleDiscoveryCandidate> {
    let max_depth = request.max_tree_depth.unwrap_or(2).clamp(1, 3);
    if max_depth < 2 || features.len() < 2 {
        return Vec::new();
    }
    let root_indices = (0..samples.len()).collect::<Vec<_>>();
    let mut tree_candidates = Vec::new();
    collect_tree_leaf_candidates(
        samples,
        features,
        root_indices,
        Vec::new(),
        max_depth,
        min_support,
        positive_count,
        baseline_rate,
        &mut tree_candidates,
    );
    tree_candidates
        .into_iter()
        .filter(|candidate| candidate.conditions.len() >= 2)
        .map(|candidate| candidate.into_response(evidence_refs))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn collect_tree_leaf_candidates(
    samples: &[MiningSample],
    features: &[String],
    indices: Vec<usize>,
    path: Vec<TreePathCondition>,
    remaining_depth: usize,
    min_support: usize,
    positive_count: usize,
    baseline_rate: f64,
    candidates: &mut Vec<TreeRuleCandidate>,
) {
    if indices.is_empty() {
        return;
    }
    if let Some(candidate) = score_tree_path(
        samples,
        &indices,
        &path,
        min_support,
        positive_count,
        baseline_rate,
    ) {
        candidates.push(candidate);
    }
    if remaining_depth == 0 || is_pure_leaf(samples, &indices) {
        return;
    }
    let used_features = path
        .iter()
        .map(|condition| condition.feature.as_str())
        .collect::<Vec<_>>();
    let available_features = features
        .iter()
        .filter(|feature| !used_features.contains(&feature.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if available_features.is_empty() {
        return;
    }
    let Some(split) = best_tree_split(samples, &indices, &available_features) else {
        return;
    };
    if split.gain <= 0.0 {
        return;
    }

    let mut left_path = path.clone();
    left_path.push(TreePathCondition {
        feature: split.feature.clone(),
        operator: "<=",
        threshold: split.threshold,
    });
    collect_tree_leaf_candidates(
        samples,
        features,
        split.left_indices,
        left_path,
        remaining_depth - 1,
        min_support,
        positive_count,
        baseline_rate,
        candidates,
    );

    let mut right_path = path;
    right_path.push(TreePathCondition {
        feature: split.feature,
        operator: ">=",
        threshold: split.threshold,
    });
    collect_tree_leaf_candidates(
        samples,
        features,
        split.right_indices,
        right_path,
        remaining_depth - 1,
        min_support,
        positive_count,
        baseline_rate,
        candidates,
    );
}

fn best_tree_split(
    samples: &[MiningSample],
    indices: &[usize],
    features: &[String],
) -> Option<TreeSplit> {
    let parent_gini = gini_impurity(samples, indices);
    features
        .iter()
        .flat_map(|feature| {
            tree_thresholds(samples, indices, feature)
                .into_iter()
                .filter_map(|threshold| {
                    let mut left_indices = Vec::new();
                    let mut right_indices = Vec::new();
                    for index in indices {
                        let value = samples[*index].features.get(feature).copied()?;
                        if value <= threshold {
                            left_indices.push(*index);
                        } else {
                            right_indices.push(*index);
                        }
                    }
                    if left_indices.is_empty() || right_indices.is_empty() {
                        return None;
                    }
                    let weighted_child_gini =
                        weighted_gini(samples, indices.len(), &left_indices, &right_indices);
                    Some(TreeSplit {
                        feature: feature.clone(),
                        threshold,
                        gain: parent_gini - weighted_child_gini,
                        left_indices,
                        right_indices,
                    })
                })
                .collect::<Vec<_>>()
        })
        .max_by(|left, right| {
            left.gain
                .partial_cmp(&right.gain)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.left_indices.len().cmp(&left.left_indices.len()))
                .then_with(|| left.feature.cmp(&right.feature))
        })
}

fn tree_thresholds(samples: &[MiningSample], indices: &[usize], feature: &str) -> Vec<f64> {
    let mut values = indices
        .iter()
        .filter_map(|index| samples[*index].features.get(feature).copied())
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    values.dedup_by(|left, right| (*left - *right).abs() < 0.000001);
    values
        .windows(2)
        .map(|window| (window[0] + window[1]) / 2.0)
        .collect()
}

fn weighted_gini(
    samples: &[MiningSample],
    parent_count: usize,
    left_indices: &[usize],
    right_indices: &[usize],
) -> f64 {
    let left_weight = left_indices.len() as f64 / parent_count as f64;
    let right_weight = right_indices.len() as f64 / parent_count as f64;
    (left_weight * gini_impurity(samples, left_indices))
        + (right_weight * gini_impurity(samples, right_indices))
}

fn gini_impurity(samples: &[MiningSample], indices: &[usize]) -> f64 {
    if indices.is_empty() {
        return 0.0;
    }
    let positive_count = indices
        .iter()
        .filter(|index| samples[**index].confirmed_fwa == Some(true))
        .count();
    let negative_count = indices
        .iter()
        .filter(|index| samples[**index].confirmed_fwa == Some(false))
        .count();
    let labeled_count = positive_count + negative_count;
    if labeled_count == 0 {
        return 0.0;
    }
    let positive_rate = positive_count as f64 / labeled_count as f64;
    let negative_rate = negative_count as f64 / labeled_count as f64;
    1.0 - positive_rate.powi(2) - negative_rate.powi(2)
}

fn is_pure_leaf(samples: &[MiningSample], indices: &[usize]) -> bool {
    gini_impurity(samples, indices) == 0.0
}

fn score_tree_path(
    samples: &[MiningSample],
    indices: &[usize],
    path: &[TreePathCondition],
    min_support: usize,
    positive_count: usize,
    baseline_rate: f64,
) -> Option<TreeRuleCandidate> {
    if path.is_empty() {
        return None;
    }
    let mut matched_claim_ids = Vec::new();
    let mut true_positive_count = 0_usize;
    let mut false_positive_count = 0_usize;
    let mut saving = Decimal::ZERO;
    for index in indices {
        let sample = &samples[*index];
        matched_claim_ids.push(sample.claim_id.clone());
        match sample.confirmed_fwa {
            Some(true) => {
                true_positive_count += 1;
                saving += sample.claim_amount * Decimal::new(10, 2);
            }
            Some(false) => false_positive_count += 1,
            None => {}
        }
    }
    let support = matched_claim_ids.len();
    if support < min_support || true_positive_count == 0 {
        return None;
    }
    let precision = true_positive_count as f64 / support as f64;
    if baseline_rate > 0.0 && precision <= baseline_rate {
        return None;
    }
    let recall = true_positive_count as f64 / positive_count as f64;
    let lift = if baseline_rate == 0.0 {
        0.0
    } else {
        precision / baseline_rate
    };
    Some(TreeRuleCandidate {
        conditions: path.to_vec(),
        support,
        true_positive_count,
        false_positive_count,
        precision,
        recall,
        lift,
        false_positive_rate: false_positive_count as f64 / support as f64,
        saving,
        matched_claim_ids,
        depth: path.len(),
        gini: gini_impurity(samples, indices),
    })
}

fn best_feature_split(
    feature: &str,
    request: &RuleDiscoveryRequest,
    samples: &[MiningSample],
    min_support: usize,
    positive_count: usize,
    baseline_rate: f64,
) -> Option<FeatureSplitCandidate> {
    let values = samples
        .iter()
        .filter_map(|sample| {
            sample
                .features
                .get(feature)
                .filter(|value| value.is_finite())
                .map(|value| (sample, *value))
        })
        .collect::<Vec<_>>();
    if values.len() < min_support {
        return None;
    }

    let positive_values = values
        .iter()
        .filter_map(|(sample, value)| (sample.confirmed_fwa == Some(true)).then_some(*value))
        .collect::<Vec<_>>();
    let negative_values = values
        .iter()
        .filter_map(|(sample, value)| (sample.confirmed_fwa == Some(false)).then_some(*value))
        .collect::<Vec<_>>();
    if positive_values.is_empty() || negative_values.is_empty() {
        return None;
    }

    let positive_mean = mean(&positive_values);
    let negative_mean = mean(&negative_values);
    let negative_stddev = stddev(&negative_values, negative_mean);
    let high_risk_when_higher = positive_mean >= negative_mean;
    let operator = if high_risk_when_higher { ">=" } else { "<=" };
    let statistical_threshold = if high_risk_when_higher {
        negative_mean + (1.5 * negative_stddev)
    } else {
        negative_mean - (1.5 * negative_stddev)
    };

    let mut thresholds = values.iter().map(|(_, value)| *value).collect::<Vec<_>>();
    thresholds.push(statistical_threshold);
    thresholds.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    thresholds.dedup_by(|left, right| (*left - *right).abs() < 0.000001);

    thresholds
        .into_iter()
        .filter_map(|threshold| {
            score_feature_threshold(
                feature,
                operator,
                threshold,
                samples,
                min_support,
                positive_count,
                baseline_rate,
                positive_mean,
                negative_mean,
                negative_stddev,
                statistical_threshold,
                model_explanation_reason(request, feature),
            )
        })
        .max_by(|left, right| {
            left.precision
                .partial_cmp(&right.precision)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    left.lift
                        .partial_cmp(&right.lift)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| left.support.cmp(&right.support))
        })
}

fn score_feature_threshold(
    feature: &str,
    operator: &'static str,
    threshold: f64,
    samples: &[MiningSample],
    min_support: usize,
    positive_count: usize,
    baseline_rate: f64,
    positive_mean: f64,
    negative_mean: f64,
    negative_stddev: f64,
    statistical_threshold: f64,
    model_reason: Option<String>,
) -> Option<FeatureSplitCandidate> {
    let mut matched_claim_ids = Vec::new();
    let mut true_positive_count = 0_usize;
    let mut false_positive_count = 0_usize;
    let mut saving = Decimal::ZERO;

    for sample in samples {
        let Some(value) = sample.features.get(feature).copied() else {
            continue;
        };
        let matched = if operator == ">=" {
            value >= threshold
        } else {
            value <= threshold
        };
        if !matched {
            continue;
        }
        matched_claim_ids.push(sample.claim_id.clone());
        match sample.confirmed_fwa {
            Some(true) => {
                true_positive_count += 1;
                saving += sample.claim_amount * Decimal::new(10, 2);
            }
            Some(false) => false_positive_count += 1,
            None => {}
        }
    }

    let support = matched_claim_ids.len();
    if support < min_support || true_positive_count == 0 {
        return None;
    }
    let precision = true_positive_count as f64 / support as f64;
    if baseline_rate > 0.0 && precision <= baseline_rate {
        return None;
    }
    let recall = true_positive_count as f64 / positive_count as f64;
    let lift = if baseline_rate == 0.0 {
        0.0
    } else {
        precision / baseline_rate
    };
    Some(FeatureSplitCandidate {
        feature: feature.into(),
        operator,
        threshold,
        support,
        true_positive_count,
        false_positive_count,
        precision,
        recall,
        lift,
        false_positive_rate: false_positive_count as f64 / support as f64,
        saving,
        matched_claim_ids,
        positive_mean,
        negative_mean,
        negative_stddev,
        statistical_threshold,
        model_reason,
    })
}

impl FeatureSplitCandidate {
    fn into_response(self, base_evidence_refs: &[String]) -> RuleDiscoveryCandidate {
        let feature_slug = rule_id_slug(&self.feature);
        let threshold_slug = threshold_slug(self.threshold);
        let op_slug = if self.operator == ">=" { "gte" } else { "lte" };
        let rule = Rule {
            rule_id: format!("candidate_mined_{feature_slug}_{op_slug}_{threshold_slug}"),
            version: 1,
            name: format!(
                "Mined rule: {} {} {}",
                self.feature,
                self.operator,
                format_threshold(self.threshold)
            ),
            review_mode: "both".into(),
            scheme_family: Some("high_risk_claim".into()),
            conditions: vec![Condition {
                field: self.feature.clone(),
                operator: self.operator.into(),
                value: serde_json::json!(round_float(self.threshold)),
            }],
            action: RuleAction {
                score: mined_rule_score(self.precision, self.lift),
                alert_code: format!("MINED_{}", feature_slug.to_uppercase()),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: format!(
                    "数据集挖掘显示 {} {} {} 的样本 FWA 命中率高于基线，需人工解释性 review",
                    self.feature,
                    self.operator,
                    format_threshold(self.threshold)
                ),
            },
        };
        let mut evidence_refs = base_evidence_refs.to_vec();
        evidence_refs.push(format!("rule_mining:{}:decision_stump", self.feature));
        evidence_refs.push(format!(
            "rule_mining:{}:negative_mean_{}_stddev_{}",
            self.feature,
            format_threshold(self.negative_mean),
            format_threshold(self.negative_stddev)
        ));
        let model_clause = self
            .model_reason
            .as_deref()
            .map(|reason| format!(" 模型解释备注：{reason}"))
            .unwrap_or_default();
        let explanation = format!(
            "{} {} {} 是从标签数据集挖掘出的单层决策树阈值规则：正样本均值 {}，负样本均值 {}，负样本标准差 {}，统计参考阈值 {}；该候选命中 {} 条，其中确认 FWA {} 条、非 FWA {} 条，precision {:.1}%，recall {:.1}%，lift {:.2}。{}仍需人工接受或拒绝后才可进入规则库。",
            self.feature,
            self.operator,
            format_threshold(self.threshold),
            format_threshold(self.positive_mean),
            format_threshold(self.negative_mean),
            format_threshold(self.negative_stddev),
            format_threshold(self.statistical_threshold),
            self.support,
            self.true_positive_count,
            self.false_positive_count,
            self.precision * 100.0,
            self.recall * 100.0,
            self.lift,
            model_clause,
        );
        RuleDiscoveryCandidate {
            explanation,
            condition_refs: condition_refs_for_rule(&rule),
            rule,
            support: self.support,
            precision: self.precision,
            recall: self.recall,
            lift: self.lift,
            estimated_saving: format!("{:.2}", self.saving.round_dp(2)),
            false_positive_rate: self.false_positive_rate,
            matched_claim_ids: self.matched_claim_ids,
            evidence_refs,
        }
    }
}

impl TreeRuleCandidate {
    fn into_response(self, base_evidence_refs: &[String]) -> RuleDiscoveryCandidate {
        let condition_slug = self
            .conditions
            .iter()
            .map(tree_condition_slug)
            .collect::<Vec<_>>()
            .join("_and_");
        let conditions = self
            .conditions
            .iter()
            .map(|condition| Condition {
                field: condition.feature.clone(),
                operator: condition.operator.into(),
                value: serde_json::json!(round_float(condition.threshold)),
            })
            .collect::<Vec<_>>();
        let rule = Rule {
            rule_id: format!("candidate_tree_{condition_slug}"),
            version: 1,
            name: format!("Decision tree rule: {}", tree_path_label(&self.conditions)),
            review_mode: "both".into(),
            scheme_family: Some("high_risk_claim".into()),
            conditions,
            action: RuleAction {
                score: mined_rule_score(self.precision, self.lift),
                alert_code: format!("TREE_{}", short_alert_slug(&condition_slug).to_uppercase()),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: format!(
                    "浅层决策树发现路径 {} 的 FWA 命中率高于基线，需人工解释性 review",
                    tree_path_label(&self.conditions)
                ),
            },
        };
        let mut evidence_refs = base_evidence_refs.to_vec();
        evidence_refs.push(format!(
            "rule_mining:shallow_decision_tree:depth_{}",
            self.depth
        ));
        evidence_refs.push(format!(
            "rule_mining:shallow_decision_tree:gini_{}",
            format_threshold(self.gini)
        ));
        let explanation = format!(
            "{} 是从标签数据集训练出的浅层决策树叶子规则：树深度 {}，叶子 Gini {}；该路径命中 {} 条，其中确认 FWA {} 条、非 FWA {} 条，precision {:.1}%，recall {:.1}%，lift {:.2}。每个树叶候选仍需人工接受或拒绝后才可进入规则库。",
            tree_path_label(&self.conditions),
            self.depth,
            format_threshold(self.gini),
            self.support,
            self.true_positive_count,
            self.false_positive_count,
            self.precision * 100.0,
            self.recall * 100.0,
            self.lift,
        );
        RuleDiscoveryCandidate {
            explanation,
            condition_refs: condition_refs_for_rule(&rule),
            rule,
            support: self.support,
            precision: self.precision,
            recall: self.recall,
            lift: self.lift,
            estimated_saving: format!("{:.2}", self.saving.round_dp(2)),
            false_positive_rate: self.false_positive_rate,
            matched_claim_ids: self.matched_claim_ids,
            evidence_refs,
        }
    }
}

fn tree_condition_slug(condition: &TreePathCondition) -> String {
    let op_slug = if condition.operator == ">=" {
        "gte"
    } else {
        "lte"
    };
    format!(
        "{}_{}_{}",
        rule_id_slug(&condition.feature),
        op_slug,
        threshold_slug(condition.threshold)
    )
}

fn short_alert_slug(value: &str) -> String {
    value.chars().take(48).collect()
}

fn tree_path_label(conditions: &[TreePathCondition]) -> String {
    conditions
        .iter()
        .map(|condition| {
            format!(
                "{} {} {}",
                condition.feature,
                condition.operator,
                format_threshold(condition.threshold)
            )
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn model_explanation_reason(request: &RuleDiscoveryRequest, feature: &str) -> Option<String> {
    let min_abs_contribution = request.min_abs_contribution.unwrap_or(0.10);
    request
        .model_explanations
        .iter()
        .find(|explanation| {
            explanation.feature == feature
                && explanation.direction == "increases_risk"
                && explanation.contribution.is_finite()
                && explanation.contribution.abs() >= min_abs_contribution
        })
        .map(|explanation| explanation.reason.clone())
}

fn mined_rule_score(precision: f64, lift: f64) -> u8 {
    ((precision * 25.0) + lift.min(4.0) * 5.0)
        .round()
        .clamp(10.0, 45.0) as u8
}

fn condition_refs_for_rule(rule: &Rule) -> Vec<String> {
    rule.conditions
        .iter()
        .enumerate()
        .map(|(index, _)| {
            format!(
                "rule_conditions:{}_v{}_c{}",
                rule_id_slug(&rule.rule_id),
                rule.version,
                index + 1
            )
        })
        .collect()
}

fn rule_id_slug(value: &str) -> String {
    let slug = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if slug.is_empty() {
        "feature".into()
    } else {
        slug
    }
}

fn rule_discovery_evidence_refs(request: &RuleDiscoveryRequest) -> Vec<String> {
    let mut refs = Vec::new();
    if let Some(dataset_uri) = normalized_optional_str(request.dataset_uri.as_deref()) {
        refs.push(format!("dataset:{dataset_uri}"));
    } else {
        refs.push("dataset:inline_labeled_samples".into());
    }
    if let (Some(model_key), Some(model_version)) = (
        request.source_model_key.as_deref(),
        request.source_model_version.as_deref(),
    ) {
        refs.push(format!("model_versions:{model_key}:{model_version}"));
    }
    if let Some(feature_importance_uri) = request.feature_importance_uri.as_deref() {
        if !feature_importance_uri.trim().is_empty() {
            refs.push(format!("feature_importance:{feature_importance_uri}"));
        }
    }
    refs
}

fn discovery_mining_samples(request: &RuleDiscoveryRequest) -> anyhow::Result<Vec<MiningSample>> {
    if let Some(dataset_uri) = normalized_optional_str(request.dataset_uri.as_deref()) {
        return read_parquet_mining_samples(
            dataset_uri,
            request.label_column.as_deref(),
            request.claim_id_column.as_deref(),
            request.candidate_feature_fields.as_deref(),
        );
    }

    Ok(request
        .samples
        .iter()
        .map(|sample| {
            mining_sample_from_backtest_sample(&sample.sample, Some(sample.confirmed_fwa))
        })
        .collect())
}

fn backtest_mining_samples(request: &RuleBacktestRequest) -> anyhow::Result<Vec<MiningSample>> {
    if let Some(dataset_uri) = normalized_optional_str(request.dataset_uri.as_deref()) {
        return read_parquet_mining_samples(
            dataset_uri,
            request.label_column.as_deref(),
            request.claim_id_column.as_deref(),
            None,
        );
    }

    Ok(request
        .samples
        .iter()
        .map(|sample| mining_sample_from_backtest_sample(sample, sample.confirmed_fwa))
        .collect())
}

fn mining_sample_from_backtest_sample(
    sample: &RuleBacktestSample,
    confirmed_fwa: Option<bool>,
) -> MiningSample {
    let context = sample_context(sample);
    let features = calculate_features(&context)
        .into_iter()
        .filter_map(|(name, feature)| feature.value.as_f64().map(|value| (name, value)))
        .collect::<BTreeMap<_, _>>();
    MiningSample {
        claim_id: sample.external_claim_id.clone(),
        claim_amount: sample.claim_amount,
        confirmed_fwa,
        features,
    }
}

fn read_parquet_mining_samples(
    dataset_uri: &str,
    label_column: Option<&str>,
    claim_id_column: Option<&str>,
    candidate_feature_fields: Option<&[String]>,
) -> anyhow::Result<Vec<MiningSample>> {
    let label_column = label_column.unwrap_or("confirmed_fwa");
    let claim_id_column = claim_id_column.unwrap_or("claim_id");
    let dataset_path = resolve_dataset_path(dataset_uri)?;
    let file =
        File::open(&dataset_path).with_context(|| format!("open {}", dataset_path.display()))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .with_context(|| format!("read parquet metadata {}", dataset_path.display()))?;
    let mut reader = builder.with_batch_size(4096).build()?;
    let mut samples = Vec::new();
    for batch in &mut reader {
        let batch = batch?;
        let schema = batch.schema();
        let label_index = schema
            .index_of(label_column)
            .with_context(|| format!("label column {label_column} not found"))?;
        let claim_id_index = schema
            .index_of(claim_id_column)
            .with_context(|| format!("claim id column {claim_id_column} not found"))?;
        let claim_amount_index = schema.index_of("claim_amount").ok();

        for row_index in 0..batch.num_rows() {
            let confirmed_fwa = bool_value_at(batch.column(label_index).as_ref(), row_index);
            let Some(confirmed_fwa) = confirmed_fwa else {
                continue;
            };
            let claim_id = string_value_at(batch.column(claim_id_index).as_ref(), row_index)
                .unwrap_or_else(|| format!("row-{row_index}"));
            let claim_amount = claim_amount_index
                .and_then(|index| numeric_value_at(batch.column(index).as_ref(), row_index))
                .and_then(Decimal::from_f64)
                .unwrap_or(Decimal::ZERO);
            let mut features = BTreeMap::new();
            for (column_index, field) in schema.fields().iter().enumerate() {
                let feature = field.name();
                if !is_candidate_feature(
                    feature,
                    label_column,
                    claim_id_column,
                    candidate_feature_fields,
                ) {
                    continue;
                }
                if let Some(value) =
                    numeric_value_at(batch.column(column_index).as_ref(), row_index)
                {
                    if value.is_finite() {
                        features.insert(feature.clone(), value);
                    }
                }
            }
            samples.push(MiningSample {
                claim_id,
                claim_amount,
                confirmed_fwa: Some(confirmed_fwa),
                features,
            });
        }
    }
    Ok(samples)
}

fn resolve_dataset_path(dataset_uri: &str) -> anyhow::Result<PathBuf> {
    if dataset_uri.starts_with("http://")
        || dataset_uri.starts_with("https://")
        || dataset_uri.starts_with("s3://")
    {
        bail!("only local parquet dataset_uri values are supported by rule discovery");
    }
    let path = PathBuf::from(dataset_uri);
    let path = if path.is_absolute() {
        path
    } else {
        let current_dir = std::env::current_dir()?;
        let mut candidate = current_dir.join(&path);
        if !candidate.exists() {
            for ancestor in current_dir.ancestors() {
                let ancestor_candidate = ancestor.join(&path);
                if ancestor_candidate.exists() {
                    candidate = ancestor_candidate;
                    break;
                }
            }
        }
        candidate
    };
    if path.extension().and_then(|value| value.to_str()) != Some("parquet") {
        bail!("dataset_uri must point to a parquet file");
    }
    if !path.exists() {
        bail!("dataset_uri not found: {}", path.display());
    }
    Ok(path)
}

fn is_candidate_feature(
    feature: &str,
    label_column: &str,
    claim_id_column: &str,
    candidate_feature_fields: Option<&[String]>,
) -> bool {
    if let Some(candidate_feature_fields) = candidate_feature_fields {
        if candidate_feature_fields.is_empty() {
            return feature != label_column
                && feature != claim_id_column
                && feature != "split"
                && feature != "service_date"
                && feature != "service_date_ord"
                && !feature.ends_with("_id");
        }
        return candidate_feature_fields
            .iter()
            .any(|candidate_feature| candidate_feature == feature);
    }
    feature != label_column
        && feature != claim_id_column
        && feature != "split"
        && feature != "service_date"
        && feature != "service_date_ord"
        && !feature.ends_with("_id")
}

fn feature_map_from_mining_sample(sample: &MiningSample) -> FeatureMap {
    sample
        .features
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                FeatureValue {
                    name: name.clone(),
                    version: 1,
                    value: serde_json::json!(value),
                    evidence_refs: vec![],
                },
            )
        })
        .collect()
}

fn numeric_value_at(array: &dyn arrow_array::Array, index: usize) -> Option<f64> {
    use arrow_array::{
        Float32Array, Float64Array, Int16Array, Int32Array, Int64Array, Int8Array, UInt16Array,
        UInt32Array, UInt64Array, UInt8Array,
    };
    if array.is_null(index) {
        return None;
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        return Some(values.value(index));
    }
    if let Some(values) = array.as_any().downcast_ref::<Float32Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<Int8Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<Int16Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<Int32Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt8Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt16Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt32Array>() {
        return Some(values.value(index) as f64);
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt64Array>() {
        return Some(values.value(index) as f64);
    }
    None
}

fn bool_value_at(array: &dyn arrow_array::Array, index: usize) -> Option<bool> {
    use arrow_array::{BooleanArray, Float64Array, Int64Array, Int8Array, StringArray};
    if array.is_null(index) {
        return None;
    }
    if let Some(values) = array.as_any().downcast_ref::<BooleanArray>() {
        return Some(values.value(index));
    }
    if let Some(values) = array.as_any().downcast_ref::<Int8Array>() {
        return Some(values.value(index) != 0);
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return Some(values.value(index) != 0);
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        return Some(values.value(index) != 0.0);
    }
    if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        return match values.value(index).to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        };
    }
    None
}

fn string_value_at(array: &dyn arrow_array::Array, index: usize) -> Option<String> {
    use arrow_array::{LargeStringArray, StringArray};
    if array.is_null(index) {
        return None;
    }
    if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        return Some(values.value(index).into());
    }
    if let Some(values) = array.as_any().downcast_ref::<LargeStringArray>() {
        return Some(values.value(index).into());
    }
    numeric_value_at(array, index).map(|value| format_threshold(value))
}

fn normalized_optional_str(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn stddev(values: &[f64], mean: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn threshold_slug(value: f64) -> String {
    format_threshold(value).replace(['.', '-'], "_")
}

fn format_threshold(value: f64) -> String {
    format!("{:.4}", round_float(value))
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn round_float(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}

fn sample_context(sample: &RuleBacktestSample) -> ClaimContext {
    let member_id = MemberId::from_external(format!("MBR-{}", sample.external_claim_id));
    let policy_id = PolicyId::from_external(sample.policy.external_policy_id.clone());
    let provider_id = ProviderId::from_external("PRV-BACKTEST");
    ClaimContext {
        claim: Claim {
            id: ClaimId::from_external(sample.external_claim_id.clone()),
            external_claim_id: sample.external_claim_id.clone(),
            member_id: member_id.clone(),
            policy_id: policy_id.clone(),
            provider_id: provider_id.clone(),
            diagnosis_code: "J10".into(),
            service_date: sample.service_date,
            amount: Money::new(sample.claim_amount, sample.currency.clone()),
        },
        items: vec![],
        member: Member {
            id: member_id.clone(),
            external_member_id: member_id.to_string(),
            dob: None,
            gender: None,
        },
        policy: Policy {
            id: policy_id,
            external_policy_id: sample.policy.external_policy_id.clone(),
            member_id,
            product_code: "MED".into(),
            coverage_start_date: sample.policy.coverage_start_date,
            coverage_end_date: sample.policy.coverage_end_date,
            coverage_limit: Money::new(sample.policy.coverage_limit, sample.currency.clone()),
        },
        provider: Provider {
            id: provider_id,
            external_provider_id: "PRV-BACKTEST".into(),
            name: "Backtest Provider".into(),
            provider_type: "hospital".into(),
            region: "SH".into(),
            risk_tier: ProviderRiskTier::Medium,
        },
    }
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}

fn bad_request<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::new(StatusCode::BAD_REQUEST, code, error.to_string())
}

fn validate_candidate_review_backtest_evidence(
    decision: &str,
    evidence_refs: &[String],
) -> Result<(), ApiError> {
    if decision != "accepted" {
        return Ok(());
    }
    let has_backtest_evidence = evidence_refs.iter().any(|reference| {
        let reference = reference.trim();
        reference.starts_with("rule_candidate_backtests:")
            || reference.starts_with("rule_backtests:")
            || reference.starts_with("rule.backtest:")
            || reference.starts_with("backtest:")
    });
    if has_backtest_evidence {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RULE_CANDIDATE_BACKTEST_EVIDENCE_REQUIRED",
            "accepted rule candidates require backtest evidence_refs",
        ))
    }
}

fn validate_candidate_review_shadow_evidence(
    evidence_refs: &[String],
    shadow_report_uri: &str,
) -> Result<(), ApiError> {
    let expected_ref = format!("rule_shadow_runs:{shadow_report_uri}");
    if evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_ref)
    {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RULE_CANDIDATE_SHADOW_EVIDENCE_REQUIRED",
            format!(
                "accepted rule candidates require shadow evidence_refs including {expected_ref}"
            ),
        ))
    }
}

fn validate_rule_shadow_run_request(
    rule_id: &str,
    request: &SubmitRuleShadowRunRequest,
) -> Result<(), ApiError> {
    if request.rule_version == 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_VERSION",
            "rule_version must be greater than zero",
        ));
    }
    if !matches!(
        request.decision.as_str(),
        "shadow_passed" | "shadow_blocked"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_DECISION",
            "decision must be shadow_passed or shadow_blocked",
        ));
    }
    if request.reviewed_count == 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_REVIEWED_COUNT",
            "reviewed_count must be greater than zero",
        ));
    }
    if request.matched_count > request.reviewed_count {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_MATCHED_COUNT",
            "matched_count must not exceed reviewed_count",
        ));
    }
    if request.false_positive_count > request.matched_count {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_FALSE_POSITIVE_COUNT",
            "false_positive_count must not exceed matched_count",
        ));
    }
    if !request.false_positive_rate.is_finite()
        || !(0.0..=1.0).contains(&request.false_positive_rate)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_FALSE_POSITIVE_RATE",
            "false_positive_rate must be between 0 and 1",
        ));
    }
    if request.report_uri.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_REPORT_URI",
            "report_uri is required",
        ));
    }
    if request.reviewer.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_REVIEWER",
            "reviewer is required",
        ));
    }
    if request.notes.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_NOTES",
            "shadow run notes are required",
        ));
    }
    if request.decision == "shadow_passed" && !request.blockers.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RULE_SHADOW_PASSED_WITH_BLOCKERS",
            "shadow_passed runs must not include blockers",
        ));
    }
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_RULE_SHADOW_EVIDENCE",
            "shadow run evidence_refs are required",
        ));
    }
    let rule_ref = format!("rules:{rule_id}:v{}", request.rule_version);
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == rule_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RULE_SHADOW_RULE_EVIDENCE_REQUIRED",
            "shadow run evidence_refs must include the rule version reference",
        ));
    }
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim().starts_with("rule_shadow_runs:"))
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RULE_SHADOW_RUN_EVIDENCE_REQUIRED",
            "shadow run evidence_refs must include a rule_shadow_runs reference",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.notes.as_str())
            .chain(request.evidence_refs.iter().map(String::as_str))
            .chain(request.blockers.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_RULE_SHADOW_RUN",
            "shadow run notes, blockers, and evidence_refs must not contain PII",
        ));
    }
    Ok(())
}

fn candidate_review_outcome(
    decision: &str,
    saved_draft_rule_id: Option<String>,
) -> CandidateReviewOutcome {
    let accepted_for_governance_review = decision == "accepted" && saved_draft_rule_id.is_some();
    CandidateReviewOutcome {
        accepted_for_governance_review,
        saved_draft_rule_id: accepted_for_governance_review
            .then(|| saved_draft_rule_id.expect("checked Some")),
        active_rule_writeback: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_candidate_review_requires_backtest_evidence() {
        let result =
            validate_candidate_review_backtest_evidence("accepted", &["rules:candidate:v1".into()]);

        assert!(result.is_err());
    }

    #[test]
    fn rejected_candidate_review_does_not_require_backtest_evidence() {
        validate_candidate_review_backtest_evidence("rejected", &["rules:candidate:v1".into()])
            .expect("rejected candidate review can record weak explanation rejection");
    }

    #[test]
    fn accepted_candidate_review_exposes_governance_review_boundary() {
        let outcome = candidate_review_outcome("accepted", Some("candidate_rule_1".into()));

        assert!(outcome.accepted_for_governance_review);
        assert_eq!(
            outcome.saved_draft_rule_id.as_deref(),
            Some("candidate_rule_1")
        );
        assert!(!outcome.active_rule_writeback);
    }
}
