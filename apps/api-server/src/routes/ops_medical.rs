use crate::{
    app::AppState,
    error::ApiError,
    repository::{AuditEventListFilter, AuditHistoryEventRecord},
};
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_auth::{validate_api_key, ApiKeyConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct MedicalReviewQueueQuery {
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct MedicalReviewQueueResponse {
    pub items: Vec<MedicalReviewQueueItem>,
}

#[derive(Debug, Serialize)]
pub struct MedicalReviewQueueItem {
    pub claim_id: String,
    pub run_id: String,
    pub audit_id: String,
    pub medical_reasonableness_score: u8,
    pub review_route: String,
    pub evidence_status: String,
    pub missing_evidence: Vec<String>,
    pub item_finding_count: u32,
    pub first_item_code: Option<String>,
    pub first_issue_type: Option<String>,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

pub async fn medical_review_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<MedicalReviewQueueQuery>,
) -> Result<Json<MedicalReviewQueueResponse>, ApiError> {
    authorize(&state, &headers)?;
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: query.limit.unwrap_or(100).clamp(1, 200),
            event_type: Some("scoring.completed".into()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("MEDICAL_REVIEW_QUEUE_FAILED"))?;
    let items = events
        .iter()
        .filter_map(medical_review_item_from_event)
        .collect::<Vec<_>>();
    Ok(Json(MedicalReviewQueueResponse { items }))
}

fn medical_review_item_from_event(
    event: &AuditHistoryEventRecord,
) -> Option<MedicalReviewQueueItem> {
    let clinical = &event.payload["clinical_evidence"];
    let review_required = clinical["review_required"].as_bool().unwrap_or(false);
    let review_route = clinical["review_route"].as_str().unwrap_or_default();
    if !review_required && review_route != "medical_review" {
        return None;
    }
    let first_finding = clinical["item_findings"]
        .as_array()
        .and_then(|findings| findings.first());
    Some(MedicalReviewQueueItem {
        claim_id: event.payload["claim_id"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        run_id: event.run_id.clone(),
        audit_id: event.audit_id.clone(),
        medical_reasonableness_score: event.payload["scores"]["medical_reasonableness_score"]
            .as_u64()
            .unwrap_or(0)
            .min(100) as u8,
        review_route: review_route.to_string(),
        evidence_status: clinical["evidence_status"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        missing_evidence: json_array_to_strings(&clinical["missing_evidence"]),
        item_finding_count: clinical["item_findings"]
            .as_array()
            .map(|findings| findings.len() as u32)
            .unwrap_or(0),
        first_item_code: first_finding
            .and_then(|finding| finding["item_code"].as_str())
            .map(str::to_string),
        first_issue_type: first_finding
            .and_then(|finding| finding["issue_type"].as_str())
            .map(str::to_string),
        evidence_refs: json_array_to_strings(&clinical["evidence_refs"]),
        created_at: event.created_at.clone(),
    })
}

fn json_array_to_strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
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
