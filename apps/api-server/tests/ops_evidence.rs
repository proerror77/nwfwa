use api_server::{
    app::{build_app, build_app_with_parts},
    config::AppConfig,
    repository::InMemoryScoringRepository,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use fwa_ml_runtime::HeuristicModelScorer;
use std::sync::Arc;
use tower::ServiceExt;

fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        api_key_principals: vec![],
        source_system: "tpa-demo".into(),
        database_url: "postgres://unused".into(),
        model_service_url: "heuristic://local".into(),
        object_storage_uri: "local://demo-artifacts".into(),
        customer_scope_id: "demo-customer".into(),
        retention_policy_id: "demo-retention-policy".into(),
        backup_restore_plan_id: "demo-backup-restore-plan".into(),
        pii_masking_policy_id: "demo-pii-masking-policy".into(),
        key_rotation_policy_id: "demo-key-rotation-policy".into(),
        network_allowlist_id: "demo-network-allowlist".into(),
        alert_routing_policy_id: "demo-alert-routing-policy".into(),
        observability_exporter_endpoint: "local://demo-observability".into(),
        agent_policy_id: "demo-agent-policy".into(),
    }
}

fn scoped_config(api_key: &str, customer_scope_id: &str) -> AppConfig {
    AppConfig {
        api_key: api_key.into(),
        customer_scope_id: customer_scope_id.into(),
        ..test_config()
    }
}

async fn json_request(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: &str,
) -> (StatusCode, serde_json::Value) {
    json_request_with_key(app, method, uri, body, "dev-secret").await
}

async fn json_request_with_key(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: &str,
    api_key: &str,
) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("x-api-key", api_key)
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}));
    (status, body)
}

#[tokio::test]
async fn registers_ai_evidence_metadata_and_audit_trail() {
    let app = build_app(test_config()).unwrap();

    let (status, document) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/evidence/documents",
        r#"{
          "document_id": "doc-001",
          "source_record_ref": "claim-documents:CLM-0287:invoice-1",
          "claim_id": "CLM-0287",
          "external_document_id": "invoice-1",
          "document_type": "invoice",
          "storage_uri": "s3://customer-approved/documents/doc-001.pdf",
          "content_checksum": "sha256:doc001",
          "ingestion_status": "registered",
          "redaction_status": "pending",
          "evidence_refs": ["claim_documents:CLM-0287:invoice-1"],
          "metadata_json": {"source": "pilot_fixture"}
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(document["document_id"], "doc-001");
    assert_eq!(document["customer_scope_id"], "demo-customer");
    assert_eq!(document["source_system"], "tpa-demo");
    assert_eq!(document["retention_policy_id"], "demo-retention-policy");
    assert!(document.get("raw_text").is_none());

    let (status, loaded) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/evidence/documents/doc-001",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(loaded["content_checksum"], "sha256:doc001");

    let (status, listed) =
        json_request(app.clone(), "GET", "/api/v1/ops/evidence/documents", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed["documents"][0]["document_id"], "doc-001");

    let (status, chunk) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/evidence/documents/doc-001/chunks",
        r#"{
          "chunk_id": "chunk-001",
          "chunk_index": 0,
          "chunking_version": "medical-record-v1",
          "redaction_status": "redacted",
          "text_checksum": "sha256:chunk001",
          "token_count": 128,
          "storage_uri": "s3://customer-approved/chunks/chunk-001.json",
          "source_offsets_json": {"page": 1},
          "evidence_refs": ["evidence_documents:doc-001"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(chunk["document_id"], "doc-001");
    assert_eq!(chunk["text_checksum"], "sha256:chunk001");

    let (status, ocr) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/evidence/documents/doc-001/ocr-outputs",
        r#"{
          "ocr_output_id": "ocr-001",
          "ocr_engine": "customer-ocr",
          "ocr_engine_version": "2026.06",
          "output_uri": "s3://customer-approved/ocr/ocr-001.json",
          "output_checksum": "sha256:ocr001",
          "confidence_score": "0.94",
          "quality_status": "passed",
          "evidence_refs": ["evidence_documents:doc-001"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ocr["ocr_engine"], "customer-ocr");
    assert!(ocr.get("output_text").is_none());

    let (status, embedding_job) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/evidence/embedding-jobs",
        r#"{
          "embedding_job_id": "emb-001",
          "target_kind": "document_chunk",
          "target_ref": "chunk-001",
          "embedding_model": "customer-approved-embedder",
          "embedding_model_version": "v1",
          "chunking_version": "medical-record-v1",
          "redaction_status": "redacted",
          "vector_store_kind": "pgvector",
          "vector_store_ref": "pgvector:evidence_chunks:chunk-001",
          "embedding_checksum": "sha256:embedding001",
          "status": "queued",
          "evidence_refs": ["evidence_chunks:chunk-001"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(embedding_job["customer_scope_id"], "demo-customer");
    assert_eq!(embedding_job["vector_store_kind"], "pgvector");

    let (status, retrieval) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/evidence/retrieval-audit-events",
        r#"{
          "retrieval_id": "ret-001",
          "query_kind": "masked_claim_context",
          "query_checksum": "sha256:masked-query-001",
          "retrieval_method": "vector_top_k",
          "embedding_model_version": "v1",
          "top_k": 5,
          "source_refs": ["claim_context:CLM-0287"],
          "result_refs": ["evidence_chunks:chunk-001"],
          "redaction_status": "redacted",
          "evidence_refs": ["retrieval:ret-001"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(retrieval["actor_id"], "tpa-demo");
    assert_eq!(retrieval["query_checksum"], "sha256:masked-query-001");
    assert!(retrieval.get("query_text").is_none());

    let (status, audits) = json_request(
        app,
        "GET",
        "/api/v1/ops/audit-events?event_group=governance&limit=20",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let event_types = audits["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["event_type"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(event_types.contains(&"evidence.document.registered"));
    assert!(event_types.contains(&"evidence.retrieval_audit.recorded"));
}

#[tokio::test]
async fn rejects_orphan_evidence_children() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/evidence/documents/missing-doc/chunks",
        r#"{
          "chunk_id": "chunk-orphan",
          "chunk_index": 0,
          "chunking_version": "v1",
          "redaction_status": "redacted",
          "text_checksum": "sha256:chunk-orphan",
          "token_count": 1,
          "storage_uri": "s3://customer-approved/chunks/chunk-orphan.json"
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "EVIDENCE_DOCUMENT_NOT_FOUND");
}

#[tokio::test]
async fn rejects_local_or_placeholder_evidence_refs() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/evidence/documents",
        r#"{
          "document_id": "doc-local-ref",
          "source_record_ref": "claim-documents:CLM-LOCAL:invoice-1",
          "claim_id": "CLM-LOCAL",
          "external_document_id": "invoice-local",
          "document_type": "invoice",
          "storage_uri": "s3://customer-approved/documents/doc-local-ref.pdf",
          "content_checksum": "sha256:doc-local-ref",
          "ingestion_status": "registered",
          "redaction_status": "redacted",
          "evidence_refs": ["claim_documents:local://template/claim-documents/doc-local-ref.json"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "EVIDENCE_REF_INVALID");

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/evidence/documents",
        r#"{
          "document_id": "doc-retrieval-ref",
          "source_record_ref": "claim-documents:CLM-RETRIEVAL:invoice-1",
          "claim_id": "CLM-RETRIEVAL",
          "external_document_id": "invoice-retrieval",
          "document_type": "invoice",
          "storage_uri": "s3://customer-approved/documents/doc-retrieval-ref.pdf",
          "content_checksum": "sha256:doc-retrieval-ref",
          "ingestion_status": "registered",
          "redaction_status": "redacted",
          "evidence_refs": ["claim_documents:CLM-RETRIEVAL:invoice-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/evidence/retrieval-audit-events",
        r#"{
          "retrieval_id": "ret-local-ref",
          "query_kind": "masked_claim_context",
          "query_checksum": "sha256:masked-query-local-ref",
          "retrieval_method": "vector_top_k",
          "embedding_model_version": "v1",
          "top_k": 5,
          "source_refs": ["claim_context:{claim_id}"],
          "result_refs": ["evidence_documents:doc-retrieval-ref"],
          "redaction_status": "redacted",
          "evidence_refs": ["retrieval:ret-local-ref"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "EVIDENCE_REF_INVALID");
}

#[tokio::test]
async fn evidence_metadata_is_scoped_to_authenticated_customer() {
    let repository = InMemoryScoringRepository::shared();
    let alpha_app = build_app_with_parts(
        scoped_config("alpha-secret", "customer-alpha"),
        Arc::new(HeuristicModelScorer),
        repository.clone(),
    );
    let beta_app = build_app_with_parts(
        scoped_config("beta-secret", "customer-beta"),
        Arc::new(HeuristicModelScorer),
        repository,
    );

    let (status, document) = json_request_with_key(
        alpha_app.clone(),
        "POST",
        "/api/v1/ops/evidence/documents",
        r#"{
          "document_id": "doc-alpha-only",
          "source_record_ref": "claim-documents:CLM-ALPHA:invoice-1",
          "claim_id": "CLM-ALPHA",
          "external_document_id": "invoice-alpha",
          "document_type": "invoice",
          "storage_uri": "s3://customer-alpha/documents/doc-alpha-only.pdf",
          "content_checksum": "sha256:alpha-doc",
          "ingestion_status": "registered",
          "redaction_status": "redacted",
          "evidence_refs": ["claim_documents:CLM-ALPHA:invoice-1"]
        }"#,
        "alpha-secret",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(document["customer_scope_id"], "customer-alpha");

    let (status, alpha_documents) = json_request_with_key(
        alpha_app,
        "GET",
        "/api/v1/ops/evidence/documents",
        "{}",
        "alpha-secret",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(alpha_documents["documents"].as_array().unwrap().len(), 1);

    let (status, beta_documents) = json_request_with_key(
        beta_app.clone(),
        "GET",
        "/api/v1/ops/evidence/documents",
        "{}",
        "beta-secret",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(beta_documents["documents"].as_array().unwrap().is_empty());

    let (status, body) = json_request_with_key(
        beta_app,
        "GET",
        "/api/v1/ops/evidence/documents/doc-alpha-only",
        "{}",
        "beta-secret",
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "EVIDENCE_DOCUMENT_NOT_FOUND");
}
