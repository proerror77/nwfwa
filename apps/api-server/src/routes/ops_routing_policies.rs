use crate::{
    app::AppState,
    error::ApiError,
    repository::{PersistedAuditEvent, RoutingPolicyRecord},
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_audit::ActorContext;
use fwa_auth::{validate_api_key, ApiKeyConfig};
use fwa_core::{AuditEventId, ScoringRunId};
use fwa_scoring::RoutingPolicy;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct RoutingPolicyListResponse {
    pub policies: Vec<RoutingPolicyRecord>,
}

#[derive(Debug, Deserialize)]
pub struct SaveRoutingPolicyCandidateRequest {
    pub policy: RoutingPolicy,
    pub owner: Option<String>,
}

pub async fn list_routing_policies(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RoutingPolicyListResponse>, ApiError> {
    let _actor = authorize(&state, &headers)?;
    let policies = state
        .repository
        .list_routing_policies()
        .await
        .map_err(internal_error("ROUTING_POLICY_LIST_FAILED"))?;
    Ok(Json(RoutingPolicyListResponse { policies }))
}

pub async fn save_routing_policy_candidate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SaveRoutingPolicyCandidateRequest>,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    let actor = authorize(&state, &headers)?;
    let owner = request.owner.unwrap_or_else(|| "policy-ops".into());
    let record = state
        .repository
        .save_routing_policy_candidate(request.policy, owner)
        .await
        .map_err(internal_error("ROUTING_POLICY_CANDIDATE_SAVE_FAILED"))?;
    record_routing_policy_audit(&state, &actor, &record)
        .await
        .map_err(internal_error("ROUTING_POLICY_AUDIT_SAVE_FAILED"))?;
    Ok(Json(record))
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<ActorContext, ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    validate_api_key(
        api_key,
        &ApiKeyConfig {
            key: state.config.api_key.clone(),
            source_system: state.config.source_system.clone(),
        },
    )
    .map_err(|_| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "invalid api key",
        )
    })
}

async fn record_routing_policy_audit(
    state: &AppState,
    actor: &ActorContext,
    record: &RoutingPolicyRecord,
) -> anyhow::Result<()> {
    let payload = serde_json::to_value(record)?;
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "routing_policy.candidate.saved".into(),
            event_status: "succeeded".into(),
            summary: "Routing policy candidate saved".into(),
            payload,
            evidence_refs: vec![Value::String(format!(
                "routing_policies:{}:v{}:{}",
                record.policy_id, record.version, record.review_mode
            ))],
        })
        .await
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, code, error.to_string())
}
