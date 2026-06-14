use super::{
    json_array_to_strings, EvidenceDocumentChunkRecord, EvidenceDocumentRecord,
    EvidenceEmbeddingJobRecord, EvidenceOcrOutputRecord, EvidenceRetrievalAuditEventRecord,
};
use rust_decimal::Decimal;
use sqlx::{postgres::PgRow, Row};

pub(super) fn evidence_document_from_row(row: PgRow) -> anyhow::Result<EvidenceDocumentRecord> {
    Ok(EvidenceDocumentRecord {
        document_id: row.try_get("document_id")?,
        customer_scope_id: row.try_get("customer_scope_id")?,
        source_system: row.try_get("source_system")?,
        source_record_ref: row.try_get("source_record_ref")?,
        claim_id: row.try_get("claim_id")?,
        external_document_id: row.try_get("external_document_id")?,
        document_type: row.try_get("document_type")?,
        storage_uri: row.try_get("storage_uri")?,
        content_checksum: row.try_get("content_checksum")?,
        ingestion_status: row.try_get("ingestion_status")?,
        redaction_status: row.try_get("redaction_status")?,
        retention_policy_id: row.try_get("retention_policy_id")?,
        evidence_refs: json_array_to_strings(row.try_get("evidence_refs")?),
        metadata_json: row.try_get("metadata_json")?,
        created_at: timestamp_from_row(&row, "created_at")?,
        updated_at: timestamp_from_row(&row, "updated_at")?,
    })
}

pub(super) fn evidence_document_chunk_from_row(
    row: PgRow,
) -> anyhow::Result<EvidenceDocumentChunkRecord> {
    Ok(EvidenceDocumentChunkRecord {
        chunk_id: row.try_get("chunk_id")?,
        document_id: row.try_get("document_id")?,
        chunk_index: row.try_get("chunk_index")?,
        chunking_version: row.try_get("chunking_version")?,
        redaction_status: row.try_get("redaction_status")?,
        text_checksum: row.try_get("text_checksum")?,
        token_count: row.try_get("token_count")?,
        storage_uri: row.try_get("storage_uri")?,
        source_offsets_json: row.try_get("source_offsets_json")?,
        evidence_refs: json_array_to_strings(row.try_get("evidence_refs")?),
        created_at: timestamp_from_row(&row, "created_at")?,
    })
}

pub(super) fn evidence_ocr_output_from_row(row: PgRow) -> anyhow::Result<EvidenceOcrOutputRecord> {
    Ok(EvidenceOcrOutputRecord {
        ocr_output_id: row.try_get("ocr_output_id")?,
        document_id: row.try_get("document_id")?,
        ocr_engine: row.try_get("ocr_engine")?,
        ocr_engine_version: row.try_get("ocr_engine_version")?,
        output_uri: row.try_get("output_uri")?,
        output_checksum: row.try_get("output_checksum")?,
        confidence_score: row.try_get("confidence_score")?,
        quality_status: row.try_get("quality_status")?,
        evidence_refs: json_array_to_strings(row.try_get("evidence_refs")?),
        created_at: timestamp_from_row(&row, "created_at")?,
    })
}

pub(super) fn evidence_embedding_job_from_row(
    row: PgRow,
) -> anyhow::Result<EvidenceEmbeddingJobRecord> {
    Ok(EvidenceEmbeddingJobRecord {
        embedding_job_id: row.try_get("embedding_job_id")?,
        customer_scope_id: row.try_get("customer_scope_id")?,
        target_kind: row.try_get("target_kind")?,
        target_ref: row.try_get("target_ref")?,
        embedding_model: row.try_get("embedding_model")?,
        embedding_model_version: row.try_get("embedding_model_version")?,
        chunking_version: row.try_get("chunking_version")?,
        redaction_status: row.try_get("redaction_status")?,
        vector_store_kind: row.try_get("vector_store_kind")?,
        vector_store_ref: row.try_get("vector_store_ref")?,
        embedding_checksum: row.try_get("embedding_checksum")?,
        status: row.try_get("status")?,
        evidence_refs: json_array_to_strings(row.try_get("evidence_refs")?),
        created_at: timestamp_from_row(&row, "created_at")?,
        completed_at: timestamp_from_row(&row, "completed_at")?,
    })
}

pub(super) fn evidence_retrieval_audit_event_from_row(
    row: PgRow,
) -> anyhow::Result<EvidenceRetrievalAuditEventRecord> {
    Ok(EvidenceRetrievalAuditEventRecord {
        retrieval_id: row.try_get("retrieval_id")?,
        customer_scope_id: row.try_get("customer_scope_id")?,
        actor_id: row.try_get("actor_id")?,
        actor_role: row.try_get("actor_role")?,
        query_kind: row.try_get("query_kind")?,
        query_checksum: row.try_get("query_checksum")?,
        retrieval_method: row.try_get("retrieval_method")?,
        embedding_model_version: row.try_get("embedding_model_version")?,
        top_k: row.try_get("top_k")?,
        source_refs: json_array_to_strings(row.try_get("source_refs")?),
        result_refs: json_array_to_strings(row.try_get("result_refs")?),
        redaction_status: row.try_get("redaction_status")?,
        evidence_refs: json_array_to_strings(row.try_get("evidence_refs")?),
        created_at: timestamp_from_row(&row, "created_at")?,
    })
}

fn timestamp_from_row(row: &PgRow, column: &str) -> anyhow::Result<Option<String>> {
    let value: Option<chrono::DateTime<chrono::Utc>> = row.try_get(column)?;
    Ok(value.map(|timestamp| timestamp.to_rfc3339()))
}

pub(super) fn _decimal_keeps_sqlx_feature_linked(_: Decimal) {}
