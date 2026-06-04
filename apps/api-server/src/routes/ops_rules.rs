use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        PersistedAuditEvent, QaFeedbackItemRecord, RuleBacktestRecord, RuleConditionLibraryRecord,
        RulePerformanceRecord, RulePromotionReviewRecord, RuleSummaryRecord,
    },
    routes::pii,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::NaiveDate;
use fwa_audit::ActorContext;
use fwa_auth::{authenticate_api_key, validate_api_key};
use fwa_core::{
    canonical_scheme_family, AuditEventId, Claim, ClaimContext, ClaimId, Member, MemberId, Money,
    Policy, PolicyId, Provider, ProviderId, ProviderRiskTier, RecommendedAction, RuleActionClass,
    ScoringRunId,
};
use fwa_features::calculate_features;
use fwa_rules::{evaluate_rules, Condition, Rule, RuleAction};
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
pub struct RuleLifecycleRequest {
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RuleBacktestRequest {
    pub rule: Rule,
    pub samples: Vec<RuleBacktestSample>,
    pub expected_review_capacity: Option<usize>,
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
    pub samples: Vec<RuleDiscoverySample>,
    #[serde(default)]
    pub model_explanations: Vec<RuleDiscoveryModelExplanation>,
    pub source_model_key: Option<String>,
    pub source_model_version: Option<String>,
    pub feature_importance_uri: Option<String>,
    pub min_abs_contribution: Option<f64>,
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
    headers: HeaderMap,
) -> Result<Json<RuleListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let rules = state
        .repository
        .list_rules()
        .await
        .map_err(internal_error("RULE_LIST_FAILED"))?;
    Ok(Json(RuleListResponse { rules }))
}

pub async fn rule_performance(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RulePerformanceResponse>, ApiError> {
    authorize(&state, &headers)?;
    let rules = state
        .repository
        .rule_performance()
        .await
        .map_err(internal_error("RULE_PERFORMANCE_FAILED"))?;
    Ok(Json(RulePerformanceResponse { rules }))
}

pub async fn list_rule_conditions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RuleConditionLibraryResponse>, ApiError> {
    authorize(&state, &headers)?;
    let conditions = state
        .repository
        .list_rule_conditions()
        .await
        .map_err(internal_error("RULE_CONDITION_LIST_FAILED"))?;
    Ok(Json(RuleConditionLibraryResponse { conditions }))
}

pub async fn rule_promotion_gates(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(rule_id): Path<String>,
) -> Result<Json<RulePromotionGatesResponse>, ApiError> {
    authorize(&state, &headers)?;
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
        &outcome_labels,
        &feedback_items,
        latest_review.as_ref(),
        latest_action.as_ref(),
    ))
}

pub async fn submit_rule_promotion_review(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(rule_id): Path<String>,
    Json(request): Json<SubmitRulePromotionReviewRequest>,
) -> Result<Json<RulePromotionReviewRecord>, ApiError> {
    let actor = authorize_permission(&state, &headers, "ops:rules:review")?;
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

pub async fn get_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(rule_id): Path<String>,
) -> Result<Json<crate::repository::RuleDetailRecord>, ApiError> {
    authorize(&state, &headers)?;
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
    let shadow_rollout = performance.trigger_count > 0 && performance.reviewed_count > 0;
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
            if shadow_rollout { "runtime" } else { "missing" },
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
    headers: HeaderMap,
    Json(request): Json<RuleBacktestRequest>,
) -> Result<Json<RuleBacktestResponse>, ApiError> {
    let actor = authorize(&state, &headers)?;
    let mut matched_claim_ids = Vec::new();
    let mut score_sum = 0_u32;
    let mut saving = Decimal::ZERO;
    let mut true_positive_count = 0_usize;
    let mut false_positive_count = 0_usize;
    let positive_count = request
        .samples
        .iter()
        .filter(|sample| sample.confirmed_fwa == Some(true))
        .count();
    let reviewed_count = request
        .samples
        .iter()
        .filter(|sample| sample.confirmed_fwa.is_some())
        .count();
    let labeled_backtest = reviewed_count > 0;

    for sample in &request.samples {
        let context = sample_context(sample);
        let features = calculate_features(&context);
        let matches = evaluate_rules(std::slice::from_ref(&request.rule), &features)
            .map_err(internal_error("RULE_BACKTEST_FAILED"))?;
        if !matches.is_empty() {
            matched_claim_ids.push(sample.external_claim_id.clone());
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

    let sample_count = request.samples.len();
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
        evidence_refs: vec![format!(
            "rules:{}:v{}",
            request.rule.rule_id, request.rule.version
        )],
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

pub async fn discover_rules(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RuleDiscoveryRequest>,
) -> Result<Json<RuleDiscoveryResponse>, ApiError> {
    authorize(&state, &headers)?;
    let min_support = request.min_support.unwrap_or(1);
    let sample_count = request.samples.len();
    let positive_count = request
        .samples
        .iter()
        .filter(|sample| sample.confirmed_fwa)
        .count();
    let baseline_rate = if sample_count == 0 {
        0.0
    } else {
        positive_count as f64 / sample_count as f64
    };

    let candidate_rules = candidate_rule_templates()
        .into_iter()
        .chain(model_explanation_candidate_rules(&request))
        .collect::<Vec<_>>();
    let discovery_evidence_refs = rule_discovery_evidence_refs(&request);

    let mut candidates = Vec::new();
    for rule in candidate_rules {
        let mut matched_claim_ids = Vec::new();
        let mut true_positive_count = 0_usize;
        let mut false_positive_count = 0_usize;
        let mut saving = Decimal::ZERO;

        for labeled_sample in &request.samples {
            let context = sample_context(&labeled_sample.sample);
            let features = calculate_features(&context);
            let matches = evaluate_rules(std::slice::from_ref(&rule), &features)
                .map_err(internal_error("RULE_DISCOVERY_FAILED"))?;
            if matches.is_empty() {
                continue;
            }

            matched_claim_ids.push(labeled_sample.sample.external_claim_id.clone());
            if labeled_sample.confirmed_fwa {
                true_positive_count += 1;
                saving += labeled_sample.sample.claim_amount * Decimal::new(10, 2);
            } else {
                false_positive_count += 1;
            }
        }

        let support = matched_claim_ids.len();
        if support < min_support {
            continue;
        }
        let precision = if support == 0 {
            0.0
        } else {
            true_positive_count as f64 / support as f64
        };
        let recall = if positive_count == 0 {
            0.0
        } else {
            true_positive_count as f64 / positive_count as f64
        };
        let lift = if baseline_rate == 0.0 {
            0.0
        } else {
            precision / baseline_rate
        };
        let false_positive_rate = if support == 0 {
            0.0
        } else {
            false_positive_count as f64 / support as f64
        };

        candidates.push(RuleDiscoveryCandidate {
            explanation: explanation_for_candidate(&rule),
            condition_refs: condition_refs_for_rule(&rule),
            rule,
            support,
            precision,
            recall,
            lift,
            estimated_saving: format!("{:.2}", saving.round_dp(2)),
            false_positive_rate,
            matched_claim_ids,
            evidence_refs: discovery_evidence_refs.clone(),
        });
    }

    candidates.sort_by(|left, right| {
        right
            .precision
            .partial_cmp(&left.precision)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.support.cmp(&left.support))
    });

    Ok(Json(RuleDiscoveryResponse {
        sample_count,
        positive_count,
        candidates,
    }))
}

pub async fn save_rule_candidate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<SaveRuleCandidateRequest>,
) -> Result<Json<crate::repository::RuleDetailRecord>, ApiError> {
    let actor = authorize(&state, &headers)?;
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

pub async fn submit_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(rule_id): Path<String>,
    Json(request): Json<RuleLifecycleRequest>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    validate_rule_lifecycle_request(&request)?;
    update_status(state, headers, rule_id, "submitted", request.evidence_refs).await
}

pub async fn approve_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(rule_id): Path<String>,
    Json(request): Json<RuleLifecycleRequest>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    validate_rule_lifecycle_request(&request)?;
    authorize_permission(&state, &headers, "ops:rules:approve")?;
    update_status_with_required_previous(
        state,
        headers,
        rule_id,
        "approved",
        Some("submitted"),
        request.evidence_refs,
    )
    .await
}

pub async fn publish_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(rule_id): Path<String>,
    Json(request): Json<RuleLifecycleRequest>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    validate_rule_lifecycle_request(&request)?;
    let actor = authorize_permission(&state, &headers, "ops:rules:publish")?;
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
    headers: HeaderMap,
    Path(rule_id): Path<String>,
    Json(request): Json<RuleLifecycleRequest>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    validate_rule_lifecycle_request(&request)?;
    let actor = authorize_permission(&state, &headers, "ops:rules:rollback")?;
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
    headers: HeaderMap,
    rule_id: String,
    status: &'static str,
    evidence_refs: Vec<String>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    update_status_with_required_previous(state, headers, rule_id, status, None, evidence_refs).await
}

async fn update_status_with_required_previous(
    state: AppState,
    headers: HeaderMap,
    rule_id: String,
    status: &'static str,
    required_previous_status: Option<&'static str>,
    evidence_refs: Vec<String>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    let actor = authorize(&state, &headers)?;
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

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<ActorContext, ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    validate_api_key(api_key, &state.config.api_key_config()).map_err(|_| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "invalid api key",
        )
    })
}

fn authorize_permission(
    state: &AppState,
    headers: &HeaderMap,
    permission: &str,
) -> Result<ActorContext, ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    let principal =
        authenticate_api_key(api_key, &state.config.api_key_config()).map_err(|_| {
            ApiError::new(
                StatusCode::UNAUTHORIZED,
                "INVALID_API_KEY",
                "invalid api key",
            )
        })?;
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

fn candidate_rule_templates() -> Vec<Rule> {
    vec![
        Rule {
            rule_id: "candidate_early_high_amount".into(),
            version: 1,
            name: "Early high amount candidate".into(),
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
                score: 30,
                alert_code: "EARLY_HIGH_AMOUNT_CANDIDATE".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "保单生效早期发生高额理赔".into(),
            },
        },
        Rule {
            rule_id: "candidate_high_amount_ratio".into(),
            version: 1,
            name: "High amount ratio candidate".into(),
            review_mode: "both".into(),
            scheme_family: Some("high_risk_claim".into()),
            conditions: vec![Condition {
                field: "claim_amount_to_limit_ratio".into(),
                operator: ">=".into(),
                value: serde_json::json!(0.8),
            }],
            action: RuleAction {
                score: 20,
                alert_code: "HIGH_AMOUNT_RATIO_CANDIDATE".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "理赔金额接近保障额度".into(),
            },
        },
    ]
}

fn model_explanation_candidate_rules(request: &RuleDiscoveryRequest) -> Vec<Rule> {
    let min_abs_contribution = request.min_abs_contribution.unwrap_or(0.10);
    let mut explanations = request
        .model_explanations
        .iter()
        .filter(|explanation| {
            explanation.direction == "increases_risk"
                && explanation.contribution.is_finite()
                && explanation.contribution.abs() >= min_abs_contribution
                && !explanation.feature.trim().is_empty()
        })
        .collect::<Vec<_>>();
    explanations.sort_by(|left, right| {
        right
            .contribution
            .abs()
            .partial_cmp(&left.contribution.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    explanations
        .into_iter()
        .filter_map(|explanation| {
            let threshold = positive_feature_threshold(request, &explanation.feature)?;
            let feature_slug = rule_id_slug(&explanation.feature);
            Some(Rule {
                rule_id: format!("candidate_ml_{feature_slug}"),
                version: 1,
                name: format!("ML explanation candidate: {}", explanation.feature),
                review_mode: "both".into(),
                scheme_family: Some("high_risk_claim".into()),
                conditions: vec![Condition {
                    field: explanation.feature.clone(),
                    operator: ">=".into(),
                    value: serde_json::json!(threshold),
                }],
                action: RuleAction {
                    score: explanation_score(explanation.contribution),
                    alert_code: format!("ML_{}", feature_slug.to_uppercase()),
                    recommended_action: RecommendedAction::ManualReview,
                    action_class: RuleActionClass::ManualReview,
                    required_evidence: vec![],
                    adjudication_policy: None,
                    reason: format!(
                        "模型解释显示 {} 对风险贡献较高：{}",
                        explanation.feature, explanation.reason
                    ),
                },
            })
        })
        .collect()
}

fn positive_feature_threshold(request: &RuleDiscoveryRequest, feature_name: &str) -> Option<f64> {
    let mut values = request
        .samples
        .iter()
        .filter(|sample| sample.confirmed_fwa)
        .filter_map(|sample| {
            let context = sample_context(&sample.sample);
            let features = calculate_features(&context);
            features
                .get(feature_name)
                .and_then(|feature| feature.value.as_f64())
                .filter(|value| value.is_finite())
        })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    values.first().copied()
}

fn explanation_score(contribution: f64) -> u8 {
    ((contribution.abs() * 20.0).round() as u8).clamp(10, 35)
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

fn explanation_for_candidate(rule: &Rule) -> String {
    match rule.rule_id.as_str() {
        "candidate_early_high_amount" => {
            "保单生效早期且理赔金额接近保障额度，历史样本中与确认 FWA 标签更集中".into()
        }
        "candidate_high_amount_ratio" => {
            "理赔金额与保障额度比例偏高，可作为金额偏离类候选规则".into()
        }
        _ if rule.rule_id.starts_with("candidate_ml_") => {
            "由模型解释贡献项提出，仍需回测、人工审阅和发布门禁".into()
        }
        _ => "基于历史标签和可解释特征生成的候选规则".into(),
    }
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
    move |error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, code, error.to_string())
}
