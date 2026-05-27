use crate::{
    app::AppState,
    error::ApiError,
    repository::{AgentApprovalRecord, AgentRunLogRecord, PersistedAuditEvent},
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_auth::{validate_api_key, ApiKeyConfig};
use fwa_core::AuditEventId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct AgentRunLogListResponse {
    pub runs: Vec<AgentRunLogRecord>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitAgentApprovalRequest {
    pub decision: String,
    pub approver: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SubmitAgentApprovalResponse {
    pub approval: AgentApprovalRecord,
    pub audit_id: String,
}

pub async fn list_agent_runs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AgentRunLogListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let runs = state
        .repository
        .list_agent_runs()
        .await
        .map_err(internal_error("AGENT_RUN_LIST_FAILED"))?;
    Ok(Json(AgentRunLogListResponse { runs }))
}

pub async fn submit_agent_approval(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(agent_run_id): Path<String>,
    Json(request): Json<SubmitAgentApprovalRequest>,
) -> Result<Json<SubmitAgentApprovalResponse>, ApiError> {
    authorize(&state, &headers)?;
    if request.decision != "approved" && request.decision != "rejected" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_AGENT_APPROVAL_DECISION",
            "decision must be approved or rejected",
        ));
    }
    let run = state
        .repository
        .list_agent_runs()
        .await
        .map_err(internal_error("AGENT_RUN_LIST_FAILED"))?
        .into_iter()
        .find(|run| run.agent_run_id == agent_run_id)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "AGENT_RUN_NOT_FOUND",
                "agent run not found",
            )
        })?;
    let approval = AgentApprovalRecord {
        approval_id: format!("approval_{}", run.agent_run_id),
        agent_run_id: run.agent_run_id.clone(),
        proposed_action: "manual_review_required".into(),
        decision: request.decision,
        approver: request.approver,
        reason: request.reason,
        evidence_refs: request.evidence_refs,
        created_at: None,
    };
    let approval = state
        .repository
        .save_agent_approval(approval)
        .await
        .map_err(internal_error("AGENT_APPROVAL_SAVE_FAILED"))?;
    let audit_id = AuditEventId::new().to_string();
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: audit_id.clone(),
            run_id: format!("agent_approval_{}", run.agent_run_id),
            claim_id: run.claim_id,
            source_system: state.config.source_system.clone(),
            actor_id: approval.approver.clone(),
            actor_role: "operations_reviewer".into(),
            event_type: "agent.approval.decided".into(),
            event_status: "succeeded".into(),
            summary: format!("Agent approval decision: {}", approval.decision),
            payload: serde_json::to_value(&approval)
                .map_err(internal_error("AGENT_APPROVAL_ENCODE_FAILED"))?,
            evidence_refs: approval
                .evidence_refs
                .iter()
                .map(|reference| Value::String(reference.clone()))
                .collect(),
        })
        .await
        .map_err(internal_error("AGENT_APPROVAL_AUDIT_FAILED"))?;
    Ok(Json(SubmitAgentApprovalResponse { approval, audit_id }))
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
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
    .map(|_| ())
    .map_err(|_| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "invalid api key",
        )
    })
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, code, error.to_string())
}
