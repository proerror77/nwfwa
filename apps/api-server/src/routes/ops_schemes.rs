use crate::{auth::AuthenticatedActor, error::ApiError};
use axum::Json;
use fwa_core::{fwa_scheme_taxonomy, FwaSchemeDefinition};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FwaSchemeListResponse {
    pub schemes: Vec<FwaSchemeDefinition>,
    pub scheme_count: usize,
}

pub async fn list_fwa_schemes(
    _actor: AuthenticatedActor,
) -> Result<Json<FwaSchemeListResponse>, ApiError> {
    let schemes = fwa_scheme_taxonomy();
    Ok(Json(FwaSchemeListResponse {
        scheme_count: schemes.len(),
        schemes,
    }))
}
