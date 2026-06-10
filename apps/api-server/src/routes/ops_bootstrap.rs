use super::ops_bootstrap_types::*;
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
use serde_json::{json, Value};
use std::collections::BTreeMap;

mod backfill;
mod evidence;
use backfill::{
    backfill_evidence_refs, backfill_job_from_event, backfill_lead_from_lead, first_claim_id,
    validate_backfill_request,
};
use evidence::{
    evidence_request_from_scoring_event, evidence_request_payload, has_document_evidence_ref,
    load_evidence_requests, validate_evidence_status_update, validate_generate_evidence_request,
};

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
