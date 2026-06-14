use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceDocumentRecord {
    pub document_id: String,
    pub customer_scope_id: String,
    pub source_system: String,
    pub source_record_ref: String,
    pub claim_id: Option<String>,
    pub external_document_id: Option<String>,
    pub document_type: String,
    pub storage_uri: String,
    pub content_checksum: String,
    pub ingestion_status: String,
    pub redaction_status: String,
    pub retention_policy_id: String,
    pub evidence_refs: Vec<String>,
    pub metadata_json: Value,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateEvidenceDocumentInput {
    pub document_id: String,
    pub customer_scope_id: String,
    pub source_system: String,
    pub source_record_ref: String,
    pub claim_id: Option<String>,
    pub external_document_id: Option<String>,
    pub document_type: String,
    pub storage_uri: String,
    pub content_checksum: String,
    pub ingestion_status: String,
    pub redaction_status: String,
    pub retention_policy_id: String,
    pub evidence_refs: Vec<String>,
    pub metadata_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceDocumentChunkRecord {
    pub chunk_id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub chunking_version: String,
    pub redaction_status: String,
    pub text_checksum: String,
    pub token_count: i32,
    pub storage_uri: String,
    pub source_offsets_json: Value,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateEvidenceDocumentChunkInput {
    pub chunk_id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub chunking_version: String,
    pub redaction_status: String,
    pub text_checksum: String,
    pub token_count: i32,
    pub storage_uri: String,
    pub source_offsets_json: Value,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceOcrOutputRecord {
    pub ocr_output_id: String,
    pub document_id: String,
    pub ocr_engine: String,
    pub ocr_engine_version: String,
    pub output_uri: String,
    pub output_checksum: String,
    pub confidence_score: Option<Decimal>,
    pub quality_status: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateEvidenceOcrOutputInput {
    pub ocr_output_id: String,
    pub document_id: String,
    pub ocr_engine: String,
    pub ocr_engine_version: String,
    pub output_uri: String,
    pub output_checksum: String,
    pub confidence_score: Option<Decimal>,
    pub quality_status: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceEmbeddingJobRecord {
    pub embedding_job_id: String,
    pub customer_scope_id: String,
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
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateEvidenceEmbeddingJobInput {
    pub embedding_job_id: String,
    pub customer_scope_id: String,
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
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceRetrievalAuditEventRecord {
    pub retrieval_id: String,
    pub customer_scope_id: String,
    pub actor_id: String,
    pub actor_role: String,
    pub query_kind: String,
    pub query_checksum: String,
    pub retrieval_method: String,
    pub embedding_model_version: Option<String>,
    pub top_k: i32,
    pub source_refs: Vec<String>,
    pub result_refs: Vec<String>,
    pub redaction_status: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateEvidenceRetrievalAuditEventInput {
    pub retrieval_id: String,
    pub customer_scope_id: String,
    pub actor_id: String,
    pub actor_role: String,
    pub query_kind: String,
    pub query_checksum: String,
    pub retrieval_method: String,
    pub embedding_model_version: Option<String>,
    pub top_k: i32,
    pub source_refs: Vec<String>,
    pub result_refs: Vec<String>,
    pub redaction_status: String,
    pub evidence_refs: Vec<String>,
}
