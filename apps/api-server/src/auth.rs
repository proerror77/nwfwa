use crate::{app::AppState, error::ApiError};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use fwa_audit::ActorContext;
use fwa_auth::{authenticate_api_key, AuthenticatedPrincipal};

pub struct AuthenticatedActor(pub ActorContext);

pub struct AuthenticatedApiPrincipal(pub AuthenticatedPrincipal);

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedActor {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        authenticate_parts(parts, state).map(|principal| Self(principal.actor))
    }
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedApiPrincipal {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        authenticate_parts(parts, state).map(Self)
    }
}

fn authenticate_parts(parts: &Parts, state: &AppState) -> Result<AuthenticatedPrincipal, ApiError> {
    let api_key = parts
        .headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    authenticate_api_key(api_key, &state.config.api_key_config()).map_err(|_| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "invalid api key",
        )
    })
}
