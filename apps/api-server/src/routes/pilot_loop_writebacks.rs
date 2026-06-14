use super::{
    pilot_loop::{internal_error, require_permission},
    pilot_loop_types::PilotWritebackResponse,
};
use crate::{
    app::AppState,
    auth::AuthenticatedApiPrincipal,
    error::ApiError,
    repository::{
        canonical_feedback_target, AuditEventListFilter, AuditHistoryEventRecord,
        InvestigationResultRecord, QaReviewRecord,
    },
    routes::pii,
};
use axum::{extract::State, http::StatusCode, Json};
use rust_decimal::Decimal;
use serde_json::Value;

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

fn is_supported_qa_feedback_target(feedback_target: &str) -> bool {
    matches!(
        canonical_feedback_target(feedback_target),
        "rules" | "model" | "features" | "provider_profile" | "workflow" | "tpa"
    )
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
