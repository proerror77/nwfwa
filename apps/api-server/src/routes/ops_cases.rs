use crate::{
    app::AppState,
    auth::AuthenticatedActor,
    error::ApiError,
    repository::{
        CaseRecord, LeadRecord, TriageLeadInput, TriageLeadRecord, UpdateCaseStatusInput,
        UpdateCaseStatusRecord,
    },
    routes::pii,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct LeadListResponse {
    pub leads: Vec<LeadRecord>,
}

#[derive(Debug, Serialize)]
pub struct CaseListResponse {
    pub cases: Vec<CaseRecord>,
}

pub async fn list_leads(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<LeadListResponse>, ApiError> {
    let leads = state
        .repository
        .list_leads(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("LEAD_LIST_FAILED"))?;
    Ok(Json(LeadListResponse { leads }))
}

pub async fn triage_lead(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(lead_id): Path<String>,
    Json(mut request): Json<TriageLeadInput>,
) -> Result<Json<TriageLeadRecord>, ApiError> {
    validate_triage_request(&lead_id, &request)?;
    request.customer_scope_id = Some(actor.customer_scope_id);
    let record = state
        .repository
        .triage_lead(&lead_id, request)
        .await
        .map_err(internal_error("LEAD_TRIAGE_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "LEAD_NOT_FOUND", "lead not found"))?;
    Ok(Json(record))
}

fn validate_triage_request(lead_id: &str, request: &TriageLeadInput) -> Result<(), ApiError> {
    if !matches!(
        request.decision.as_str(),
        "open_case" | "reject_lead" | "request_evidence" | "merge_lead"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "UNSUPPORTED_TRIAGE_DECISION",
            "decision must be one of open_case, reject_lead, request_evidence, or merge_lead",
        ));
    }
    if request.decision == "merge_lead" {
        let target = request
            .merge_target_lead_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_MERGE_TARGET_LEAD",
                    "merge_target_lead_id is required for merge_lead",
                )
            })?;
        if target == lead_id {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_MERGE_TARGET_LEAD",
                "merge_target_lead_id must differ from lead_id",
            ));
        }
    }
    if request.assignee.trim().is_empty()
        || request.reviewer.trim().is_empty()
        || request.priority.trim().is_empty()
        || request.notes.trim().is_empty()
        || request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_TRIAGE_REVIEW_CONTEXT",
            "assignee, reviewer, priority, notes, and evidence_refs are required",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.notes.as_str())
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_CASE_WORKFLOW",
            "case workflow notes and evidence_refs must not contain PII",
        ));
    }
    validate_case_workflow_production_evidence_refs(
        &request.evidence_refs,
        "INVALID_TRIAGE_REVIEW_EVIDENCE",
        "triage evidence_refs must not use local dry-run or placeholder evidence",
    )?;
    Ok(())
}

pub async fn list_cases(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<CaseListResponse>, ApiError> {
    let cases = state
        .repository
        .list_cases(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("CASE_LIST_FAILED"))?;
    Ok(Json(CaseListResponse { cases }))
}

pub async fn update_case_status(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(case_id): Path<String>,
    Json(mut request): Json<UpdateCaseStatusInput>,
) -> Result<Json<UpdateCaseStatusRecord>, ApiError> {
    if !is_supported_case_status(&request.status) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "UNSUPPORTED_CASE_STATUS",
            "case status must be one of triage, investigating, pending_evidence, confirmed, rejected, closed",
        ));
    }
    if request.actor_id.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_CASE_STATUS_UPDATE",
            "actor_id is required",
        ));
    }
    if request.notes.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_CASE_STATUS_NOTES",
            "case status updates require notes",
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
            "MISSING_CASE_STATUS_EVIDENCE",
            "case status updates require evidence_refs",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.notes.as_str())
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_CASE_WORKFLOW",
            "case workflow notes and evidence_refs must not contain PII",
        ));
    }
    validate_case_workflow_production_evidence_refs(
        &request.evidence_refs,
        "INVALID_CASE_STATUS_EVIDENCE",
        "case status evidence_refs must not use local dry-run or placeholder evidence",
    )?;
    request.customer_scope_id = Some(actor.customer_scope_id);
    let record = state
        .repository
        .update_case_status(&case_id, request)
        .await
        .map_err(internal_error("CASE_STATUS_UPDATE_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "CASE_NOT_FOUND", "case not found"))?;
    Ok(Json(record))
}

fn is_supported_case_status(status: &str) -> bool {
    matches!(
        status,
        "triage" | "investigating" | "pending_evidence" | "confirmed" | "rejected" | "closed"
    )
}

fn validate_case_workflow_production_evidence_refs(
    evidence_refs: &[String],
    code: &'static str,
    message: &'static str,
) -> Result<(), ApiError> {
    if evidence_refs.iter().any(|reference| {
        let reference = reference.trim();
        let normalized = reference.to_ascii_lowercase();
        normalized.contains("local://")
            || normalized.contains("file://")
            || normalized.contains("://localhost")
            || normalized.contains("://127.")
            || normalized.contains("://0.0.0.0")
            || normalized.contains("://[::1]")
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
