use api_server::{app::build_app, config::AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
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

async fn json_request(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: &str,
) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}));
    (status, body)
}

async fn score_claim_with_missing_clinical_evidence(app: axum::Router, claim_id: &str) {
    let suffix = claim_id.replace('-', "_");
    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/claims/score",
        &format!(
            r#"{{
              "source_system": "tpa-demo",
              "claim": {{
                "external_claim_id": "{claim_id}",
                "claim_amount": "12000",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "diagnosis_code": "J10"
              }},
              "items": [
                {{
                  "item_code": "IMG-{suffix}",
                  "item_type": "procedure",
                  "description": "High cost imaging",
                  "quantity": 1,
                  "unit_amount": "12000",
                  "total_amount": "12000"
                }}
              ],
              "member": {{
                "external_member_id": "MBR-{suffix}"
              }},
              "policy": {{
                "external_policy_id": "POL-{suffix}",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "15000",
                "currency": "CNY"
              }},
              "provider": {{
                "external_provider_id": "PRV-{suffix}",
                "name": "Northwind Hospital",
                "provider_type": "hospital",
                "region": "SH",
                "risk_tier": "High"
              }}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
async fn backfill_evidence_request_and_label_bootstrap_flow() {
    let app = build_app(test_config());
    score_claim_with_missing_clinical_evidence(app.clone(), "CLM-BOOTSTRAP-1").await;

    let (status, backfill) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/backfills",
        r#"{
          "job_id": "backfill_bootstrap_test",
          "dataset_refs": ["dataset:historical_scoring_2026"],
          "rule_refs": ["rule:high_cost_imaging"],
          "reviewer": "ops-lead",
          "notes": "Replay governed historical scoring leads for label bootstrap.",
          "limit": 10
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{backfill}");
    assert_eq!(backfill["job"]["job_id"], "backfill_bootstrap_test");
    assert_eq!(backfill["job"]["candidate_count"], 1);
    assert_eq!(backfill["job"]["leads"][0]["claim_id"], "CLM-BOOTSTRAP-1");

    let (status, backfill_leads) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/backfills/backfill_bootstrap_test/leads",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(backfill_leads["leads"][0]["claim_id"], "CLM-BOOTSTRAP-1");

    let (status, generated) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/evidence-requests/generate",
        r#"{
          "claim_id": "CLM-BOOTSTRAP-1",
          "requested_by": "clinical-ops",
          "reviewer_queue": "clinical-evidence",
          "notes": "Generate missing clinical evidence checklist.",
          "limit": 10
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{generated}");
    let request = &generated["requests"][0];
    assert_eq!(request["claim_id"], "CLM-BOOTSTRAP-1");
    assert_eq!(request["status"], "open");
    assert!(request["missing_evidence"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("medical_record")));

    let request_id = request["request_id"].as_str().unwrap();
    let (status, updated) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/evidence-requests/{request_id}/status"),
        r#"{
          "status": "received",
          "actor_id": "clinical-ops",
          "notes": "Clinical document package received and linked.",
          "evidence_refs": ["evidence_documents:doc_bootstrap_1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["status"], "received");

    let (status, queue) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/label-bootstrap/queue",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{queue}");
    let item = &queue["items"][0];
    assert_eq!(item["source_type"], "evidence_request");
    assert_eq!(item["suggested_label_name"], "clinical_evidence_sufficient");
    assert_eq!(item["training_eligible"], false);

    let item_id = item["item_id"].as_str().unwrap();
    let (status, review) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/label-bootstrap/items/{item_id}/review"),
        r#"{
          "reviewer": "label-governance",
          "label_name": "clinical_evidence_sufficient",
          "label_value": "true",
          "governance_status": "approved_for_training",
          "feedback_target": "model",
          "notes": "Evidence was reviewed and can be used as a supervised label.",
          "evidence_refs": ["evidence_documents:doc_bootstrap_1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{review}");
    assert_eq!(review["item"]["training_eligible"], true);
    assert_eq!(review["item"]["review_status"], "reviewed");

    let (status, labels) = json_request(app, "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK, "{labels}");
    let bootstrap_label = labels["labels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|label| label["source_type"] == "label_bootstrap")
        .expect("bootstrap label should be visible to MLOps label pool");
    assert_eq!(bootstrap_label["claim_id"], "CLM-BOOTSTRAP-1");
    assert_eq!(
        bootstrap_label["governance_status"],
        "approved_for_training"
    );
    assert_eq!(bootstrap_label["feedback_target"], "model");
}

#[tokio::test]
async fn label_bootstrap_rejects_training_approval_before_evidence_is_received() {
    let app = build_app(test_config());
    score_claim_with_missing_clinical_evidence(app.clone(), "CLM-BOOTSTRAP-2").await;

    let (status, generated) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/evidence-requests/generate",
        r#"{
          "claim_id": "CLM-BOOTSTRAP-2",
          "requested_by": "clinical-ops",
          "reviewer_queue": "clinical-evidence",
          "notes": "Generate missing clinical evidence checklist.",
          "limit": 10
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{generated}");

    let (status, queue) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/label-bootstrap/queue",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{queue}");
    let item = &queue["items"][0];
    assert_eq!(item["suggested_label_name"], "insufficient_evidence");
    let item_id = item["item_id"].as_str().unwrap();

    let (status, review) = json_request(
        app,
        "POST",
        &format!("/api/v1/ops/label-bootstrap/items/{item_id}/review"),
        r#"{
          "reviewer": "label-governance",
          "label_name": "clinical_evidence_sufficient",
          "label_value": "true",
          "governance_status": "approved_for_training",
          "feedback_target": "model",
          "notes": "Attempt to approve before evidence is received.",
          "evidence_refs": ["evidence_requests:placeholder"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{review}");
    assert_eq!(review["code"], "LABEL_BOOTSTRAP_EVIDENCE_NOT_RECEIVED");
}

#[tokio::test]
async fn evidence_request_rejects_received_status_without_document_evidence() {
    let app = build_app(test_config());
    score_claim_with_missing_clinical_evidence(app.clone(), "CLM-BOOTSTRAP-3").await;

    let (status, generated) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/evidence-requests/generate",
        r#"{
          "claim_id": "CLM-BOOTSTRAP-3",
          "requested_by": "clinical-ops",
          "reviewer_queue": "clinical-evidence",
          "notes": "Generate missing clinical evidence checklist.",
          "limit": 10
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{generated}");
    let request_id = generated["requests"][0]["request_id"].as_str().unwrap();

    let (status, update) = json_request(
        app,
        "POST",
        &format!("/api/v1/ops/evidence-requests/{request_id}/status"),
        r#"{
          "status": "received",
          "actor_id": "clinical-ops",
          "notes": "Attempt to mark received without document evidence.",
          "evidence_refs": ["evidence_requests:placeholder"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{update}");
    assert_eq!(
        update["code"],
        "EVIDENCE_REQUEST_DOCUMENT_EVIDENCE_REQUIRED"
    );
}
