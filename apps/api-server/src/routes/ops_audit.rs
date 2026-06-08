use crate::{
    app::AppState,
    auth::AuthenticatedActor,
    error::ApiError,
    repository::{AuditEventListFilter, AuditHistoryEventRecord},
};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AuditEventListQuery {
    pub limit: Option<u32>,
    pub event_group: Option<String>,
    pub event_type: Option<String>,
    pub actor_id: Option<String>,
    pub run_id: Option<String>,
    pub claim_id: Option<String>,
    pub rule_id: Option<String>,
    pub rule_version: Option<String>,
    pub model_key: Option<String>,
    pub model_version: Option<String>,
    pub routing_policy_id: Option<String>,
    pub routing_policy_version: Option<String>,
    pub review_mode: Option<String>,
    pub feedback_id: Option<String>,
    pub qa_case_id: Option<String>,
    pub sample_id: Option<String>,
    pub agent_run_id: Option<String>,
    pub dataset_id: Option<String>,
    pub feature_set_id: Option<String>,
    pub model_dataset_id: Option<String>,
    pub evaluation_run_id: Option<String>,
    pub has_canonical_trace: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ApiCallListQuery {
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct AuditEventListResponse {
    pub events: Vec<AuditHistoryEventRecord>,
}

#[derive(Debug, Serialize)]
pub struct ApiCallRecord {
    pub call_id: String,
    pub endpoint: String,
    pub method: String,
    pub status_code: u16,
    pub result: String,
    pub source_system: String,
    pub actor_role: String,
    pub customer_scope_id: String,
    pub claim_id: String,
    pub run_id: String,
    pub audit_id: String,
    pub event_type: String,
    pub idempotency_key: Option<String>,
    pub evidence_refs: Vec<String>,
    pub observed_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiCallListResponse {
    pub calls: Vec<ApiCallRecord>,
}

pub async fn list_audit_events(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Query(query): Query<AuditEventListQuery>,
) -> Result<Json<AuditEventListResponse>, ApiError> {
    let filter = AuditEventListFilter {
        limit: query.limit.unwrap_or(50).clamp(1, 200),
        event_group: normalize_filter(query.event_group),
        event_type: normalize_filter(query.event_type),
        actor_id: normalize_filter(query.actor_id),
        run_id: normalize_filter(query.run_id),
        claim_id: normalize_filter(query.claim_id),
        rule_id: normalize_filter(query.rule_id),
        rule_version: normalize_filter(query.rule_version),
        model_key: normalize_filter(query.model_key),
        model_version: normalize_filter(query.model_version),
        routing_policy_id: normalize_filter(query.routing_policy_id),
        routing_policy_version: normalize_filter(query.routing_policy_version),
        review_mode: normalize_filter(query.review_mode),
        feedback_id: normalize_filter(query.feedback_id),
        qa_case_id: normalize_filter(query.qa_case_id),
        sample_id: normalize_filter(query.sample_id),
        agent_run_id: normalize_filter(query.agent_run_id),
        dataset_id: normalize_filter(query.dataset_id),
        feature_set_id: normalize_filter(query.feature_set_id),
        model_dataset_id: normalize_filter(query.model_dataset_id),
        evaluation_run_id: normalize_filter(query.evaluation_run_id),
        has_canonical_trace: query.has_canonical_trace,
        customer_scope_id: Some(actor.customer_scope_id),
    };
    let events = state
        .repository
        .list_audit_events(filter)
        .await
        .map_err(internal_error("AUDIT_EVENT_LIST_FAILED"))?;
    Ok(Json(AuditEventListResponse { events }))
}

pub async fn list_api_calls(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Query(query): Query<ApiCallListQuery>,
) -> Result<Json<ApiCallListResponse>, ApiError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 200,
            customer_scope_id: Some(actor.customer_scope_id),
            ..Default::default()
        })
        .await
        .map_err(internal_error("API_CALL_LIST_FAILED"))?;
    let calls = events
        .into_iter()
        .filter_map(|event| api_call_from_audit_event(event, &state.config.source_system))
        .take(limit as usize)
        .collect();
    Ok(Json(ApiCallListResponse { calls }))
}

fn api_call_from_audit_event(
    event: AuditHistoryEventRecord,
    default_source_system: &str,
) -> Option<ApiCallRecord> {
    let (method, endpoint) = tpa_endpoint_for_event(&event)?;
    let claim_id = event
        .payload
        .get("claim_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let source_system = event
        .payload
        .get("source_system")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(default_source_system)
        .to_string();
    let customer_scope_id = event
        .payload
        .get("customer_scope_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let status_code = event
        .payload
        .get("status_code")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .unwrap_or(200);
    let idempotency_key = match event.event_type.as_str() {
        "inbox.claim.normalized" => event
            .payload
            .get("idempotency_key")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        "investigation.result.received" | "qa.result.received" => Some(format!(
            "tpa-writeback:{}:{}",
            event.event_type, event.audit_id
        )),
        _ => None,
    };
    Some(ApiCallRecord {
        call_id: event.audit_id.clone(),
        endpoint: endpoint.to_string(),
        method: method.to_string(),
        status_code,
        result: event.event_status.clone(),
        source_system,
        actor_role: event.actor_role.clone(),
        customer_scope_id,
        claim_id,
        run_id: event.run_id.clone(),
        audit_id: event.audit_id,
        event_type: event.event_type,
        idempotency_key,
        evidence_refs: event.evidence_refs,
        observed_at: event.created_at,
    })
}

fn tpa_endpoint_for_event(event: &AuditHistoryEventRecord) -> Option<(&'static str, &'static str)> {
    match event.event_type.as_str() {
        "inbox.claim.normalized" => Some(("POST", "/api/v1/inbox/claims/normalize")),
        "scoring.completed" => Some(("POST", "/api/v1/claims/score")),
        "investigation.result.received" => Some(("POST", "/api/v1/investigations/results")),
        "qa.result.received" => Some(("POST", "/api/v1/qa/results")),
        _ => None,
    }
}

fn normalize_filter(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}
