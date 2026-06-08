use super::*;

pub(super) async fn save_evidence_document(
    repository: &PostgresScoringRepository,
    input: CreateEvidenceDocumentInput,
) -> anyhow::Result<EvidenceDocumentRecord> {
    let row = sqlx::query(
        "WITH input_claim AS (
           SELECT id FROM claims WHERE external_claim_id = $5 LIMIT 1
         )
         INSERT INTO evidence_documents
         (document_id, customer_scope_id, source_system, source_record_ref, claim_id, external_document_id, document_type, storage_uri, content_checksum, ingestion_status, redaction_status, retention_policy_id, evidence_refs, metadata_json)
         VALUES ($1, $2, $3, $4, (SELECT id FROM input_claim), $6, $7, $8, $9, $10, $11, $12, $13, $14)
         ON CONFLICT (document_id) DO UPDATE SET
           customer_scope_id = EXCLUDED.customer_scope_id,
           source_system = EXCLUDED.source_system,
           source_record_ref = EXCLUDED.source_record_ref,
           claim_id = EXCLUDED.claim_id,
           external_document_id = EXCLUDED.external_document_id,
           document_type = EXCLUDED.document_type,
           storage_uri = EXCLUDED.storage_uri,
           content_checksum = EXCLUDED.content_checksum,
           ingestion_status = EXCLUDED.ingestion_status,
           redaction_status = EXCLUDED.redaction_status,
           retention_policy_id = EXCLUDED.retention_policy_id,
           evidence_refs = EXCLUDED.evidence_refs,
           metadata_json = EXCLUDED.metadata_json,
           updated_at = now()
         RETURNING document_id, customer_scope_id, source_system, source_record_ref,
           (SELECT external_claim_id FROM claims WHERE id = evidence_documents.claim_id) AS claim_id,
           external_document_id, document_type, storage_uri, content_checksum, ingestion_status,
           redaction_status, retention_policy_id, evidence_refs, metadata_json, created_at, updated_at",
    )
    .bind(&input.document_id)
    .bind(&input.customer_scope_id)
    .bind(&input.source_system)
    .bind(&input.source_record_ref)
    .bind(&input.claim_id)
    .bind(&input.external_document_id)
    .bind(&input.document_type)
    .bind(&input.storage_uri)
    .bind(&input.content_checksum)
    .bind(&input.ingestion_status)
    .bind(&input.redaction_status)
    .bind(&input.retention_policy_id)
    .bind(string_values(&input.evidence_refs))
    .bind(&input.metadata_json)
    .fetch_one(&repository.pool)
    .await?;
    evidence_document_from_row(row)
}

pub(super) async fn list_evidence_documents(
    repository: &PostgresScoringRepository,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<EvidenceDocumentRecord>> {
    let rows = sqlx::query(
        "SELECT d.document_id, d.customer_scope_id, d.source_system, d.source_record_ref,
                c.external_claim_id AS claim_id, d.external_document_id, d.document_type,
                d.storage_uri, d.content_checksum, d.ingestion_status, d.redaction_status,
                d.retention_policy_id, d.evidence_refs, d.metadata_json, d.created_at, d.updated_at
         FROM evidence_documents d
         LEFT JOIN claims c ON c.id = d.claim_id
         WHERE ($1::text IS NULL OR d.customer_scope_id = $1)
         ORDER BY d.document_id",
    )
    .bind(customer_scope_id)
    .fetch_all(&repository.pool)
    .await?;
    rows.into_iter().map(evidence_document_from_row).collect()
}

pub(super) async fn get_evidence_document(
    repository: &PostgresScoringRepository,
    document_id: &str,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Option<EvidenceDocumentRecord>> {
    let row = sqlx::query(
        "SELECT d.document_id, d.customer_scope_id, d.source_system, d.source_record_ref,
                c.external_claim_id AS claim_id, d.external_document_id, d.document_type,
                d.storage_uri, d.content_checksum, d.ingestion_status, d.redaction_status,
                d.retention_policy_id, d.evidence_refs, d.metadata_json, d.created_at, d.updated_at
         FROM evidence_documents d
         LEFT JOIN claims c ON c.id = d.claim_id
         WHERE d.document_id = $1
           AND ($2::text IS NULL OR d.customer_scope_id = $2)",
    )
    .bind(document_id)
    .bind(customer_scope_id)
    .fetch_optional(&repository.pool)
    .await?;
    row.map(evidence_document_from_row).transpose()
}

pub(super) async fn save_evidence_document_chunk(
    repository: &PostgresScoringRepository,
    input: CreateEvidenceDocumentChunkInput,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Option<EvidenceDocumentChunkRecord>> {
    if get_evidence_document(repository, &input.document_id, customer_scope_id)
        .await?
        .is_none()
    {
        return Ok(None);
    }
    let row = sqlx::query(
        "INSERT INTO evidence_document_chunks
         (chunk_id, document_id, chunk_index, chunking_version, redaction_status, text_checksum, token_count, storage_uri, source_offsets_json, evidence_refs)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         ON CONFLICT (document_id, chunk_index, chunking_version) DO UPDATE SET
           redaction_status = EXCLUDED.redaction_status,
           text_checksum = EXCLUDED.text_checksum,
           token_count = EXCLUDED.token_count,
           storage_uri = EXCLUDED.storage_uri,
           source_offsets_json = EXCLUDED.source_offsets_json,
           evidence_refs = EXCLUDED.evidence_refs
         RETURNING chunk_id, document_id, chunk_index, chunking_version, redaction_status, text_checksum, token_count, storage_uri, source_offsets_json, evidence_refs, created_at",
    )
    .bind(&input.chunk_id)
    .bind(&input.document_id)
    .bind(input.chunk_index)
    .bind(&input.chunking_version)
    .bind(&input.redaction_status)
    .bind(&input.text_checksum)
    .bind(input.token_count)
    .bind(&input.storage_uri)
    .bind(&input.source_offsets_json)
    .bind(string_values(&input.evidence_refs))
    .fetch_one(&repository.pool)
    .await?;
    Ok(Some(evidence_document_chunk_from_row(row)?))
}

pub(super) async fn list_evidence_document_chunks(
    repository: &PostgresScoringRepository,
    document_id: &str,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<EvidenceDocumentChunkRecord>> {
    if get_evidence_document(repository, document_id, customer_scope_id)
        .await?
        .is_none()
    {
        return Ok(Vec::new());
    }
    let rows = sqlx::query(
        "SELECT chunk_id, document_id, chunk_index, chunking_version, redaction_status, text_checksum, token_count, storage_uri, source_offsets_json, evidence_refs, created_at
         FROM evidence_document_chunks
         WHERE document_id = $1
         ORDER BY chunk_index, chunk_id",
    )
    .bind(document_id)
    .fetch_all(&repository.pool)
    .await?;
    rows.into_iter()
        .map(evidence_document_chunk_from_row)
        .collect()
}

pub(super) async fn save_evidence_ocr_output(
    repository: &PostgresScoringRepository,
    input: CreateEvidenceOcrOutputInput,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Option<EvidenceOcrOutputRecord>> {
    if get_evidence_document(repository, &input.document_id, customer_scope_id)
        .await?
        .is_none()
    {
        return Ok(None);
    }
    let row = sqlx::query(
        "INSERT INTO evidence_ocr_outputs
         (ocr_output_id, document_id, ocr_engine, ocr_engine_version, output_uri, output_checksum, confidence_score, quality_status, evidence_refs)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (ocr_output_id) DO UPDATE SET
           ocr_engine = EXCLUDED.ocr_engine,
           ocr_engine_version = EXCLUDED.ocr_engine_version,
           output_uri = EXCLUDED.output_uri,
           output_checksum = EXCLUDED.output_checksum,
           confidence_score = EXCLUDED.confidence_score,
           quality_status = EXCLUDED.quality_status,
           evidence_refs = EXCLUDED.evidence_refs
         RETURNING ocr_output_id, document_id, ocr_engine, ocr_engine_version, output_uri, output_checksum, confidence_score, quality_status, evidence_refs, created_at",
    )
    .bind(&input.ocr_output_id)
    .bind(&input.document_id)
    .bind(&input.ocr_engine)
    .bind(&input.ocr_engine_version)
    .bind(&input.output_uri)
    .bind(&input.output_checksum)
    .bind(input.confidence_score)
    .bind(&input.quality_status)
    .bind(string_values(&input.evidence_refs))
    .fetch_one(&repository.pool)
    .await?;
    Ok(Some(evidence_ocr_output_from_row(row)?))
}

pub(super) async fn list_evidence_ocr_outputs(
    repository: &PostgresScoringRepository,
    document_id: &str,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<EvidenceOcrOutputRecord>> {
    if get_evidence_document(repository, document_id, customer_scope_id)
        .await?
        .is_none()
    {
        return Ok(Vec::new());
    }
    let rows = sqlx::query(
        "SELECT ocr_output_id, document_id, ocr_engine, ocr_engine_version, output_uri, output_checksum, confidence_score, quality_status, evidence_refs, created_at
         FROM evidence_ocr_outputs
         WHERE document_id = $1
         ORDER BY ocr_output_id",
    )
    .bind(document_id)
    .fetch_all(&repository.pool)
    .await?;
    rows.into_iter().map(evidence_ocr_output_from_row).collect()
}

pub(super) async fn save_evidence_embedding_job(
    repository: &PostgresScoringRepository,
    input: CreateEvidenceEmbeddingJobInput,
) -> anyhow::Result<EvidenceEmbeddingJobRecord> {
    let row = sqlx::query(
        "INSERT INTO evidence_embedding_jobs
         (embedding_job_id, customer_scope_id, target_kind, target_ref, embedding_model, embedding_model_version, chunking_version, redaction_status, vector_store_kind, vector_store_ref, embedding_checksum, status, evidence_refs)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         ON CONFLICT (embedding_job_id) DO UPDATE SET
           customer_scope_id = EXCLUDED.customer_scope_id,
           target_kind = EXCLUDED.target_kind,
           target_ref = EXCLUDED.target_ref,
           embedding_model = EXCLUDED.embedding_model,
           embedding_model_version = EXCLUDED.embedding_model_version,
           chunking_version = EXCLUDED.chunking_version,
           redaction_status = EXCLUDED.redaction_status,
           vector_store_kind = EXCLUDED.vector_store_kind,
           vector_store_ref = EXCLUDED.vector_store_ref,
           embedding_checksum = EXCLUDED.embedding_checksum,
           status = EXCLUDED.status,
           evidence_refs = EXCLUDED.evidence_refs
         RETURNING embedding_job_id, customer_scope_id, target_kind, target_ref, embedding_model, embedding_model_version, chunking_version, redaction_status, vector_store_kind, vector_store_ref, embedding_checksum, status, evidence_refs, created_at, completed_at",
    )
    .bind(&input.embedding_job_id)
    .bind(&input.customer_scope_id)
    .bind(&input.target_kind)
    .bind(&input.target_ref)
    .bind(&input.embedding_model)
    .bind(&input.embedding_model_version)
    .bind(&input.chunking_version)
    .bind(&input.redaction_status)
    .bind(&input.vector_store_kind)
    .bind(&input.vector_store_ref)
    .bind(&input.embedding_checksum)
    .bind(&input.status)
    .bind(string_values(&input.evidence_refs))
    .fetch_one(&repository.pool)
    .await?;
    evidence_embedding_job_from_row(row)
}

pub(super) async fn list_evidence_embedding_jobs(
    repository: &PostgresScoringRepository,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<EvidenceEmbeddingJobRecord>> {
    let rows = sqlx::query(
        "SELECT embedding_job_id, customer_scope_id, target_kind, target_ref, embedding_model, embedding_model_version, chunking_version, redaction_status, vector_store_kind, vector_store_ref, embedding_checksum, status, evidence_refs, created_at, completed_at
         FROM evidence_embedding_jobs
         WHERE ($1::text IS NULL OR customer_scope_id = $1)
         ORDER BY embedding_job_id",
    )
    .bind(customer_scope_id)
    .fetch_all(&repository.pool)
    .await?;
    rows.into_iter()
        .map(evidence_embedding_job_from_row)
        .collect()
}

pub(super) async fn save_evidence_retrieval_audit_event(
    repository: &PostgresScoringRepository,
    input: CreateEvidenceRetrievalAuditEventInput,
) -> anyhow::Result<EvidenceRetrievalAuditEventRecord> {
    let row = sqlx::query(
        "INSERT INTO evidence_retrieval_audit_events
         (retrieval_id, customer_scope_id, actor_id, actor_role, query_kind, query_checksum, retrieval_method, embedding_model_version, top_k, source_refs, result_refs, redaction_status, evidence_refs)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         ON CONFLICT (retrieval_id) DO UPDATE SET
           customer_scope_id = EXCLUDED.customer_scope_id,
           actor_id = EXCLUDED.actor_id,
           actor_role = EXCLUDED.actor_role,
           query_kind = EXCLUDED.query_kind,
           query_checksum = EXCLUDED.query_checksum,
           retrieval_method = EXCLUDED.retrieval_method,
           embedding_model_version = EXCLUDED.embedding_model_version,
           top_k = EXCLUDED.top_k,
           source_refs = EXCLUDED.source_refs,
           result_refs = EXCLUDED.result_refs,
           redaction_status = EXCLUDED.redaction_status,
           evidence_refs = EXCLUDED.evidence_refs
         RETURNING retrieval_id, customer_scope_id, actor_id, actor_role, query_kind, query_checksum, retrieval_method, embedding_model_version, top_k, source_refs, result_refs, redaction_status, evidence_refs, created_at",
    )
    .bind(&input.retrieval_id)
    .bind(&input.customer_scope_id)
    .bind(&input.actor_id)
    .bind(&input.actor_role)
    .bind(&input.query_kind)
    .bind(&input.query_checksum)
    .bind(&input.retrieval_method)
    .bind(&input.embedding_model_version)
    .bind(input.top_k)
    .bind(string_values(&input.source_refs))
    .bind(string_values(&input.result_refs))
    .bind(&input.redaction_status)
    .bind(string_values(&input.evidence_refs))
    .fetch_one(&repository.pool)
    .await?;
    evidence_retrieval_audit_event_from_row(row)
}

pub(super) async fn list_evidence_retrieval_audit_events(
    repository: &PostgresScoringRepository,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<EvidenceRetrievalAuditEventRecord>> {
    let rows = sqlx::query(
        "SELECT retrieval_id, customer_scope_id, actor_id, actor_role, query_kind, query_checksum, retrieval_method, embedding_model_version, top_k, source_refs, result_refs, redaction_status, evidence_refs, created_at
         FROM evidence_retrieval_audit_events
         WHERE ($1::text IS NULL OR customer_scope_id = $1)
         ORDER BY created_at DESC, retrieval_id",
    )
    .bind(customer_scope_id)
    .fetch_all(&repository.pool)
    .await?;
    rows.into_iter()
        .map(evidence_retrieval_audit_event_from_row)
        .collect()
}
