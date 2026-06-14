use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceDocumentListResponse {
    pub(crate) documents: Vec<EvidenceDocumentRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceDocumentChunkListResponse {
    pub(crate) chunks: Vec<EvidenceDocumentChunkRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceOcrOutputListResponse {
    pub(crate) ocr_outputs: Vec<EvidenceOcrOutputRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceEmbeddingJobListResponse {
    pub(crate) embedding_jobs: Vec<EvidenceEmbeddingJobRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceRetrievalAuditEventListResponse {
    pub(crate) retrieval_audit_events: Vec<EvidenceRetrievalAuditEventRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceDocumentRecord {
    pub(crate) document_id: String,
    pub(crate) customer_scope_id: String,
    pub(crate) source_system: String,
    pub(crate) source_record_ref: String,
    pub(crate) claim_id: Option<String>,
    pub(crate) external_document_id: Option<String>,
    pub(crate) document_type: String,
    pub(crate) storage_uri: String,
    pub(crate) content_checksum: String,
    pub(crate) ingestion_status: String,
    pub(crate) redaction_status: String,
    pub(crate) retention_policy_id: String,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) metadata_json: Value,
    pub(crate) created_at: Option<String>,
    pub(crate) updated_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceDocumentChunkRecord {
    pub(crate) chunk_id: String,
    pub(crate) document_id: String,
    pub(crate) chunk_index: i32,
    pub(crate) chunking_version: String,
    pub(crate) redaction_status: String,
    pub(crate) text_checksum: String,
    pub(crate) token_count: i32,
    pub(crate) storage_uri: String,
    pub(crate) source_offsets_json: Value,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceOcrOutputRecord {
    pub(crate) ocr_output_id: String,
    pub(crate) document_id: String,
    pub(crate) ocr_engine: String,
    pub(crate) ocr_engine_version: String,
    pub(crate) output_uri: String,
    pub(crate) output_checksum: String,
    pub(crate) confidence_score: Option<Value>,
    pub(crate) quality_status: String,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceEmbeddingJobRecord {
    pub(crate) embedding_job_id: String,
    pub(crate) customer_scope_id: String,
    pub(crate) target_kind: String,
    pub(crate) target_ref: String,
    pub(crate) embedding_model: String,
    pub(crate) embedding_model_version: String,
    pub(crate) chunking_version: String,
    pub(crate) redaction_status: String,
    pub(crate) vector_store_kind: String,
    pub(crate) vector_store_ref: String,
    pub(crate) embedding_checksum: String,
    pub(crate) status: String,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
    pub(crate) completed_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceRetrievalAuditEventRecord {
    pub(crate) retrieval_id: String,
    pub(crate) customer_scope_id: String,
    pub(crate) actor_id: String,
    pub(crate) actor_role: String,
    pub(crate) query_kind: String,
    pub(crate) query_checksum: String,
    pub(crate) retrieval_method: String,
    pub(crate) embedding_model_version: Option<String>,
    pub(crate) top_k: i32,
    pub(crate) source_refs: Vec<String>,
    pub(crate) result_refs: Vec<String>,
    pub(crate) redaction_status: String,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EvidenceRuntimeSnapshot {
    pub(crate) documents: Vec<EvidenceDocumentRecord>,
    pub(crate) selected_document_id: Option<String>,
    pub(crate) chunks: Vec<EvidenceDocumentChunkRecord>,
    pub(crate) ocr_outputs: Vec<EvidenceOcrOutputRecord>,
    pub(crate) embedding_jobs: Vec<EvidenceEmbeddingJobRecord>,
    pub(crate) retrieval_audit_events: Vec<EvidenceRetrievalAuditEventRecord>,
}
