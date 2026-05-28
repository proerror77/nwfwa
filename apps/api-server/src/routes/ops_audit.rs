use crate::{app::AppState, error::ApiError, repository::AuditHistoryEventRecord};
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
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let events = state
        .repository
        .list_audit_events(limit)
        .await
        .map_err(internal_error("AUDIT_EVENT_LIST_FAILED"))?;
    Ok(Json(AuditEventListResponse { events }))
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
