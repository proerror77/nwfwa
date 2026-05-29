use crate::{
    app::AppState,
    error::ApiError,
    repository::{AuditSampleRecord, CreateAuditSampleInput},
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_auth::{validate_api_key, ApiKeyConfig};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AuditSampleListResponse {
    pub samples: Vec<AuditSampleRecord>,
}

pub async fn list_audit_samples(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuditSampleListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let samples = state
        .repository
        .list_audit_samples()
        .await
        .map_err(internal_error("AUDIT_SAMPLE_LIST_FAILED"))?;
    Ok(Json(AuditSampleListResponse { samples }))
}

pub async fn create_audit_sample(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateAuditSampleInput>,
) -> Result<Json<AuditSampleRecord>, ApiError> {
    authorize(&state, &headers)?;
    if !matches!(
        request.sample_mode.as_str(),
        "risk_ranked" | "random_control" | "stratified" | "post_payment_audit" | "qa_calibration"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SAMPLE_MODE",
            "sample_mode must be risk_ranked, random_control, stratified, post_payment_audit, or qa_calibration",
        ));
    }
    if request.population_definition.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_POPULATION_DEFINITION",
            "population_definition is required",
        ));
    }
    if request.sample_size == 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SAMPLE_SIZE",
            "sample_size must be greater than zero",
        ));
    }
    if request.reviewer.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SAMPLE_REVIEWER",
            "reviewer is required",
        ));
    }
    if request.assignment_queue.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ASSIGNMENT_QUEUE",
            "assignment_queue is required",
        ));
    }
    let sample = state
        .repository
        .create_audit_sample(request)
        .await
        .map_err(internal_error("AUDIT_SAMPLE_CREATE_FAILED"))?;
    Ok(Json(sample))
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
