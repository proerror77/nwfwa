use crate::{app::AppState, error::ApiError, repository::AgentRunLogRecord};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_auth::{validate_api_key, ApiKeyConfig};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AgentRunLogListResponse {
    pub runs: Vec<AgentRunLogRecord>,
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
