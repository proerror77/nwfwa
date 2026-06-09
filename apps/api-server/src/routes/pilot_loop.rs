use crate::{
    app::AppState,
    auth::{AuthenticatedActor, AuthenticatedApiPrincipal},
    error::ApiError,
    repository::{
        canonical_feedback_target, AuditEventListFilter, AuditHistoryEventRecord,
        InvestigationResultRecord, MemberProfileSummaryRecord, QaReviewRecord,
        UpdateQaFeedbackStatusInput, UpdateQaFeedbackStatusRecord, WebhookDeliveryAttemptInput,
        WebhookDeliveryAttemptRecord,
    },
    routes::pii,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use fwa_audit::ActorContext;
use fwa_auth::AuthenticatedPrincipal;
use rust_decimal::Decimal;
use serde_json::Value;

use super::pilot_loop_alerts::build_ops_alerts;
use super::pilot_loop_qa_queue::{
    build_qa_queue_items_from_scoring_events, build_qa_queue_summary,
};
pub use super::pilot_loop_types::*;

pub async fn member_profile_summary(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(member_id): Path<String>,
) -> Result<Json<MemberProfileSummaryRecord>, ApiError> {
    let actor = require_permission(principal, "tpa:members:read")?;
    let profile = state
        .repository
        .member_profile_summary(&member_id, Some(&actor.customer_scope_id))
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
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Json(mut request): Json<InvestigationResultRecord>,
) -> Result<Json<PilotWritebackResponse>, ApiError> {
    let actor = require_permission(principal, "tpa:investigations:write")?;
    validate_investigation_result_request(&request)?;
    ensure_writeback_id_is_available_for_customer(
        &state,
        "investigation.result.received",
        "investigation_id",
        &request.investigation_id,
        &actor.customer_scope_id,
        "INVESTIGATION_RESULT_SCOPE_CONFLICT",
        "investigation_id is already used by another customer scope",
    )
    .await?;
    validate_investigation_case_link(&state, &request, &actor.customer_scope_id).await?;
    merge_latest_canonical_evidence_refs_for_investigation(
        &state,
        &actor.customer_scope_id,
        &mut request,
    )
    .await?;
    request.customer_scope_id = Some(actor.customer_scope_id.clone());
    request.actor_id = Some(actor.actor_id);
    request.actor_role = Some(actor.actor_role);
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
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Json(mut request): Json<QaReviewRecord>,
) -> Result<Json<PilotWritebackResponse>, ApiError> {
    let actor = require_permission(principal, "tpa:qa:write")?;
    validate_qa_review_request(&request)?;
    request.feedback_target = canonical_feedback_target(&request.feedback_target).into();
    ensure_writeback_id_is_available_for_customer(
        &state,
        "qa.result.received",
        "qa_case_id",
        &request.qa_case_id,
        &actor.customer_scope_id,
        "QA_CASE_SCOPE_CONFLICT",
        "qa_case_id is already used by another customer scope",
    )
    .await?;
    merge_latest_canonical_evidence_refs(&state, &actor.customer_scope_id, &mut request).await?;
    request.customer_scope_id = Some(actor.customer_scope_id.clone());
    request.actor_id = Some(actor.actor_id);
    request.actor_role = Some(actor.actor_role);
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
    customer_scope_id: &str,
) -> Result<(), ApiError> {
    let Some(case_id) = request.case_id.as_deref() else {
        return Ok(());
    };
    let cases = state
        .repository
        .list_cases(Some(customer_scope_id))
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

async fn merge_latest_canonical_evidence_refs_for_investigation(
    state: &AppState,
    customer_scope_id: &str,
    request: &mut InvestigationResultRecord,
) -> Result<(), ApiError> {
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 1,
            event_type: Some("scoring.completed".into()),
            claim_id: Some(request.claim_id.clone()),
            customer_scope_id: Some(customer_scope_id.into()),
            has_canonical_trace: Some(true),
            ..Default::default()
        })
        .await
        .map_err(internal_error(
            "INVESTIGATION_CANONICAL_TRACE_LOOKUP_FAILED",
        ))?;
    let Some(event) = events
        .iter()
        .find(|event| event.event_status == "succeeded")
    else {
        return Ok(());
    };
    for reference in
        unique_json_string_values(&event.payload["canonical_claim_context_trace"]["evidence_refs"])
    {
        if !request.evidence_refs.contains(&reference) {
            request.evidence_refs.push(reference);
        }
    }
    Ok(())
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
            "feedback_target must be rules, model, features, provider_profile, workflow, or tpa",
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

async fn merge_latest_canonical_evidence_refs(
    state: &AppState,
    customer_scope_id: &str,
    request: &mut QaReviewRecord,
) -> Result<(), ApiError> {
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 1,
            event_type: Some("scoring.completed".into()),
            claim_id: Some(request.claim_id.clone()),
            customer_scope_id: Some(customer_scope_id.into()),
            has_canonical_trace: Some(true),
            ..Default::default()
        })
        .await
        .map_err(internal_error("QA_CANONICAL_TRACE_LOOKUP_FAILED"))?;
    let Some(event) = events
        .iter()
        .find(|event| event.event_status == "succeeded")
    else {
        return Ok(());
    };
    for reference in
        unique_json_string_values(&event.payload["canonical_claim_context_trace"]["evidence_refs"])
    {
        if !request.evidence_refs.contains(&reference) {
            request.evidence_refs.push(reference);
        }
    }
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
    AuthenticatedActor(actor): AuthenticatedActor,
    Query(query): Query<QaFeedbackItemListQuery>,
) -> Result<Json<QaFeedbackItemListResponse>, ApiError> {
    validate_qa_feedback_item_list_query(&query)?;
    let mut items = state
        .repository
        .list_qa_feedback_items(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("QA_FEEDBACK_LIST_FAILED"))?;
    if let Some(status) = &query.status {
        items.retain(|item| item.status == *status);
    }
    if let Some(feedback_target) = &query.feedback_target {
        let canonical_target = canonical_feedback_target(feedback_target);
        items.retain(|item| canonical_feedback_target(&item.feedback_target) == canonical_target);
    }
    Ok(Json(QaFeedbackItemListResponse { items }))
}

pub async fn update_qa_feedback_status(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(feedback_id): Path<String>,
    Json(mut request): Json<UpdateQaFeedbackStatusInput>,
) -> Result<Json<UpdateQaFeedbackStatusRecord>, ApiError> {
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
    request.customer_scope_id = Some(actor.customer_scope_id.clone());
    let record = state
        .repository
        .update_qa_feedback_status(&feedback_id, request, Some(&actor.customer_scope_id))
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
                "feedback_target must be rules, model, features, provider_profile, workflow, or tpa",
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
        canonical_feedback_target(feedback_target),
        "rules" | "model" | "features" | "provider_profile" | "workflow" | "tpa"
    )
}

pub async fn list_qa_queue(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<QaQueueListResponse>, ApiError> {
    let samples = state
        .repository
        .list_audit_samples(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("AUDIT_SAMPLE_LIST_FAILED"))?;
    let reviews = state
        .repository
        .list_qa_reviews(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("QA_REVIEW_LIST_FAILED"))?;
    let scoring_events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 1_000,
            event_type: Some("scoring.completed".into()),
            has_canonical_trace: Some(true),
            customer_scope_id: Some(actor.customer_scope_id),
            ..Default::default()
        })
        .await
        .map_err(internal_error("QA_QUEUE_AUDIT_LIST_FAILED"))?;
    Ok(Json(QaQueueListResponse {
        items: build_qa_queue_items_from_scoring_events(&samples, &reviews, &scoring_events),
    }))
}

pub async fn qa_queue_summary(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<QaQueueSummaryResponse>, ApiError> {
    let items = state
        .repository
        .list_qa_feedback_items(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("QA_FEEDBACK_LIST_FAILED"))?;
    Ok(Json(build_qa_queue_summary(&items)))
}

pub async fn list_outcome_labels(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<OutcomeLabelListResponse>, ApiError> {
    let labels = state
        .repository
        .list_outcome_labels(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("OUTCOME_LABEL_LIST_FAILED"))?;
    Ok(Json(OutcomeLabelListResponse { labels }))
}

pub async fn claim_audit_history(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(claim_id): Path<String>,
) -> Result<Json<ClaimAuditHistoryResponse>, ApiError> {
    let actor = require_permission(principal, "tpa:audit:read")?;
    let events = state
        .repository
        .claim_audit_history(&claim_id, Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("CLAIM_AUDIT_HISTORY_FAILED"))?;
    Ok(Json(ClaimAuditHistoryResponse { claim_id, events }))
}

pub async fn list_webhook_events(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<WebhookEventListResponse>, ApiError> {
    let events = state
        .repository
        .list_webhook_events()
        .await
        .map_err(internal_error("WEBHOOK_EVENT_LIST_FAILED"))?
        .into_iter()
        .filter(|event| event.customer_scope_id == actor.customer_scope_id)
        .collect();
    Ok(Json(WebhookEventListResponse { events }))
}

pub async fn submit_webhook_delivery_attempt(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(event_id): Path<String>,
    Json(request): Json<SubmitWebhookDeliveryAttemptRequest>,
) -> Result<Json<WebhookDeliveryAttemptRecord>, ApiError> {
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
        .any(|event| {
            event.event_id == event_id && event.customer_scope_id == actor.customer_scope_id
        });
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
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<OpsAlertListResponse>, ApiError> {
    let leads = state
        .repository
        .list_leads(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("LEAD_LIST_FAILED"))?;
    let cases = state
        .repository
        .list_cases(Some(&actor.customer_scope_id))
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
        .list_agent_runs(Some(&actor.customer_scope_id))
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

fn unique_json_string_values(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .fold(Vec::new(), |mut values, value| {
                    let value = value.to_string();
                    if !values.contains(&value) {
                        values.push(value);
                    }
                    values
                })
        })
        .unwrap_or_default()
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

async fn ensure_writeback_id_is_available_for_customer(
    state: &AppState,
    event_type: &str,
    id_field: &str,
    id_value: &str,
    customer_scope_id: &str,
    conflict_code: &'static str,
    conflict_message: &'static str,
) -> Result<(), ApiError> {
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 10_000,
            event_type: Some(event_type.into()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("WRITEBACK_SCOPE_LOOKUP_FAILED"))?;
    let has_cross_scope_match = events.iter().any(|event| {
        event.payload[id_field].as_str() == Some(id_value)
            && event.payload["customer_scope_id"].as_str() != Some(customer_scope_id)
    });
    if has_cross_scope_match {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            conflict_code,
            conflict_message,
        ));
    }
    Ok(())
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}
