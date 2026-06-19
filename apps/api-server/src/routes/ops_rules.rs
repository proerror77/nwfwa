use super::ops_rules_gates::{
    build_rule_promotion_gates, empty_rule_performance, latest_rule_action,
};
use crate::{app::AppState, auth::AuthenticatedApiPrincipal, error::ApiError};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use fwa_audit::ActorContext;
use fwa_auth::AuthenticatedPrincipal;

pub use super::ops_rules_backtest::{backtest_rule, submit_rule_shadow_run};
pub use super::ops_rules_lifecycle::{
    approve_rule, publish_rule, rollback_rule, submit_rule, submit_rule_promotion_review,
};
pub use super::ops_rules_mining::{discover_rules, review_rule_candidate, save_rule_candidate};
pub use super::ops_rules_types::*;

pub async fn list_rules(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
) -> Result<Json<RuleListResponse>, ApiError> {
    let _ = require_permission(principal, "ops:rules:read")?;
    let rules = state
        .repository
        .list_rules()
        .await
        .map_err(internal_error("RULE_LIST_FAILED"))?;
    Ok(Json(RuleListResponse { rules }))
}

pub async fn rule_performance(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
) -> Result<Json<RulePerformanceResponse>, ApiError> {
    let _ = require_permission(principal, "ops:rules:read")?;
    let rules = state
        .repository
        .rule_performance()
        .await
        .map_err(internal_error("RULE_PERFORMANCE_FAILED"))?;
    Ok(Json(RulePerformanceResponse { rules }))
}

pub async fn list_rule_conditions(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
) -> Result<Json<RuleConditionLibraryResponse>, ApiError> {
    let _ = require_permission(principal, "ops:rules:read")?;
    let conditions = state
        .repository
        .list_rule_conditions()
        .await
        .map_err(internal_error("RULE_CONDITION_LIST_FAILED"))?;
    Ok(Json(RuleConditionLibraryResponse { conditions }))
}

pub async fn rule_promotion_gates(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(rule_id): Path<String>,
) -> Result<Json<RulePromotionGatesResponse>, ApiError> {
    let _ = require_permission(principal, "ops:rules:read")?;
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

pub async fn get_rule(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(rule_id): Path<String>,
) -> Result<Json<crate::repository::RuleDetailRecord>, ApiError> {
    let _ = require_permission(principal, "ops:rules:read")?;
    let rule = state
        .repository
        .get_rule(&rule_id)
        .await
        .map_err(internal_error("RULE_LOAD_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?;
    Ok(Json(rule))
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

pub(super) fn bad_request<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::new(StatusCode::BAD_REQUEST, code, error.to_string())
}
