use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        normalize_scheme_family, scheme_family_from_knowledge_signals, AuditEventListFilter,
        KnowledgeCaseRecord, PersistedAuditEvent, SimilarCaseQuery, SimilarCaseRecord,
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

#[derive(Debug, Serialize)]
pub struct KnowledgeCaseListResponse {
    pub cases: Vec<KnowledgeCaseRecord>,
}

#[derive(Debug, Deserialize)]
pub struct PublishKnowledgeCaseRequest {
    pub case_id: String,
    pub title: String,
    pub fwa_type: String,
    pub scheme_family: Option<String>,
    pub diagnosis_code: String,
    pub provider_region: String,
    pub provider_type: String,
    pub summary: String,
    pub outcome: String,
    pub tags: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub source_claim_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PublishKnowledgeCaseResponse {
    pub case: KnowledgeCaseRecord,
    pub audit_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SimilarCaseSearchRequest {
    pub claim_id: Option<String>,
    pub diagnosis_code: String,
    pub provider_region: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SimilarCaseSearchResponse {
    pub results: Vec<SimilarCaseRecord>,
}

pub async fn list_cases(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<KnowledgeCaseListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let cases = state
        .repository
        .list_knowledge_cases()
        .await
        .map_err(internal_error("KNOWLEDGE_CASE_LIST_FAILED"))?;
    Ok(Json(KnowledgeCaseListResponse { cases }))
}

pub async fn publish_case(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<PublishKnowledgeCaseRequest>,
) -> Result<Json<PublishKnowledgeCaseResponse>, ApiError> {
    let actor = authorize(&state, &headers)?;
    validate_publish_knowledge_case(&request)?;
    merge_latest_canonical_evidence_refs(&state, &mut request).await?;

    let scheme_family = request
        .scheme_family
        .map(|value| normalize_scheme_family(&value))
        .unwrap_or_else(|| scheme_family_from_knowledge_signals(&request.fwa_type, &request.tags));
    let case = KnowledgeCaseRecord {
        case_id: request.case_id,
        title: request.title,
        fwa_type: request.fwa_type,
        scheme_family,
        diagnosis_code: request.diagnosis_code,
        provider_region: request.provider_region,
        provider_type: request.provider_type,
        summary: request.summary,
        outcome: request.outcome,
        tags: request.tags,
        evidence_refs: request.evidence_refs,
    };
    let source_claim_id = request.source_claim_id.clone();
    let case = state
        .repository
        .save_knowledge_case(case)
        .await
        .map_err(internal_error("KNOWLEDGE_CASE_SAVE_FAILED"))?;
    let audit_id = AuditEventId::new().to_string();
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: audit_id.clone(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: source_claim_id
                .clone()
                .unwrap_or_else(|| case.case_id.clone()),
            source_system: actor.source_system,
            actor_id: actor.actor_id,
            actor_role: actor.actor_role,
            event_type: "knowledge.case.published".into(),
            event_status: "succeeded".into(),
            summary: format!("Knowledge case published: {}", case.case_id),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "claim_id": source_claim_id,
                "case_id": case.case_id,
                "fwa_type": case.fwa_type,
                "scheme_family": case.scheme_family,
                "diagnosis_code": case.diagnosis_code,
                "provider_region": case.provider_region,
                "tags": case.tags,
                "evidence_ref_count": case.evidence_refs.len()
            }),
            evidence_refs: case
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
        .map_err(internal_error("KNOWLEDGE_CASE_AUDIT_SAVE_FAILED"))?;
    Ok(Json(PublishKnowledgeCaseResponse { case, audit_id }))
}

async fn merge_latest_canonical_evidence_refs(
    state: &AppState,
    request: &mut PublishKnowledgeCaseRequest,
) -> Result<(), ApiError> {
    let Some(source_claim_id) = request.source_claim_id.as_ref() else {
        return Ok(());
    };
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 1,
            event_type: Some("scoring.completed".into()),
            claim_id: Some(source_claim_id.clone()),
            has_canonical_trace: Some(true),
            ..Default::default()
        })
        .await
        .map_err(internal_error("KNOWLEDGE_CANONICAL_TRACE_LOOKUP_FAILED"))?;
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

fn validate_publish_knowledge_case(request: &PublishKnowledgeCaseRequest) -> Result<(), ApiError> {
    if request.case_id.trim().is_empty()
        || request.title.trim().is_empty()
        || request.fwa_type.trim().is_empty()
        || request.diagnosis_code.trim().is_empty()
        || request.provider_region.trim().is_empty()
        || request.provider_type.trim().is_empty()
        || request.summary.trim().is_empty()
        || request.outcome.trim().is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_KNOWLEDGE_CASE",
            "case_id, title, fwa_type, diagnosis_code, provider_region, provider_type, summary, and outcome are required",
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
            "INVALID_KNOWLEDGE_CASE",
            "evidence_refs are required",
        ));
    }
    if !request.evidence_refs.iter().any(|reference| {
        reference.starts_with("investigation_results:") || reference.starts_with("qa_reviews:")
    }) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_KNOWLEDGE_CASE",
            "confirmed knowledge cases require investigation_results or qa_reviews evidence",
        ));
    }
    if request.tags.is_empty() || request.tags.iter().any(|tag| tag.trim().is_empty()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_KNOWLEDGE_CASE",
            "tags are required",
        ));
    }
    if pii::contains_pii(
        [
            request.title.as_str(),
            request.summary.as_str(),
            request.outcome.as_str(),
        ]
        .into_iter()
        .chain(request.tags.iter().map(String::as_str))
        .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_KNOWLEDGE_CASE",
            "knowledge case title, summary, outcome, tags, and evidence_refs must not contain PII",
        ));
    }
    Ok(())
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

pub async fn search_similar(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SimilarCaseSearchRequest>,
) -> Result<Json<SimilarCaseSearchResponse>, ApiError> {
    authorize(&state, &headers)?;
    validate_similar_case_search(&request)?;
    let results = state
        .repository
        .search_similar_cases(SimilarCaseQuery {
            claim_id: request.claim_id,
            diagnosis_code: request.diagnosis_code,
            provider_region: request.provider_region,
            tags: request.tags,
        })
        .await
        .map_err(internal_error("KNOWLEDGE_SEARCH_FAILED"))?;
    Ok(Json(SimilarCaseSearchResponse { results }))
}

fn validate_similar_case_search(request: &SimilarCaseSearchRequest) -> Result<(), ApiError> {
    if request.diagnosis_code.trim().is_empty() || request.provider_region.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SIMILAR_CASE_QUERY",
            "diagnosis_code and provider_region are required",
        ));
    }
    if request.tags.is_empty() || request.tags.iter().any(|tag| tag.trim().is_empty()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SIMILAR_CASE_QUERY",
            "at least one non-empty tag is required",
        ));
    }
    Ok(())
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

fn internal_error(
    code: &'static str,
) -> impl Fn(anyhow::Error) -> ApiError + Clone + Send + Sync + 'static {
    move |error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, code, error.to_string())
}
