use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        AuditSampleLeadRecord, AuditSampleRecord, InvestigationResultRecord,
        MemberProfileSummaryRecord, OutcomeLabelRecord, QaFeedbackItemRecord, QaReviewRecord,
        WebhookEventRecord,
    },
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_auth::{validate_api_key, ApiKeyConfig};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PilotWritebackResponse {
    pub claim_id: String,
    pub event_type: String,
    pub event_status: String,
    pub audit_id: String,
    pub run_id: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ClaimAuditHistoryResponse {
    pub claim_id: String,
    pub events: Vec<crate::repository::AuditHistoryEventRecord>,
}

#[derive(Debug, Serialize)]
pub struct WebhookEventListResponse {
    pub events: Vec<WebhookEventRecord>,
}

#[derive(Debug, Serialize)]
pub struct QaFeedbackItemListResponse {
    pub items: Vec<QaFeedbackItemRecord>,
}

#[derive(Debug, Serialize)]
pub struct QaQueueItemResponse {
    pub qa_case_id: String,
    pub sample_id: String,
    pub lead_id: String,
    pub claim_id: String,
    pub scheme_family: String,
    pub rag: String,
    pub risk_score: u8,
    pub reviewer: String,
    pub assignment_queue: String,
    pub status: String,
    pub qa_conclusion: Option<String>,
    pub issue_type: Option<String>,
    pub feedback_target: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct QaQueueListResponse {
    pub items: Vec<QaQueueItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct QaQueueSummaryResponse {
    pub open_count: u32,
    pub rules_feedback_count: u32,
    pub models_feedback_count: u32,
    pub tpa_feedback_count: u32,
    pub high_priority_count: u32,
    pub evidence_backed_count: u32,
    pub highest_priority: String,
}

#[derive(Debug, Serialize)]
pub struct OutcomeLabelListResponse {
    pub labels: Vec<OutcomeLabelRecord>,
}

pub async fn member_profile_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(member_id): Path<String>,
) -> Result<Json<MemberProfileSummaryRecord>, ApiError> {
    authorize(&state, &headers)?;
    let profile = state
        .repository
        .member_profile_summary(&member_id)
        .await
        .map_err(internal_error("MEMBER_PROFILE_SUMMARY_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MEMBER_NOT_FOUND",
                "member not found",
            )
        })?;
    Ok(Json(profile))
}

pub async fn write_investigation_result(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<InvestigationResultRecord>,
) -> Result<Json<PilotWritebackResponse>, ApiError> {
    authorize(&state, &headers)?;
    let claim_id = request.claim_id.clone();
    let event = state
        .repository
        .save_investigation_result(request)
        .await
        .map_err(internal_error("INVESTIGATION_RESULT_SAVE_FAILED"))?;
    Ok(Json(PilotWritebackResponse {
        claim_id,
        event_type: event.event_type,
        event_status: event.event_status,
        audit_id: event.audit_id,
        run_id: event.run_id,
        evidence_refs: event.evidence_refs,
    }))
}

pub async fn write_qa_result(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<QaReviewRecord>,
) -> Result<Json<PilotWritebackResponse>, ApiError> {
    authorize(&state, &headers)?;
    let claim_id = request.claim_id.clone();
    let event = state
        .repository
        .save_qa_review(request)
        .await
        .map_err(internal_error("QA_RESULT_SAVE_FAILED"))?;
    Ok(Json(PilotWritebackResponse {
        claim_id,
        event_type: event.event_type,
        event_status: event.event_status,
        audit_id: event.audit_id,
        run_id: event.run_id,
        evidence_refs: event.evidence_refs,
    }))
}

pub async fn list_qa_feedback_items(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<QaFeedbackItemListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let items = state
        .repository
        .list_qa_feedback_items()
        .await
        .map_err(internal_error("QA_FEEDBACK_LIST_FAILED"))?;
    Ok(Json(QaFeedbackItemListResponse { items }))
}

pub async fn list_qa_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<QaQueueListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let samples = state
        .repository
        .list_audit_samples()
        .await
        .map_err(internal_error("AUDIT_SAMPLE_LIST_FAILED"))?;
    let reviews = state
        .repository
        .list_qa_reviews()
        .await
        .map_err(internal_error("QA_REVIEW_LIST_FAILED"))?;
    Ok(Json(QaQueueListResponse {
        items: build_qa_queue_items(&samples, &reviews),
    }))
}

pub async fn qa_queue_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<QaQueueSummaryResponse>, ApiError> {
    authorize(&state, &headers)?;
    let items = state
        .repository
        .list_qa_feedback_items()
        .await
        .map_err(internal_error("QA_FEEDBACK_LIST_FAILED"))?;
    Ok(Json(build_qa_queue_summary(&items)))
}

pub async fn list_outcome_labels(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OutcomeLabelListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let labels = state
        .repository
        .list_outcome_labels()
        .await
        .map_err(internal_error("OUTCOME_LABEL_LIST_FAILED"))?;
    Ok(Json(OutcomeLabelListResponse { labels }))
}

pub async fn claim_audit_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(claim_id): Path<String>,
) -> Result<Json<ClaimAuditHistoryResponse>, ApiError> {
    authorize(&state, &headers)?;
    let events = state
        .repository
        .claim_audit_history(&claim_id)
        .await
        .map_err(internal_error("CLAIM_AUDIT_HISTORY_FAILED"))?;
    Ok(Json(ClaimAuditHistoryResponse { claim_id, events }))
}

pub async fn list_webhook_events(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<WebhookEventListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let events = state
        .repository
        .list_webhook_events()
        .await
        .map_err(internal_error("WEBHOOK_EVENT_LIST_FAILED"))?;
    Ok(Json(WebhookEventListResponse { events }))
}

fn build_qa_queue_items(
    samples: &[AuditSampleRecord],
    reviews: &[QaReviewRecord],
) -> Vec<QaQueueItemResponse> {
    let reviews_by_case_id = reviews
        .iter()
        .map(|review| (review.qa_case_id.as_str(), review))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut items = samples
        .iter()
        .flat_map(|sample| {
            let reviews_by_case_id = &reviews_by_case_id;
            sample.selected_leads.iter().map(move |lead| {
                let qa_case_id = qa_case_id_for_sample_lead(sample, lead);
                let review = reviews_by_case_id.get(qa_case_id.as_str()).copied();
                qa_queue_item_from_sample(sample, lead, qa_case_id, review)
            })
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        right
            .risk_score
            .cmp(&left.risk_score)
            .then_with(|| left.qa_case_id.cmp(&right.qa_case_id))
    });
    items
}

fn qa_queue_item_from_sample(
    sample: &AuditSampleRecord,
    lead: &AuditSampleLeadRecord,
    qa_case_id: String,
    review: Option<&QaReviewRecord>,
) -> QaQueueItemResponse {
    QaQueueItemResponse {
        qa_case_id,
        sample_id: sample.sample_id.clone(),
        lead_id: lead.lead_id.clone(),
        claim_id: lead.claim_id.clone(),
        scheme_family: lead.scheme_family.clone(),
        rag: lead.rag.clone(),
        risk_score: lead.risk_score,
        reviewer: sample.reviewer.clone(),
        assignment_queue: sample.assignment_queue.clone(),
        status: if review.is_some() { "reviewed" } else { "open" }.into(),
        qa_conclusion: review.map(|review| review.qa_conclusion.clone()),
        issue_type: review.map(|review| review.issue_type.clone()),
        feedback_target: review.map(|review| review.feedback_target.clone()),
        evidence_refs: lead.evidence_refs.clone(),
    }
}

fn qa_case_id_for_sample_lead(sample: &AuditSampleRecord, lead: &AuditSampleLeadRecord) -> String {
    format!("qa_{}_{}", sample.sample_id, lead.lead_id)
}

fn build_qa_queue_summary(items: &[QaFeedbackItemRecord]) -> QaQueueSummaryResponse {
    let open_items = items
        .iter()
        .filter(|item| item.status == "open")
        .collect::<Vec<_>>();
    QaQueueSummaryResponse {
        open_count: open_items.len() as u32,
        rules_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "rules")
            .count() as u32,
        models_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "models")
            .count() as u32,
        tpa_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "tpa")
            .count() as u32,
        high_priority_count: open_items
            .iter()
            .filter(|item| item.priority == "high")
            .count() as u32,
        evidence_backed_count: open_items
            .iter()
            .filter(|item| !item.evidence_refs.is_empty())
            .count() as u32,
        highest_priority: highest_priority(&open_items).into(),
    }
}

fn highest_priority(items: &[&QaFeedbackItemRecord]) -> &'static str {
    if items.iter().any(|item| item.priority == "high") {
        "high"
    } else if items.iter().any(|item| item.priority == "medium") {
        "medium"
    } else if items.iter().any(|item| item.priority == "low") {
        "low"
    } else {
        "none"
    }
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
