use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        CaseRecord, LeadRecord, TriageLeadInput, TriageLeadRecord, UpdateCaseStatusInput,
        UpdateCaseStatusRecord,
    },
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_auth::{validate_api_key, ApiKeyConfig};
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
    headers: HeaderMap,
) -> Result<Json<LeadListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let leads = state
        .repository
        .list_leads()
        .await
        .map_err(internal_error("LEAD_LIST_FAILED"))?;
    Ok(Json(LeadListResponse { leads }))
}

pub async fn triage_lead(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(lead_id): Path<String>,
    Json(request): Json<TriageLeadInput>,
) -> Result<Json<TriageLeadRecord>, ApiError> {
    authorize(&state, &headers)?;
    if !is_supported_triage_decision(&request.decision) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "UNSUPPORTED_TRIAGE_DECISION",
            "decision must be one of open_case, reject_lead, or request_evidence",
        ));
    }
    let record = state
        .repository
        .triage_lead(&lead_id, request)
        .await
        .map_err(internal_error("LEAD_TRIAGE_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "LEAD_NOT_FOUND", "lead not found"))?;
    Ok(Json(record))
}

fn is_supported_triage_decision(decision: &str) -> bool {
    matches!(decision, "open_case" | "reject_lead" | "request_evidence")
}

pub async fn list_cases(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CaseListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let cases = state
        .repository
        .list_cases()
        .await
        .map_err(internal_error("CASE_LIST_FAILED"))?;
    Ok(Json(CaseListResponse { cases }))
}

pub async fn update_case_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(case_id): Path<String>,
    Json(request): Json<UpdateCaseStatusInput>,
) -> Result<Json<UpdateCaseStatusRecord>, ApiError> {
    authorize(&state, &headers)?;
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
