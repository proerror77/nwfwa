use super::{
    ops_rules::{internal_error, load_rule_promotion_gates, require_permission},
    ops_rules_audit::{record_rule_audit, record_rule_promotion_audit, RuleAuditInput},
    ops_rules_types::{
        RuleLifecycleRequest, RuleLifecycleResponse, SubmitRulePromotionReviewRequest,
    },
    ops_rules_validation::validate_rule_lifecycle_request,
};
use crate::{
    app::AppState, auth::AuthenticatedApiPrincipal, error::ApiError,
    repository::RulePromotionReviewRecord, routes::pii,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use fwa_audit::ActorContext;

pub async fn submit_rule(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(rule_id): Path<String>,
    Json(request): Json<RuleLifecycleRequest>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    validate_rule_lifecycle_request(&request)?;
    let actor = require_permission(principal, "ops:rules:write")?;
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
        true,
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
        .update_rule_status(&rule_id, "active", None)
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
        .update_rule_status(&rule_id, "approved", None)
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
    update_status_with_required_previous(state, actor, rule_id, status, None, false, evidence_refs)
        .await
}

async fn update_status_with_required_previous(
    state: AppState,
    actor: ActorContext,
    rule_id: String,
    status: &'static str,
    required_previous_status: Option<&'static str>,
    enforce_four_eyes: bool,
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
    if enforce_four_eyes {
        match previous.submitted_by_actor_id.as_deref() {
            Some(submitter) if submitter != actor.actor_id => {}
            Some(_) => {
                return Err(ApiError::new(
                    StatusCode::CONFLICT,
                    "RULE_APPROVER_MUST_DIFFER_FROM_SUBMITTER",
                    "rule approval requires a different actor from the submitter",
                ));
            }
            None => {
                return Err(ApiError::new(
                    StatusCode::CONFLICT,
                    "RULE_SUBMITTER_REQUIRED",
                    "rule must record a submitter before approval",
                ));
            }
        }
    }
    let rule = state
        .repository
        .update_rule_status(
            &rule_id,
            status,
            (status == "submitted").then_some(actor.actor_id.as_str()),
        )
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
