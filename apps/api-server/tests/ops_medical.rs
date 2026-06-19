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
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .header("x-api-key", "dev-secret")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}));
    (status, body)
}

#[tokio::test]
async fn lists_medical_review_queue_from_clinical_evidence_audit() {
    let app = build_app(test_config()).unwrap();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-MEDICAL-QUEUE-1",
            "claim_amount": "12000",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "items": [
            {
              "item_code": "IMG-900",
              "item_type": "procedure",
              "description": "High cost imaging",
              "quantity": 1,
              "unit_amount": "12000",
              "total_amount": "12000"
            }
          ],
          "member": {
            "external_member_id": "MBR-MEDICAL-QUEUE-1"
          },
          "policy": {
            "external_policy_id": "POL-MEDICAL-QUEUE-1",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "15000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-MEDICAL-QUEUE-1",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "High"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, queue) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/medical-review/queue?limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = queue["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["claim_id"], "CLM-MEDICAL-QUEUE-1");
    // High-risk provider + missing medical_record → fraud_investigation_review
    assert_eq!(items[0]["review_route"], "fraud_investigation_review");
    assert_eq!(items[0]["evidence_status"], "missing_required_evidence");
    assert_eq!(items[0]["medical_reasonableness_score"], 100);
    assert_eq!(items[0]["first_item_code"], "IMG-900");
    assert_eq!(
        items[0]["first_issue_type"],
        "medical_necessity_review_required"
    );
    assert!(items[0]["missing_evidence"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("medical_record")));
    assert!(items[0]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("claim_items:IMG-900")));
    assert_eq!(items[0]["review_status"], "open");

    let scoring_audit_id = items[0]["audit_id"].as_str().unwrap();
    let (status, review) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/medical-review/results",
        &format!(
            r#"{{
              "claim_id": "CLM-MEDICAL-QUEUE-1",
              "scoring_audit_id": "{scoring_audit_id}",
              "reviewer": "medical-reviewer-1",
              "decision": "request_more_evidence",
              "notes": "Medical record is required before necessity can be confirmed.",
              "evidence_refs": ["audit:{scoring_audit_id}", "claim_items:IMG-900"]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(review["event_type"], "medical.review.recorded");
    assert_eq!(review["review_status"], "pending_evidence");

    let (status, queue) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/medical-review/queue?limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let item = &queue["items"].as_array().unwrap()[0];
    assert_eq!(item["review_status"], "pending_evidence");
    assert_eq!(item["review_decision"], "request_more_evidence");
    assert_eq!(item["reviewer"], "medical-reviewer-1");
    assert_eq!(item["review_audit_id"], review["audit_id"]);

    let (status, webhooks) = json_request(app, "GET", "/api/v1/ops/webhook-events", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let medical_review_event = webhooks["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "fwa.medical.reviewed")
        .expect("missing medical review webhook event");
    assert_eq!(medical_review_event["claim_id"], "CLM-MEDICAL-QUEUE-1");
    assert_eq!(medical_review_event["customer_scope_id"], "demo-customer");
    assert_eq!(
        medical_review_event["source_event_type"],
        "medical.review.recorded"
    );
    assert_eq!(medical_review_event["source_audit_id"], review["audit_id"]);
    assert_eq!(medical_review_event["delivery_status"], "pending");
    assert_eq!(
        medical_review_event["idempotency_key"],
        format!(
            "fwa-webhook:fwa.medical.reviewed:{}",
            review["audit_id"].as_str().unwrap()
        )
    );
    assert!(medical_review_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(format!("audit:{scoring_audit_id}"))));
}

#[tokio::test]
async fn medical_review_preserves_canonical_trace_from_scoring_audit() {
    let app = build_app(test_config()).unwrap();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "canonical_claim_context": {
            "claim_header": {
              "external_claim_id": "CLM-MEDICAL-CANONICAL",
              "total_amount": 12000,
              "currency": "CNY",
              "service_date": "2026-01-06"
            },
            "member_policy_snapshot": {
              "masked_member_id": "masked-member-medical",
              "masked_certificate_id": "masked-cert-medical",
              "policy_id": "POL-MEDICAL-CANONICAL",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": 15000
            },
            "provider_snapshot": {
              "provider_id": "PRV-MEDICAL-CANONICAL",
              "name": "Medical Trace Hospital",
              "provider_type": "hospital",
              "region": "SH",
              "risk_tier": "High"
            },
            "itemized_bill_lines": [
              {
                "item_code": "IMG-CANONICAL",
                "item_name": "High cost imaging",
                "fee_category": "procedure",
                "amount": 12000,
                "diagnosis_list": [{ "code": "J10", "name": "Influenza" }],
                "source_path": "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]",
                "evidence_refs": ["invoice:INV-MEDICAL:fee_detail:LINE-1"]
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, queue) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/medical-review/queue?limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let item = queue["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["claim_id"] == "CLM-MEDICAL-CANONICAL")
        .expect("canonical claim should require medical review");
    assert!(
        item["canonical_source_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]"
            )),
        "medical review queue should expose normalized bill-line source path"
    );
    assert!(
        item["canonical_evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("invoice:INV-MEDICAL:fee_detail:LINE-1")),
        "medical review queue should expose canonical evidence refs"
    );

    let scoring_audit_id = item["audit_id"].as_str().unwrap();
    let (status, review) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/medical-review/results",
        &format!(
            r#"{{
              "claim_id": "CLM-MEDICAL-CANONICAL",
              "scoring_audit_id": "{scoring_audit_id}",
              "reviewer": "medical-reviewer-1",
              "decision": "request_more_evidence",
              "notes": "Medical record is required before necessity can be confirmed.",
              "evidence_refs": ["audit:{scoring_audit_id}", "claim_items:IMG-CANONICAL"]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        review["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("invoice:INV-MEDICAL:fee_detail:LINE-1")),
        "medical review response should preserve canonical evidence refs"
    );

    let (status, audit) = json_request(
        app,
        "GET",
        "/api/v1/audit/claims/CLM-MEDICAL-CANONICAL",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let medical_review_event = audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "medical.review.recorded")
        .expect("medical review event should be in audit history");
    assert!(
        medical_review_event["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("invoice:INV-MEDICAL:fee_detail:LINE-1")),
        "medical review audit event should preserve canonical evidence refs"
    );
}

#[tokio::test]
async fn medical_review_records_controlled_clinical_outcomes_for_labels() {
    let app = build_app(test_config()).unwrap();

    let (status, review) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/medical-review/results",
        r#"{
          "claim_id": "CLM-MEDICAL-OUTCOMES",
          "scoring_audit_id": "audit_scoring_outcomes",
          "reviewer": "medical-reviewer-1",
          "decision": "request_more_evidence",
          "clinical_outcomes": ["documentation_issue", "insufficient_evidence"],
          "notes": "Medical record and order evidence are required before necessity can be confirmed.",
          "evidence_refs": ["audit:audit_scoring_outcomes", "claim_items:IMG-OUTCOMES"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        review["clinical_outcomes"],
        serde_json::json!(["documentation_issue", "insufficient_evidence"])
    );

    let (status, audit) = json_request(
        app.clone(),
        "GET",
        "/api/v1/audit/claims/CLM-MEDICAL-OUTCOMES",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let medical_review_event = audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "medical.review.recorded")
        .expect("medical review event should be in audit history");
    assert_eq!(
        medical_review_event["payload"]["clinical_outcomes"],
        serde_json::json!(["documentation_issue", "insufficient_evidence"])
    );
    assert_eq!(
        medical_review_event["payload"]["customer_scope_id"],
        "demo-customer"
    );
    assert_eq!(medical_review_event["actor_role"], "tpa_system");
    assert_eq!(medical_review_event["payload"]["actor_id"], "tpa-demo");
    assert_eq!(medical_review_event["payload"]["actor_role"], "tpa_system");

    let (status, labels) = json_request(app, "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let label_names = labels["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|label| label["label_name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(label_names.contains(&"documentation_issue"));
    assert!(label_names.contains(&"insufficient_evidence"));
}

#[tokio::test]
async fn medical_review_derives_false_positive_outcome_for_no_medical_issue() {
    let app = build_app(test_config()).unwrap();

    let (status, review) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/medical-review/results",
        r#"{
          "claim_id": "CLM-MEDICAL-FALSE-POSITIVE",
          "scoring_audit_id": "audit_scoring_false_positive",
          "reviewer": "medical-reviewer-1",
          "decision": "no_medical_issue",
          "notes": "Reviewed medical evidence supports the billed item.",
          "evidence_refs": ["audit:audit_scoring_false_positive", "claim_items:IMG-FP"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        review["clinical_outcomes"],
        serde_json::json!(["false_positive"])
    );

    let (status, labels) = json_request(app, "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let false_positive_label = labels["labels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|label| label["label_name"] == "false_positive")
        .expect("no_medical_issue should keep false_positive label compatibility");
    assert_eq!(false_positive_label["feedback_target"], "model");
    assert_eq!(
        false_positive_label["governance_status"],
        "approved_for_training"
    );
}

#[tokio::test]
async fn rejects_medical_review_result_without_evidence() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/medical-review/results",
        r#"{
          "claim_id": "CLM-MEDICAL-QUEUE-2",
          "scoring_audit_id": "audit_scoring_1",
          "reviewer": "medical-reviewer-1",
          "decision": "request_more_evidence",
          "notes": "",
          "evidence_refs": []
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_MEDICAL_REVIEW_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/medical-review/results",
        r#"{
          "claim_id": "CLM-MEDICAL-QUEUE-2",
          "scoring_audit_id": "audit_scoring_1",
          "reviewer": "medical-reviewer-1",
          "decision": "request_more_evidence",
          "notes": "Medical record is required before necessity can be confirmed.",
          "evidence_refs": [" "]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_MEDICAL_REVIEW_EVIDENCE");

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/medical-review/results",
        r#"{
          "claim_id": "CLM-MEDICAL-QUEUE-2",
          "scoring_audit_id": "audit_scoring_1",
          "reviewer": "medical-reviewer-1",
          "decision": "request_more_evidence",
          "notes": "Medical record is required before necessity can be confirmed.",
          "evidence_refs": ["audit:scoring.completed", " "]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_MEDICAL_REVIEW_EVIDENCE");
}

#[tokio::test]
async fn rejects_pii_in_medical_review_writeback() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/medical-review/results",
        r#"{
          "claim_id": "CLM-MEDICAL-PII",
          "scoring_audit_id": "audit_scoring_1",
          "reviewer": "medical-reviewer-1",
          "decision": "request_more_evidence",
          "notes": "Member ID 11010519491231002X was copied into the review note.",
          "evidence_refs": ["audit:scoring.completed"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_WRITEBACK");

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/medical-review/results",
        r#"{
          "claim_id": "CLM-MEDICAL-PII",
          "scoring_audit_id": "audit_scoring_1",
          "reviewer": "medical-reviewer-1",
          "decision": "request_more_evidence",
          "notes": "Medical record is required before necessity can be confirmed.",
          "evidence_refs": ["email:alice@example.com"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_WRITEBACK");
}

#[tokio::test]
async fn rejects_local_or_placeholder_medical_review_evidence_refs() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/medical-review/results",
        r#"{
          "claim_id": "CLM-MEDICAL-LOCAL-EVIDENCE",
          "scoring_audit_id": "audit_scoring_local",
          "reviewer": "medical-reviewer-1",
          "decision": "request_more_evidence",
          "notes": "Medical record is required before necessity can be confirmed.",
          "evidence_refs": ["audit:scoring.completed", "medical_review:local://template/review.json"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MEDICAL_REVIEW_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/medical-review/results",
        r#"{
          "claim_id": "CLM-MEDICAL-FILE-EVIDENCE",
          "scoring_audit_id": "audit_scoring_file",
          "reviewer": "medical-reviewer-1",
          "decision": "request_more_evidence",
          "notes": "Medical record is required before necessity can be confirmed.",
          "evidence_refs": ["audit:scoring.completed", "medical_review:file://tmp/review.json"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MEDICAL_REVIEW_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/medical-review/results",
        r#"{
          "claim_id": "CLM-MEDICAL-LOOPBACK-EVIDENCE",
          "scoring_audit_id": "audit_scoring_loopback",
          "reviewer": "medical-reviewer-1",
          "decision": "request_more_evidence",
          "notes": "Medical record is required before necessity can be confirmed.",
          "evidence_refs": ["audit:scoring.completed", "medical_review:http://127.0.0.1:8080/review.json"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MEDICAL_REVIEW_EVIDENCE");

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/medical-review/results",
        r#"{
          "claim_id": "CLM-MEDICAL-TEMPLATE-EVIDENCE",
          "scoring_audit_id": "audit_scoring_template",
          "reviewer": "medical-reviewer-1",
          "decision": "request_more_evidence",
          "notes": "Medical record is required before necessity can be confirmed.",
          "evidence_refs": ["audit:scoring.completed", "medical_review:{review_id}"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MEDICAL_REVIEW_EVIDENCE");
}
