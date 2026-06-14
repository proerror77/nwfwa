use serde_json::{json, Value};

pub(super) fn evidence_schemas() -> Value {
    json!({
        "EvidenceDocumentRegistrationRequest": {
            "type": "object",
            "required": ["document_id", "source_record_ref", "document_type", "storage_uri", "content_checksum", "ingestion_status", "redaction_status"],
            "properties": {
                "document_id": { "type": "string", "minLength": 1 },
                "source_record_ref": { "type": "string", "minLength": 1 },
                "claim_id": { "type": ["string", "null"] },
                "external_document_id": { "type": ["string", "null"] },
                "document_type": { "type": "string", "minLength": 1 },
                "storage_uri": { "type": "string", "minLength": 1 },
                "content_checksum": { "type": "string", "minLength": 1 },
                "ingestion_status": { "type": "string", "minLength": 1 },
                "redaction_status": { "type": "string", "minLength": 1 },
                "retention_policy_id": { "type": ["string", "null"] },
                "evidence_refs": { "type": "array", "items": { "type": "string", "minLength": 1 } },
                "metadata_json": { "type": "object", "additionalProperties": true }
            },
            "description": "Evidence document metadata only. Raw document text and payloads remain in customer-approved object storage."
        },
        "EvidenceDocumentChunkRegistrationRequest": {
            "type": "object",
            "required": ["chunk_id", "chunk_index", "chunking_version", "redaction_status", "text_checksum", "token_count", "storage_uri"],
            "properties": {
                "chunk_id": { "type": "string", "minLength": 1 },
                "chunk_index": { "type": "integer", "minimum": 0 },
                "chunking_version": { "type": "string", "minLength": 1 },
                "redaction_status": { "type": "string", "minLength": 1 },
                "text_checksum": { "type": "string", "minLength": 1 },
                "token_count": { "type": "integer", "minimum": 0 },
                "storage_uri": { "type": "string", "minLength": 1 },
                "source_offsets_json": { "type": "object", "additionalProperties": true },
                "evidence_refs": { "type": "array", "items": { "type": "string", "minLength": 1 } }
            }
        },
        "EvidenceOcrOutputRegistrationRequest": {
            "type": "object",
            "required": ["ocr_output_id", "ocr_engine", "ocr_engine_version", "output_uri", "output_checksum", "quality_status"],
            "properties": {
                "ocr_output_id": { "type": "string", "minLength": 1 },
                "ocr_engine": { "type": "string", "minLength": 1 },
                "ocr_engine_version": { "type": "string", "minLength": 1 },
                "output_uri": { "type": "string", "minLength": 1 },
                "output_checksum": { "type": "string", "minLength": 1 },
                "confidence_score": { "type": ["string", "null"] },
                "quality_status": { "type": "string", "minLength": 1 },
                "evidence_refs": { "type": "array", "items": { "type": "string", "minLength": 1 } }
            },
            "description": "OCR output metadata only; OCR text is addressed by output_uri and checksum."
        },
        "EvidenceEmbeddingJobRegistrationRequest": {
            "type": "object",
            "required": ["embedding_job_id", "target_kind", "target_ref", "embedding_model", "embedding_model_version", "chunking_version", "redaction_status", "vector_store_kind", "vector_store_ref", "embedding_checksum", "status"],
            "properties": {
                "embedding_job_id": { "type": "string", "minLength": 1 },
                "target_kind": { "type": "string", "enum": ["document", "document_chunk", "knowledge_case"] },
                "target_ref": { "type": "string", "minLength": 1 },
                "embedding_model": { "type": "string", "minLength": 1 },
                "embedding_model_version": { "type": "string", "minLength": 1 },
                "chunking_version": { "type": "string", "minLength": 1 },
                "redaction_status": { "type": "string", "minLength": 1 },
                "vector_store_kind": { "type": "string", "minLength": 1 },
                "vector_store_ref": { "type": "string", "minLength": 1 },
                "embedding_checksum": { "type": "string", "minLength": 1 },
                "status": { "type": "string", "minLength": 1 },
                "evidence_refs": { "type": "array", "items": { "type": "string", "minLength": 1 } }
            }
        },
        "EvidenceRetrievalAuditRegistrationRequest": {
            "type": "object",
            "required": ["retrieval_id", "query_kind", "query_checksum", "retrieval_method", "top_k", "redaction_status"],
            "properties": {
                "retrieval_id": { "type": "string", "minLength": 1 },
                "query_kind": { "type": "string", "minLength": 1 },
                "query_checksum": { "type": "string", "minLength": 1 },
                "retrieval_method": { "type": "string", "minLength": 1 },
                "embedding_model_version": { "type": ["string", "null"] },
                "top_k": { "type": "integer", "minimum": 1 },
                "source_refs": { "type": "array", "items": { "type": "string", "minLength": 1 } },
                "result_refs": { "type": "array", "items": { "type": "string", "minLength": 1 } },
                "redaction_status": { "type": "string", "minLength": 1 },
                "evidence_refs": { "type": "array", "items": { "type": "string", "minLength": 1 } }
            },
            "description": "Retrieval audit metadata uses query_checksum instead of raw query text."
        },
    })
}
