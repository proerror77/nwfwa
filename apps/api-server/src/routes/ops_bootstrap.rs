use crate::{
    app::AppState,
    auth::AuthenticatedActor,
    error::ApiError,
    repository::{AuditEventListFilter, AuditHistoryEventRecord, LeadRecord, PersistedAuditEvent},
    routes::pii,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use fwa_audit::ActorContext;
use fwa_core::{AuditEventId, ScoringRunId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct CreateHistoricalBackfillRequest {
    pub job_id: Option<String>,
    #[serde(default)]
    pub dataset_refs: Vec<String>,
    #[serde(default)]
    pub rule_refs: Vec<String>,
    pub reviewer: Option<String>,
    pub notes: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct HistoricalBackfillResponse {
    pub job: HistoricalBackfillJobRecord,
}

#[derive(Debug, Serialize)]
pub struct HistoricalBackfillListResponse {
    pub jobs: Vec<HistoricalBackfillJobRecord>,
}

#[derive(Debug, Serialize)]
pub struct HistoricalBackfillLeadResponse {
    pub job_id: String,
    pub leads: Vec<HistoricalBackfillLeadRecord>,
}

#[derive(Debug, Serialize, Clone)]
pub struct HistoricalBackfillJobRecord {
    pub job_id: String,
    pub status: String,
    pub dataset_refs: Vec<String>,
    pub rule_refs: Vec<String>,
    pub candidate_count: u32,
    pub leads: Vec<HistoricalBackfillLeadRecord>,
    pub reviewer: Option<String>,
    pub notes: Option<String>,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct HistoricalBackfillLeadRecord {
    pub lead_id: String,
    pub claim_id: String,
    pub scheme_family: String,
    pub risk_score: u8,
    pub rag: String,
    pub status: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateEvidenceRequestsRequest {
    pub claim_id: Option<String>,
    pub scoring_audit_id: Option<String>,
    pub requested_by: Option<String>,
    pub reviewer_queue: Option<String>,
    pub notes: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEvidenceRequestStatusRequest {
    pub status: String,
    pub actor_id: String,
    pub notes: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EvidenceRequestListResponse {
    pub requests: Vec<EvidenceRequestRecord>,
}

#[derive(Debug, Serialize)]
pub struct EvidenceRequestGenerateResponse {
    pub requests: Vec<EvidenceRequestRecord>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EvidenceRequestRecord {
    pub request_id: String,
    pub claim_id: String,
    pub scoring_audit_id: String,
    pub status: String,
    pub request_reason: String,
    pub missing_evidence: Vec<String>,
    pub items: Vec<EvidenceRequestItemRecord>,
    pub reviewer_queue: String,
    pub requested_by: String,
    pub notes: Option<String>,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EvidenceRequestItemRecord {
    pub item_id: String,
    pub document_type: String,
    pub status: String,
    pub reason: String,
    pub blocking: bool,
    pub policy_authority_ref: Option<String>,
    pub exception_check: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewLabelBootstrapItemRequest {
    pub reviewer: String,
    pub label_name: String,
    pub label_value: String,
    pub governance_status: String,
    pub feedback_target: String,
    pub notes: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LabelBootstrapQueueResponse {
    pub items: Vec<LabelBootstrapItemRecord>,
}

#[derive(Debug, Serialize, Clone)]
pub struct LabelBootstrapItemRecord {
    pub item_id: String,
    pub claim_id: String,
    pub source_type: String,
    pub source_id: String,
    pub suggested_label_name: String,
    pub suggested_label_value: String,
    pub governance_status: String,
    pub training_eligible: bool,
    pub review_status: String,
    pub review_audit_id: Option<String>,
    pub reviewer: Option<String>,
    pub feedback_target: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LabelBootstrapReviewResponse {
    pub item: LabelBootstrapItemRecord,
    pub audit_id: String,
}

pub async fn create_historical_backfill(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Json(request): Json<CreateHistoricalBackfillRequest>,
) -> Result<Json<HistoricalBackfillResponse>, ApiError> {
    validate_backfill_request(&request)?;
    let leads = state
        .repository
        .list_leads(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("BACKFILL_LEAD_LIST_FAILED"))?;
    let limit = request.limit.unwrap_or(25).clamp(1, 200) as usize;
    let selected = leads
        .into_iter()
        .take(limit)
        .map(backfill_lead_from_lead)
        .collect::<Vec<_>>();
    let job_id = request
        .job_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("backfill_{}", AuditEventId::new()));
    let audit_id = AuditEventId::new().to_string();
    let evidence_refs = backfill_evidence_refs(&request, &selected);
    let job = HistoricalBackfillJobRecord {
        job_id: job_id.clone(),
        status: "completed".into(),
        dataset_refs: request.dataset_refs.clone(),
        rule_refs: request.rule_refs.clone(),
        candidate_count: selected.len() as u32,
        leads: selected,
        reviewer: request.reviewer.clone(),
        notes: request.notes.clone(),
        evidence_refs: evidence_refs.clone(),
        created_at: None,
    };
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id,
            run_id: ScoringRunId::new().to_string(),
            claim_id: first_claim_id(&job.leads),
            source_system: "ops-studio".into(),
            actor_id: request
                .reviewer
                .clone()
                .unwrap_or_else(|| actor.actor_id.clone()),
            actor_role: actor.actor_role.clone(),
            event_type: "historical_backfill.created".into(),
            event_status: "succeeded".into(),
            summary: format!("Historical backfill completed: {job_id}"),
            payload: json!({
                "customer_scope_id": actor.customer_scope_id,
                "job_id": job.job_id,
                "status": job.status,
                "dataset_refs": job.dataset_refs,
                "rule_refs": job.rule_refs,
                "candidate_count": job.candidate_count,
                "leads": job.leads,
                "reviewer": job.reviewer,
                "notes": job.notes,
            }),
            evidence_refs: evidence_refs
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        })
        .await
        .map_err(internal_error("BACKFILL_SAVE_FAILED"))?;
    Ok(Json(HistoricalBackfillResponse { job }))
}

pub async fn list_historical_backfills(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<HistoricalBackfillListResponse>, ApiError> {
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 200,
            event_type: Some("historical_backfill.created".into()),
            customer_scope_id: Some(actor.customer_scope_id),
            ..Default::default()
        })
        .await
        .map_err(internal_error("BACKFILL_LIST_FAILED"))?;
    Ok(Json(HistoricalBackfillListResponse {
        jobs: events.iter().filter_map(backfill_job_from_event).collect(),
    }))
}

pub async fn list_historical_backfill_leads(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(job_id): Path<String>,
) -> Result<Json<HistoricalBackfillLeadResponse>, ApiError> {
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 500,
            event_type: Some("historical_backfill.created".into()),
            customer_scope_id: Some(actor.customer_scope_id),
            ..Default::default()
        })
        .await
        .map_err(internal_error("BACKFILL_LEADS_FAILED"))?;
    let job = events
        .iter()
        .filter_map(backfill_job_from_event)
        .find(|job| job.job_id == job_id)
        .ok_or_else(not_found(
            "BACKFILL_JOB_NOT_FOUND",
            "backfill job not found",
        ))?;
    Ok(Json(HistoricalBackfillLeadResponse {
        job_id,
        leads: job.leads,
    }))
}

pub async fn generate_evidence_requests(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Json(request): Json<GenerateEvidenceRequestsRequest>,
) -> Result<Json<EvidenceRequestGenerateResponse>, ApiError> {
    validate_generate_evidence_request(&request)?;
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: request.limit.unwrap_or(50).clamp(1, 200),
            event_type: Some("scoring.completed".into()),
            claim_id: request.claim_id.clone(),
            customer_scope_id: Some(actor.customer_scope_id.clone()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("EVIDENCE_REQUEST_SOURCE_LOOKUP_FAILED"))?;
    let mut generated = Vec::new();
    for event in events
        .iter()
        .filter(|event| {
            request
                .scoring_audit_id
                .as_deref()
                .is_none_or(|id| event.audit_id == id)
        })
        .filter_map(|event| evidence_request_from_scoring_event(event, &request, &actor))
    {
        let audit_id = AuditEventId::new().to_string();
        state
            .repository
            .save_audit_event(PersistedAuditEvent {
                audit_id,
                run_id: ScoringRunId::new().to_string(),
                claim_id: event.claim_id.clone(),
                source_system: "ops-studio".into(),
                actor_id: event.requested_by.clone(),
                actor_role: actor.actor_role.clone(),
                event_type: "evidence.request.generated".into(),
                event_status: "succeeded".into(),
                summary: format!("Evidence request generated: {}", event.request_id),
                payload: evidence_request_payload(&actor.customer_scope_id, &event),
                evidence_refs: event
                    .evidence_refs
                    .iter()
                    .map(|value| Value::String(value.clone()))
                    .collect(),
            })
            .await
            .map_err(internal_error("EVIDENCE_REQUEST_SAVE_FAILED"))?;
        generated.push(event);
    }
    Ok(Json(EvidenceRequestGenerateResponse {
        requests: generated,
    }))
}

pub async fn list_evidence_requests(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<EvidenceRequestListResponse>, ApiError> {
    Ok(Json(EvidenceRequestListResponse {
        requests: load_evidence_requests(&state, &actor.customer_scope_id).await?,
    }))
}

pub async fn update_evidence_request_status(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(request_id): Path<String>,
    Json(request): Json<UpdateEvidenceRequestStatusRequest>,
) -> Result<Json<EvidenceRequestRecord>, ApiError> {
    validate_evidence_status_update(&request)?;
    let current = load_evidence_requests(&state, &actor.customer_scope_id)
        .await?
        .into_iter()
        .find(|record| record.request_id == request_id)
        .ok_or_else(not_found(
            "EVIDENCE_REQUEST_NOT_FOUND",
            "evidence request not found",
        ))?;
    let audit_id = AuditEventId::new().to_string();
    let mut evidence_refs = current.evidence_refs.clone();
    for reference in &request.evidence_refs {
        if !evidence_refs.contains(reference) {
            evidence_refs.push(reference.clone());
        }
    }
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id,
            run_id: ScoringRunId::new().to_string(),
            claim_id: current.claim_id.clone(),
            source_system: "ops-studio".into(),
            actor_id: request.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "evidence.request.status_updated".into(),
            event_status: "succeeded".into(),
            summary: format!("Evidence request status updated: {request_id}"),
            payload: json!({
                "customer_scope_id": actor.customer_scope_id,
                "request_id": request_id,
                "claim_id": current.claim_id,
                "scoring_audit_id": current.scoring_audit_id,
                "from_status": current.status,
                "to_status": request.status,
                "actor_id": request.actor_id,
                "notes": request.notes,
            }),
            evidence_refs: evidence_refs
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        })
        .await
        .map_err(internal_error("EVIDENCE_REQUEST_STATUS_SAVE_FAILED"))?;
    load_evidence_requests(&state, &actor.customer_scope_id)
        .await?
        .into_iter()
        .find(|record| record.request_id == request_id)
        .map(Json)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "EVIDENCE_REQUEST_RELOAD_FAILED",
                "evidence request update saved but could not be reloaded",
            )
        })
}

pub async fn label_bootstrap_queue(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<LabelBootstrapQueueResponse>, ApiError> {
    Ok(Json(LabelBootstrapQueueResponse {
        items: load_label_bootstrap_items(&state, &actor.customer_scope_id).await?,
    }))
}

pub async fn review_label_bootstrap_item(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(item_id): Path<String>,
    Json(request): Json<ReviewLabelBootstrapItemRequest>,
) -> Result<Json<LabelBootstrapReviewResponse>, ApiError> {
    validate_label_review(&request)?;
    let mut item = load_label_bootstrap_items(&state, &actor.customer_scope_id)
        .await?
        .into_iter()
        .find(|item| item.item_id == item_id)
        .ok_or_else(not_found(
            "LABEL_BOOTSTRAP_ITEM_NOT_FOUND",
            "label bootstrap item not found",
        ))?;
    validate_label_training_approval(&item, &request)?;
    let audit_id = AuditEventId::new().to_string();
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: audit_id.clone(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: item.claim_id.clone(),
            source_system: "ops-studio".into(),
            actor_id: request.reviewer.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "label.bootstrap.reviewed".into(),
            event_status: "succeeded".into(),
            summary: format!("Label bootstrap item reviewed: {item_id}"),
            payload: json!({
                "customer_scope_id": actor.customer_scope_id,
                "item_id": item_id,
                "claim_id": item.claim_id,
                "source_type": item.source_type,
                "source_id": item.source_id,
                "reviewer": request.reviewer,
                "label_name": request.label_name,
                "label_value": request.label_value,
                "governance_status": request.governance_status,
                "feedback_target": request.feedback_target,
                "notes": request.notes,
            }),
            evidence_refs: request
                .evidence_refs
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        })
        .await
        .map_err(internal_error("LABEL_BOOTSTRAP_REVIEW_SAVE_FAILED"))?;
    item.review_status = "reviewed".into();
    item.review_audit_id = Some(audit_id.clone());
    item.reviewer = Some(request.reviewer);
    item.suggested_label_name = request.label_name;
    item.suggested_label_value = request.label_value;
    item.governance_status = request.governance_status;
    item.feedback_target = request.feedback_target;
    item.training_eligible = item.governance_status == "approved_for_training";
    item.evidence_refs = request.evidence_refs;
    Ok(Json(LabelBootstrapReviewResponse { item, audit_id }))
}

async fn load_label_bootstrap_items(
    state: &AppState,
    customer_scope_id: &str,
) -> Result<Vec<LabelBootstrapItemRecord>, ApiError> {
    let requests = load_evidence_requests(state, customer_scope_id).await?;
    let reviews = load_label_bootstrap_reviews(state, customer_scope_id).await?;
    let mut items = requests
        .into_iter()
        .map(label_item_from_evidence_request)
        .collect::<Vec<_>>();
    for item in &mut items {
        if let Some(review) = reviews.get(&item.item_id) {
            apply_label_review(item, review);
        }
    }
    items.sort_by(|left, right| left.item_id.cmp(&right.item_id));
    Ok(items)
}

async fn load_evidence_requests(
    state: &AppState,
    customer_scope_id: &str,
) -> Result<Vec<EvidenceRequestRecord>, ApiError> {
    let generated_events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 10_000,
            event_type: Some("evidence.request.generated".into()),
            customer_scope_id: Some(customer_scope_id.into()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("EVIDENCE_REQUEST_LIST_FAILED"))?;
    let status_events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 10_000,
            event_type: Some("evidence.request.status_updated".into()),
            customer_scope_id: Some(customer_scope_id.into()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("EVIDENCE_REQUEST_LIST_FAILED"))?;
    let mut requests = generated_events
        .iter()
        .filter_map(evidence_request_from_event)
        .map(|request| (request.request_id.clone(), request))
        .collect::<BTreeMap<_, _>>();
    for event in status_events.iter().rev() {
        let Some(request_id) = event.payload["request_id"].as_str() else {
            continue;
        };
        let Some(request) = requests.get_mut(request_id) else {
            continue;
        };
        if let Some(status) = event.payload["to_status"].as_str() {
            request.status = status.to_string();
        }
        request.updated_at = event.created_at.clone();
        for reference in &event.evidence_refs {
            if !request.evidence_refs.contains(reference) {
                request.evidence_refs.push(reference.clone());
            }
        }
    }
    let mut requests = requests.into_values().collect::<Vec<_>>();
    requests.sort_by(|left, right| left.request_id.cmp(&right.request_id));
    Ok(requests)
}

async fn load_label_bootstrap_reviews(
    state: &AppState,
    customer_scope_id: &str,
) -> Result<BTreeMap<String, AuditHistoryEventRecord>, ApiError> {
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 10_000,
            event_type: Some("label.bootstrap.reviewed".into()),
            customer_scope_id: Some(customer_scope_id.into()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("LABEL_BOOTSTRAP_REVIEW_LIST_FAILED"))?;
    let mut reviews = BTreeMap::new();
    for event in events.into_iter().rev() {
        if let Some(item_id) = event.payload["item_id"].as_str() {
            reviews.insert(item_id.to_string(), event);
        }
    }
    Ok(reviews)
}

fn validate_backfill_request(request: &CreateHistoricalBackfillRequest) -> Result<(), ApiError> {
    validate_optional_notes(request.notes.as_deref(), "BACKFILL_NOTES_CONTAIN_PII")?;
    if request
        .dataset_refs
        .iter()
        .chain(request.rule_refs.iter())
        .any(|value| value.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_BACKFILL_REFS",
            "dataset_refs and rule_refs cannot contain blank values",
        ));
    }
    Ok(())
}

fn validate_generate_evidence_request(
    request: &GenerateEvidenceRequestsRequest,
) -> Result<(), ApiError> {
    validate_optional_notes(
        request.notes.as_deref(),
        "EVIDENCE_REQUEST_NOTES_CONTAIN_PII",
    )
}

fn validate_evidence_status_update(
    request: &UpdateEvidenceRequestStatusRequest,
) -> Result<(), ApiError> {
    if !matches!(
        request.status.as_str(),
        "open" | "requested" | "received" | "cancelled"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "UNSUPPORTED_EVIDENCE_REQUEST_STATUS",
            "status must be one of open, requested, received, or cancelled",
        ));
    }
    if request.actor_id.trim().is_empty() || request.notes.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_EVIDENCE_REQUEST_STATUS_UPDATE",
            "actor_id and notes are required",
        ));
    }
    if request.status == "received" && !has_document_evidence_ref(&request.evidence_refs) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "EVIDENCE_REQUEST_DOCUMENT_EVIDENCE_REQUIRED",
            "received evidence requests require at least one evidence_documents reference",
        ));
    }
    validate_optional_notes(Some(&request.notes), "EVIDENCE_REQUEST_NOTES_CONTAIN_PII")
}

fn validate_label_review(request: &ReviewLabelBootstrapItemRequest) -> Result<(), ApiError> {
    if request.reviewer.trim().is_empty()
        || request.label_name.trim().is_empty()
        || request.label_value.trim().is_empty()
        || request.feedback_target.trim().is_empty()
        || request.notes.trim().is_empty()
        || request.evidence_refs.is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_LABEL_BOOTSTRAP_REVIEW",
            "reviewer, label fields, feedback_target, notes, and evidence_refs are required",
        ));
    }
    if !matches!(
        request.governance_status.as_str(),
        "needs_review" | "approved_for_training" | "rejected_for_training"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "UNSUPPORTED_LABEL_GOVERNANCE_STATUS",
            "governance_status must be needs_review, approved_for_training, or rejected_for_training",
        ));
    }
    validate_optional_notes(Some(&request.notes), "LABEL_REVIEW_NOTES_CONTAIN_PII")
}

fn validate_label_training_approval(
    item: &LabelBootstrapItemRecord,
    request: &ReviewLabelBootstrapItemRequest,
) -> Result<(), ApiError> {
    if request.governance_status != "approved_for_training" {
        return Ok(());
    }
    if item.suggested_label_name == "insufficient_evidence" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "LABEL_BOOTSTRAP_EVIDENCE_NOT_RECEIVED",
            "evidence must be received before a bootstrap label can be approved for training",
        ));
    }
    if !has_document_evidence_ref(&request.evidence_refs) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "LABEL_BOOTSTRAP_DOCUMENT_EVIDENCE_REQUIRED",
            "approved training labels require at least one evidence_documents reference",
        ));
    }
    Ok(())
}

fn has_document_evidence_ref(evidence_refs: &[String]) -> bool {
    evidence_refs
        .iter()
        .any(|reference| reference.starts_with("evidence_documents:"))
}

fn validate_optional_notes(notes: Option<&str>, code: &'static str) -> Result<(), ApiError> {
    if notes
        .filter(|value| pii::contains_pii(std::iter::once(*value)))
        .is_some()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            code,
            "notes must not contain PII",
        ));
    }
    Ok(())
}

fn backfill_lead_from_lead(lead: LeadRecord) -> HistoricalBackfillLeadRecord {
    HistoricalBackfillLeadRecord {
        lead_id: lead.lead_id,
        claim_id: lead.claim_id,
        scheme_family: lead.scheme_family,
        risk_score: lead.risk_score,
        rag: lead.rag,
        status: lead.status,
        reason: lead.reason,
        evidence_refs: lead.evidence_refs,
    }
}

fn first_claim_id(leads: &[HistoricalBackfillLeadRecord]) -> String {
    leads
        .first()
        .map(|lead| lead.claim_id.clone())
        .unwrap_or_default()
}

fn backfill_evidence_refs(
    request: &CreateHistoricalBackfillRequest,
    leads: &[HistoricalBackfillLeadRecord],
) -> Vec<String> {
    let mut refs = request
        .dataset_refs
        .iter()
        .map(|value| format!("datasets:{value}"))
        .chain(
            request
                .rule_refs
                .iter()
                .map(|value| format!("rules:{value}")),
        )
        .collect::<Vec<_>>();
    refs.extend(
        leads
            .iter()
            .map(|lead| format!("fwa_leads:{}", lead.lead_id)),
    );
    refs
}

fn backfill_job_from_event(event: &AuditHistoryEventRecord) -> Option<HistoricalBackfillJobRecord> {
    Some(HistoricalBackfillJobRecord {
        job_id: event.payload["job_id"].as_str()?.to_string(),
        status: event.payload["status"]
            .as_str()
            .unwrap_or("completed")
            .to_string(),
        dataset_refs: json_array_to_strings(&event.payload["dataset_refs"]),
        rule_refs: json_array_to_strings(&event.payload["rule_refs"]),
        candidate_count: event.payload["candidate_count"].as_u64().unwrap_or(0) as u32,
        leads: event
            .payload
            .get("leads")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(backfill_lead_from_value)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        reviewer: event.payload["reviewer"].as_str().map(str::to_string),
        notes: event.payload["notes"].as_str().map(str::to_string),
        evidence_refs: event.evidence_refs.clone(),
        created_at: event.created_at.clone(),
    })
}

fn backfill_lead_from_value(value: &Value) -> Option<HistoricalBackfillLeadRecord> {
    Some(HistoricalBackfillLeadRecord {
        lead_id: value["lead_id"].as_str()?.to_string(),
        claim_id: value["claim_id"].as_str()?.to_string(),
        scheme_family: value["scheme_family"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        risk_score: value["risk_score"].as_u64().unwrap_or(0).min(100) as u8,
        rag: value["rag"].as_str().unwrap_or_default().to_string(),
        status: value["status"].as_str().unwrap_or_default().to_string(),
        reason: value["reason"].as_str().unwrap_or_default().to_string(),
        evidence_refs: json_array_to_strings(&value["evidence_refs"]),
    })
}

fn evidence_request_from_scoring_event(
    event: &AuditHistoryEventRecord,
    request: &GenerateEvidenceRequestsRequest,
    actor: &ActorContext,
) -> Option<EvidenceRequestRecord> {
    let clinical = &event.payload["clinical_evidence"];
    let missing_evidence = json_array_to_strings(&clinical["missing_evidence"]);
    if missing_evidence.is_empty() {
        return None;
    }
    let claim_id = event.payload["claim_id"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    if claim_id.is_empty() {
        return None;
    }
    let request_id = format!("evidence_request_{}", event.audit_id);
    let items = missing_evidence
        .iter()
        .enumerate()
        .map(|(index, document_type)| EvidenceRequestItemRecord {
            item_id: format!("{request_id}_item_{}", index + 1),
            document_type: document_type.clone(),
            status: "open".into(),
            reason: evidence_request_reason(document_type).into(),
            blocking: true,
            policy_authority_ref: Some("policy:clinical-evidence:v1".into()),
            exception_check: Some("required_clinical_documents_not_present".into()),
        })
        .collect::<Vec<_>>();
    Some(EvidenceRequestRecord {
        request_id,
        claim_id,
        scoring_audit_id: event.audit_id.clone(),
        status: "open".into(),
        request_reason: "missing_clinical_evidence".into(),
        missing_evidence,
        items,
        reviewer_queue: request
            .reviewer_queue
            .clone()
            .unwrap_or_else(|| "clinical-evidence".into()),
        requested_by: request
            .requested_by
            .clone()
            .unwrap_or_else(|| actor.actor_id.clone()),
        notes: request.notes.clone(),
        evidence_refs: event.evidence_refs.clone(),
        created_at: event.created_at.clone(),
        updated_at: None,
    })
}

fn evidence_request_payload(customer_scope_id: &str, request: &EvidenceRequestRecord) -> Value {
    json!({
        "customer_scope_id": customer_scope_id,
        "request_id": request.request_id,
        "claim_id": request.claim_id,
        "scoring_audit_id": request.scoring_audit_id,
        "status": request.status,
        "request_reason": request.request_reason,
        "missing_evidence": request.missing_evidence,
        "items": request.items,
        "reviewer_queue": request.reviewer_queue,
        "requested_by": request.requested_by,
        "notes": request.notes,
    })
}

fn evidence_request_from_event(event: &AuditHistoryEventRecord) -> Option<EvidenceRequestRecord> {
    Some(EvidenceRequestRecord {
        request_id: event.payload["request_id"].as_str()?.to_string(),
        claim_id: event.payload["claim_id"].as_str()?.to_string(),
        scoring_audit_id: event.payload["scoring_audit_id"].as_str()?.to_string(),
        status: event.payload["status"]
            .as_str()
            .unwrap_or("open")
            .to_string(),
        request_reason: event.payload["request_reason"]
            .as_str()
            .unwrap_or("missing_clinical_evidence")
            .to_string(),
        missing_evidence: json_array_to_strings(&event.payload["missing_evidence"]),
        items: event
            .payload
            .get("items")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(evidence_request_item_from_value)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        reviewer_queue: event.payload["reviewer_queue"]
            .as_str()
            .unwrap_or("clinical-evidence")
            .to_string(),
        requested_by: event.payload["requested_by"]
            .as_str()
            .unwrap_or("ops")
            .to_string(),
        notes: event.payload["notes"].as_str().map(str::to_string),
        evidence_refs: event.evidence_refs.clone(),
        created_at: event.created_at.clone(),
        updated_at: None,
    })
}

fn evidence_request_item_from_value(value: &Value) -> Option<EvidenceRequestItemRecord> {
    Some(EvidenceRequestItemRecord {
        item_id: value["item_id"].as_str()?.to_string(),
        document_type: value["document_type"].as_str()?.to_string(),
        status: value["status"].as_str().unwrap_or("open").to_string(),
        reason: value["reason"].as_str().unwrap_or_default().to_string(),
        blocking: value["blocking"].as_bool().unwrap_or(true),
        policy_authority_ref: value["policy_authority_ref"].as_str().map(str::to_string),
        exception_check: value["exception_check"].as_str().map(str::to_string),
    })
}

fn evidence_request_reason(document_type: &str) -> &'static str {
    match document_type {
        "radiology_report" => "radiology evidence is required before clinical necessity review",
        "dental_xray" => "dental X-ray evidence is required before clinical necessity review",
        "medication_order" | "prescription" | "prescription_detail" => {
            "medication detail is required before pharmacy review"
        }
        "operation_record" => "operation record is required before surgical necessity review",
        "clinical_order" | "medical_record" => {
            "source clinical documentation is required before adjudication"
        }
        _ => "supporting evidence is required before label approval",
    }
}

fn label_item_from_evidence_request(request: EvidenceRequestRecord) -> LabelBootstrapItemRecord {
    let received = request.status == "received";
    LabelBootstrapItemRecord {
        item_id: format!("label_bootstrap_{}", request.request_id),
        claim_id: request.claim_id,
        source_type: "evidence_request".into(),
        source_id: request.request_id,
        suggested_label_name: if received {
            "clinical_evidence_sufficient".into()
        } else {
            "insufficient_evidence".into()
        },
        suggested_label_value: "true".into(),
        governance_status: "needs_review".into(),
        training_eligible: false,
        review_status: "open".into(),
        review_audit_id: None,
        reviewer: None,
        feedback_target: "workflow".into(),
        evidence_refs: request.evidence_refs,
        created_at: request.created_at,
    }
}

fn apply_label_review(item: &mut LabelBootstrapItemRecord, event: &AuditHistoryEventRecord) {
    item.review_status = "reviewed".into();
    item.review_audit_id = Some(event.audit_id.clone());
    item.reviewer = event.payload["reviewer"].as_str().map(str::to_string);
    item.suggested_label_name = event.payload["label_name"]
        .as_str()
        .unwrap_or(&item.suggested_label_name)
        .to_string();
    item.suggested_label_value = event.payload["label_value"]
        .as_str()
        .unwrap_or(&item.suggested_label_value)
        .to_string();
    item.governance_status = event.payload["governance_status"]
        .as_str()
        .unwrap_or(&item.governance_status)
        .to_string();
    item.feedback_target = event.payload["feedback_target"]
        .as_str()
        .unwrap_or(&item.feedback_target)
        .to_string();
    item.training_eligible = item.governance_status == "approved_for_training";
    if !event.evidence_refs.is_empty() {
        item.evidence_refs = event.evidence_refs.clone();
    }
}

fn json_array_to_strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn not_found(code: &'static str, message: &'static str) -> impl FnOnce() -> ApiError {
    move || ApiError::new(StatusCode::NOT_FOUND, code, message)
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}
