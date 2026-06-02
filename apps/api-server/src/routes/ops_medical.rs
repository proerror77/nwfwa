use crate::{
    app::AppState,
    error::ApiError,
    repository::{AuditEventListFilter, AuditHistoryEventRecord, PersistedAuditEvent},
    routes::pii,
};
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_audit::ActorContext;
use fwa_auth::validate_api_key;
use fwa_core::{AuditEventId, ScoringRunId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct MedicalReviewQueueQuery {
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct MedicalReviewQueueResponse {
    pub items: Vec<MedicalReviewQueueItem>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitMedicalReviewResultRequest {
    pub claim_id: String,
    pub scoring_audit_id: String,
    pub reviewer: String,
    pub decision: String,
    #[serde(default)]
    pub clinical_outcomes: Vec<String>,
    pub notes: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MedicalReviewResultResponse {
    pub claim_id: String,
    pub event_type: String,
    pub event_status: String,
    pub audit_id: String,
    pub run_id: String,
    pub review_status: String,
    pub clinical_outcomes: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MedicalReviewQueueItem {
    pub claim_id: String,
    pub run_id: String,
    pub audit_id: String,
    pub medical_reasonableness_score: u8,
    pub review_route: String,
    pub evidence_status: String,
    pub missing_evidence: Vec<String>,
    pub item_finding_count: u32,
    pub first_item_code: Option<String>,
    pub first_issue_type: Option<String>,
    pub evidence_refs: Vec<String>,
    pub canonical_source_refs: Vec<String>,
    pub canonical_evidence_refs: Vec<String>,
    pub created_at: Option<String>,
    pub review_status: String,
    pub review_audit_id: Option<String>,
    pub review_decision: Option<String>,
    pub reviewer: Option<String>,
    pub reviewed_at: Option<String>,
}

pub async fn medical_review_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<MedicalReviewQueueQuery>,
) -> Result<Json<MedicalReviewQueueResponse>, ApiError> {
    authorize(&state, &headers)?;
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: query.limit.unwrap_or(100).clamp(1, 200),
            event_type: Some("scoring.completed".into()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("MEDICAL_REVIEW_QUEUE_FAILED"))?;
    let review_events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 10_000,
            event_type: Some("medical.review.recorded".into()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("MEDICAL_REVIEW_QUEUE_FAILED"))?;
    let review_statuses = latest_medical_review_statuses(&review_events);
    let items = events
        .iter()
        .filter_map(|event| medical_review_item_from_event(event, &review_statuses))
        .collect::<Vec<_>>();
    Ok(Json(MedicalReviewQueueResponse { items }))
}

pub async fn submit_medical_review_result(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<SubmitMedicalReviewResultRequest>,
) -> Result<Json<MedicalReviewResultResponse>, ApiError> {
    let actor = authorize(&state, &headers)?;
    validate_medical_review_result(&request)?;
    merge_canonical_evidence_refs_for_medical_review(&state, &mut request).await?;
    let audit_id = AuditEventId::new().to_string();
    let run_id = ScoringRunId::new().to_string();
    let review_status = medical_review_status(&request.decision).to_string();
    let clinical_outcomes = controlled_clinical_outcomes(&request);
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: audit_id.clone(),
            run_id: run_id.clone(),
            claim_id: request.claim_id.clone(),
            source_system: "ops-studio".into(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "medical.review.recorded".into(),
            event_status: "succeeded".into(),
            summary: format!("Medical review recorded: {}", request.decision),
            payload: json!({
                "customer_scope_id": actor.customer_scope_id.clone(),
                "actor_id": actor.actor_id.clone(),
                "actor_role": actor.actor_role.clone(),
                "claim_id": request.claim_id,
                "scoring_audit_id": request.scoring_audit_id,
                "reviewer": request.reviewer,
                "decision": request.decision,
                "review_status": review_status,
                "clinical_outcomes": clinical_outcomes.clone(),
                "notes": request.notes,
            }),
            evidence_refs: request
                .evidence_refs
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        })
        .await
        .map_err(internal_error("MEDICAL_REVIEW_RESULT_SAVE_FAILED"))?;
    Ok(Json(MedicalReviewResultResponse {
        claim_id: request.claim_id,
        event_type: "medical.review.recorded".into(),
        event_status: "succeeded".into(),
        audit_id,
        run_id,
        review_status,
        clinical_outcomes,
        evidence_refs: request.evidence_refs,
    }))
}

fn medical_review_item_from_event(
    event: &AuditHistoryEventRecord,
    review_statuses: &BTreeMap<String, MedicalReviewStatus>,
) -> Option<MedicalReviewQueueItem> {
    let clinical = &event.payload["clinical_evidence"];
    let review_required = clinical["review_required"].as_bool().unwrap_or(false);
    let review_route = clinical["review_route"].as_str().unwrap_or_default();
    if !review_required && review_route != "medical_review" {
        return None;
    }
    let first_finding = clinical["item_findings"]
        .as_array()
        .and_then(|findings| findings.first());
    let review_status = review_statuses.get(&event.audit_id);
    Some(MedicalReviewQueueItem {
        claim_id: event.payload["claim_id"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        run_id: event.run_id.clone(),
        audit_id: event.audit_id.clone(),
        medical_reasonableness_score: event.payload["scores"]["medical_reasonableness_score"]
            .as_u64()
            .unwrap_or(0)
            .min(100) as u8,
        review_route: review_route.to_string(),
        evidence_status: clinical["evidence_status"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        missing_evidence: json_array_to_strings(&clinical["missing_evidence"]),
        item_finding_count: clinical["item_findings"]
            .as_array()
            .map(|findings| findings.len() as u32)
            .unwrap_or(0),
        first_item_code: first_finding
            .and_then(|finding| finding["item_code"].as_str())
            .map(str::to_string),
        first_issue_type: first_finding
            .and_then(|finding| finding["issue_type"].as_str())
            .map(str::to_string),
        evidence_refs: json_array_to_strings(&clinical["evidence_refs"]),
        canonical_source_refs: canonical_trace_refs(event, "source_refs"),
        canonical_evidence_refs: canonical_trace_refs(event, "evidence_refs"),
        created_at: event.created_at.clone(),
        review_status: review_status
            .map(|status| status.review_status.clone())
            .unwrap_or_else(|| "open".into()),
        review_audit_id: review_status.map(|status| status.audit_id.clone()),
        review_decision: review_status.map(|status| status.decision.clone()),
        reviewer: review_status.map(|status| status.reviewer.clone()),
        reviewed_at: review_status.and_then(|status| status.reviewed_at.clone()),
    })
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

async fn merge_canonical_evidence_refs_for_medical_review(
    state: &AppState,
    request: &mut SubmitMedicalReviewResultRequest,
) -> Result<(), ApiError> {
    let events = state
        .repository
        .claim_audit_history(&request.claim_id)
        .await
        .map_err(internal_error(
            "MEDICAL_REVIEW_CANONICAL_TRACE_LOOKUP_FAILED",
        ))?;
    let Some(scoring_event) = events.iter().find(|event| {
        event.audit_id == request.scoring_audit_id
            && event.event_type == "scoring.completed"
            && event.event_status == "succeeded"
    }) else {
        return Ok(());
    };
    for reference in canonical_trace_refs(scoring_event, "evidence_refs") {
        if !request.evidence_refs.contains(&reference) {
            request.evidence_refs.push(reference);
        }
    }
    Ok(())
}

fn canonical_trace_refs(event: &AuditHistoryEventRecord, field: &str) -> Vec<String> {
    unique_json_string_values(&event.payload["canonical_claim_context_trace"][field])
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

#[derive(Debug)]
struct MedicalReviewStatus {
    audit_id: String,
    review_status: String,
    decision: String,
    reviewer: String,
    reviewed_at: Option<String>,
}

fn latest_medical_review_statuses(
    events: &[AuditHistoryEventRecord],
) -> BTreeMap<String, MedicalReviewStatus> {
    let mut statuses = BTreeMap::new();
    for event in events {
        let Some(scoring_audit_id) = event.payload["scoring_audit_id"].as_str() else {
            continue;
        };
        statuses
            .entry(scoring_audit_id.to_string())
            .or_insert_with(|| MedicalReviewStatus {
                audit_id: event.audit_id.clone(),
                review_status: event.payload["review_status"]
                    .as_str()
                    .unwrap_or("completed")
                    .to_string(),
                decision: event.payload["decision"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
                reviewer: event.payload["reviewer"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
                reviewed_at: event.created_at.clone(),
            });
    }
    statuses
}

fn validate_medical_review_result(
    request: &SubmitMedicalReviewResultRequest,
) -> Result<(), ApiError> {
    if request.claim_id.trim().is_empty()
        || request.scoring_audit_id.trim().is_empty()
        || request.reviewer.trim().is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MEDICAL_REVIEW_RESULT",
            "claim_id, scoring_audit_id, and reviewer are required",
        ));
    }
    if !matches!(
        request.decision.as_str(),
        "evidence_sufficient"
            | "request_more_evidence"
            | "medical_necessity_issue"
            | "no_medical_issue"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "UNSUPPORTED_MEDICAL_REVIEW_DECISION",
            "decision must be evidence_sufficient, request_more_evidence, medical_necessity_issue, or no_medical_issue",
        ));
    }
    if request.clinical_outcomes.iter().any(|outcome| {
        let outcome = outcome.trim();
        outcome.is_empty() || !is_allowed_clinical_outcome(outcome)
    }) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "UNSUPPORTED_CLINICAL_OUTCOME",
            "clinical_outcomes must use controlled clinical review outcome fields",
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
            "MISSING_MEDICAL_REVIEW_EVIDENCE",
            "notes and evidence_refs are required for medical review auditability",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.notes.as_str())
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_WRITEBACK",
            "medical review notes and evidence_refs must not contain PII",
        ));
    }
    Ok(())
}

fn controlled_clinical_outcomes(request: &SubmitMedicalReviewResultRequest) -> Vec<String> {
    let outcomes = if request.clinical_outcomes.is_empty() {
        vec![default_clinical_outcome(&request.decision).to_string()]
    } else {
        request
            .clinical_outcomes
            .iter()
            .map(|outcome| outcome.trim().to_string())
            .collect()
    };
    outcomes
        .into_iter()
        .fold(Vec::new(), |mut values, outcome| {
            if !values.contains(&outcome) {
                values.push(outcome);
            }
            values
        })
}

fn default_clinical_outcome(decision: &str) -> &'static str {
    match decision {
        "request_more_evidence" => "insufficient_evidence",
        "medical_necessity_issue" => "medical_necessity_issue",
        "no_medical_issue" => "false_positive",
        _ => "clinical_evidence_sufficient",
    }
}

fn is_allowed_clinical_outcome(outcome: &str) -> bool {
    matches!(
        outcome,
        "documentation_issue"
            | "medical_necessity_review_required"
            | "insufficient_evidence"
            | "medical_necessity_issue"
            | "clinical_evidence_sufficient"
            | "false_positive"
    )
}

fn medical_review_status(decision: &str) -> &'static str {
    match decision {
        "request_more_evidence" => "pending_evidence",
        "medical_necessity_issue" => "completed_issue_found",
        _ => "completed",
    }
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
    move |error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, code, error.to_string())
}
