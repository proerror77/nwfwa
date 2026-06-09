use super::*;

impl InMemoryScoringRepository {
    pub(super) async fn in_memory_save_evidence_document(
        &self,
        input: CreateEvidenceDocumentInput,
    ) -> anyhow::Result<EvidenceDocumentRecord> {
        let record = EvidenceDocumentRecord {
            document_id: input.document_id,
            customer_scope_id: input.customer_scope_id,
            source_system: input.source_system,
            source_record_ref: input.source_record_ref,
            claim_id: input.claim_id,
            external_document_id: input.external_document_id,
            document_type: input.document_type,
            storage_uri: input.storage_uri,
            content_checksum: input.content_checksum,
            ingestion_status: input.ingestion_status,
            redaction_status: input.redaction_status,
            retention_policy_id: input.retention_policy_id,
            evidence_refs: input.evidence_refs,
            metadata_json: input.metadata_json,
            created_at: None,
            updated_at: None,
        };
        self.evidence_documents
            .lock()
            .await
            .insert(record.document_id.clone(), record.clone());
        Ok(record)
    }

    pub(super) async fn in_memory_list_evidence_documents(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentRecord>> {
        let mut records = self
            .evidence_documents
            .lock()
            .await
            .values()
            .filter(|record| {
                customer_scope_id.is_none_or(|scope| record.customer_scope_id == scope)
            })
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.document_id.cmp(&right.document_id));
        Ok(records)
    }

    pub(super) async fn in_memory_get_evidence_document(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentRecord>> {
        Ok(self
            .evidence_documents
            .lock()
            .await
            .get(document_id)
            .filter(|record| {
                customer_scope_id.is_none_or(|scope| record.customer_scope_id == scope)
            })
            .cloned())
    }

    pub(super) async fn in_memory_save_evidence_document_chunk(
        &self,
        input: CreateEvidenceDocumentChunkInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentChunkRecord>> {
        if self
            .in_memory_get_evidence_document(&input.document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let record = EvidenceDocumentChunkRecord {
            chunk_id: input.chunk_id,
            document_id: input.document_id,
            chunk_index: input.chunk_index,
            chunking_version: input.chunking_version,
            redaction_status: input.redaction_status,
            text_checksum: input.text_checksum,
            token_count: input.token_count,
            storage_uri: input.storage_uri,
            source_offsets_json: input.source_offsets_json,
            evidence_refs: input.evidence_refs,
            created_at: None,
        };
        self.evidence_document_chunks
            .lock()
            .await
            .insert(record.chunk_id.clone(), record.clone());
        Ok(Some(record))
    }

    pub(super) async fn in_memory_list_evidence_document_chunks(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentChunkRecord>> {
        if self
            .in_memory_get_evidence_document(document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(Vec::new());
        }
        let mut records = self
            .evidence_document_chunks
            .lock()
            .await
            .values()
            .filter(|record| record.document_id == document_id)
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by_key(|record| record.chunk_index);
        Ok(records)
    }

    pub(super) async fn in_memory_save_evidence_ocr_output(
        &self,
        input: CreateEvidenceOcrOutputInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceOcrOutputRecord>> {
        if self
            .in_memory_get_evidence_document(&input.document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let record = EvidenceOcrOutputRecord {
            ocr_output_id: input.ocr_output_id,
            document_id: input.document_id,
            ocr_engine: input.ocr_engine,
            ocr_engine_version: input.ocr_engine_version,
            output_uri: input.output_uri,
            output_checksum: input.output_checksum,
            confidence_score: input.confidence_score,
            quality_status: input.quality_status,
            evidence_refs: input.evidence_refs,
            created_at: None,
        };
        self.evidence_ocr_outputs
            .lock()
            .await
            .insert(record.ocr_output_id.clone(), record.clone());
        Ok(Some(record))
    }

    pub(super) async fn in_memory_list_evidence_ocr_outputs(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceOcrOutputRecord>> {
        if self
            .in_memory_get_evidence_document(document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(Vec::new());
        }
        let mut records = self
            .evidence_ocr_outputs
            .lock()
            .await
            .values()
            .filter(|record| record.document_id == document_id)
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.ocr_output_id.cmp(&right.ocr_output_id));
        Ok(records)
    }

    pub(super) async fn in_memory_save_evidence_embedding_job(
        &self,
        input: CreateEvidenceEmbeddingJobInput,
    ) -> anyhow::Result<EvidenceEmbeddingJobRecord> {
        let record = EvidenceEmbeddingJobRecord {
            embedding_job_id: input.embedding_job_id,
            customer_scope_id: input.customer_scope_id,
            target_kind: input.target_kind,
            target_ref: input.target_ref,
            embedding_model: input.embedding_model,
            embedding_model_version: input.embedding_model_version,
            chunking_version: input.chunking_version,
            redaction_status: input.redaction_status,
            vector_store_kind: input.vector_store_kind,
            vector_store_ref: input.vector_store_ref,
            embedding_checksum: input.embedding_checksum,
            status: input.status,
            evidence_refs: input.evidence_refs,
            created_at: None,
            completed_at: None,
        };
        self.evidence_embedding_jobs
            .lock()
            .await
            .insert(record.embedding_job_id.clone(), record.clone());
        Ok(record)
    }

    pub(super) async fn in_memory_list_evidence_embedding_jobs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceEmbeddingJobRecord>> {
        let mut records = self
            .evidence_embedding_jobs
            .lock()
            .await
            .values()
            .filter(|record| {
                customer_scope_id.is_none_or(|scope| record.customer_scope_id == scope)
            })
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.embedding_job_id.cmp(&right.embedding_job_id));
        Ok(records)
    }

    pub(super) async fn in_memory_save_evidence_retrieval_audit_event(
        &self,
        input: CreateEvidenceRetrievalAuditEventInput,
    ) -> anyhow::Result<EvidenceRetrievalAuditEventRecord> {
        let record = EvidenceRetrievalAuditEventRecord {
            retrieval_id: input.retrieval_id,
            customer_scope_id: input.customer_scope_id,
            actor_id: input.actor_id,
            actor_role: input.actor_role,
            query_kind: input.query_kind,
            query_checksum: input.query_checksum,
            retrieval_method: input.retrieval_method,
            embedding_model_version: input.embedding_model_version,
            top_k: input.top_k,
            source_refs: input.source_refs,
            result_refs: input.result_refs,
            redaction_status: input.redaction_status,
            evidence_refs: input.evidence_refs,
            created_at: None,
        };
        self.evidence_retrieval_audit_events
            .lock()
            .await
            .insert(record.retrieval_id.clone(), record.clone());
        Ok(record)
    }

    pub(super) async fn in_memory_list_evidence_retrieval_audit_events(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceRetrievalAuditEventRecord>> {
        let mut records = self
            .evidence_retrieval_audit_events
            .lock()
            .await
            .values()
            .filter(|record| {
                customer_scope_id.is_none_or(|scope| record.customer_scope_id == scope)
            })
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.retrieval_id.cmp(&right.retrieval_id));
        Ok(records)
    }
}
