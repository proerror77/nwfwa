use crate::{app::AppState, error::ApiError};
use axum::{extract::State, http::HeaderMap, Json};
use fwa_auth::validate_api_key;
use fwa_core::{fwa_scheme_taxonomy, FwaSchemeDefinition};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FwaSchemeListResponse {
    pub schemes: Vec<FwaSchemeDefinition>,
    pub scheme_count: usize,
}

pub async fn list_fwa_schemes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<FwaSchemeListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let schemes = fwa_scheme_taxonomy();
    Ok(Json(FwaSchemeListResponse {
        scheme_count: schemes.len(),
        schemes,
    }))
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
