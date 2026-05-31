use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        AgentRunLogRecord, AuditEventListFilter, AuditHistoryEventRecord, AuditSampleLeadRecord,
        AuditSampleRecord, CaseRecord, InvestigationResultRecord, LeadRecord,
        MemberProfileSummaryRecord, OutcomeLabelRecord, QaFeedbackItemRecord, QaReviewRecord,
        UpdateQaFeedbackStatusInput, UpdateQaFeedbackStatusRecord, WebhookDeliveryAttemptInput,
        WebhookDeliveryAttemptRecord, WebhookEventRecord,
    },
    routes::pii,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_auth::{validate_api_key, ApiKeyConfig};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct PilotWritebackResponse {
    pub claim_id: String,
    pub event_type: String,
    pub event_status: String,
    pub audit_id: String,
    pub run_id: String,
    pub idempotency_key: String,
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

#[derive(Debug, Deserialize)]
pub struct SubmitWebhookDeliveryAttemptRequest {
    pub delivery_status: String,
    pub response_status_code: Option<u16>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpsAlertRecord {
    pub alert_id: String,
    pub alert_type: String,
    pub severity: String,
    pub status: String,
    pub claim_id: String,
    pub lead_id: Option<String>,
    pub case_id: Option<String>,
    pub scheme_family: String,
    pub message: String,
    pub recommended_action: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct OpsAlertListResponse {
    pub alerts: Vec<OpsAlertRecord>,
}

#[derive(Debug, Serialize)]
pub struct QaFeedbackItemListResponse {
    pub items: Vec<QaFeedbackItemRecord>,
}

#[derive(Debug, Default, Deserialize)]
pub struct QaFeedbackItemListQuery {
    pub status: Option<String>,
    pub feedback_target: Option<String>,
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
    pub in_progress_count: u32,
    pub resolved_count: u32,
    pub dismissed_count: u32,
    pub unresolved_count: u32,
    pub rules_feedback_count: u32,
    pub models_feedback_count: u32,
    pub features_feedback_count: u32,
    pub provider_profile_feedback_count: u32,
    pub workflow_feedback_count: u32,
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
    validate_investigation_result_request(&request)?;
    validate_investigation_case_link(&state, &request).await?;
    let claim_id = request.claim_id.clone();
    let event = state
        .repository
        .save_investigation_result(request)
        .await
        .map_err(internal_error("INVESTIGATION_RESULT_SAVE_FAILED"))?;
    Ok(Json(PilotWritebackResponse {
        claim_id,
        idempotency_key: writeback_idempotency_key(&event),
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
    validate_qa_review_request(&request)?;
    let claim_id = request.claim_id.clone();
    let event = state
        .repository
        .save_qa_review(request)
        .await
        .map_err(internal_error("QA_RESULT_SAVE_FAILED"))?;
    Ok(Json(PilotWritebackResponse {
        claim_id,
        idempotency_key: writeback_idempotency_key(&event),
        event_type: event.event_type,
        event_status: event.event_status,
        audit_id: event.audit_id,
        run_id: event.run_id,
        evidence_refs: event.evidence_refs,
    }))
}

fn writeback_idempotency_key(event: &AuditHistoryEventRecord) -> String {
    format!("tpa-writeback:{}:{}", event.event_type, event.audit_id)
}

fn validate_investigation_result_request(
    request: &InvestigationResultRecord,
) -> Result<(), ApiError> {
    if request.claim_id.trim().is_empty()
        || request.investigation_id.trim().is_empty()
        || request.outcome.trim().is_empty()
        || request
            .case_id
            .as_ref()
            .is_some_and(|case_id| case_id.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_INVESTIGATION_RESULT_IDENTITY",
            "claim_id, investigation_id, outcome, and nonblank case_id when provided are required",
        ));
    }
    if let Some(financial_impact_type) = &request.financial_impact_type {
        if !matches!(
            financial_impact_type.as_str(),
            "prevented_payment"
                | "recovered_amount"
                | "avoided_future_exposure"
                | "deterrence_estimate"
                | "estimated_impact"
        ) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "UNSUPPORTED_FINANCIAL_IMPACT_TYPE",
                "financial_impact_type is not supported",
            ));
        }
    }
    if request
        .saving_amount
        .is_some_and(|amount| amount < Decimal::ZERO)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_INVESTIGATION_SAVING_AMOUNT",
            "saving_amount must be non-negative",
        ));
    }
    if request.notes.trim().is_empty()
        || request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_INVESTIGATION_RESULT_EVIDENCE",
            "investigation writeback requires notes and evidence_refs",
        ));
    }
    validate_writeback_pii(&request.notes, &request.evidence_refs)?;
    Ok(())
}

async fn validate_investigation_case_link(
    state: &AppState,
    request: &InvestigationResultRecord,
) -> Result<(), ApiError> {
    let Some(case_id) = request.case_id.as_deref() else {
        return Ok(());
    };
    let cases = state
        .repository
        .list_cases()
        .await
        .map_err(internal_error("CASE_LOOKUP_FAILED"))?;
    if cases
        .iter()
        .any(|case| case.case_id == case_id && case.claim_id == request.claim_id)
    {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "CASE_NOT_FOUND",
            "case not found for investigation result claim",
        ))
    }
}

fn validate_qa_review_request(request: &QaReviewRecord) -> Result<(), ApiError> {
    if request.qa_case_id.trim().is_empty() || request.claim_id.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_QA_RESULT_IDENTITY",
            "qa_case_id and claim_id are required",
        ));
    }
    if !matches!(
        request.qa_conclusion.as_str(),
        "pass" | "issue_found_return" | "issue_found_escalate"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "UNSUPPORTED_QA_CONCLUSION",
            "qa_conclusion must be pass, issue_found_return, or issue_found_escalate",
        ));
    }
    if !matches!(
        request.issue_type.as_str(),
        "none"
            | "confirmed_fwa"
            | "false_positive"
            | "improper_payment"
            | "insufficient_evidence"
            | "abuse_not_fraud"
            | "documentation_issue"
            | "medical_necessity_issue"
            | "policy_exclusion"
            | "qa_review_completed"
            | "alert_handling_incomplete"
            | "medical_reasonableness"
            | "provider_pattern"
            | "model_under_scored_confirmed_issue"
            | "workflow_missing_evidence"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "UNSUPPORTED_QA_ISSUE_TYPE",
            "issue_type is not supported",
        ));
    }
    if !is_supported_qa_feedback_target(&request.feedback_target) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "UNSUPPORTED_FEEDBACK_TARGET",
            "feedback_target must be rules, models, features, provider_profile, workflow, or tpa",
        ));
    }
    if request.notes.trim().is_empty()
        || request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_QA_RESULT_EVIDENCE",
            "QA writeback requires notes and evidence_refs",
        ));
    }
    validate_writeback_pii(&request.notes, &request.evidence_refs)?;
    Ok(())
}

fn validate_writeback_pii(notes: &str, evidence_refs: &[String]) -> Result<(), ApiError> {
    if pii::contains_pii(std::iter::once(notes).chain(evidence_refs.iter().map(String::as_str))) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_WRITEBACK",
            "writeback notes and evidence_refs must not contain PII",
        ));
    }
    Ok(())
}

pub async fn list_qa_feedback_items(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<QaFeedbackItemListQuery>,
) -> Result<Json<QaFeedbackItemListResponse>, ApiError> {
    authorize(&state, &headers)?;
    validate_qa_feedback_item_list_query(&query)?;
    let mut items = state
        .repository
        .list_qa_feedback_items()
        .await
        .map_err(internal_error("QA_FEEDBACK_LIST_FAILED"))?;
    if let Some(status) = &query.status {
        items.retain(|item| item.status == *status);
    }
    if let Some(feedback_target) = &query.feedback_target {
        items.retain(|item| item.feedback_target == *feedback_target);
    }
    Ok(Json(QaFeedbackItemListResponse { items }))
}

pub async fn update_qa_feedback_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(feedback_id): Path<String>,
    Json(request): Json<UpdateQaFeedbackStatusInput>,
) -> Result<Json<UpdateQaFeedbackStatusRecord>, ApiError> {
    authorize(&state, &headers)?;
    if !is_supported_qa_feedback_status(&request.status) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "UNSUPPORTED_QA_FEEDBACK_STATUS",
            "feedback status must be one of open, in_progress, resolved, dismissed",
        ));
    }
    if request.actor_id.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_QA_FEEDBACK_STATUS_UPDATE",
            "actor_id is required",
        ));
    }
    if request.notes.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_QA_FEEDBACK_STATUS_NOTES",
            "QA feedback status updates require notes",
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
            "MISSING_QA_FEEDBACK_STATUS_EVIDENCE",
            "QA feedback status updates require evidence_refs",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.notes.as_str())
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_QA_FEEDBACK_STATUS",
            "QA feedback status notes and evidence_refs must not contain PII",
        ));
    }
    let required_ref = format!("qa_feedback:{feedback_id}");
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == required_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_QA_FEEDBACK_TARGET_EVIDENCE",
            format!("QA feedback status evidence_refs must include {required_ref}"),
        ));
    }
    let record = state
        .repository
        .update_qa_feedback_status(&feedback_id, request)
        .await
        .map_err(internal_error("QA_FEEDBACK_STATUS_UPDATE_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "QA_FEEDBACK_NOT_FOUND",
                "QA feedback item not found",
            )
        })?;
    Ok(Json(record))
}

fn validate_qa_feedback_item_list_query(query: &QaFeedbackItemListQuery) -> Result<(), ApiError> {
    if let Some(status) = &query.status {
        if !is_supported_qa_feedback_status(status) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "UNSUPPORTED_QA_FEEDBACK_STATUS",
                "feedback status must be one of open, in_progress, resolved, dismissed",
            ));
        }
    }
    if let Some(feedback_target) = &query.feedback_target {
        if !is_supported_qa_feedback_target(feedback_target) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "UNSUPPORTED_FEEDBACK_TARGET",
                "feedback_target must be rules, models, features, provider_profile, workflow, or tpa",
            ));
        }
    }
    Ok(())
}

fn is_supported_qa_feedback_status(status: &str) -> bool {
    matches!(status, "open" | "in_progress" | "resolved" | "dismissed")
}

fn is_supported_qa_feedback_target(feedback_target: &str) -> bool {
    matches!(
        feedback_target,
        "rules" | "models" | "features" | "provider_profile" | "workflow" | "tpa"
    )
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

pub async fn submit_webhook_delivery_attempt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(event_id): Path<String>,
    Json(request): Json<SubmitWebhookDeliveryAttemptRequest>,
) -> Result<Json<WebhookDeliveryAttemptRecord>, ApiError> {
    authorize(&state, &headers)?;
    if !matches!(request.delivery_status.as_str(), "delivered" | "failed") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_WEBHOOK_DELIVERY_STATUS",
            "delivery_status must be delivered or failed",
        ));
    }
    if request
        .error_message
        .as_deref()
        .is_some_and(|message| pii::contains_pii([message]))
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_WEBHOOK_DELIVERY",
            "webhook delivery error_message must not contain PII",
        ));
    }
    let known_event = state
        .repository
        .list_webhook_events()
        .await
        .map_err(internal_error("WEBHOOK_EVENT_LIST_FAILED"))?
        .into_iter()
        .any(|event| event.event_id == event_id);
    if !known_event {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "WEBHOOK_EVENT_NOT_FOUND",
            "webhook event not found",
        ));
    }
    let attempt = state
        .repository
        .save_webhook_delivery_attempt(WebhookDeliveryAttemptInput {
            event_id,
            delivery_status: request.delivery_status,
            response_status_code: request.response_status_code,
            error_message: request.error_message,
        })
        .await
        .map_err(internal_error("WEBHOOK_DELIVERY_ATTEMPT_SAVE_FAILED"))?;
    Ok(Json(attempt))
}

pub async fn list_ops_alerts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OpsAlertListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let leads = state
        .repository
        .list_leads()
        .await
        .map_err(internal_error("LEAD_LIST_FAILED"))?;
    let cases = state
        .repository
        .list_cases()
        .await
        .map_err(internal_error("CASE_LIST_FAILED"))?;
    let scoring_events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 1_000,
            event_type: Some("scoring.completed".into()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("ALERT_AUDIT_LIST_FAILED"))?;
    let medical_review_events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 1_000,
            event_type: Some("medical.review.recorded".into()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("ALERT_AUDIT_LIST_FAILED"))?;
    let agent_runs = state
        .repository
        .list_agent_runs()
        .await
        .map_err(internal_error("AGENT_RUN_LIST_FAILED"))?;
    Ok(Json(OpsAlertListResponse {
        alerts: build_ops_alerts(
            &leads,
            &cases,
            &scoring_events,
            &medical_review_events,
            &agent_runs,
        ),
    }))
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
    let in_progress_count = items
        .iter()
        .filter(|item| item.status == "in_progress")
        .count() as u32;
    QaQueueSummaryResponse {
        open_count: open_items.len() as u32,
        in_progress_count,
        resolved_count: items
            .iter()
            .filter(|item| item.status == "resolved")
            .count() as u32,
        dismissed_count: items
            .iter()
            .filter(|item| item.status == "dismissed")
            .count() as u32,
        unresolved_count: open_items.len() as u32 + in_progress_count,
        rules_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "rules")
            .count() as u32,
        models_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "models")
            .count() as u32,
        features_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "features")
            .count() as u32,
        provider_profile_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "provider_profile")
            .count() as u32,
        workflow_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "workflow")
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

fn build_ops_alerts(
    leads: &[LeadRecord],
    cases: &[CaseRecord],
    scoring_events: &[AuditHistoryEventRecord],
    medical_review_events: &[AuditHistoryEventRecord],
    agent_runs: &[AgentRunLogRecord],
) -> Vec<OpsAlertRecord> {
    let mut alerts = leads
        .iter()
        .filter(|lead| lead.status != "triaged" && (lead.risk_score >= 70 || lead.rag == "RED"))
        .map(high_risk_routing_alert)
        .chain(
            cases
                .iter()
                .filter(|case| matches!(case.sla_status.as_str(), "breached" | "closed_breached"))
                .map(sla_breach_alert),
        )
        .chain(build_medical_review_alerts(
            scoring_events,
            medical_review_events,
        ))
        .chain(build_agent_approval_alerts(agent_runs))
        .collect::<Vec<_>>();
    alerts.sort_by(|left, right| {
        severity_rank(&left.severity)
            .cmp(&severity_rank(&right.severity))
            .then_with(|| left.alert_type.cmp(&right.alert_type))
            .then_with(|| left.alert_id.cmp(&right.alert_id))
    });
    alerts
}

fn build_agent_approval_alerts(agent_runs: &[AgentRunLogRecord]) -> Vec<OpsAlertRecord> {
    agent_runs
        .iter()
        .flat_map(|run| {
            run.approvals
                .iter()
                .filter(|approval| approval.decision == "pending")
                .map(move |approval| agent_approval_alert(run, approval))
        })
        .collect()
}

fn agent_approval_alert(
    run: &AgentRunLogRecord,
    approval: &crate::repository::AgentApprovalRecord,
) -> OpsAlertRecord {
    let mut evidence_refs = approval.evidence_refs.clone();
    evidence_refs.extend(run.evidence_refs.clone());
    evidence_refs.push(format!("agent_run:{}", run.agent_run_id));
    OpsAlertRecord {
        alert_id: format!("alert_agent_approval_{}", approval.approval_id),
        alert_type: "agent_approval_pending".into(),
        severity: "high".into(),
        status: "open".into(),
        claim_id: run.claim_id.clone(),
        lead_id: None,
        case_id: None,
        scheme_family: run.output_json["evidence_sufficiency"]["scheme_family"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        message: format!(
            "Agent output {} for claim {} is waiting for human approval.",
            run.agent_run_id, run.claim_id
        ),
        recommended_action: "Review the evidence package and approve or reject the Agent output."
            .into(),
        evidence_refs: dedupe_strings(evidence_refs),
    }
}

fn build_medical_review_alerts(
    scoring_events: &[AuditHistoryEventRecord],
    medical_review_events: &[AuditHistoryEventRecord],
) -> Vec<OpsAlertRecord> {
    let reviewed_scoring_audit_ids = medical_review_events
        .iter()
        .filter_map(|event| event.payload["scoring_audit_id"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    scoring_events
        .iter()
        .filter(|event| !reviewed_scoring_audit_ids.contains(event.audit_id.as_str()))
        .filter_map(medical_review_alert_from_scoring_event)
        .collect()
}

fn medical_review_alert_from_scoring_event(
    event: &AuditHistoryEventRecord,
) -> Option<OpsAlertRecord> {
    let clinical = &event.payload["clinical_evidence"];
    let review_required = clinical["review_required"].as_bool().unwrap_or(false);
    let review_route = clinical["review_route"].as_str().unwrap_or_default();
    if !review_required && review_route != "medical_review" {
        return None;
    }
    let claim_id = event.payload["claim_id"].as_str()?.to_string();
    let medical_score = event.payload["scores"]["medical_reasonableness_score"]
        .as_u64()
        .unwrap_or_default();
    let mut evidence_refs = vec![format!("audit:{}", event.audit_id)];
    evidence_refs.extend(json_string_array(&clinical["evidence_refs"]));
    Some(OpsAlertRecord {
        alert_id: format!("alert_medical_review_{}", event.audit_id),
        alert_type: "medical_review_required".into(),
        severity: if medical_score >= 80 {
            "high"
        } else {
            "medium"
        }
        .into(),
        status: "open".into(),
        claim_id: claim_id.clone(),
        lead_id: None,
        case_id: None,
        scheme_family: "medically_unnecessary_service".into(),
        message: format!("Claim {claim_id} has clinical evidence gaps requiring medical review."),
        recommended_action:
            "Assign a medical reviewer and record an evidence-backed review result.".into(),
        evidence_refs: dedupe_strings(evidence_refs),
    })
}

fn high_risk_routing_alert(lead: &LeadRecord) -> OpsAlertRecord {
    OpsAlertRecord {
        alert_id: format!("alert_high_risk_{}", lead.lead_id),
        alert_type: "high_risk_routing".into(),
        severity: if lead.risk_score >= 90 || lead.rag == "RED" {
            "critical".into()
        } else {
            "high".into()
        },
        status: "open".into(),
        claim_id: lead.claim_id.clone(),
        lead_id: Some(lead.lead_id.clone()),
        case_id: None,
        scheme_family: lead.scheme_family.clone(),
        message: format!(
            "High-risk FWA lead {} for claim {} is pending triage.",
            lead.lead_id, lead.claim_id
        ),
        recommended_action: "Open an investigation case and assign reviewer ownership.".into(),
        evidence_refs: lead.evidence_refs.clone(),
    }
}

fn sla_breach_alert(case: &CaseRecord) -> OpsAlertRecord {
    OpsAlertRecord {
        alert_id: format!("alert_sla_{}", case.case_id),
        alert_type: "sla_breach".into(),
        severity: match case.priority.as_str() {
            "critical" | "high" => "critical",
            "medium" => "high",
            _ => "medium",
        }
        .into(),
        status: if case.sla_status == "closed_breached" {
            "closed".into()
        } else {
            "open".into()
        },
        claim_id: case.claim_id.clone(),
        lead_id: Some(case.lead_id.clone()),
        case_id: Some(case.case_id.clone()),
        scheme_family: case.scheme_family.clone(),
        message: format!(
            "Case {} for claim {} breached the {}h SLA target.",
            case.case_id, case.claim_id, case.sla_target_hours
        ),
        recommended_action: "Escalate the overdue case and record owner follow-up.".into(),
        evidence_refs: case_evidence_refs(case),
    }
}

fn case_evidence_refs(case: &CaseRecord) -> Vec<String> {
    let refs = case
        .evidence_package
        .get("evidence_refs")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if refs.is_empty() {
        vec![format!("investigation_cases:{}", case.case_id)]
    } else {
        refs
    }
}

fn json_string_array(value: &serde_json::Value) -> Vec<String> {
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

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        _ => 3,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_ops_alerts_includes_sla_breach_alerts() {
        let case = CaseRecord {
            case_id: "case_CLM-SLA-1".into(),
            lead_id: "lead_CLM-SLA-1".into(),
            claim_id: "CLM-SLA-1".into(),
            member_id: "MBR-SLA-1".into(),
            provider_id: "PRV-SLA-1".into(),
            source_system: "tpa-demo".into(),
            scheme_family: "provider_peer_outlier".into(),
            lead_source: "scoring_run".into(),
            status: "investigating".into(),
            assignee: "siu-owner".into(),
            reviewer: "medical-owner".into(),
            priority: "high".into(),
            routing_reason: "Provider peer outlier".into(),
            evidence_package: serde_json::json!({
                "evidence_refs": ["rule_runs:PROVIDER_PROFILE_HIGH", "case_workflow:overdue"]
            }),
            sla_target_hours: 24,
            sla_status: "breached".into(),
            time_to_triage_hours: 0.0,
            time_to_closure_hours: None,
            final_outcome: None,
            reviewer_notes: None,
            investigation_result_id: None,
        };

        let alerts = build_ops_alerts(&[], &[case], &[], &[], &[]);

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, "sla_breach");
        assert_eq!(alerts[0].severity, "critical");
        assert_eq!(alerts[0].status, "open");
        assert_eq!(alerts[0].claim_id, "CLM-SLA-1");
        assert_eq!(alerts[0].case_id.as_deref(), Some("case_CLM-SLA-1"));
        assert_eq!(
            alerts[0].evidence_refs,
            vec![
                "rule_runs:PROVIDER_PROFILE_HIGH".to_string(),
                "case_workflow:overdue".to_string()
            ]
        );
    }

    #[test]
    fn build_ops_alerts_includes_open_medical_review_alerts() {
        let scoring_event = AuditHistoryEventRecord {
            audit_id: "audit_scoring_medical_1".into(),
            run_id: "run_medical_1".into(),
            event_type: "scoring.completed".into(),
            event_status: "succeeded".into(),
            summary: "FWA scoring completed".into(),
            payload: serde_json::json!({
                "claim_id": "CLM-MED-ALERT-1",
                "scores": {
                    "medical_reasonableness_score": 88
                },
                "clinical_evidence": {
                    "review_required": true,
                    "review_route": "medical_review",
                    "evidence_refs": ["claim_items:IMG-900"]
                }
            }),
            evidence_refs: vec!["claim_items:IMG-900".into()],
            created_at: None,
        };

        let alerts = build_ops_alerts(&[], &[], &[scoring_event], &[], &[]);

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, "medical_review_required");
        assert_eq!(alerts[0].severity, "high");
        assert_eq!(alerts[0].claim_id, "CLM-MED-ALERT-1");
        assert!(alerts[0]
            .evidence_refs
            .contains(&"audit:audit_scoring_medical_1".to_string()));
    }

    #[test]
    fn build_ops_alerts_includes_pending_agent_approval_alerts() {
        let run = AgentRunLogRecord {
            agent_run_id: "agent_CLM-AGENT-ALERT-1".into(),
            claim_id: "CLM-AGENT-ALERT-1".into(),
            status: "succeeded".into(),
            decision_boundary: "assistive_only".into(),
            output_json: serde_json::json!({
                "evidence_sufficiency": {
                    "scheme_family": "provider_peer_outlier"
                }
            }),
            evidence_refs: vec!["knowledge_cases:KC-1001".into()],
            steps: vec![],
            context_snapshots: vec![],
            policy_checks: vec![],
            tool_calls: vec![],
            tool_results: vec![],
            approvals: vec![crate::repository::AgentApprovalRecord {
                approval_id: "approval_agent_CLM-AGENT-ALERT-1".into(),
                agent_run_id: "agent_CLM-AGENT-ALERT-1".into(),
                proposed_action: "manual_review_required".into(),
                decision: "pending".into(),
                approver: "unassigned".into(),
                reason: "Agent output requires human approval before downstream action.".into(),
                evidence_refs: vec!["agent_run:agent_CLM-AGENT-ALERT-1".into()],
                created_at: None,
            }],
            created_at: None,
            completed_at: None,
        };

        let alerts = build_ops_alerts(&[], &[], &[], &[], &[run]);

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, "agent_approval_pending");
        assert_eq!(alerts[0].severity, "high");
        assert_eq!(alerts[0].claim_id, "CLM-AGENT-ALERT-1");
        assert_eq!(alerts[0].scheme_family, "provider_peer_outlier");
        assert!(alerts[0]
            .evidence_refs
            .contains(&"agent_run:agent_CLM-AGENT-ALERT-1".to_string()));
        assert!(alerts[0]
            .evidence_refs
            .contains(&"knowledge_cases:KC-1001".to_string()));
    }
}
