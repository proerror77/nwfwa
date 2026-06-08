use crate::constants::API_UNAVAILABLE_MESSAGE;
use crate::types::*;
use gloo_net::http::Request;
use serde::Deserialize;
use serde_json::{json, Value};

mod models;

pub(crate) use models::*;

pub(crate) async fn request_json<T>(
    path: &str,
    api_key: String,
    payload: Value,
) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let request = Request::post(path)
        .header("content-type", "application/json")
        .header("x-api-key", &api_key)
        .body(payload.to_string())
        .map_err(|error| error.to_string())?;
    let response = request.send().await.map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response.text().await.map_err(|error| error.to_string())?;
    parse_json_response(path, status, &body)
}

pub(crate) async fn request_get_json<T>(path: &str, api_key: String) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let response = Request::get(path)
        .header("x-api-key", &api_key)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response.text().await.map_err(|error| error.to_string())?;
    parse_json_response(path, status, &body)
}

fn parse_json_response<T>(path: &str, status: u16, body: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let body = body.trim();
    if !(200..300).contains(&status) {
        return Err(api_error_message(path, status, body));
    }
    if body.is_empty() {
        return Err(API_UNAVAILABLE_MESSAGE.to_string());
    }
    let body: Value = serde_json::from_str(body)
        .map_err(|error| format!("Invalid API response from {path}: {error}"))?;
    serde_json::from_value(body).map_err(|error| error.to_string())
}

fn api_error_message(path: &str, status: u16, body: &str) -> String {
    if body.is_empty() {
        return API_UNAVAILABLE_MESSAGE.to_string();
    }
    match serde_json::from_str::<Value>(body) {
        Ok(body) => body
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("HTTP {status}: {}", pretty_json(&body))),
        Err(_) => format!("HTTP {status} from {path}: {body}"),
    }
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

pub(crate) async fn normalize_claim(
    payload: Value,
    api_key: String,
) -> Result<InboxNormalizeResponse, String> {
    request_json("/api/v1/inbox/claims/normalize", api_key, payload).await
}

pub(crate) async fn score_canonical_claim(
    payload: Value,
    api_key: String,
) -> Result<ScoreResponse, String> {
    request_json("/api/v1/claims/score", api_key, payload).await
}

pub(crate) async fn get_dashboard_summary(api_key: String) -> Result<DashboardSummary, String> {
    request_get_json("/api/v1/ops/dashboard/summary", api_key).await
}

pub(crate) async fn get_rule_ops_snapshot(
    api_key: String,
    rule_id: String,
) -> Result<RuleOpsSnapshot, String> {
    let rules = request_get_json::<RuleListResponse>("/api/v1/ops/rules", api_key.clone())
        .await?
        .rules;
    let selected_rule_id = rules
        .iter()
        .find(|rule| rule.rule_id == rule_id)
        .map(|rule| rule.rule_id.clone())
        .or_else(|| rules.first().map(|rule| rule.rule_id.clone()))
        .unwrap_or(rule_id);
    let performance = request_get_json::<RulePerformanceResponse>(
        "/api/v1/ops/rules/performance",
        api_key.clone(),
    )
    .await?
    .rules;
    let gates = request_get_json::<RulePromotionGates>(
        &format!("/api/v1/ops/rules/{selected_rule_id}/promotion-gates"),
        api_key,
    )
    .await?;
    Ok(RuleOpsSnapshot {
        rules,
        performance,
        gates,
    })
}

pub(crate) async fn get_factor_readiness(
    api_key: String,
) -> Result<FactorReadinessResponse, String> {
    request_get_json("/api/v1/ops/factors/readiness", api_key).await
}

pub(crate) async fn get_data_sources_snapshot(
    api_key: String,
) -> Result<DataSourcesSnapshot, String> {
    let datasets =
        request_get_json::<DatasetListResponse>("/api/v1/ops/datasets", api_key.clone()).await?;
    let evaluations =
        request_get_json::<ModelEvaluationListResponse>("/api/v1/ops/model-evaluations", api_key)
            .await?;
    Ok(DataSourcesSnapshot {
        datasets: datasets.datasets,
        health: datasets.health,
        evaluations: evaluations.evaluations,
        lineage: evaluations.lineage,
    })
}

pub(crate) async fn get_leads_cases_snapshot(
    api_key: String,
) -> Result<LeadsCasesSnapshot, String> {
    let leads = request_get_json::<LeadListResponse>("/api/v1/ops/leads", api_key.clone())
        .await?
        .leads;
    let cases = request_get_json::<CaseListResponse>("/api/v1/ops/cases", api_key)
        .await?
        .cases;
    Ok(LeadsCasesSnapshot { leads, cases })
}

pub(crate) async fn post_triage_lead(
    api_key: String,
    lead_id: String,
    payload: Value,
) -> Result<TriageLeadRecord, String> {
    request_json(
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        api_key,
        payload,
    )
    .await
}

pub(crate) async fn post_case_status(
    api_key: String,
    case_id: String,
    payload: Value,
) -> Result<UpdateCaseStatusRecord, String> {
    request_json(
        &format!("/api/v1/ops/cases/{case_id}/status"),
        api_key,
        payload,
    )
    .await
}

pub(crate) async fn post_investigation_result(
    api_key: String,
    payload: Value,
) -> Result<PilotWritebackResponse, String> {
    request_json("/api/v1/investigations/results", api_key, payload).await
}

pub(crate) async fn get_member_profile_summary(
    api_key: String,
    member_id: String,
) -> Result<MemberProfileSummary, String> {
    let member_id = member_id.trim();
    if member_id.is_empty() {
        return Err("member id is required".into());
    }
    request_get_json(
        &format!("/api/v1/members/{member_id}/profile-summary"),
        api_key,
    )
    .await
}

pub(crate) async fn get_provider_risk_summary(
    api_key: String,
) -> Result<ProviderRiskSummary, String> {
    request_get_json("/api/v1/ops/providers/risk-summary", api_key).await
}

pub(crate) async fn get_audit_samples(api_key: String) -> Result<Vec<AuditSampleRecord>, String> {
    Ok(
        request_get_json::<AuditSampleListResponse>("/api/v1/ops/audit-samples", api_key)
            .await?
            .samples,
    )
}

pub(crate) async fn post_audit_sample(
    api_key: String,
    payload: Value,
) -> Result<AuditSampleRecord, String> {
    request_json("/api/v1/ops/audit-samples", api_key, payload).await
}

pub(crate) async fn get_audit_events_for_sample(
    api_key: String,
    sample_id: String,
) -> Result<Vec<AuditEventRecord>, String> {
    let sample_id = sample_id.trim();
    if sample_id.is_empty() {
        return Err("audit sample id is required".into());
    }
    Ok(request_get_json::<AuditEventListResponse>(
        &format!("/api/v1/ops/audit-events?sample_id={sample_id}&limit=20"),
        api_key,
    )
    .await?
    .events)
}

pub(crate) async fn get_medical_review_queue(
    api_key: String,
    limit: String,
) -> Result<Vec<MedicalReviewQueueItem>, String> {
    let limit = limit
        .trim()
        .parse::<u32>()
        .ok()
        .map(|value| value.clamp(1, 200))
        .unwrap_or(100);
    Ok(request_get_json::<MedicalReviewQueueResponse>(
        &format!("/api/v1/ops/medical-review/queue?limit={limit}"),
        api_key,
    )
    .await?
    .items)
}

pub(crate) async fn post_medical_review_result(
    api_key: String,
    payload: Value,
) -> Result<MedicalReviewResultResponse, String> {
    request_json("/api/v1/ops/medical-review/results", api_key, payload).await
}

pub(crate) async fn get_qa_review_snapshot(api_key: String) -> Result<QaReviewSnapshot, String> {
    let queue = request_get_json::<QaQueueListResponse>("/api/v1/ops/qa/queue", api_key.clone())
        .await?
        .items;
    let summary =
        request_get_json::<QaQueueSummary>("/api/v1/ops/qa/queue-summary", api_key.clone()).await?;
    let feedback_items =
        request_get_json::<QaFeedbackItemListResponse>("/api/v1/ops/qa/feedback-items", api_key)
            .await?
            .items;
    Ok(QaReviewSnapshot {
        queue,
        summary,
        feedback_items,
    })
}

pub(crate) async fn get_bootstrap_ops_snapshot(
    api_key: String,
) -> Result<BootstrapOpsSnapshot, String> {
    let backfills = request_get_json::<HistoricalBackfillListResponse>(
        "/api/v1/ops/backfills",
        api_key.clone(),
    )
    .await?
    .jobs;
    let evidence_requests = request_get_json::<EvidenceRequestListResponse>(
        "/api/v1/ops/evidence-requests",
        api_key.clone(),
    )
    .await?
    .requests;
    let label_items = request_get_json::<LabelBootstrapQueueResponse>(
        "/api/v1/ops/label-bootstrap/queue",
        api_key,
    )
    .await?
    .items;
    Ok(BootstrapOpsSnapshot {
        backfills,
        evidence_requests,
        label_items,
    })
}

pub(crate) async fn create_bootstrap_backfill(
    api_key: String,
) -> Result<HistoricalBackfillResponse, String> {
    request_json(
        "/api/v1/ops/backfills",
        api_key,
        json!({
            "dataset_refs": ["ops:current_scoring_audit"],
            "rule_refs": ["ops:active_rule_library"],
            "reviewer": "ops-lead",
            "notes": "Create a governed replay snapshot for label handoff.",
            "limit": 25,
        }),
    )
    .await
}

pub(crate) async fn generate_bootstrap_evidence_requests(
    api_key: String,
) -> Result<EvidenceRequestGenerateResponse, String> {
    request_json(
        "/api/v1/ops/evidence-requests/generate",
        api_key,
        json!({
            "requested_by": "clinical-ops",
            "reviewer_queue": "clinical-evidence",
            "notes": "Generate missing-evidence requests from scoring audits.",
            "limit": 50,
        }),
    )
    .await
}

pub(crate) async fn mark_bootstrap_evidence_received(
    api_key: String,
    request_id: String,
    evidence_refs: Vec<String>,
    notes: String,
) -> Result<EvidenceRequestRecord, String> {
    request_json(
        &format!("/api/v1/ops/evidence-requests/{request_id}/status"),
        api_key,
        json!({
            "status": "received",
            "actor_id": "clinical-ops",
            "notes": notes,
            "evidence_refs": evidence_refs,
        }),
    )
    .await
}

pub(crate) async fn review_bootstrap_label(
    api_key: String,
    item_id: String,
    label_name: String,
    label_value: String,
    governance_status: String,
    feedback_target: String,
    notes: String,
    evidence_refs: Vec<String>,
) -> Result<LabelBootstrapReviewResponse, String> {
    request_json(
        &format!("/api/v1/ops/label-bootstrap/items/{item_id}/review"),
        api_key,
        json!({
            "reviewer": "label-governance",
            "label_name": label_name,
            "label_value": label_value,
            "governance_status": governance_status,
            "feedback_target": feedback_target,
            "notes": notes,
            "evidence_refs": evidence_refs,
        }),
    )
    .await
}

pub(crate) async fn get_agent_runs(api_key: String) -> Result<Vec<AgentRunRecord>, String> {
    Ok(
        request_get_json::<AgentRunListResponse>("/api/v1/ops/agent-runs", api_key)
            .await?
            .runs,
    )
}

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

pub(crate) async fn post_agent_investigation(
    api_key: String,
    payload: Value,
) -> Result<AgentInvestigationResponse, String> {
    request_json("/api/v1/agent/cases/investigate", api_key, payload).await
}

pub(crate) async fn get_governance_snapshot(
    api_key: String,
    event_group: String,
) -> Result<GovernanceSnapshot, String> {
    let health = request_get_json::<HealthResponse>("/api/v1/health", api_key.clone()).await?;
    let event_group = event_group.trim();
    let audit_path = if event_group.is_empty() {
        "/api/v1/ops/audit-events?limit=20".to_string()
    } else {
        format!("/api/v1/ops/audit-events?event_group={event_group}&limit=20")
    };
    let audit_events = request_get_json::<AuditEventListResponse>(&audit_path, api_key.clone())
        .await?
        .events;
    let api_calls =
        request_get_json::<ApiCallListResponse>("/api/v1/ops/api-calls?limit=20", api_key.clone())
            .await?
            .calls;
    let agent_runs = get_agent_runs(api_key).await?;
    Ok(GovernanceSnapshot {
        health,
        audit_events,
        api_calls,
        agent_runs,
    })
}
