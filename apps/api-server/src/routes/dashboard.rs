use crate::{app::AppState, error::ApiError, repository::DashboardSummaryRecord};
use axum::{extract::State, http::HeaderMap, Json};
use fwa_auth::authenticate_api_key;

pub async fn dashboard_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<DashboardSummaryRecord>, ApiError> {
    let actor = authorize(&state, &headers)?;
    let summary = state
        .repository
        .dashboard_summary(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("DASHBOARD_SUMMARY_FAILED"))?;
    Ok(Json(summary))
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<fwa_audit::ActorContext, ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    authenticate_api_key(api_key, &state.config.api_key_config())
        .map(|principal| principal.actor)
        .map_err(|_| {
            ApiError::new(
                axum::http::StatusCode::UNAUTHORIZED,
                "INVALID_API_KEY",
                "invalid api key",
            )
        })
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| {
        ApiError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            code,
            error.to_string(),
        )
    }
}
