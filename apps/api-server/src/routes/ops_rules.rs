use super::ops_rules_audit::{
    default_rule_evidence_refs, record_rule_audit, record_rule_backtest_audit,
    record_rule_promotion_audit, record_rule_shadow_run_audit, RuleAuditInput,
};
use super::ops_rules_backtest::{build_rule_backtest_response, rule_backtest_record_from_response};
use super::ops_rules_gates::{
    build_rule_promotion_gates, empty_rule_performance, latest_rule_action,
};
use super::ops_rules_mining::{
    mine_statistical_rule_candidates, rule_discovery_evidence_refs, rule_id_slug,
};
use super::ops_rules_mining_samples::discovery_mining_samples;
use super::ops_rules_validation::{
    candidate_review_outcome, validate_candidate_review_backtest_evidence,
    validate_candidate_review_shadow_evidence, validate_rule_candidate,
    validate_rule_shadow_run_request,
};
use crate::{
    app::AppState,
    auth::{AuthenticatedActor, AuthenticatedApiPrincipal},
    error::ApiError,
    repository::{PersistedAuditEvent, RulePromotionReviewRecord, RuleShadowRunRecord},
    routes::pii,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use fwa_audit::ActorContext;
use fwa_auth::AuthenticatedPrincipal;
use fwa_core::AuditEventId;

pub use super::ops_rules_lifecycle::{approve_rule, publish_rule, rollback_rule, submit_rule};
pub use super::ops_rules_types::*;

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

pub(super) async fn load_rule_promotion_gates(
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
    let response = build_rule_backtest_response(&request)?;
    let record = state
        .repository
        .save_rule_backtest(rule_backtest_record_from_response(
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

pub(super) fn require_permission(
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

pub(super) fn internal_error<E: std::fmt::Display>(
    code: &'static str,
) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}

fn bad_request<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::new(StatusCode::BAD_REQUEST, code, error.to_string())
}
