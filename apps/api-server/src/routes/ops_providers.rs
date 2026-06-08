use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        AuditEventListFilter, AuditHistoryEventRecord, PersistedAuditEvent,
        ProviderRiskSummaryRecord,
    },
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
use std::collections::HashMap;

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
    headers: HeaderMap,
    Json(request): Json<SubmitAnomalyClusteringReportRequest>,
) -> Result<Json<SubmitAnomalyClusteringReportResponse>, ApiError> {
    let actor = authorize(&state, &headers)?;
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

pub async fn anomaly_review_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AnomalyReviewQueueResponse>, ApiError> {
    let actor = authorize(&state, &headers)?;
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

fn validate_anomaly_clustering_report_submission(
    request: &SubmitAnomalyClusteringReportRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REPORT_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REPORT_NOTES",
            "notes are required",
        ),
        (
            request.source_report_uri.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REPORT_URI",
            "source_report_uri is required",
        ),
        (
            request.report_kind.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REPORT_KIND",
            "report_kind is required",
        ),
        (
            request.dataset_key.as_str(),
            "INVALID_ANOMALY_CLUSTERING_DATASET",
            "dataset_key is required",
        ),
        (
            request.dataset_version.as_str(),
            "INVALID_ANOMALY_CLUSTERING_DATASET",
            "dataset_version is required",
        ),
        (
            request.label_policy.as_str(),
            "INVALID_ANOMALY_CLUSTERING_LABEL_POLICY",
            "label_policy is required",
        ),
        (
            request.governance_boundary.as_str(),
            "INVALID_ANOMALY_CLUSTERING_GOVERNANCE",
            "governance_boundary is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if !matches!(
        request.report_kind.as_str(),
        "provider_peer_clustering"
            | "provider_graph_community_clustering"
            | "claim_entity_clustering"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ANOMALY_CLUSTERING_REPORT_KIND",
            "report_kind must be provider_peer_clustering, provider_graph_community_clustering, or claim_entity_clustering",
        ));
    }
    if !request.source_report_uri.ends_with(".json") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ANOMALY_CLUSTERING_REPORT_URI",
            "source_report_uri must point to a JSON clustering report",
        ));
    }
    if request.review_tasks.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_ANOMALY_CLUSTERING_REVIEW_TASKS",
            "review_tasks are required",
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
            "MISSING_ANOMALY_CLUSTERING_REPORT_EVIDENCE",
            format!("anomaly clustering report evidence_refs must include {expected_report_ref}"),
        ));
    }
    for task in &request.review_tasks {
        validate_anomaly_clustering_review_task(task, &request.source_report_uri)?;
    }
    if pii::contains_pii(
        std::iter::once(request.actor.as_str())
            .chain(std::iter::once(request.notes.as_str()))
            .chain(std::iter::once(request.source_report_uri.as_str()))
            .chain(request.evidence_refs.iter().map(String::as_str))
            .chain(request.review_tasks.iter().flat_map(|task| {
                std::iter::once(task.candidate_id.as_str())
                    .chain(task.evidence_refs.iter().map(String::as_str))
            })),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_ANOMALY_CLUSTERING_REPORT",
            "anomaly clustering actor, notes, report URI, candidate IDs, and evidence_refs must not contain PII",
        ));
    }
    Ok(())
}

fn validate_anomaly_clustering_review_task(
    task: &AnomalyClusteringReviewTaskInput,
    source_report_uri: &str,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            task.candidate_kind.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REVIEW_TASK",
            "candidate_kind is required",
        ),
        (
            task.candidate_id.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REVIEW_TASK",
            "candidate_id is required",
        ),
        (
            task.task_kind.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REVIEW_TASK",
            "task_kind is required",
        ),
        (
            task.review_queue.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REVIEW_TASK",
            "review_queue is required",
        ),
        (
            task.required_review.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REVIEW_TASK",
            "required_review is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if !matches!(
        task.candidate_kind.as_str(),
        "provider_peer_anomaly" | "provider_graph_anomaly" | "claim_entity_anomaly"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ANOMALY_CLUSTERING_REVIEW_TASK_KIND",
            "review task candidate_kind must be provider_peer_anomaly, provider_graph_anomaly, or claim_entity_anomaly",
        ));
    }
    let expected_report_ref = format!("anomaly_clustering_reports:{source_report_uri}");
    if !task
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_report_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_ANOMALY_CLUSTERING_REVIEW_TASK_EVIDENCE",
            format!("review task evidence_refs must include {expected_report_ref}"),
        ));
    }
    Ok(())
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
    move |error| ApiError::internal(code, error)
}
