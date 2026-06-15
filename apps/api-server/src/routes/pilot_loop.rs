use crate::{
    app::AppState,
    auth::{AuthenticatedActor, AuthenticatedApiPrincipal},
    error::ApiError,
    repository::{
        canonical_feedback_target, AuditEventListFilter, MemberProfileSummaryRecord,
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

use super::pilot_loop_alerts::build_ops_alerts;
use super::pilot_loop_qa_queue::{
    build_qa_queue_items_from_scoring_events, build_qa_queue_summary,
};
pub use super::pilot_loop_types::*;
pub use super::pilot_loop_writebacks::{write_investigation_result, write_qa_result};

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
    validate_qa_feedback_status_production_evidence_refs(&request.evidence_refs)?;
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

fn validate_qa_feedback_status_production_evidence_refs(
    evidence_refs: &[String],
) -> Result<(), ApiError> {
    if evidence_refs.iter().any(|reference| {
        let reference = reference.trim();
        reference.contains("local://")
            || reference.contains("file://")
            || reference.contains('{')
            || reference.contains('}')
    }) {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_QA_FEEDBACK_STATUS_EVIDENCE",
            "QA feedback status evidence_refs must not use local dry-run or placeholder evidence",
        ))
    } else {
        Ok(())
    }
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

pub(super) fn require_permission(
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

pub(super) fn internal_error<E: std::fmt::Display>(
    code: &'static str,
) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}
