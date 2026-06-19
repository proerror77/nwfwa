use crate::{auth::AuthenticatedApiPrincipal, error::ApiError};
use axum::{http::StatusCode, Json};
use fwa_auth::AuthenticatedPrincipal;
use fwa_core::{fwa_scheme_taxonomy, FwaSchemeDefinition};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FwaSchemeListResponse {
    pub schemes: Vec<FwaSchemeDefinition>,
    pub scheme_count: usize,
}

pub async fn list_fwa_schemes(
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
) -> Result<Json<FwaSchemeListResponse>, ApiError> {
    require_permission(principal, "ops:schemes:read")?;
    let schemes = fwa_scheme_taxonomy();
    Ok(Json(FwaSchemeListResponse {
        scheme_count: schemes.len(),
        schemes,
    }))
}

fn require_permission(principal: AuthenticatedPrincipal, permission: &str) -> Result<(), ApiError> {
    if !principal.has_permission(permission) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "PERMISSION_DENIED",
            format!("missing permission: {permission}"),
        ));
    }
    Ok(())
}
