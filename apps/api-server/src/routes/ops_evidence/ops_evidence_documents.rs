use crate::{
    app::AppState,
    auth::AuthenticatedActor,
    error::ApiError,
    repository::{
        CreateEvidenceDocumentChunkInput, CreateEvidenceDocumentInput,
        CreateEvidenceOcrOutputInput, EvidenceDocumentChunkRecord, EvidenceDocumentRecord,
        EvidenceOcrOutputRecord, PersistedAuditEvent,
    },
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use fwa_audit::ActorContext;
use fwa_core::{AuditEventId, ScoringRunId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct CreateEvidenceDocumentRequest {
    pub document_id: String,
    pub source_record_ref: String,
    pub claim_id: Option<String>,
    pub external_document_id: Option<String>,
    pub document_type: String,
    pub storage_uri: String,
    pub content_checksum: String,
    pub ingestion_status: String,
    pub redaction_status: String,
    pub retention_policy_id: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default = "empty_object")]
    pub metadata_json: Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateEvidenceDocumentChunkRequest {
    pub chunk_id: String,
    pub chunk_index: i32,
    pub chunking_version: String,
    pub redaction_status: String,
    pub text_checksum: String,
    pub token_count: i32,
    pub storage_uri: String,
    #[serde(default = "empty_object")]
    pub source_offsets_json: Value,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEvidenceOcrOutputRequest {
    pub ocr_output_id: String,
    pub ocr_engine: String,
    pub ocr_engine_version: String,
    pub output_uri: String,
    pub output_checksum: String,
    pub confidence_score: Option<Decimal>,
    pub quality_status: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EvidenceDocumentListResponse {
    pub documents: Vec<EvidenceDocumentRecord>,
}

#[derive(Debug, Serialize)]
pub struct EvidenceDocumentChunkListResponse {
    pub chunks: Vec<EvidenceDocumentChunkRecord>,
}

#[derive(Debug, Serialize)]
pub struct EvidenceOcrOutputListResponse {
    pub ocr_outputs: Vec<EvidenceOcrOutputRecord>,
}

pub async fn create_document(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Json(request): Json<CreateEvidenceDocumentRequest>,
) -> Result<Json<EvidenceDocumentRecord>, ApiError> {
    validate_document_request(&request)?;
    let document = state
        .repository
        .save_evidence_document(CreateEvidenceDocumentInput {
            document_id: request.document_id,
            customer_scope_id: actor.customer_scope_id.clone(),
            source_system: actor.source_system.clone(),
            source_record_ref: request.source_record_ref,
            claim_id: request.claim_id,
            external_document_id: request.external_document_id,
            document_type: request.document_type,
            storage_uri: request.storage_uri,
            content_checksum: request.content_checksum,
            ingestion_status: request.ingestion_status,
            redaction_status: request.redaction_status,
            retention_policy_id: request
                .retention_policy_id
                .unwrap_or_else(|| state.config.retention_policy_id.clone()),
            evidence_refs: request.evidence_refs,
            metadata_json: request.metadata_json,
        })
        .await
        .map_err(internal_error("EVIDENCE_DOCUMENT_SAVE_FAILED"))?;
    record_evidence_audit(
        &state,
        &actor,
        "evidence.document.registered",
        "Evidence document registered",
        json!({
            "document_id": document.document_id,
            "claim_id": document.claim_id,
            "document_type": document.document_type,
            "storage_uri": document.storage_uri,
            "content_checksum": document.content_checksum,
            "ingestion_status": document.ingestion_status,
            "redaction_status": document.redaction_status,
        }),
        document.claim_id.clone().unwrap_or_default(),
        evidence_refs_with_anchor(
            &document.evidence_refs,
            "evidence_documents",
            &document.document_id,
        ),
    )
    .await
    .map_err(internal_error("EVIDENCE_AUDIT_SAVE_FAILED"))?;
    Ok(Json(document))
}

pub async fn list_documents(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<EvidenceDocumentListResponse>, ApiError> {
    let documents = state
        .repository
        .list_evidence_documents(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("EVIDENCE_DOCUMENT_LIST_FAILED"))?;
    Ok(Json(EvidenceDocumentListResponse { documents }))
}

pub async fn get_document(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(document_id): Path<String>,
) -> Result<Json<EvidenceDocumentRecord>, ApiError> {
    let document = state
        .repository
        .get_evidence_document(&document_id, Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("EVIDENCE_DOCUMENT_LOAD_FAILED"))?
        .ok_or_else(not_found(
            "EVIDENCE_DOCUMENT_NOT_FOUND",
            "evidence document not found",
        ))?;
    Ok(Json(document))
}

pub async fn create_document_chunk(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(document_id): Path<String>,
    Json(request): Json<CreateEvidenceDocumentChunkRequest>,
) -> Result<Json<EvidenceDocumentChunkRecord>, ApiError> {
    validate_chunk_request(&request)?;
    let chunk = state
        .repository
        .save_evidence_document_chunk(
            CreateEvidenceDocumentChunkInput {
                chunk_id: request.chunk_id,
                document_id: document_id.clone(),
                chunk_index: request.chunk_index,
                chunking_version: request.chunking_version,
                redaction_status: request.redaction_status,
                text_checksum: request.text_checksum,
                token_count: request.token_count,
                storage_uri: request.storage_uri,
                source_offsets_json: request.source_offsets_json,
                evidence_refs: request.evidence_refs,
            },
            Some(&actor.customer_scope_id),
        )
        .await
        .map_err(internal_error("EVIDENCE_CHUNK_SAVE_FAILED"))?
        .ok_or_else(not_found(
            "EVIDENCE_DOCUMENT_NOT_FOUND",
            "evidence document not found",
        ))?;
    record_evidence_audit(
        &state,
        &actor,
        "evidence.document_chunk.registered",
        "Evidence document chunk registered",
        json!({
            "document_id": chunk.document_id,
            "chunk_id": chunk.chunk_id,
            "chunk_index": chunk.chunk_index,
            "redaction_status": chunk.redaction_status,
            "text_checksum": chunk.text_checksum,
        }),
        String::new(),
        evidence_refs_with_anchor(&chunk.evidence_refs, "evidence_chunks", &chunk.chunk_id),
    )
    .await
    .map_err(internal_error("EVIDENCE_AUDIT_SAVE_FAILED"))?;
    Ok(Json(chunk))
}

pub async fn list_document_chunks(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(document_id): Path<String>,
) -> Result<Json<EvidenceDocumentChunkListResponse>, ApiError> {
    let chunks = state
        .repository
        .list_evidence_document_chunks(&document_id, Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("EVIDENCE_CHUNK_LIST_FAILED"))?;
    Ok(Json(EvidenceDocumentChunkListResponse { chunks }))
}

pub async fn create_ocr_output(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(document_id): Path<String>,
    Json(request): Json<CreateEvidenceOcrOutputRequest>,
) -> Result<Json<EvidenceOcrOutputRecord>, ApiError> {
    validate_ocr_request(&request)?;
    let output = state
        .repository
        .save_evidence_ocr_output(
            CreateEvidenceOcrOutputInput {
                ocr_output_id: request.ocr_output_id,
                document_id: document_id.clone(),
                ocr_engine: request.ocr_engine,
                ocr_engine_version: request.ocr_engine_version,
                output_uri: request.output_uri,
                output_checksum: request.output_checksum,
                confidence_score: request.confidence_score,
                quality_status: request.quality_status,
                evidence_refs: request.evidence_refs,
            },
            Some(&actor.customer_scope_id),
        )
        .await
        .map_err(internal_error("EVIDENCE_OCR_SAVE_FAILED"))?
        .ok_or_else(not_found(
            "EVIDENCE_DOCUMENT_NOT_FOUND",
            "evidence document not found",
        ))?;
    record_evidence_audit(
        &state,
        &actor,
        "evidence.ocr_output.registered",
        "Evidence OCR output registered",
        json!({
            "document_id": output.document_id,
            "ocr_output_id": output.ocr_output_id,
            "ocr_engine": output.ocr_engine,
            "ocr_engine_version": output.ocr_engine_version,
            "output_checksum": output.output_checksum,
            "quality_status": output.quality_status,
        }),
        String::new(),
        evidence_refs_with_anchor(
            &output.evidence_refs,
            "evidence_ocr_outputs",
            &output.ocr_output_id,
        ),
    )
    .await
    .map_err(internal_error("EVIDENCE_AUDIT_SAVE_FAILED"))?;
    Ok(Json(output))
}

pub async fn list_ocr_outputs(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(document_id): Path<String>,
) -> Result<Json<EvidenceOcrOutputListResponse>, ApiError> {
    let ocr_outputs = state
        .repository
        .list_evidence_ocr_outputs(&document_id, Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("EVIDENCE_OCR_LIST_FAILED"))?;
    Ok(Json(EvidenceOcrOutputListResponse { ocr_outputs }))
}

pub(super) async fn record_evidence_audit(
    state: &AppState,
    actor: &ActorContext,
    event_type: &str,
    summary: &str,
    mut payload: Value,
    claim_id: String,
    evidence_refs: Vec<String>,
) -> anyhow::Result<()> {
    if let Some(payload) = payload.as_object_mut() {
        payload.insert("customer_scope_id".into(), json!(actor.customer_scope_id));
    }
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id,
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: event_type.into(),
            event_status: "succeeded".into(),
            summary: summary.into(),
            payload,
            evidence_refs: evidence_refs.into_iter().map(Value::String).collect(),
        })
        .await
}

fn validate_document_request(request: &CreateEvidenceDocumentRequest) -> Result<(), ApiError> {
    require_non_empty("document_id", &request.document_id)?;
    require_non_empty("source_record_ref", &request.source_record_ref)?;
    require_non_empty("document_type", &request.document_type)?;
    require_non_empty("storage_uri", &request.storage_uri)?;
    require_non_empty("content_checksum", &request.content_checksum)?;
    require_non_empty("ingestion_status", &request.ingestion_status)?;
    require_non_empty("redaction_status", &request.redaction_status)?;
    validate_evidence_refs(&request.evidence_refs)
}

fn validate_chunk_request(request: &CreateEvidenceDocumentChunkRequest) -> Result<(), ApiError> {
    require_non_empty("chunk_id", &request.chunk_id)?;
    require_non_empty("chunking_version", &request.chunking_version)?;
    require_non_empty("redaction_status", &request.redaction_status)?;
    require_non_empty("text_checksum", &request.text_checksum)?;
    require_non_empty("storage_uri", &request.storage_uri)?;
    if request.chunk_index < 0 || request.token_count < 0 {
        return Err(bad_request(
            "EVIDENCE_CHUNK_INVALID",
            "chunk_index and token_count must be non-negative",
        ));
    }
    validate_evidence_refs(&request.evidence_refs)
}

fn validate_ocr_request(request: &CreateEvidenceOcrOutputRequest) -> Result<(), ApiError> {
    require_non_empty("ocr_output_id", &request.ocr_output_id)?;
    require_non_empty("ocr_engine", &request.ocr_engine)?;
    require_non_empty("ocr_engine_version", &request.ocr_engine_version)?;
    require_non_empty("output_uri", &request.output_uri)?;
    require_non_empty("output_checksum", &request.output_checksum)?;
    require_non_empty("quality_status", &request.quality_status)?;
    validate_evidence_refs(&request.evidence_refs)
}

pub(super) fn validate_evidence_refs(values: &[String]) -> Result<(), ApiError> {
    if values.iter().any(|value| value.trim().is_empty()) {
        Err(bad_request(
            "EVIDENCE_REF_INVALID",
            "evidence refs must be non-empty strings",
        ))
    } else if values.iter().any(|value| {
        let value = value.trim();
        let normalized = value.to_ascii_lowercase();
        normalized.contains("local://")
            || normalized.contains("file://")
            || normalized.contains("://localhost")
            || normalized.contains("://127.")
            || normalized.contains("://0.0.0.0")
            || normalized.contains("://[::1]")
            || value.contains('{')
            || value.contains('}')
    }) {
        Err(bad_request(
            "EVIDENCE_REF_INVALID",
            "evidence refs must not use local dry-run or placeholder evidence",
        ))
    } else {
        Ok(())
    }
}

pub(super) fn require_non_empty(field: &str, value: &str) -> Result<(), ApiError> {
    if value.trim().is_empty() {
        Err(bad_request(
            "EVIDENCE_FIELD_REQUIRED",
            format!("{field} is required"),
        ))
    } else {
        Ok(())
    }
}

pub(super) fn evidence_refs_with_anchor(values: &[String], kind: &str, id: &str) -> Vec<String> {
    let mut refs = values.to_vec();
    refs.push(format!("{kind}:{id}"));
    refs.sort();
    refs.dedup();
    refs
}

pub(super) fn empty_object() -> Value {
    json!({})
}

pub(super) fn not_found(code: &'static str, message: &'static str) -> impl FnOnce() -> ApiError {
    move || ApiError::new(StatusCode::NOT_FOUND, code, message)
}

pub(super) fn bad_request(code: &'static str, message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, code, message)
}

pub(super) fn internal_error<E: std::fmt::Display>(
    code: &'static str,
) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}
