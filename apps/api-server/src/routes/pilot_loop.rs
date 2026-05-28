use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        InvestigationResultRecord, MemberProfileSummaryRecord, OutcomeLabelRecord,
        QaFeedbackItemRecord, QaReviewRecord,
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
pub struct PilotWritebackResponse {
    pub claim_id: String,
    pub event_type: String,
    pub event_status: String,
    pub audit_id: String,
    pub run_id: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ClaimAuditHistoryResponse {
    pub claim_id: String,
    pub events: Vec<crate::repository::AuditHistoryEventRecord>,
}

#[derive(Debug, Serialize)]
pub struct QaFeedbackItemListResponse {
    pub items: Vec<QaFeedbackItemRecord>,
}

#[derive(Debug, Serialize)]
pub struct OutcomeLabelListResponse {
    pub labels: Vec<OutcomeLabelRecord>,
}

pub async fn member_profile_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(member_id): Path<String>,
) -> Result<Json<MemberProfileSummaryRecord>, ApiError> {
    authorize(&state, &headers)?;
    let profile = state
        .repository
        .member_profile_summary(&member_id)
        .await
        .map_err(internal_error("MEMBER_PROFILE_SUMMARY_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MEMBER_NOT_FOUND",
                "member not found",
            )
        })?;
    Ok(Json(profile))
}

pub async fn write_investigation_result(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<InvestigationResultRecord>,
) -> Result<Json<PilotWritebackResponse>, ApiError> {
    authorize(&state, &headers)?;
    let claim_id = request.claim_id.clone();
    let event = state
        .repository
        .save_investigation_result(request)
        .await
        .map_err(internal_error("INVESTIGATION_RESULT_SAVE_FAILED"))?;
    Ok(Json(PilotWritebackResponse {
        claim_id,
        event_type: event.event_type,
        event_status: event.event_status,
        audit_id: event.audit_id,
        run_id: event.run_id,
        evidence_refs: event.evidence_refs,
    }))
}

pub async fn write_qa_result(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<QaReviewRecord>,
) -> Result<Json<PilotWritebackResponse>, ApiError> {
    authorize(&state, &headers)?;
    let claim_id = request.claim_id.clone();
    let event = state
        .repository
        .save_qa_review(request)
        .await
        .map_err(internal_error("QA_RESULT_SAVE_FAILED"))?;
    Ok(Json(PilotWritebackResponse {
        claim_id,
        event_type: event.event_type,
        event_status: event.event_status,
        audit_id: event.audit_id,
        run_id: event.run_id,
        evidence_refs: event.evidence_refs,
    }))
}

pub async fn list_qa_feedback_items(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<QaFeedbackItemListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let items = state
        .repository
        .list_qa_feedback_items()
        .await
        .map_err(internal_error("QA_FEEDBACK_LIST_FAILED"))?;
    Ok(Json(QaFeedbackItemListResponse { items }))
}

pub async fn list_outcome_labels(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OutcomeLabelListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let labels = state
        .repository
        .list_outcome_labels()
        .await
        .map_err(internal_error("OUTCOME_LABEL_LIST_FAILED"))?;
    Ok(Json(OutcomeLabelListResponse { labels }))
}

pub async fn claim_audit_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(claim_id): Path<String>,
) -> Result<Json<ClaimAuditHistoryResponse>, ApiError> {
    authorize(&state, &headers)?;
    let events = state
        .repository
        .claim_audit_history(&claim_id)
        .await
        .map_err(internal_error("CLAIM_AUDIT_HISTORY_FAILED"))?;
    Ok(Json(ClaimAuditHistoryResponse { claim_id, events }))
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
