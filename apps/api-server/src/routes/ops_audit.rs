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

#[derive(Debug, Deserialize)]
pub struct AuditEventListQuery {
    pub limit: Option<u32>,
    pub event_group: Option<String>,
    pub event_type: Option<String>,
    pub actor_id: Option<String>,
    pub run_id: Option<String>,
    pub claim_id: Option<String>,
    pub routing_policy_id: Option<String>,
    pub routing_policy_version: Option<String>,
    pub review_mode: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuditEventListResponse {
    pub events: Vec<AuditHistoryEventRecord>,
}

pub async fn list_audit_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuditEventListQuery>,
) -> Result<Json<AuditEventListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let filter = AuditEventListFilter {
        limit: query.limit.unwrap_or(50).clamp(1, 200),
        event_group: normalize_filter(query.event_group),
        event_type: normalize_filter(query.event_type),
        actor_id: normalize_filter(query.actor_id),
        run_id: normalize_filter(query.run_id),
        claim_id: normalize_filter(query.claim_id),
        routing_policy_id: normalize_filter(query.routing_policy_id),
        routing_policy_version: normalize_filter(query.routing_policy_version),
        review_mode: normalize_filter(query.review_mode),
    };
    let events = state
        .repository
        .list_audit_events(filter)
        .await
        .map_err(internal_error("AUDIT_EVENT_LIST_FAILED"))?;
    Ok(Json(AuditEventListResponse { events }))
}

fn normalize_filter(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
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
