use crate::{
    app::AppState,
    error::ApiError,
    repository::{AuditSampleRecord, CreateAuditSampleInput, PersistedAuditEvent},
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_auth::validate_api_key;
use fwa_core::AuditEventId;
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
    record_audit_sample_created(&state, &sample)
        .await
        .map_err(internal_error("AUDIT_SAMPLE_AUDIT_FAILED"))?;
    Ok(Json(sample))
}

async fn record_audit_sample_created(
    state: &AppState,
    sample: &AuditSampleRecord,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: format!("audit_sample_{}", sample.sample_id),
            claim_id: String::new(),
            source_system: state.config.source_system.clone(),
            actor_id: state.config.source_system.clone(),
            actor_role: "fwa_operator".into(),
            event_type: "audit_sample.created".into(),
            event_status: "succeeded".into(),
            summary: format!("Audit sample created: {}", sample.sample_mode),
            payload: serde_json::json!({
                "sample_id": sample.sample_id,
                "sample_mode": sample.sample_mode,
                "population_definition": sample.population_definition,
                "inclusion_criteria": sample.inclusion_criteria,
                "deterministic_seed": sample.deterministic_seed,
                "selection_method": sample.selection_method,
                "sample_size": sample.sample_size,
                "reviewer": sample.reviewer,
                "assignment_queue": sample.assignment_queue,
                "selected_lead_count": sample.selected_leads.len(),
                "outcome_distribution": sample.outcome_distribution
            }),
            evidence_refs: vec![serde_json::Value::String(format!(
                "audit_samples:{}",
                sample.sample_id
            ))],
        })
        .await
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    validate_api_key(api_key, &state.config.api_key_config())
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
