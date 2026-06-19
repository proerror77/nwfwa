use crate::{
    app::AppState,
    auth::AuthenticatedApiPrincipal,
    error::ApiError,
    repository::{PersistedAuditEvent, RoutingPolicyRecord},
};
use axum::{extract::State, http::StatusCode, Json};
use fwa_audit::ActorContext;
use fwa_auth::AuthenticatedPrincipal;
use fwa_core::{AuditEventId, ScoringRunId};
use fwa_scoring::RoutingPolicy;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use super::ops_routing_policies_lifecycle::{
    activate_routing_policy, approve_routing_policy, rollback_routing_policy,
    routing_policy_promotion_gates, submit_routing_policy,
};

#[derive(Debug, Serialize)]
pub struct RoutingPolicyListResponse {
    pub policies: Vec<RoutingPolicyRecord>,
}

#[derive(Debug, Serialize)]
pub struct RoutingPolicyPromotionGate {
    pub label: String,
    pub passed: bool,
    pub blocker: String,
    pub evidence_source: String,
}

#[derive(Debug, Serialize)]
pub struct RoutingPolicyPromotionGatesResponse {
    pub policy_id: String,
    pub version: u32,
    pub review_mode: String,
    pub status: String,
    pub decision: String,
    pub passed_count: usize,
    pub total_count: usize,
    pub gates: Vec<RoutingPolicyPromotionGate>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveRoutingPolicyCandidateRequest {
    pub policy: RoutingPolicy,
    pub owner: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RoutingPolicyLifecycleRequest {
    pub evidence_refs: Vec<String>,
}

pub async fn list_routing_policies(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
) -> Result<Json<RoutingPolicyListResponse>, ApiError> {
    let _ = require_permission(principal, "ops:routing:read")?;
    let policies = state
        .repository
        .list_routing_policies()
        .await
        .map_err(internal_error("ROUTING_POLICY_LIST_FAILED"))?;
    Ok(Json(RoutingPolicyListResponse { policies }))
}

pub async fn save_routing_policy_candidate(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Json(request): Json<SaveRoutingPolicyCandidateRequest>,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    validate_routing_policy_candidate(&request)?;
    let actor = require_permission(principal, "ops:routing:write")?;
    let owner = request.owner.unwrap_or_else(|| "policy-ops".into());
    if let Some(existing) = state
        .repository
        .get_routing_policy(
            &request.policy.policy_id,
            request.policy.version,
            &request.policy.review_mode,
        )
        .await
        .map_err(internal_error("ROUTING_POLICY_LOAD_FAILED"))?
    {
        if existing.status == "active" {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "ROUTING_POLICY_ACTIVE_OVERWRITE_BLOCKED",
                "active routing policy versions cannot be overwritten as draft",
            ));
        }
    }
    let record = state
        .repository
        .save_routing_policy_candidate(request.policy, owner)
        .await
        .map_err(internal_error("ROUTING_POLICY_CANDIDATE_SAVE_FAILED"))?;
    record_routing_policy_audit(
        &state,
        &actor,
        RoutingPolicyAuditInput {
            record: &record,
            event_type: "routing_policy.candidate.saved",
            from_status: None,
            summary: "Routing policy candidate saved",
            evidence_refs: default_routing_policy_evidence_refs(&record),
        },
    )
    .await
    .map_err(internal_error("ROUTING_POLICY_AUDIT_SAVE_FAILED"))?;
    Ok(Json(record))
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

fn validate_routing_policy_candidate(
    request: &SaveRoutingPolicyCandidateRequest,
) -> Result<(), ApiError> {
    if request.policy.policy_id.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ROUTING_POLICY_ID",
            "policy_id is required",
        ));
    }
    if request.policy.version == 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ROUTING_POLICY_VERSION",
            "version must be greater than zero",
        ));
    }
    if !matches!(
        request.policy.review_mode.as_str(),
        "pre_payment" | "post_payment" | "both"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ROUTING_POLICY_REVIEW_MODE",
            "review_mode must be pre_payment, post_payment, or both",
        ));
    }
    if request
        .owner
        .as_ref()
        .is_some_and(|owner| owner.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ROUTING_POLICY_OWNER",
            "owner must not be blank when provided",
        ));
    }
    Ok(())
}

pub(super) struct RoutingPolicyAuditInput<'a> {
    pub record: &'a RoutingPolicyRecord,
    pub event_type: &'static str,
    pub from_status: Option<&'a str>,
    pub summary: &'static str,
    pub evidence_refs: Vec<String>,
}

pub(super) async fn record_routing_policy_audit(
    state: &AppState,
    actor: &ActorContext,
    input: RoutingPolicyAuditInput<'_>,
) -> anyhow::Result<()> {
    let payload = serde_json::json!({
        "customer_scope_id": actor.customer_scope_id,
        "policy_id": &input.record.policy_id,
        "version": input.record.version,
        "review_mode": &input.record.review_mode,
        "from_status": input.from_status,
        "to_status": &input.record.status,
        "owner": &input.record.owner,
        "risk_thresholds": &input.record.risk_thresholds,
        "confidence_thresholds": &input.record.confidence_thresholds,
        "provider_review_threshold": input.record.provider_review_threshold,
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
            event_type: input.event_type.into(),
            event_status: "succeeded".into(),
            summary: input.summary.into(),
            payload,
            evidence_refs: input.evidence_refs.into_iter().map(Value::String).collect(),
        })
        .await
}

pub(super) fn default_routing_policy_evidence_refs(record: &RoutingPolicyRecord) -> Vec<String> {
    vec![format!(
        "routing_policies:{}:v{}:{}",
        record.policy_id, record.version, record.review_mode
    )]
}

pub(super) fn internal_error<E: std::fmt::Display>(
    code: &'static str,
) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}
