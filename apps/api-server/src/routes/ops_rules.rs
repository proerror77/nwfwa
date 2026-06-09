use super::ops_rules_audit::{
    default_rule_evidence_refs, record_rule_audit, record_rule_backtest_audit,
    record_rule_promotion_audit, record_rule_shadow_run_audit, RuleAuditInput,
};
use super::ops_rules_gates::{
    build_rule_promotion_gates, empty_rule_performance, latest_rule_action,
};
use super::ops_rules_mining::{
    backtest_mining_samples, discovery_mining_samples, feature_map_from_mining_sample,
    mine_statistical_rule_candidates, normalized_optional_str, rule_discovery_evidence_refs,
    rule_id_slug,
};
use crate::{
    app::AppState,
    auth::{AuthenticatedActor, AuthenticatedApiPrincipal},
    error::ApiError,
    repository::{
        PersistedAuditEvent, RuleBacktestRecord, RuleConditionLibraryRecord,
        RulePromotionReviewRecord, RuleShadowRunRecord,
    },
    routes::pii,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use fwa_audit::ActorContext;
use fwa_auth::AuthenticatedPrincipal;
use fwa_core::{canonical_scheme_family, AuditEventId};
use fwa_rules::{evaluate_rules, Rule};
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
    state.scoring_lookup_cache.invalidate_all().await;
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
    state.scoring_lookup_cache.invalidate_all().await;
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
