use crate::{app::AppState, error::ApiError, repository::ProviderRiskSummaryRecord};
use axum::{extract::State, http::HeaderMap, Json};
use fwa_auth::validate_api_key;

pub async fn provider_risk_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ProviderRiskSummaryRecord>, ApiError> {
    authorize(&state, &headers)?;
    let summary = state
        .repository
        .provider_risk_summary()
        .await
        .map_err(internal_error("PROVIDER_RISK_SUMMARY_FAILED"))?;
    Ok(Json(summary))
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    validate_api_key(api_key, &state.config.api_key_config())
        .map(|_| ())
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
