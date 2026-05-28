use crate::{
    app::AppState,
    error::ApiError,
    repository::{PersistedAuditEvent, RoutingPolicyRecord},
};
use axum::{
    extract::{Path, State},
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
        },
    )
    .await
    .map_err(internal_error("ROUTING_POLICY_AUDIT_SAVE_FAILED"))?;
    Ok(Json(record))
}

pub async fn submit_routing_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((policy_id, review_mode, version)): Path<(String, String, u32)>,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    update_routing_policy_status(
        state,
        headers,
        policy_id,
        version,
        review_mode,
        "draft",
        "submitted",
    )
    .await
}

pub async fn approve_routing_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((policy_id, review_mode, version)): Path<(String, String, u32)>,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    update_routing_policy_status(
        state,
        headers,
        policy_id,
        version,
        review_mode,
        "submitted",
        "approved",
    )
    .await
}

pub async fn activate_routing_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((policy_id, review_mode, version)): Path<(String, String, u32)>,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    let actor = authorize(&state, &headers)?;
    let previous = load_routing_policy(&state, &policy_id, version, &review_mode).await?;
    require_status(&previous, "approved", "ROUTING_POLICY_APPROVAL_REQUIRED")?;
    let record = state
        .repository
        .activate_routing_policy(&policy_id, version, &review_mode)
        .await
        .map_err(internal_error("ROUTING_POLICY_STATUS_UPDATE_FAILED"))?
        .ok_or_else(routing_policy_not_found)?;
    record_routing_policy_audit(
        &state,
        &actor,
        RoutingPolicyAuditInput {
            record: &record,
            event_type: "routing_policy.activation.completed",
            from_status: Some(&previous.status),
            summary: "Routing policy activation completed",
        },
    )
    .await
    .map_err(internal_error("ROUTING_POLICY_AUDIT_SAVE_FAILED"))?;
    Ok(Json(record))
}

pub async fn rollback_routing_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((policy_id, review_mode, version)): Path<(String, String, u32)>,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    update_routing_policy_status(
        state,
        headers,
        policy_id,
        version,
        review_mode,
        "active",
        "approved",
    )
    .await
}

async fn update_routing_policy_status(
    state: AppState,
    headers: HeaderMap,
    policy_id: String,
    version: u32,
    review_mode: String,
    required_status: &'static str,
    next_status: &'static str,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    let actor = authorize(&state, &headers)?;
    let previous = load_routing_policy(&state, &policy_id, version, &review_mode).await?;
    require_status(&previous, required_status, "ROUTING_POLICY_STATUS_REQUIRED")?;
    let record = state
        .repository
        .update_routing_policy_status(&policy_id, version, &review_mode, next_status)
        .await
        .map_err(internal_error("ROUTING_POLICY_STATUS_UPDATE_FAILED"))?
        .ok_or_else(routing_policy_not_found)?;
    record_routing_policy_audit(
        &state,
        &actor,
        RoutingPolicyAuditInput {
            record: &record,
            event_type: if next_status == "approved" && required_status == "active" {
                "routing_policy.rollback.completed"
            } else {
                "routing_policy.status.changed"
            },
            from_status: Some(&previous.status),
            summary: if next_status == "approved" && required_status == "active" {
                "Routing policy rollback completed"
            } else {
                "Routing policy status changed"
            },
        },
    )
    .await
    .map_err(internal_error("ROUTING_POLICY_AUDIT_SAVE_FAILED"))?;
    Ok(Json(record))
}

async fn load_routing_policy(
    state: &AppState,
    policy_id: &str,
    version: u32,
    review_mode: &str,
) -> Result<RoutingPolicyRecord, ApiError> {
    state
        .repository
        .get_routing_policy(policy_id, version, review_mode)
        .await
        .map_err(internal_error("ROUTING_POLICY_LOAD_FAILED"))?
        .ok_or_else(routing_policy_not_found)
}

fn require_status(
    record: &RoutingPolicyRecord,
    required_status: &'static str,
    code: &'static str,
) -> Result<(), ApiError> {
    if record.status == required_status {
        return Ok(());
    }
    Err(ApiError::new(
        StatusCode::CONFLICT,
        code,
        format!(
            "routing policy {}:{}:{} must be {required_status}",
            record.policy_id, record.review_mode, record.version
        ),
    ))
}

fn routing_policy_not_found() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "ROUTING_POLICY_NOT_FOUND",
        "routing policy not found",
    )
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

struct RoutingPolicyAuditInput<'a> {
    record: &'a RoutingPolicyRecord,
    event_type: &'static str,
    from_status: Option<&'a str>,
    summary: &'static str,
}

async fn record_routing_policy_audit(
    state: &AppState,
    actor: &ActorContext,
    input: RoutingPolicyAuditInput<'_>,
) -> anyhow::Result<()> {
    let payload = serde_json::json!({
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
            evidence_refs: vec![Value::String(format!(
                "routing_policies:{}:v{}:{}",
                input.record.policy_id, input.record.version, input.record.review_mode
            ))],
        })
        .await
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, code, error.to_string())
}
