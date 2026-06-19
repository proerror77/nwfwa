use super::ops_evidence_documents::{
    bad_request, evidence_refs_with_anchor, internal_error, record_evidence_audit,
    require_non_empty, validate_evidence_refs,
};
use crate::{
    app::AppState,
    auth::{AuthenticatedActor, AuthenticatedApiPrincipal},
    error::ApiError,
    repository::{
        CreateEvidenceEmbeddingJobInput, CreateEvidenceRetrievalAuditEventInput,
        EvidenceEmbeddingJobRecord, EvidenceRetrievalAuditEventRecord,
    },
};
use axum::{extract::State, http::StatusCode, Json};
use fwa_auth::AuthenticatedPrincipal;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize)]
pub struct CreateEvidenceEmbeddingJobRequest {
    pub embedding_job_id: String,
    pub target_kind: String,
    pub target_ref: String,
    pub embedding_model: String,
    pub embedding_model_version: String,
    pub chunking_version: String,
    pub redaction_status: String,
    pub vector_store_kind: String,
    pub vector_store_ref: String,
    pub embedding_checksum: String,
    pub status: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEvidenceRetrievalAuditEventRequest {
    pub retrieval_id: String,
    pub query_kind: String,
    pub query_checksum: String,
    pub retrieval_method: String,
    pub embedding_model_version: Option<String>,
    pub top_k: i32,
    #[serde(default)]
    pub source_refs: Vec<String>,
    #[serde(default)]
    pub result_refs: Vec<String>,
    pub redaction_status: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EvidenceEmbeddingJobListResponse {
    pub embedding_jobs: Vec<EvidenceEmbeddingJobRecord>,
}

#[derive(Debug, Serialize)]
pub struct EvidenceRetrievalAuditEventListResponse {
    pub retrieval_audit_events: Vec<EvidenceRetrievalAuditEventRecord>,
}

pub async fn create_embedding_job(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Json(request): Json<CreateEvidenceEmbeddingJobRequest>,
) -> Result<Json<EvidenceEmbeddingJobRecord>, ApiError> {
    let actor = require_permission(principal, "ops:evidence:write")?;
    validate_embedding_job_request(&request)?;
    let job = state
        .repository
        .save_evidence_embedding_job(CreateEvidenceEmbeddingJobInput {
            embedding_job_id: request.embedding_job_id,
            customer_scope_id: actor.customer_scope_id.clone(),
            target_kind: request.target_kind,
            target_ref: request.target_ref,
            embedding_model: request.embedding_model,
            embedding_model_version: request.embedding_model_version,
            chunking_version: request.chunking_version,
            redaction_status: request.redaction_status,
            vector_store_kind: request.vector_store_kind,
            vector_store_ref: request.vector_store_ref,
            embedding_checksum: request.embedding_checksum,
            status: request.status,
            evidence_refs: request.evidence_refs,
        })
        .await
        .map_err(internal_error("EVIDENCE_EMBEDDING_JOB_SAVE_FAILED"))?;
    record_evidence_audit(
        &state,
        &actor,
        "evidence.embedding_job.registered",
        "Evidence embedding job registered",
        json!({
            "embedding_job_id": job.embedding_job_id,
            "target_kind": job.target_kind,
            "target_ref": job.target_ref,
            "embedding_model_version": job.embedding_model_version,
            "vector_store_kind": job.vector_store_kind,
            "status": job.status,
        }),
        String::new(),
        evidence_refs_with_anchor(
            &job.evidence_refs,
            "evidence_embedding_jobs",
            &job.embedding_job_id,
        ),
    )
    .await
    .map_err(internal_error("EVIDENCE_AUDIT_SAVE_FAILED"))?;
    Ok(Json(job))
}

pub async fn list_embedding_jobs(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<EvidenceEmbeddingJobListResponse>, ApiError> {
    let embedding_jobs = state
        .repository
        .list_evidence_embedding_jobs(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("EVIDENCE_EMBEDDING_JOB_LIST_FAILED"))?;
    Ok(Json(EvidenceEmbeddingJobListResponse { embedding_jobs }))
}

pub async fn create_retrieval_audit_event(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Json(request): Json<CreateEvidenceRetrievalAuditEventRequest>,
) -> Result<Json<EvidenceRetrievalAuditEventRecord>, ApiError> {
    let actor = require_permission(principal, "ops:evidence:write")?;
    validate_retrieval_audit_request(&request)?;
    let event = state
        .repository
        .save_evidence_retrieval_audit_event(CreateEvidenceRetrievalAuditEventInput {
            retrieval_id: request.retrieval_id,
            customer_scope_id: actor.customer_scope_id.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            query_kind: request.query_kind,
            query_checksum: request.query_checksum,
            retrieval_method: request.retrieval_method,
            embedding_model_version: request.embedding_model_version,
            top_k: request.top_k,
            source_refs: request.source_refs,
            result_refs: request.result_refs,
            redaction_status: request.redaction_status,
            evidence_refs: request.evidence_refs,
        })
        .await
        .map_err(internal_error("EVIDENCE_RETRIEVAL_AUDIT_SAVE_FAILED"))?;
    record_evidence_audit(
        &state,
        &actor,
        "evidence.retrieval_audit.recorded",
        "Evidence retrieval audit recorded",
        json!({
            "retrieval_id": event.retrieval_id,
            "query_kind": event.query_kind,
            "query_checksum": event.query_checksum,
            "retrieval_method": event.retrieval_method,
            "top_k": event.top_k,
            "redaction_status": event.redaction_status,
        }),
        String::new(),
        evidence_refs_with_anchor(
            &event.evidence_refs,
            "evidence_retrieval_audit_events",
            &event.retrieval_id,
        ),
    )
    .await
    .map_err(internal_error("EVIDENCE_AUDIT_SAVE_FAILED"))?;
    Ok(Json(event))
}

pub async fn list_retrieval_audit_events(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<EvidenceRetrievalAuditEventListResponse>, ApiError> {
    let retrieval_audit_events = state
        .repository
        .list_evidence_retrieval_audit_events(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("EVIDENCE_RETRIEVAL_AUDIT_LIST_FAILED"))?;
    Ok(Json(EvidenceRetrievalAuditEventListResponse {
        retrieval_audit_events,
    }))
}

fn validate_embedding_job_request(
    request: &CreateEvidenceEmbeddingJobRequest,
) -> Result<(), ApiError> {
    require_non_empty("embedding_job_id", &request.embedding_job_id)?;
    require_non_empty("target_kind", &request.target_kind)?;
    if !matches!(
        request.target_kind.as_str(),
        "document" | "document_chunk" | "knowledge_case"
    ) {
        return Err(bad_request(
            "EVIDENCE_EMBEDDING_TARGET_INVALID",
            "target_kind must be document, document_chunk, or knowledge_case",
        ));
    }
    require_non_empty("target_ref", &request.target_ref)?;
    require_non_empty("embedding_model", &request.embedding_model)?;
    require_non_empty("embedding_model_version", &request.embedding_model_version)?;
    require_non_empty("chunking_version", &request.chunking_version)?;
    require_non_empty("redaction_status", &request.redaction_status)?;
    require_non_empty("vector_store_kind", &request.vector_store_kind)?;
    require_non_empty("vector_store_ref", &request.vector_store_ref)?;
    require_non_empty("embedding_checksum", &request.embedding_checksum)?;
    require_non_empty("status", &request.status)?;
    validate_evidence_refs(&request.evidence_refs)
}

fn validate_retrieval_audit_request(
    request: &CreateEvidenceRetrievalAuditEventRequest,
) -> Result<(), ApiError> {
    require_non_empty("retrieval_id", &request.retrieval_id)?;
    require_non_empty("query_kind", &request.query_kind)?;
    require_non_empty("query_checksum", &request.query_checksum)?;
    require_non_empty("retrieval_method", &request.retrieval_method)?;
    require_non_empty("redaction_status", &request.redaction_status)?;
    if request.top_k <= 0 {
        return Err(bad_request(
            "EVIDENCE_RETRIEVAL_TOP_K_INVALID",
            "top_k must be positive",
        ));
    }
    validate_evidence_refs(&request.source_refs)?;
    validate_evidence_refs(&request.result_refs)?;
    validate_evidence_refs(&request.evidence_refs)
}

fn require_permission(
    principal: AuthenticatedPrincipal,
    permission: &str,
) -> Result<fwa_audit::ActorContext, ApiError> {
    if !principal.has_permission(permission) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "PERMISSION_DENIED",
            format!("missing permission: {permission}"),
        ));
    }
    Ok(principal.actor)
}
