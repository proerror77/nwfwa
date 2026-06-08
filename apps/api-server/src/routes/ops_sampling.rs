use crate::{
    app::AppState,
    auth::{AuthenticatedActor, AuthenticatedApiPrincipal},
    error::ApiError,
    repository::{AuditSampleRecord, CreateAuditSampleInput, PersistedAuditEvent},
};
use axum::{extract::State, http::StatusCode, Json};
use fwa_audit::ActorContext;
use fwa_auth::AuthenticatedPrincipal;
use fwa_core::AuditEventId;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AuditSampleListResponse {
    pub samples: Vec<AuditSampleRecord>,
}

pub async fn list_audit_samples(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<AuditSampleListResponse>, ApiError> {
    let samples = state
        .repository
        .list_audit_samples(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("AUDIT_SAMPLE_LIST_FAILED"))?;
    Ok(Json(AuditSampleListResponse { samples }))
}

pub async fn create_audit_sample(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Json(mut request): Json<CreateAuditSampleInput>,
) -> Result<Json<AuditSampleRecord>, ApiError> {
    let actor = require_permission(principal, "ops:audit-samples:create")?;
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
    request.customer_scope_id = Some(actor.customer_scope_id.clone());
    let sample = state
        .repository
        .create_audit_sample(request)
        .await
        .map_err(internal_error("AUDIT_SAMPLE_CREATE_FAILED"))?;
    record_audit_sample_created(&state, &actor, &sample)
        .await
        .map_err(internal_error("AUDIT_SAMPLE_AUDIT_FAILED"))?;
    Ok(Json(sample))
}

async fn record_audit_sample_created(
    state: &AppState,
    actor: &ActorContext,
    sample: &AuditSampleRecord,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: format!("audit_sample_{}", sample.sample_id),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "audit_sample.created".into(),
            event_status: "succeeded".into(),
            summary: format!("Audit sample created: {}", sample.sample_mode),
            payload: serde_json::json!({
                "customer_scope_id": sample.customer_scope_id,
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

fn require_permission(
    principal: AuthenticatedPrincipal,
    permission: &str,
) -> Result<ActorContext, ApiError> {
    if !principal.has_permission(permission) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "PERMISSION_DENIED",
            format!("missing permission: {permission}"),
        ));
    }
    Ok(principal.actor)
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal_with_permissions(permissions: Vec<&str>) -> AuthenticatedPrincipal {
        AuthenticatedPrincipal {
            actor: ActorContext {
                actor_id: "ops-viewer".into(),
                actor_role: "operations_reviewer".into(),
                source_system: "ops-studio".into(),
                customer_scope_id: "customer-alpha".into(),
            },
            permissions: permissions.into_iter().map(str::to_string).collect(),
        }
    }

    #[test]
    fn require_permission_rejects_audit_sample_create_without_ops_permission() {
        let error = require_permission(
            principal_with_permissions(vec!["ops:read", "audit:read"]),
            "ops:audit-samples:create",
        )
        .unwrap_err();

        assert_eq!(error.status, StatusCode::FORBIDDEN);
        assert_eq!(error.code, "PERMISSION_DENIED");
    }

    #[test]
    fn require_permission_accepts_ops_wildcard_for_audit_sample_create() {
        let actor = require_permission(
            principal_with_permissions(vec!["ops:*"]),
            "ops:audit-samples:create",
        )
        .unwrap();

        assert_eq!(actor.customer_scope_id, "customer-alpha");
    }
}
