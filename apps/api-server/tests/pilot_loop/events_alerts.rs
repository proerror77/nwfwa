use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{json_request, test_config};

#[tokio::test]
async fn lists_webhook_events_for_tpa_integrations() {
    let app = build_app(test_config()).unwrap();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-WEBHOOK-1",
            "claim_amount": "9000",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "items": [
            {
              "item_code": "PROC-001",
              "item_type": "procedure",
              "description": "Imaging",
              "quantity": 1,
              "unit_amount": "9000",
              "total_amount": "9000"
            }
          ],
          "member": {
            "external_member_id": "MBR-WEBHOOK-1"
          },
          "policy": {
            "external_policy_id": "POL-WEBHOOK-1",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-WEBHOOK-1",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "High"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, leads) = json_request(app.clone(), "GET", "/api/v1/ops/leads", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let lead_id = leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lead| lead["claim_id"] == "CLM-WEBHOOK-1")
        .unwrap()["lead_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "open_case",
          "assignee": "siu-reviewer-1",
          "reviewer": "medical-reviewer-1",
          "priority": "high",
          "notes": "Open case for webhook contract coverage.",
          "evidence_refs": ["triage_decisions:webhook_contract"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-WEBHOOK-1",
          "investigation_id": "INV-WEBHOOK-1",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "1200.00",
          "currency": "CNY",
          "notes": "Investigation closed with confirmed FWA outcome.",
          "evidence_refs": ["investigation_results:INV-WEBHOOK-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-WEBHOOK-1",
          "claim_id": "CLM-WEBHOOK-1",
          "qa_conclusion": "pass",
          "issue_type": "none",
          "feedback_target": "workflow",
          "notes": "QA review completed.",
          "evidence_refs": ["qa_reviews:QA-WEBHOOK-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, webhooks) =
        json_request(app.clone(), "GET", "/api/v1/ops/webhook-events", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let events = webhooks["events"].as_array().unwrap();
    for expected in [
        "fwa.score.completed",
        "fwa.case.routed",
        "fwa.investigation.closed",
        "fwa.qa.reviewed",
    ] {
        assert!(
            events.iter().any(|event| {
                event["event_type"] == expected
                    && event["claim_id"] == "CLM-WEBHOOK-1"
                    && event["customer_scope_id"] == "demo-customer"
                    && event["delivery_status"] == "pending"
                    && event["retry_count"] == 0
                    && event["max_attempts"] == 3
                    && event["signature_algorithm"] == "hmac-sha256"
                    && event["idempotency_key"]
                        .as_str()
                        .unwrap()
                        .starts_with("fwa-webhook:")
                    && !event["source_audit_id"].as_str().unwrap().is_empty()
            }),
            "missing webhook event {expected}"
        );
    }
    let score_event_id = events
        .iter()
        .find(|event| event["event_type"] == "fwa.score.completed")
        .unwrap()["event_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/webhook-events/{score_event_id}/delivery-attempts"),
        r#"{
          "delivery_status": "failed",
          "response_status_code": 503,
          "error_message": "TPA webhook failed for alice@example.com"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_WEBHOOK_DELIVERY");

    let (status, attempt) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/webhook-events/{score_event_id}/delivery-attempts"),
        r#"{
          "delivery_status": "failed",
          "response_status_code": 503,
          "error_message": "TPA webhook endpoint unavailable"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(attempt["event_id"], score_event_id);
    assert_eq!(attempt["attempt_number"], 1);
    assert_eq!(attempt["delivery_status"], "failed");
    assert!(attempt["next_attempt_at"].is_string());

    let (status, webhooks) = json_request(app, "GET", "/api/v1/ops/webhook-events", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let score_event = webhooks["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_id"] == score_event_id)
        .unwrap();
    assert_eq!(score_event["delivery_status"], "retry_wait");
    assert_eq!(score_event["retry_count"], 1);
    assert_eq!(score_event["last_response_status_code"], 503);
    assert_eq!(
        score_event["last_error_message"],
        "TPA webhook endpoint unavailable"
    );
    assert!(score_event["next_attempt_at"].is_string());
}

#[tokio::test]
async fn lists_ops_alerts_for_high_risk_routing() {
    let app = build_app(test_config()).unwrap();

    let (status, score) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-ALERT-1",
            "claim_amount": "9500",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "items": [
            {
              "item_code": "PROC-ALERT-1",
              "item_type": "procedure",
              "description": "High-cost imaging",
              "quantity": 1,
              "unit_amount": "9500",
              "total_amount": "9500"
            }
          ],
          "member": { "external_member_id": "MBR-ALERT-1" },
          "policy": {
            "external_policy_id": "POL-ALERT-1",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-ALERT-1",
            "name": "Alert Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "High"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(score["risk_score"].as_u64().unwrap() >= 70);

    let (status, alerts) = json_request(app.clone(), "GET", "/api/v1/ops/alerts", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let medical_alert = alerts["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|alert| {
            alert["claim_id"] == "CLM-ALERT-1" && alert["alert_type"] == "medical_review_required"
        })
        .expect("clinical evidence gap should create a medical review alert");
    assert_eq!(medical_alert["status"], "open");
    assert_eq!(medical_alert["case_id"], serde_json::Value::Null);
    assert!(medical_alert["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            == &serde_json::json!(format!("audit:{}", score["audit_id"].as_str().unwrap()))));

    let alert = alerts["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|alert| alert["claim_id"] == "CLM-ALERT-1")
        .expect("high-risk lead should create a routing alert");
    assert_eq!(alert["alert_type"], "high_risk_routing");
    assert_eq!(alert["status"], "open");
    assert!(matches!(
        alert["severity"].as_str().unwrap(),
        "critical" | "high"
    ));
    assert!(alert["case_id"].is_null());
    assert!(alert["lead_id"].as_str().unwrap().starts_with("lead_"));
    assert!(!alert["recommended_action"].as_str().unwrap().is_empty());
    assert!(!alert["evidence_refs"].as_array().unwrap().is_empty());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/medical-review/results",
        &format!(
            r#"{{
          "claim_id": "CLM-ALERT-1",
          "scoring_audit_id": "{}",
          "reviewer": "medical-alert-owner",
          "decision": "request_more_evidence",
          "notes": "Medical review alert accepted and evidence request recorded.",
          "evidence_refs": ["audit:{}", "claim_items:PROC-ALERT-1"]
        }}"#,
            score["audit_id"].as_str().unwrap(),
            score["audit_id"].as_str().unwrap()
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let lead_id = alert["lead_id"].as_str().unwrap();
    let (status, _) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "open_case",
          "assignee": "siu-alert-owner",
          "reviewer": "medical-alert-owner",
          "priority": "high",
          "notes": "Alert accepted into investigation workflow.",
          "evidence_refs": ["triage_decisions:alert_acceptance"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, alerts) = json_request(app, "GET", "/api/v1/ops/alerts", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!alerts["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|alert| alert["claim_id"] == "CLM-ALERT-1"
            && matches!(
                alert["alert_type"].as_str().unwrap(),
                "high_risk_routing" | "medical_review_required"
            )));
}
