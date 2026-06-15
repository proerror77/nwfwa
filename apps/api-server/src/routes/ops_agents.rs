use crate::{
    app::AppState,
    auth::AuthenticatedActor,
    error::ApiError,
    repository::{AgentApprovalRecord, AgentRunLogRecord, PersistedAuditEvent},
    routes::pii,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
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

#[derive(Debug, Deserialize)]
pub struct CancelAgentRunRequest {
    pub canceller: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CancelAgentRunResponse {
    pub run: AgentRunLogRecord,
    pub audit_id: String,
}

pub async fn list_agent_runs(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<AgentRunLogListResponse>, ApiError> {
    let runs = state
        .repository
        .list_agent_runs(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("AGENT_RUN_LIST_FAILED"))?;
    Ok(Json(AgentRunLogListResponse { runs }))
}

pub async fn cancel_agent_run(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(agent_run_id): Path<String>,
    Json(request): Json<CancelAgentRunRequest>,
) -> Result<Json<CancelAgentRunResponse>, ApiError> {
    validate_agent_cancel_request_shape(&request)?;
    let run = state
        .repository
        .list_agent_runs(Some(&actor.customer_scope_id))
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
    validate_agent_cancel_run_evidence(&request, &run)?;
    validate_agent_run_is_cancellable(&run)?;

    let mut evidence_refs = request.evidence_refs;
    let policy_evidence_ref = format!("policy:{}", state.config.agent_policy_id);
    if !evidence_refs
        .iter()
        .any(|reference| reference == &policy_evidence_ref)
    {
        evidence_refs.push(policy_evidence_ref);
    }
    let audit_evidence_refs = evidence_refs
        .iter()
        .map(|reference| Value::String(reference.clone()))
        .collect::<Vec<_>>();
    let canceller = request.canceller;
    let reason = request.reason;

    state
        .repository
        .cancel_agent_run(&run.agent_run_id)
        .await
        .map_err(internal_error("AGENT_RUN_CANCEL_FAILED"))?;

    let cancelled_run = state
        .repository
        .list_agent_runs(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("AGENT_RUN_LIST_FAILED"))?
        .into_iter()
        .find(|candidate| candidate.agent_run_id == run.agent_run_id)
        .unwrap_or_else(|| AgentRunLogRecord {
            status: "cancelled".into(),
            ..run.clone()
        });

    let audit_id = AuditEventId::new().to_string();
    let payload = serde_json::json!({
        "agent_run_id": cancelled_run.agent_run_id,
        "investigation_id": cancelled_run.investigation_id,
        "claim_id": cancelled_run.claim_id,
        "previous_status": run.status,
        "status": cancelled_run.status,
        "canceller": canceller.clone(),
        "reason": reason,
        "customer_scope_id": actor.customer_scope_id,
        "agent_policy_id": state.config.agent_policy_id.clone(),
        "evidence_refs": evidence_refs,
    });
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: audit_id.clone(),
            run_id: format!("agent_cancel_{}", cancelled_run.agent_run_id),
            claim_id: cancelled_run.claim_id.clone(),
            source_system: state.config.source_system.clone(),
            actor_id: canceller,
            actor_role: "operations_reviewer".into(),
            event_type: "agent.run.cancelled".into(),
            event_status: "succeeded".into(),
            summary: format!("Agent run cancelled: {}", cancelled_run.agent_run_id),
            evidence_refs: audit_evidence_refs,
            payload,
        })
        .await
        .map_err(internal_error("AGENT_CANCEL_AUDIT_FAILED"))?;

    Ok(Json(CancelAgentRunResponse {
        run: cancelled_run,
        audit_id,
    }))
}

pub async fn submit_agent_approval(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(agent_run_id): Path<String>,
    Json(request): Json<SubmitAgentApprovalRequest>,
) -> Result<Json<SubmitAgentApprovalResponse>, ApiError> {
    validate_agent_approval_request(&request)?;
    let run = state
        .repository
        .list_agent_runs(Some(&actor.customer_scope_id))
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
    validate_agent_approval_run_evidence(&request, &run)?;
    validate_agent_approval_is_pending(&run)?;
    let mut evidence_refs = request.evidence_refs;
    let policy_evidence_ref = format!("policy:{}", state.config.agent_policy_id);
    if !evidence_refs
        .iter()
        .any(|reference| reference == &policy_evidence_ref)
    {
        evidence_refs.push(policy_evidence_ref);
    }
    let approval = AgentApprovalRecord {
        approval_id: format!("approval_{}", run.agent_run_id),
        agent_run_id: run.agent_run_id.clone(),
        proposed_action: "manual_review_required".into(),
        decision: request.decision,
        approver: request.approver,
        reason: request.reason,
        evidence_refs,
        created_at: None,
    };
    let approval = state
        .repository
        .save_agent_approval(approval)
        .await
        .map_err(internal_error("AGENT_APPROVAL_SAVE_FAILED"))?;
    let audit_id = AuditEventId::new().to_string();
    let mut payload =
        serde_json::to_value(&approval).map_err(internal_error("AGENT_APPROVAL_ENCODE_FAILED"))?;
    if let Some(payload) = payload.as_object_mut() {
        payload.insert(
            "customer_scope_id".into(),
            Value::String(actor.customer_scope_id),
        );
        payload.insert(
            "agent_policy_id".into(),
            Value::String(state.config.agent_policy_id.clone()),
        );
    }
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
            payload,
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

fn validate_agent_approval_run_evidence(
    request: &SubmitAgentApprovalRequest,
    run: &AgentRunLogRecord,
) -> Result<(), ApiError> {
    let required_ref = format!("agent_run:{}", run.agent_run_id);
    if request
        .evidence_refs
        .iter()
        .any(|reference| reference == &required_ref)
    {
        return Ok(());
    }
    Err(ApiError::new(
        StatusCode::BAD_REQUEST,
        "MISSING_AGENT_APPROVAL_RUN_EVIDENCE",
        format!("agent approval evidence_refs must include {required_ref}"),
    ))
}

fn validate_agent_approval_is_pending(run: &AgentRunLogRecord) -> Result<(), ApiError> {
    if run.approvals.iter().any(|approval| {
        approval.proposed_action == "manual_review_required" && approval.decision == "pending"
    }) {
        return Ok(());
    }
    Err(ApiError::new(
        StatusCode::CONFLICT,
        "AGENT_APPROVAL_NOT_PENDING",
        "agent approval has already been decided or is not pending",
    ))
}

fn validate_agent_cancel_run_evidence(
    request: &CancelAgentRunRequest,
    run: &AgentRunLogRecord,
) -> Result<(), ApiError> {
    let required_ref = format!("agent_run:{}", run.agent_run_id);
    if request
        .evidence_refs
        .iter()
        .any(|reference| reference == &required_ref)
    {
        return Ok(());
    }
    Err(ApiError::new(
        StatusCode::BAD_REQUEST,
        "MISSING_AGENT_CANCEL_RUN_EVIDENCE",
        format!("agent cancellation evidence_refs must include {required_ref}"),
    ))
}

fn validate_agent_run_is_cancellable(run: &AgentRunLogRecord) -> Result<(), ApiError> {
    if run.status == "queued" || run.status == "running" {
        return Ok(());
    }
    Err(ApiError::new(
        StatusCode::CONFLICT,
        "AGENT_RUN_NOT_CANCELLABLE",
        "only queued or running agent runs can be cancelled",
    ))
}

fn validate_agent_cancel_request_shape(request: &CancelAgentRunRequest) -> Result<(), ApiError> {
    if request.canceller.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_AGENT_CANCELLER",
            "canceller is required",
        ));
    }
    if request.reason.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_AGENT_CANCEL_REASON",
            "cancellation reason is required",
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
            "MISSING_AGENT_CANCEL_EVIDENCE",
            "agent cancellation decisions require evidence_refs",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.reason.as_str())
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_AGENT_CANCEL",
            "agent cancellation reason and evidence_refs must not contain PII",
        ));
    }
    validate_agent_production_evidence_refs(
        &request.evidence_refs,
        "INVALID_AGENT_CANCEL_EVIDENCE",
        "agent cancellation evidence_refs must not use local dry-run or placeholder evidence",
    )?;
    Ok(())
}

fn validate_agent_approval_request(request: &SubmitAgentApprovalRequest) -> Result<(), ApiError> {
    if request.decision != "approved" && request.decision != "rejected" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_AGENT_APPROVAL_DECISION",
            "decision must be approved or rejected",
        ));
    }
    if request.approver.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_AGENT_APPROVER",
            "approver is required",
        ));
    }
    if request.reason.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_AGENT_APPROVAL_REASON",
            "approval reason is required",
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
            "MISSING_AGENT_APPROVAL_EVIDENCE",
            "agent approval decisions require evidence_refs",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.reason.as_str())
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_AGENT_APPROVAL",
            "agent approval reason and evidence_refs must not contain PII",
        ));
    }
    validate_agent_production_evidence_refs(
        &request.evidence_refs,
        "INVALID_AGENT_APPROVAL_EVIDENCE",
        "agent approval evidence_refs must not use local dry-run or placeholder evidence",
    )?;
    Ok(())
}

fn validate_agent_production_evidence_refs(
    evidence_refs: &[String],
    code: &'static str,
    message: &'static str,
) -> Result<(), ApiError> {
    if evidence_refs.iter().any(|reference| {
        let reference = reference.trim();
        reference.contains("local://")
            || reference.contains("file://")
            || reference.contains('{')
            || reference.contains('}')
    }) {
        Err(ApiError::new(StatusCode::BAD_REQUEST, code, message))
    } else {
        Ok(())
    }
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}
