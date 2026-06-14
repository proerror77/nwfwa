use crate::{
    app::AppState,
    auth::{AuthenticatedActor, AuthenticatedApiPrincipal},
    error::ApiError,
    repository::{
        AuditEventListFilter, AuditHistoryEventRecord, PersistedAuditEvent,
        ProviderRiskSummaryRecord, ProviderSanctionRecord, ProviderSanctionUpsertInput,
        SaveProviderSanctionsInput,
    },
};
use axum::{extract::State, Json};
use fwa_audit::ActorContext;
use fwa_auth::AuthenticatedPrincipal;
use fwa_core::{AuditEventId, ScoringRunId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

mod validation;

use validation::{
    validate_anomaly_candidate_review, validate_anomaly_clustering_report_submission,
    validate_sanctions_sync_report_submission,
};

pub async fn provider_risk_summary(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
) -> Result<Json<ProviderRiskSummaryRecord>, ApiError> {
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

#[derive(Debug, Deserialize)]
pub struct SubmitAnomalyClusteringReportRequest {
    pub actor: String,
    pub notes: String,
    pub source_report_uri: String,
    pub report_kind: String,
    pub dataset_key: String,
    pub dataset_version: String,
    pub label_policy: String,
    pub governance_boundary: String,
    #[serde(default)]
    pub review_tasks: Vec<AnomalyClusteringReviewTaskInput>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnomalyClusteringReviewTaskInput {
    pub candidate_kind: String,
    pub candidate_id: String,
    pub task_kind: String,
    pub review_queue: String,
    pub required_review: String,
    #[serde(default)]
    pub decision_options: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub candidate_payload: Value,
}

#[derive(Debug, Serialize)]
pub struct SubmitAnomalyClusteringReportResponse {
    pub report_kind: String,
    pub source_report_uri: String,
    pub review_task_count: usize,
    pub accepted_for_review_queue: bool,
    pub active_rule_writeback: bool,
    pub model_activation: bool,
    pub label_assignment: bool,
    pub case_creation: bool,
    pub governance_boundary: String,
    pub audit_event_type: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitSanctionsSyncReportRequest {
    pub actor: String,
    pub notes: String,
    pub source_report_uri: String,
    pub report_kind: String,
    pub run_date: String,
    pub source_uri: String,
    pub source_date: Option<String>,
    pub sync_status: String,
    pub governance_boundary: String,
    #[serde(default)]
    pub provider_upserts: Vec<ProviderSanctionUpsertInput>,
    #[serde(default)]
    pub review_tasks: Vec<Value>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SubmitSanctionsSyncReportResponse {
    pub report_kind: String,
    pub source_report_uri: String,
    pub provider_upsert_count: usize,
    pub review_task_count: usize,
    pub persisted_provider_sanctions: Vec<ProviderSanctionRecord>,
    pub active_scoring_policy_change: bool,
    pub label_assignment: bool,
    pub governance_boundary: String,
    pub audit_event_type: String,
}

#[derive(Debug, Serialize)]
pub struct AnomalyReviewQueueResponse {
    pub tasks: Vec<AnomalyReviewQueueTask>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnomalyReviewQueueTask {
    pub candidate_kind: String,
    pub candidate_id: String,
    pub task_kind: String,
    pub review_queue: String,
    pub required_review: String,
    pub decision_options: Vec<String>,
    pub source_report_uri: String,
    pub report_kind: String,
    pub dataset_key: String,
    pub dataset_version: String,
    pub label_policy: String,
    pub governance_boundary: String,
    pub review_status: String,
    pub reviewer: Option<String>,
    pub decision: Option<String>,
    pub candidate_payload: Value,
    pub evidence_refs: Vec<String>,
}

pub async fn submit_anomaly_clustering_report(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Json(request): Json<SubmitAnomalyClusteringReportRequest>,
) -> Result<Json<SubmitAnomalyClusteringReportResponse>, ApiError> {
    validate_anomaly_clustering_report_submission(&request)?;
    let response = SubmitAnomalyClusteringReportResponse {
        report_kind: request.report_kind.clone(),
        source_report_uri: request.source_report_uri.clone(),
        review_task_count: request.review_tasks.len(),
        accepted_for_review_queue: true,
        active_rule_writeback: false,
        model_activation: false,
        label_assignment: false,
        case_creation: false,
        governance_boundary:
            "unlabeled clustering report submission creates anomaly review queue tasks only; it must not activate models, write rules, assign fraud labels, or auto-create cases"
                .into(),
        audit_event_type: "provider.anomaly_clustering.report_submitted".into(),
    };
    record_anomaly_clustering_report_audit(&state, &actor, &request, &response)
        .await
        .map_err(internal_error("ANOMALY_CLUSTERING_REPORT_AUDIT_FAILED"))?;
    Ok(Json(response))
}

pub async fn submit_sanctions_sync_report(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Json(request): Json<SubmitSanctionsSyncReportRequest>,
) -> Result<Json<SubmitSanctionsSyncReportResponse>, ApiError> {
    let actor = require_permission(principal, "ops:providers:write")?;
    validate_sanctions_sync_report_submission(&request)?;
    let persisted = state
        .repository
        .save_provider_sanctions(SaveProviderSanctionsInput {
            customer_scope_id: actor.customer_scope_id.clone(),
            source_report_uri: request.source_report_uri.clone(),
            submitted_by: request.actor.clone(),
            notes: request.notes.clone(),
            provider_upserts: request.provider_upserts.clone(),
        })
        .await
        .map_err(internal_error("PROVIDER_SANCTIONS_SAVE_FAILED"))?;
    let response = SubmitSanctionsSyncReportResponse {
        report_kind: request.report_kind.clone(),
        source_report_uri: request.source_report_uri.clone(),
        provider_upsert_count: persisted.len(),
        review_task_count: request.review_tasks.len(),
        persisted_provider_sanctions: persisted,
        active_scoring_policy_change: false,
        label_assignment: false,
        governance_boundary:
            "sanctions sync submission writes provider sanctions only; it must not change scoring policy, assign fraud labels, or adjudicate claims"
                .into(),
        audit_event_type: "provider.sanctions_sync.submitted".into(),
    };
    record_sanctions_sync_report_audit(&state, &actor, &request, &response)
        .await
        .map_err(internal_error("PROVIDER_SANCTIONS_AUDIT_FAILED"))?;
    Ok(Json(response))
}

pub async fn anomaly_review_queue(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<AnomalyReviewQueueResponse>, ApiError> {
    let report_events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 100,
            event_type: Some("provider.anomaly_clustering.report_submitted".into()),
            customer_scope_id: Some(actor.customer_scope_id.clone()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("ANOMALY_REVIEW_QUEUE_LIST_FAILED"))?;
    let review_events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 200,
            event_type: Some("anomaly.candidate.reviewed".into()),
            customer_scope_id: Some(actor.customer_scope_id),
            ..Default::default()
        })
        .await
        .map_err(internal_error("ANOMALY_REVIEW_QUEUE_LIST_FAILED"))?;

    Ok(Json(AnomalyReviewQueueResponse {
        tasks: anomaly_review_tasks_from_events(report_events, review_events),
    }))
}

pub async fn review_anomaly_candidate(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Json(request): Json<ReviewAnomalyCandidateRequest>,
) -> Result<Json<ReviewAnomalyCandidateResponse>, ApiError> {
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

async fn record_anomaly_clustering_report_audit(
    state: &AppState,
    actor: &ActorContext,
    request: &SubmitAnomalyClusteringReportRequest,
    response: &SubmitAnomalyClusteringReportResponse,
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
                "Anomaly clustering report submitted for review: {}",
                request.source_report_uri
            ),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "actor": request.actor,
                "notes": request.notes,
                "note_present": !request.notes.trim().is_empty(),
                "source_report_uri": request.source_report_uri,
                "report_kind": request.report_kind,
                "dataset_key": request.dataset_key,
                "dataset_version": request.dataset_version,
                "label_policy": request.label_policy,
                "governance_boundary": response.governance_boundary,
                "source_governance_boundary": request.governance_boundary,
                "review_tasks": request.review_tasks,
                "review_task_count": request.review_tasks.len(),
                "active_rule_writeback": response.active_rule_writeback,
                "model_activation": response.model_activation,
                "label_assignment": response.label_assignment,
                "case_creation": response.case_creation,
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

async fn record_sanctions_sync_report_audit(
    state: &AppState,
    actor: &ActorContext,
    request: &SubmitSanctionsSyncReportRequest,
    response: &SubmitSanctionsSyncReportResponse,
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
                "Provider sanctions sync report submitted: {}",
                request.source_report_uri
            ),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "actor": request.actor,
                "notes": request.notes,
                "source_report_uri": request.source_report_uri,
                "report_kind": request.report_kind,
                "run_date": request.run_date,
                "source_uri": request.source_uri,
                "source_date": request.source_date,
                "sync_status": request.sync_status,
                "provider_upsert_count": response.provider_upsert_count,
                "review_task_count": response.review_task_count,
                "governance_boundary": response.governance_boundary,
                "source_governance_boundary": request.governance_boundary,
                "active_scoring_policy_change": response.active_scoring_policy_change,
                "label_assignment": response.label_assignment,
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

fn anomaly_review_tasks_from_events(
    report_events: Vec<AuditHistoryEventRecord>,
    review_events: Vec<AuditHistoryEventRecord>,
) -> Vec<AnomalyReviewQueueTask> {
    let reviews = review_events
        .into_iter()
        .filter_map(|event| {
            let source_report_uri = event
                .payload
                .get("source_report_uri")?
                .as_str()?
                .to_string();
            let candidate_kind = event.payload.get("candidate_kind")?.as_str()?.to_string();
            let candidate_id = event.payload.get("candidate_id")?.as_str()?.to_string();
            Some((
                queue_key(&source_report_uri, &candidate_kind, &candidate_id),
                event,
            ))
        })
        .collect::<HashMap<_, _>>();

    let mut tasks_by_key = HashMap::new();
    for event in report_events {
        let source_report_uri = event
            .payload
            .get("source_report_uri")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let report_kind = event
            .payload
            .get("report_kind")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let dataset_key = event
            .payload
            .get("dataset_key")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let dataset_version = event
            .payload
            .get("dataset_version")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let label_policy = event
            .payload
            .get("label_policy")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let governance_boundary = event
            .payload
            .get("governance_boundary")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(review_tasks) = event.payload.get("review_tasks").and_then(Value::as_array) else {
            continue;
        };
        for task in review_tasks {
            let candidate_kind = task
                .get("candidate_kind")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let candidate_id = task
                .get("candidate_id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if candidate_kind.is_empty() || candidate_id.is_empty() {
                continue;
            }
            let key = queue_key(source_report_uri, candidate_kind, candidate_id);
            let review = reviews.get(&key);
            tasks_by_key.insert(
                key,
                AnomalyReviewQueueTask {
                    candidate_kind: candidate_kind.into(),
                    candidate_id: candidate_id.into(),
                    task_kind: task
                        .get("task_kind")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    review_queue: task
                        .get("review_queue")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    required_review: task
                        .get("required_review")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    decision_options: task
                        .get("decision_options")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(|value| value.as_str().map(str::to_string))
                        .collect(),
                    source_report_uri: source_report_uri.into(),
                    report_kind: report_kind.into(),
                    dataset_key: dataset_key.into(),
                    dataset_version: dataset_version.into(),
                    label_policy: label_policy.into(),
                    governance_boundary: governance_boundary.into(),
                    review_status: review
                        .map(|_| "reviewed")
                        .unwrap_or("pending_human_review")
                        .into(),
                    reviewer: review.and_then(|event| {
                        event
                            .payload
                            .get("reviewer")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    }),
                    decision: review.and_then(|event| {
                        event
                            .payload
                            .get("decision")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    }),
                    candidate_payload: task
                        .get("candidate_payload")
                        .cloned()
                        .unwrap_or(Value::Null),
                    evidence_refs: task
                        .get("evidence_refs")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(|value| value.as_str().map(str::to_string))
                        .collect(),
                },
            );
        }
    }

    let mut tasks = tasks_by_key.into_values().collect::<Vec<_>>();
    tasks.sort_by(|left, right| {
        left.source_report_uri
            .cmp(&right.source_report_uri)
            .then_with(|| left.candidate_kind.cmp(&right.candidate_kind))
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });
    tasks
}

fn queue_key(source_report_uri: &str, candidate_kind: &str, candidate_id: &str) -> String {
    format!("{source_report_uri}\n{candidate_kind}\n{candidate_id}")
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}

fn require_permission(
    principal: AuthenticatedPrincipal,
    permission: &str,
) -> Result<ActorContext, ApiError> {
    if !principal.has_permission(permission) {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "PERMISSION_DENIED",
            format!("missing permission: {permission}"),
        ));
    }
    Ok(principal.actor)
}
