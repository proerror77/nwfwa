use super::{request_get_json, request_json};
use crate::types::*;
use serde_json::json;

pub(crate) async fn get_evidence_runtime_snapshot(
    api_key: String,
    selected_document_id: String,
) -> Result<EvidenceRuntimeSnapshot, String> {
    let documents = request_get_json::<EvidenceDocumentListResponse>(
        "/api/v1/ops/evidence/documents",
        api_key.clone(),
    )
    .await?
    .documents;
    let selected_document_id = selected_document_id.trim().to_string();
    let selected_document_id = if selected_document_id.is_empty() {
        documents
            .first()
            .map(|document| document.document_id.clone())
    } else {
        Some(selected_document_id)
    };
    let (chunks, ocr_outputs) = if let Some(document_id) = &selected_document_id {
        let chunks = request_get_json::<EvidenceDocumentChunkListResponse>(
            &format!("/api/v1/ops/evidence/documents/{document_id}/chunks"),
            api_key.clone(),
        )
        .await?
        .chunks;
        let ocr_outputs = request_get_json::<EvidenceOcrOutputListResponse>(
            &format!("/api/v1/ops/evidence/documents/{document_id}/ocr-outputs"),
            api_key.clone(),
        )
        .await?
        .ocr_outputs;
        (chunks, ocr_outputs)
    } else {
        (Vec::new(), Vec::new())
    };
    let embedding_jobs = request_get_json::<EvidenceEmbeddingJobListResponse>(
        "/api/v1/ops/evidence/embedding-jobs",
        api_key.clone(),
    )
    .await?
    .embedding_jobs;
    let retrieval_audit_events = request_get_json::<EvidenceRetrievalAuditEventListResponse>(
        "/api/v1/ops/evidence/retrieval-audit-events",
        api_key,
    )
    .await?
    .retrieval_audit_events;
    Ok(EvidenceRuntimeSnapshot {
        documents,
        selected_document_id,
        chunks,
        ocr_outputs,
        embedding_jobs,
        retrieval_audit_events,
    })
}

pub(crate) async fn post_evidence_demo_lifecycle(
    api_key: String,
    next_index: usize,
) -> Result<String, String> {
    let document_id = format!("web-doc-{next_index:03}");
    let chunk_id = format!("web-chunk-{next_index:03}");
    let ocr_output_id = format!("web-ocr-{next_index:03}");
    let embedding_job_id = format!("web-emb-{next_index:03}");
    let retrieval_id = format!("web-ret-{next_index:03}");
    let claim_id = "CLM-0287";

    let document_payload = json!({
        "document_id": document_id,
        "source_record_ref": format!("claim_documents:{claim_id}"),
        "claim_id": claim_id,
        "external_document_id": format!("TPA-DOC-{next_index:03}"),
        "document_type": "medical_record",
        "storage_uri": format!("s3://customer-approved/evidence/{document_id}.json"),
        "content_checksum": format!("sha256:{document_id}"),
        "ingestion_status": "registered",
        "redaction_status": "redacted",
        "retention_policy_id": "pilot-7y",
        "evidence_refs": [format!("claim_context:{claim_id}")],
        "metadata_json": {
            "demo_source": "web-console",
            "raw_text_present": false,
            "pii_masking": "required"
        }
    });
    let document = request_json::<EvidenceDocumentRecord>(
        "/api/v1/ops/evidence/documents",
        api_key.clone(),
        document_payload,
    )
    .await?;

    let chunk_payload = json!({
        "chunk_id": chunk_id,
        "chunk_index": 0,
        "chunking_version": "medical-record-v1",
        "redaction_status": "redacted",
        "text_checksum": format!("sha256:{chunk_id}"),
        "token_count": 128,
        "storage_uri": format!("s3://customer-approved/evidence/chunks/{chunk_id}.json"),
        "source_offsets_json": {"page": 1, "raw_text_present": false},
        "evidence_refs": [format!("evidence_documents:{}", document.document_id)]
    });
    let chunk = request_json::<EvidenceDocumentChunkRecord>(
        &format!(
            "/api/v1/ops/evidence/documents/{}/chunks",
            document.document_id
        ),
        api_key.clone(),
        chunk_payload,
    )
    .await?;

    let ocr_payload = json!({
        "ocr_output_id": ocr_output_id,
        "ocr_engine": "customer-ocr",
        "ocr_engine_version": "2026.06",
        "output_uri": format!("s3://customer-approved/evidence/ocr/{ocr_output_id}.json"),
        "output_checksum": format!("sha256:{ocr_output_id}"),
        "confidence_score": "0.94",
        "quality_status": "passed",
        "evidence_refs": [format!("evidence_documents:{}", document.document_id)]
    });
    request_json::<EvidenceOcrOutputRecord>(
        &format!(
            "/api/v1/ops/evidence/documents/{}/ocr-outputs",
            document.document_id
        ),
        api_key.clone(),
        ocr_payload,
    )
    .await?;

    let embedding_payload = json!({
        "embedding_job_id": embedding_job_id,
        "target_kind": "document_chunk",
        "target_ref": chunk.chunk_id,
        "embedding_model": "customer-approved-embedder",
        "embedding_model_version": "v1",
        "chunking_version": "medical-record-v1",
        "redaction_status": "redacted",
        "vector_store_kind": "pgvector",
        "vector_store_ref": format!("pgvector:evidence_chunks:{}", chunk.chunk_id),
        "embedding_checksum": format!("sha256:{embedding_job_id}"),
        "status": "queued",
        "evidence_refs": [format!("evidence_chunks:{}", chunk.chunk_id)]
    });
    request_json::<EvidenceEmbeddingJobRecord>(
        "/api/v1/ops/evidence/embedding-jobs",
        api_key.clone(),
        embedding_payload,
    )
    .await?;

    let retrieval_payload = json!({
        "retrieval_id": retrieval_id,
        "query_kind": "masked_claim_context",
        "query_checksum": format!("sha256:masked-query-{next_index:03}"),
        "retrieval_method": "vector_top_k",
        "embedding_model_version": "v1",
        "top_k": 5,
        "source_refs": [format!("claim_context:{claim_id}")],
        "result_refs": [format!("evidence_chunks:{}", chunk.chunk_id)],
        "redaction_status": "redacted",
        "evidence_refs": [format!("retrieval:{retrieval_id}")]
    });
    request_json::<EvidenceRetrievalAuditEventRecord>(
        "/api/v1/ops/evidence/retrieval-audit-events",
        api_key,
        retrieval_payload,
    )
    .await?;

    Ok(document.document_id)
}
