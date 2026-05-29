use api_server::{app::build_app, config::AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        source_system: "tpa-demo".into(),
        database_url: "postgres://unused".into(),
        model_service_url: "http://unused".into(),
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
    let app = build_app(test_config());

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
    assert_eq!(items[0]["review_route"], "medical_review");
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
async fn rejects_medical_review_result_without_evidence() {
    let app = build_app(test_config());

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
