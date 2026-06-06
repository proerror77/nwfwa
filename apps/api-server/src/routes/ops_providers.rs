use crate::{
    app::AppState,
    error::ApiError,
    repository::{PersistedAuditEvent, ProviderRiskSummaryRecord},
    routes::pii,
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_audit::ActorContext;
use fwa_auth::validate_api_key;
use fwa_core::{AuditEventId, ScoringRunId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

#[derive(Debug, Deserialize)]
pub struct ReviewAnomalyCandidateRequest {
    pub candidate_kind: String,
    pub candidate_id: String,
    pub source_report_uri: String,
    pub decision: String,
    pub reviewer: String,
    pub notes: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub candidate_payload: Value,
}

#[derive(Debug, Serialize)]
pub struct ReviewAnomalyCandidateResponse {
    pub candidate_kind: String,
    pub candidate_id: String,
    pub decision: String,
    pub reviewer: String,
    pub accepted_for_review: bool,
    pub active_rule_writeback: bool,
    pub model_activation: bool,
    pub label_assignment: bool,
    pub governance_boundary: String,
    pub audit_event_type: String,
}

pub async fn review_anomaly_candidate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ReviewAnomalyCandidateRequest>,
) -> Result<Json<ReviewAnomalyCandidateResponse>, ApiError> {
    let actor = authorize(&state, &headers)?;
    validate_anomaly_candidate_review(&request)?;
    let response = ReviewAnomalyCandidateResponse {
        candidate_kind: request.candidate_kind.clone(),
        candidate_id: request.candidate_id.clone(),
        decision: request.decision.clone(),
        reviewer: request.reviewer.clone(),
        accepted_for_review: request.decision == "accepted_for_review"
            || request.decision == "open_investigation_review",
        active_rule_writeback: false,
        model_activation: false,
        label_assignment: false,
        governance_boundary: "unsupervised anomaly candidate review records human governance only; it must not activate models, write rules, assign fraud labels, or auto-create claim dispositions".into(),
        audit_event_type: "anomaly.candidate.reviewed".into(),
    };
    record_anomaly_candidate_review_audit(&state, &actor, &request, &response)
        .await
        .map_err(internal_error("ANOMALY_CANDIDATE_REVIEW_AUDIT_FAILED"))?;
    Ok(Json(response))
}

fn validate_anomaly_candidate_review(
    request: &ReviewAnomalyCandidateRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.candidate_kind.as_str(),
            "INVALID_ANOMALY_CANDIDATE_KIND",
            "candidate_kind is required",
        ),
        (
            request.candidate_id.as_str(),
            "INVALID_ANOMALY_CANDIDATE_ID",
            "candidate_id is required",
        ),
        (
            request.source_report_uri.as_str(),
            "INVALID_ANOMALY_CANDIDATE_REPORT",
            "source_report_uri is required",
        ),
        (
            request.reviewer.as_str(),
            "INVALID_ANOMALY_CANDIDATE_REVIEWER",
            "reviewer is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_ANOMALY_CANDIDATE_NOTES",
            "review notes are required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if !matches!(
        request.candidate_kind.as_str(),
        "provider_peer_anomaly" | "provider_graph_anomaly" | "claim_entity_anomaly"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ANOMALY_CANDIDATE_KIND",
            "candidate_kind must be provider_peer_anomaly, provider_graph_anomaly, or claim_entity_anomaly",
        ));
    }
    if !matches!(
        request.decision.as_str(),
        "accepted_for_review" | "rejected" | "open_investigation_review" | "request_more_evidence"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ANOMALY_CANDIDATE_DECISION",
            "decision must be accepted_for_review, rejected, open_investigation_review, or request_more_evidence",
        ));
    }
    if !request.source_report_uri.ends_with(".json") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ANOMALY_CANDIDATE_REPORT",
            "source_report_uri must point to a JSON clustering report",
        ));
    }
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_ANOMALY_CANDIDATE_EVIDENCE",
            "anomaly candidate review evidence_refs are required",
        ));
    }
    let expected_report_ref = format!("anomaly_clustering_reports:{}", request.source_report_uri);
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_report_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_ANOMALY_CANDIDATE_EVIDENCE",
            format!("anomaly candidate evidence_refs must include {expected_report_ref}"),
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.reviewer.as_str())
            .chain(std::iter::once(request.notes.as_str()))
            .chain(std::iter::once(request.source_report_uri.as_str()))
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_ANOMALY_CANDIDATE_REVIEW",
            "anomaly candidate reviewer, notes, report URI, and evidence_refs must not contain PII",
        ));
    }
    Ok(())
}

async fn record_anomaly_candidate_review_audit(
    state: &AppState,
    actor: &ActorContext,
    request: &ReviewAnomalyCandidateRequest,
    response: &ReviewAnomalyCandidateResponse,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: response.audit_event_type.clone(),
            event_status: "succeeded".into(),
            summary: format!(
                "Anomaly candidate {} reviewed: {}",
                request.candidate_id, request.decision
            ),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "candidate_kind": request.candidate_kind,
                "candidate_id": request.candidate_id,
                "source_report_uri": request.source_report_uri,
                "decision": request.decision,
                "reviewer": request.reviewer,
                "notes": request.notes,
                "note_present": !request.notes.trim().is_empty(),
                "candidate_payload": request.candidate_payload,
                "active_rule_writeback": response.active_rule_writeback,
                "model_activation": response.model_activation,
                "label_assignment": response.label_assignment,
                "governance_boundary": response.governance_boundary,
            }),
            evidence_refs: request
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<ActorContext, ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    validate_api_key(api_key, &state.config.api_key_config()).map_err(|_| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
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
