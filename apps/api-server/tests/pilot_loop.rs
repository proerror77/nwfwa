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

fn scoped_config(customer_scope_id: &str) -> AppConfig {
    let mut config = test_config();
    config.customer_scope_id = customer_scope_id.into();
    config
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

async fn unauthenticated_request(method: &str, uri: &str, body: &str) -> StatusCode {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = build_app(test_config()).oneshot(request).await.unwrap();
    response.status()
}

#[tokio::test]
async fn writes_investigation_and_qa_results_then_returns_claim_audit_history() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-0287",
          "investigation_id": "INV-MISSING-EVIDENCE",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": " ",
          "evidence_refs": ["agent_run:agent_CLM-0287"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_INVESTIGATION_RESULT_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": " ",
          "investigation_id": "INV-MISSING-CLAIM",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "TPA investigation confirmed over-treatment signals.",
          "evidence_refs": ["agent_run:agent_CLM-0287"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_INVESTIGATION_RESULT_IDENTITY");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-0287",
          "investigation_id": "INV-BLANK-EVIDENCE",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "TPA investigation confirmed over-treatment signals.",
          "evidence_refs": ["agent_run:agent_CLM-0287", " "]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_INVESTIGATION_RESULT_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-0287",
          "investigation_id": "INV-BAD-IMPACT",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "financial_impact_type": "unsupported_impact",
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "TPA investigation confirmed over-treatment signals.",
          "evidence_refs": ["agent_run:agent_CLM-0287"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "UNSUPPORTED_FINANCIAL_IMPACT_TYPE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-0287",
          "investigation_id": "INV-NEGATIVE-SAVING",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "financial_impact_type": "prevented_payment",
          "saving_amount": "-1.00",
          "currency": "CNY",
          "notes": "TPA investigation confirmed over-treatment signals.",
          "evidence_refs": ["agent_run:agent_CLM-0287"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_INVESTIGATION_SAVING_AMOUNT");

    let (status, investigation) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-0287",
          "investigation_id": "INV-1001",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "TPA investigation confirmed over-treatment signals.",
          "customer_scope_id": "spoofed-customer",
          "evidence_refs": ["agent_run:agent_CLM-0287", "knowledge_cases:KC-1001"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(investigation["claim_id"], "CLM-0287");
    assert_eq!(investigation["event_type"], "investigation.result.received");
    assert!(investigation["idempotency_key"]
        .as_str()
        .unwrap()
        .starts_with("tpa-writeback:investigation.result.received:"));

    let (status, repeated_investigation) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-0287",
          "investigation_id": "INV-1001",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "TPA investigation confirmed over-treatment signals.",
          "evidence_refs": ["agent_run:agent_CLM-0287", "knowledge_cases:KC-1001"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        repeated_investigation["idempotency_key"],
        investigation["idempotency_key"]
    );

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-MISSING-EVIDENCE",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Reviewer should attach provider history evidence.",
          "evidence_refs": []
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_QA_RESULT_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": " ",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Reviewer should attach provider history evidence.",
          "evidence_refs": ["audit:investigation.result.received"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_QA_RESULT_IDENTITY");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-BLANK-EVIDENCE",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Reviewer should attach provider history evidence.",
          "evidence_refs": ["audit:investigation.result.received", " "]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_QA_RESULT_EVIDENCE");

    let (status, qa) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-9001",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Reviewer should attach provider history evidence.",
          "customer_scope_id": "spoofed-customer",
          "evidence_refs": ["audit:investigation.result.received", "rule_runs:EARLY_CLAIM"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(qa["claim_id"], "CLM-0287");
    assert_eq!(qa["event_type"], "qa.result.received");
    assert!(qa["idempotency_key"]
        .as_str()
        .unwrap()
        .starts_with("tpa-writeback:qa.result.received:"));

    let (status, repeated_qa) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-9001",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Reviewer should attach provider history evidence.",
          "evidence_refs": ["audit:investigation.result.received", "rule_runs:EARLY_CLAIM"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(repeated_qa["idempotency_key"], qa["idempotency_key"]);

    let (status, audit) = json_request(app, "GET", "/api/v1/audit/claims/CLM-0287", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(audit["claim_id"], "CLM-0287");
    let events = audit["events"].as_array().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["event_type"], "investigation.result.received");
    assert_eq!(events[1]["event_type"], "qa.result.received");
    assert_eq!(events[0]["actor_role"], "tpa_system");
    assert_eq!(events[1]["actor_role"], "tpa_system");
    assert_eq!(events[0]["payload"]["customer_scope_id"], "demo-customer");
    assert_eq!(events[1]["payload"]["customer_scope_id"], "demo-customer");
    assert_eq!(events[0]["payload"]["actor_id"], "tpa-demo");
    assert_eq!(events[1]["payload"]["actor_id"], "tpa-demo");
    assert_eq!(events[0]["payload"]["actor_role"], "tpa_system");
    assert_eq!(events[1]["payload"]["actor_role"], "tpa_system");
    assert!(events
        .iter()
        .all(|event| !event["evidence_refs"].as_array().unwrap().is_empty()));
}

#[tokio::test]
async fn investigation_result_writeback_preserves_canonical_evidence_refs_from_scoring_audit() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "canonical_claim_context": {
            "claim_header": {
              "external_claim_id": "CLM-INVESTIGATION-CANONICAL",
              "total_amount": 12000,
              "currency": "CNY",
              "service_date": "2026-01-06"
            },
            "member_policy_snapshot": {
              "masked_member_id": "masked-member-investigation",
              "masked_certificate_id": "masked-cert-investigation",
              "policy_id": "POL-INVESTIGATION-CANONICAL",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": 15000
            },
            "provider_snapshot": {
              "provider_id": "PRV-INVESTIGATION-CANONICAL",
              "name": "Investigation Trace Hospital",
              "provider_type": "hospital",
              "region": "SH",
              "risk_tier": "High"
            },
            "itemized_bill_lines": [
              {
                "item_code": "IMG-INVESTIGATION",
                "item_name": "High cost imaging",
                "fee_category": "procedure",
                "amount": 12000,
                "diagnosis_list": [{ "code": "J10", "name": "Influenza" }],
                "source_path": "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]",
                "evidence_refs": ["invoice:INV-INVESTIGATION:fee_detail:LINE-1"]
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, investigation) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-INVESTIGATION-CANONICAL",
          "investigation_id": "INV-CANONICAL-WRITEBACK",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "financial_impact_type": "prevented_payment",
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "TPA investigation confirmed over-treatment signals.",
          "evidence_refs": ["investigation_results:INV-CANONICAL-WRITEBACK"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        investigation["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "invoice:INV-INVESTIGATION:fee_detail:LINE-1"
            )),
        "investigation response should preserve canonical evidence refs"
    );

    let (status, audit) = json_request(
        app.clone(),
        "GET",
        "/api/v1/audit/claims/CLM-INVESTIGATION-CANONICAL",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let investigation_event = audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "investigation.result.received")
        .expect("investigation event should be in audit history");
    assert!(
        investigation_event["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "invoice:INV-INVESTIGATION:fee_detail:LINE-1"
            )),
        "investigation audit event should preserve canonical evidence refs"
    );
    assert!(
        investigation_event["payload"]["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "invoice:INV-INVESTIGATION:fee_detail:LINE-1"
            )),
        "investigation audit payload should preserve canonical evidence refs"
    );

    let (status, labels) = json_request(app, "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let label = labels["labels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|label| label["source_id"] == "INV-CANONICAL-WRITEBACK")
        .expect("investigation label should be listed");
    assert!(
        label["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "invoice:INV-INVESTIGATION:fee_detail:LINE-1"
            )),
        "investigation outcome label should preserve canonical evidence refs"
    );
}

#[tokio::test]
async fn rejects_pii_in_investigation_and_qa_writebacks() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-PII-WRITEBACK",
          "investigation_id": "INV-PII-NOTES",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "Reviewer copied member email alice@example.com into notes.",
          "evidence_refs": ["agent_run:agent_CLM-PII-WRITEBACK"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_WRITEBACK");

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-PII-EVIDENCE",
          "claim_id": "CLM-PII-WRITEBACK",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Reviewer should attach provider history evidence.",
          "evidence_refs": ["audit:scoring.completed", "phone:13800138000"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_WRITEBACK");
}

#[tokio::test]
async fn rejects_unsupported_qa_feedback_target() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-UNSUPPORTED-TARGET",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "unknown_target",
          "notes": "Unsupported feedback target should not enter QA governance.",
          "evidence_refs": ["audit:scoring.completed"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "UNSUPPORTED_FEEDBACK_TARGET");
}

#[tokio::test]
async fn rejects_unsupported_qa_conclusion() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-UNSUPPORTED-CONCLUSION",
          "claim_id": "CLM-0287",
          "qa_conclusion": "unknown_conclusion",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Unsupported QA conclusion should not enter QA governance.",
          "evidence_refs": ["audit:scoring.completed"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "UNSUPPORTED_QA_CONCLUSION");
}

#[tokio::test]
async fn rejects_unsupported_qa_issue_type() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-UNSUPPORTED-ISSUE",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "unknown_issue",
          "feedback_target": "rules",
          "notes": "Unsupported QA issue type should not enter QA governance.",
          "evidence_refs": ["audit:scoring.completed"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "UNSUPPORTED_QA_ISSUE_TYPE");
}

#[tokio::test]
async fn accepts_prd_issue_types_for_qa_writeback() {
    let app = build_app(test_config());

    for issue_type in [
        "confirmed_fwa",
        "false_positive",
        "improper_payment",
        "insufficient_evidence",
        "abuse_not_fraud",
        "documentation_issue",
        "medical_necessity_issue",
        "policy_exclusion",
    ] {
        let (status, body) = json_request(
            app.clone(),
            "POST",
            "/api/v1/qa/results",
            &format!(
                r#"{{
                  "qa_case_id": "QA-PRD-{issue_type}",
                  "claim_id": "CLM-0287",
                  "qa_conclusion": "issue_found_escalate",
                  "issue_type": "{issue_type}",
                  "feedback_target": "rules",
                  "notes": "PRD governed QA label should enter feedback and label governance.",
                  "evidence_refs": ["audit:scoring.completed", "qa_reviews:QA-PRD-{issue_type}"]
                }}"#
            ),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "expected PRD issue_type {issue_type} to be accepted: {body:?}"
        );
        assert_eq!(body["event_type"], "qa.result.received");
    }
}

#[tokio::test]
async fn lists_webhook_events_for_tpa_integrations() {
    let app = build_app(test_config());

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
    let app = build_app(test_config());

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

#[tokio::test]
async fn lists_qa_feedback_items_for_rule_and_model_operators() {
    let app = build_app(test_config());

    for (qa_case_id, feedback_target, issue_type) in [
        ("QA-RULE-1001", "rules", "alert_handling_incomplete"),
        (
            "QA-MODEL-1001",
            "model",
            "model_under_scored_confirmed_issue",
        ),
    ] {
        let (status, _) = json_request(
            app.clone(),
            "POST",
            "/api/v1/qa/results",
            &format!(
                r#"{{
                  "qa_case_id": "{qa_case_id}",
                  "claim_id": "CLM-0287",
                  "qa_conclusion": "issue_found_escalate",
                  "issue_type": "{issue_type}",
                  "feedback_target": "{feedback_target}",
                  "notes": "Reviewer notes stay in the source QA review, not the feedback queue summary.",
                  "evidence_refs": ["audit:scoring.completed", "rule_runs:EARLY_CLAIM"]
                }}"#,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    let (status, feedback) =
        json_request(app.clone(), "GET", "/api/v1/ops/qa/feedback-items", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let items = feedback["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["feedback_id"], "qa_feedback_QA-RULE-1001");
    assert_eq!(items[0]["feedback_target"], "rules");
    assert_eq!(items[0]["status"], "open");
    assert_eq!(items[0]["source"], "qa_review");
    assert_eq!(items[0]["note_present"], true);
    assert_eq!(items[0]["status_updated_by"], serde_json::Value::Null);
    assert_eq!(items[0]["status_audit_id"], serde_json::Value::Null);
    assert!(items[0]["status_evidence_refs"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(items[0]["summary"]
        .as_str()
        .unwrap()
        .contains("QA-RULE-1001"));
    assert!(items[0].get("notes").is_none());
    assert_eq!(items[1]["feedback_target"], "model");
    assert!(items
        .iter()
        .all(|item| !item["evidence_refs"].as_array().unwrap().is_empty()));

    let (status, feedback) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/qa/feedback-items?feedback_target=rules&status=open",
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let items = feedback["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["feedback_id"], "qa_feedback_QA-RULE-1001");

    let (status, body) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/qa/feedback-items?status=unknown",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "UNSUPPORTED_QA_FEEDBACK_STATUS");

    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/ops/qa/feedback-items?feedback_target=unknown",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "UNSUPPORTED_FEEDBACK_TARGET");
}

#[tokio::test]
async fn accepts_prd_model_feedback_target_and_canonicalizes_legacy_alias() {
    let app = build_app(test_config());

    for (qa_case_id, feedback_target) in [
        ("QA-MODEL-PRD-1001", "model"),
        ("QA-MODEL-LEGACY-1001", "models"),
    ] {
        let (status, body) = json_request(
            app.clone(),
            "POST",
            "/api/v1/qa/results",
            &format!(
                r#"{{
                  "qa_case_id": "{qa_case_id}",
                  "claim_id": "CLM-MODEL-FEEDBACK",
                  "qa_conclusion": "issue_found_escalate",
                  "issue_type": "model_under_scored_confirmed_issue",
                  "feedback_target": "{feedback_target}",
                  "notes": "QA feedback is directed to model operations.",
                  "evidence_refs": ["qa_reviews:{qa_case_id}", "model_versions:baseline_fwa:0.1.0"]
                }}"#
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    let (status, feedback) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/qa/feedback-items?feedback_target=model",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = feedback["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|item| item["feedback_target"] == "model"));

    let (status, feedback) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/qa/feedback-items?feedback_target=models",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(feedback["items"].as_array().unwrap().len(), 2);

    let (status, labels) = json_request(app, "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let labels = labels["labels"].as_array().unwrap();
    assert!(labels.iter().any(|label| {
        label["source_id"] == "QA-MODEL-PRD-1001"
            && label["label_name"] == "model_under_scored_confirmed_issue"
            && label["feedback_target"] == "model"
    }));
    assert!(labels.iter().any(|label| {
        label["source_id"] == "QA-MODEL-LEGACY-1001"
            && label["label_name"] == "model_under_scored_confirmed_issue"
            && label["feedback_target"] == "model"
    }));
}

#[tokio::test]
async fn updates_qa_feedback_item_status_with_audit_trail() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-FEEDBACK-STATUS-1",
          "claim_id": "CLM-FEEDBACK-STATUS-1",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Rule feedback should be worked by rule ops.",
          "evidence_refs": ["audit:scoring.completed", "rule_runs:EARLY_CLAIM"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-FEEDBACK-STATUS-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "rule-ops",
          "notes": "Missing evidence should be rejected.",
          "evidence_refs": []
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_QA_FEEDBACK_STATUS_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-FEEDBACK-STATUS-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "rule-ops",
          "notes": "Blank evidence reference should be rejected.",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-FEEDBACK-STATUS-1", " "]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_QA_FEEDBACK_STATUS_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-FEEDBACK-STATUS-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "rule-ops",
          "notes": " ",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-FEEDBACK-STATUS-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_QA_FEEDBACK_STATUS_NOTES");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-FEEDBACK-STATUS-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "rule-ops",
          "notes": "Reviewer contacted alice@example.com about the feedback.",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-FEEDBACK-STATUS-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_QA_FEEDBACK_STATUS");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-FEEDBACK-STATUS-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "rule-ops",
          "notes": "Rule threshold reviewed and accepted.",
          "evidence_refs": ["phone:13800138000"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_QA_FEEDBACK_STATUS");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-FEEDBACK-STATUS-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "rule-ops",
          "notes": "Rule threshold reviewed and accepted.",
          "evidence_refs": ["rule_runs:EARLY_CLAIM"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_QA_FEEDBACK_TARGET_EVIDENCE");

    let (status, update) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-FEEDBACK-STATUS-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "rule-ops",
          "notes": "Rule threshold reviewed and accepted.",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-FEEDBACK-STATUS-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(update["item"]["status"], "resolved");
    assert_eq!(
        update["item"]["feedback_id"],
        "qa_feedback_QA-FEEDBACK-STATUS-1"
    );
    assert!(!update["audit_id"].as_str().unwrap().is_empty());
    assert_eq!(update["item"]["status_updated_by"], "rule-ops");
    assert_eq!(update["item"]["status_audit_id"], update["audit_id"]);
    assert_eq!(
        update["item"]["status_evidence_refs"],
        serde_json::json!(["qa_feedback:qa_feedback_QA-FEEDBACK-STATUS-1"])
    );

    let (status, feedback) =
        json_request(app.clone(), "GET", "/api/v1/ops/qa/feedback-items", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(feedback["items"][0]["status"], "resolved");
    assert_eq!(feedback["items"][0]["status_updated_by"], "rule-ops");
    assert_eq!(feedback["items"][0]["status_audit_id"], update["audit_id"]);
    assert_eq!(
        feedback["items"][0]["status_evidence_refs"],
        serde_json::json!(["qa_feedback:qa_feedback_QA-FEEDBACK-STATUS-1"])
    );

    let (status, audit) = json_request(
        app,
        "GET",
        "/api/v1/audit/claims/CLM-FEEDBACK-STATUS-1",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let status_event = audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| {
            event["event_type"] == "qa.feedback.status.updated"
                && event["payload"]["to_status"] == "resolved"
        })
        .expect("feedback status update should be audited");
    assert_eq!(
        status_event["payload"]["customer_scope_id"],
        "demo-customer"
    );
}

#[tokio::test]
async fn summarizes_qa_feedback_queue_for_review_operations() {
    let app = build_app(test_config());

    for (qa_case_id, feedback_target, issue_type, qa_conclusion) in [
        (
            "QA-QUEUE-RULE-1001",
            "rules",
            "alert_handling_incomplete",
            "issue_found_escalate",
        ),
        (
            "QA-QUEUE-MODEL-1001",
            "model",
            "model_under_scored_confirmed_issue",
            "issue_found_return",
        ),
        (
            "QA-QUEUE-TPA-1001",
            "tpa",
            "workflow_missing_evidence",
            "issue_found_escalate",
        ),
        (
            "QA-QUEUE-FEATURES-1001",
            "features",
            "medical_reasonableness",
            "issue_found_return",
        ),
        (
            "QA-QUEUE-PROVIDER-1001",
            "provider_profile",
            "provider_pattern",
            "issue_found_escalate",
        ),
        (
            "QA-QUEUE-WORKFLOW-1001",
            "workflow",
            "qa_review_completed",
            "issue_found_return",
        ),
    ] {
        let (status, _) = json_request(
            app.clone(),
            "POST",
            "/api/v1/qa/results",
            &format!(
                r#"{{
                  "qa_case_id": "{qa_case_id}",
                  "claim_id": "CLM-QA-QUEUE",
                  "qa_conclusion": "{qa_conclusion}",
                  "issue_type": "{issue_type}",
                  "feedback_target": "{feedback_target}",
                  "notes": "QA feedback needs operational follow-up.",
                  "evidence_refs": ["audit:scoring.completed", "qa_reviews:{qa_case_id}"]
                }}"#,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    for (feedback_id, status) in [
        ("qa_feedback_QA-QUEUE-MODEL-1001", "in_progress"),
        ("qa_feedback_QA-QUEUE-TPA-1001", "resolved"),
        ("qa_feedback_QA-QUEUE-WORKFLOW-1001", "dismissed"),
    ] {
        let (status_code, _) = json_request(
            app.clone(),
            "POST",
            &format!("/api/v1/ops/qa/feedback-items/{feedback_id}/status"),
            &format!(
                r#"{{
                  "status": "{status}",
                  "actor_id": "qa-lead",
                  "notes": "Update QA feedback status for queue distribution.",
                  "evidence_refs": ["qa_feedback:{feedback_id}"]
                }}"#,
            ),
        )
        .await;
        assert_eq!(status_code, StatusCode::OK);
    }

    let (status, summary) = json_request(app, "GET", "/api/v1/ops/qa/queue-summary", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary["open_count"], 3);
    assert_eq!(summary["in_progress_count"], 1);
    assert_eq!(summary["resolved_count"], 1);
    assert_eq!(summary["dismissed_count"], 1);
    assert_eq!(summary["unresolved_count"], 4);
    assert_eq!(summary["rules_feedback_count"], 1);
    assert_eq!(summary["models_feedback_count"], 0);
    assert_eq!(summary["features_feedback_count"], 1);
    assert_eq!(summary["provider_profile_feedback_count"], 1);
    assert_eq!(summary["workflow_feedback_count"], 0);
    assert_eq!(summary["tpa_feedback_count"], 0);
    assert_eq!(summary["high_priority_count"], 2);
    assert_eq!(summary["evidence_backed_count"], 3);
    assert_eq!(summary["highest_priority"], "high");
}

#[tokio::test]
async fn lists_qa_queue_items_from_audit_samples() {
    let app = build_app(test_config());

    let (status, score) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-QA-QUEUE-ITEM",
            "claim_amount": "9300.00",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "items": [
            {
              "item_code": "IMG-QA-1",
              "item_type": "procedure",
              "description": "Imaging",
              "quantity": 1,
              "unit_amount": "9300.00",
              "total_amount": "9300.00"
            }
          ],
          "member": { "external_member_id": "MBR-QA-QUEUE-ITEM" },
          "policy": {
            "external_policy_id": "POL-QA-QUEUE-ITEM",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000.00",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-QA-QUEUE-ITEM",
            "name": "QA Queue Hospital",
            "provider_type": "hospital",
            "region": "Shanghai",
            "risk_tier": "High"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, sample) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/audit-samples",
        r#"{
          "sample_mode": "qa_calibration",
          "population_definition": "High risk claims for QA queue",
          "inclusion_criteria": { "min_risk_score": 70 },
          "deterministic_seed": "qa-week-1",
          "sample_size": 1,
          "reviewer": "qa-reviewer-1",
          "assignment_queue": "QA Review"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, queue) = json_request(app.clone(), "GET", "/api/v1/ops/qa/queue", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let items = queue["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["claim_id"], "CLM-QA-QUEUE-ITEM");
    assert_eq!(items[0]["sample_id"], sample["sample_id"]);
    assert_eq!(items[0]["risk_score"], score["risk_score"]);
    assert_eq!(items[0]["assignment_queue"], "QA Review");
    assert_eq!(items[0]["reviewer"], "qa-reviewer-1");
    assert_eq!(items[0]["status"], "open");
    assert!(items[0]["qa_case_id"]
        .as_str()
        .unwrap()
        .starts_with("qa_sample_"));
    assert!(!items[0]["evidence_refs"].as_array().unwrap().is_empty());

    let qa_case_id = items[0]["qa_case_id"].as_str().unwrap();
    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        &format!(
            r#"{{
              "qa_case_id": "{qa_case_id}",
              "claim_id": "CLM-QA-QUEUE-ITEM",
              "qa_conclusion": "pass",
              "issue_type": "qa_review_completed",
              "feedback_target": "workflow",
              "notes": "Reviewer completed sampled QA case.",
              "evidence_refs": ["qa_queue:{qa_case_id}", "audit:scoring.completed"]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, queue) = json_request(app, "GET", "/api/v1/ops/qa/queue", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let items = queue["items"].as_array().unwrap();
    assert_eq!(items[0]["qa_case_id"], qa_case_id);
    assert_eq!(items[0]["status"], "reviewed");
    assert_eq!(items[0]["qa_conclusion"], "pass");
    assert_eq!(items[0]["issue_type"], "qa_review_completed");
}

#[tokio::test]
async fn qa_queue_items_include_canonical_trace_from_prior_scoring_audit() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "canonical_claim_context": {
            "claim_header": {
              "external_claim_id": "CLM-QA-CANONICAL",
              "total_amount": 9300,
              "currency": "CNY",
              "service_date": "2026-01-06"
            },
            "member_policy_snapshot": {
              "masked_member_id": "masked-member-qa",
              "masked_certificate_id": "masked-cert-qa",
              "member_birth_date": "1988-03-12",
              "member_gender": "F",
              "policy_id": "POL-QA-CANONICAL",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": 10000
            },
            "provider_snapshot": {
              "provider_id": "PRV-QA-CANONICAL",
              "name": "QA Trace Hospital",
              "provider_type": "hospital",
              "region": "SH",
              "risk_tier": "High"
            },
            "itemized_bill_lines": [
              {
                "item_name": "High cost imaging",
                "fee_category": "procedure",
                "amount": 9300,
                "diagnosis_list": [{ "code": "J10", "name": "Influenza" }],
                "source_path": "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]",
                "evidence_refs": ["invoice:INV-QA:fee_detail:LINE-1"]
              }
            ],
            "document_evidence": [
              {
                "document_id": "MR-QA-1",
                "medical_record_type": "outpatient_record",
                "source_refs": ["medical_record:MR-QA-1"]
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/audit-samples",
        r#"{
          "sample_mode": "qa_calibration",
          "population_definition": "Canonical high risk claims for QA queue",
          "inclusion_criteria": { "min_risk_score": 70 },
          "deterministic_seed": "qa-canonical-trace",
          "sample_size": 1,
          "reviewer": "qa-reviewer-1",
          "assignment_queue": "QA Review"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, queue) = json_request(app, "GET", "/api/v1/ops/qa/queue", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let item = queue["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["claim_id"] == "CLM-QA-CANONICAL")
        .expect("canonical scored claim should enter QA queue");
    assert!(
        item["canonical_source_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]"
            )),
        "QA queue should expose normalized bill-line source path"
    );
    assert!(
        item["canonical_source_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("medical_record:MR-QA-1")),
        "QA queue should expose normalized document source ref"
    );
    assert!(
        item["canonical_evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("invoice:INV-QA:fee_detail:LINE-1")),
        "QA queue should expose canonical evidence refs for QA writeback"
    );
}

#[tokio::test]
async fn qa_result_writeback_preserves_canonical_evidence_refs_from_scoring_audit() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "canonical_claim_context": {
            "claim_header": {
              "external_claim_id": "CLM-QA-WRITEBACK-CANONICAL",
              "total_amount": 9300,
              "currency": "CNY",
              "service_date": "2026-01-06"
            },
            "member_policy_snapshot": {
              "masked_member_id": "masked-member-qa-writeback",
              "masked_certificate_id": "masked-cert-qa-writeback",
              "policy_id": "POL-QA-WRITEBACK-CANONICAL",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": 10000
            },
            "provider_snapshot": {
              "provider_id": "PRV-QA-WRITEBACK-CANONICAL",
              "name": "QA Trace Hospital",
              "provider_type": "hospital",
              "region": "SH",
              "risk_tier": "High"
            },
            "itemized_bill_lines": [
              {
                "item_name": "High cost imaging",
                "fee_category": "procedure",
                "amount": 9300,
                "diagnosis_list": [{ "code": "J10", "name": "Influenza" }],
                "source_path": "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]",
                "evidence_refs": ["invoice:INV-QA-WRITEBACK:fee_detail:LINE-1"]
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, qa) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-WRITEBACK-CANONICAL",
          "claim_id": "CLM-QA-WRITEBACK-CANONICAL",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "workflow_missing_evidence",
          "feedback_target": "workflow",
          "notes": "Reviewer found incomplete evidence handling.",
          "evidence_refs": ["qa_reviews:QA-WRITEBACK-CANONICAL"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        qa["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "invoice:INV-QA-WRITEBACK:fee_detail:LINE-1"
            )),
        "QA writeback response should preserve canonical evidence refs"
    );

    let (status, audit) = json_request(
        app,
        "GET",
        "/api/v1/audit/claims/CLM-QA-WRITEBACK-CANONICAL",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let qa_event = audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "qa.result.received")
        .expect("QA result should be in audit history");
    assert!(
        qa_event["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "invoice:INV-QA-WRITEBACK:fee_detail:LINE-1"
            )),
        "QA audit event should preserve canonical evidence refs"
    );
    assert!(
        qa_event["payload"]["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "invoice:INV-QA-WRITEBACK:fee_detail:LINE-1"
            )),
        "QA audit payload should preserve canonical evidence refs"
    );
}

#[tokio::test]
async fn lists_governed_outcome_labels_from_investigation_and_qa() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-LABEL-1001",
          "investigation_id": "INV-LABEL-1001",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "Confirmed over-treatment after manual investigation.",
          "evidence_refs": ["investigation_results:INV-LABEL-1001", "knowledge_cases:KC-1001"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-LABEL-1001",
          "claim_id": "CLM-LABEL-1001",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "medical_necessity_issue",
          "feedback_target": "model",
          "notes": "QA found missing clinical support and model under-scored the claim.",
          "evidence_refs": ["qa_reviews:QA-LABEL-1001", "model_scores:baseline_fwa"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-LABEL-1002",
          "investigation_id": "INV-LABEL-1002",
          "outcome": "recovery_confirmed",
          "confirmed_fwa": true,
          "financial_impact_type": "recovered_amount",
          "saving_amount": "1200.00",
          "currency": "CNY",
          "notes": "Post-payment recovery confirmed.",
          "evidence_refs": ["investigation_results:INV-LABEL-1002"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/medical-review/results",
        r#"{
          "claim_id": "CLM-LABEL-1001",
          "scoring_audit_id": "audit_scoring_label_1001",
          "reviewer": "medical-reviewer-1",
          "decision": "medical_necessity_issue",
          "notes": "Medical reviewer confirmed the billed service lacks clinical necessity support.",
          "evidence_refs": ["audit:audit_scoring_label_1001", "medical_review:MR-LABEL-1001"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, labels) = json_request(app.clone(), "GET", "/api/v1/ops/labels", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let labels = labels["labels"].as_array().unwrap();
    assert!(labels.iter().any(|label| {
        label["claim_id"] == "CLM-LABEL-1001"
            && label["label_name"] == "confirmed_fwa"
            && label["label_value"] == "true"
            && label["source_type"] == "investigation_result"
            && label["governance_status"] == "approved_for_training"
    }));
    assert!(labels.iter().any(|label| {
        label["claim_id"] == "CLM-LABEL-1001"
            && label["label_name"] == "amount_prevented"
            && label["label_value"] == "8200.00"
            && label["currency"] == "CNY"
    }));
    assert!(labels.iter().any(|label| {
        label["claim_id"] == "CLM-LABEL-1002"
            && label["label_name"] == "amount_recovered"
            && label["label_value"] == "1200.00"
            && label["currency"] == "CNY"
    }));
    assert!(labels.iter().any(|label| {
        label["claim_id"] == "CLM-LABEL-1001"
            && label["label_name"] == "medical_necessity_issue"
            && label["label_value"] == "true"
            && label["source_type"] == "qa_review"
            && label["feedback_target"] == "model"
            && label["governance_status"] == "needs_review"
    }));
    assert!(labels.iter().any(|label| {
        label["claim_id"] == "CLM-LABEL-1001"
            && label["label_name"] == "medical_necessity_issue"
            && label["label_value"] == "true"
            && label["source_type"] == "medical_review"
            && label["feedback_target"] == "model"
            && label["governance_status"] == "approved_for_training"
            && label["source_id"].as_str().unwrap().starts_with("aud_")
    }));
    assert!(labels
        .iter()
        .all(|label| !label["evidence_refs"].as_array().unwrap().is_empty()));

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-LABEL-1001/status",
        r#"{
          "status": "resolved",
          "actor_id": "model-ops",
          "notes": "Model operator approved the QA feedback label for training.",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-LABEL-1001"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, labels) = json_request(app, "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert!(labels["labels"].as_array().unwrap().iter().any(|label| {
        label["claim_id"] == "CLM-LABEL-1001"
            && label["label_name"] == "medical_necessity_issue"
            && label["source_type"] == "qa_review"
            && label["feedback_target"] == "model"
            && label["governance_status"] == "approved_for_training"
    }));
}

#[tokio::test]
async fn lists_governed_outcome_labels_from_terminal_case_status() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-CASE-LABEL-1",
            "claim_amount": "9000",
            "currency": "CNY",
            "service_date": "2026-01-05",
            "diagnosis_code": "J10",
            "member": {
              "external_member_id": "MBR-CASE-LABEL-1"
            },
            "policy": {
              "external_policy_id": "POL-CASE-LABEL-1",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": "10000",
              "currency": "CNY"
            },
            "provider": {
              "external_provider_id": "PRV-CASE-LABEL-1",
              "name": "Northwind Hospital",
              "provider_type": "hospital",
              "region": "SH",
              "risk_tier": "High"
            },
            "items": [
              {
                "item_code": "PROC-001",
                "item_type": "procedure",
                "description": "Imaging",
                "quantity": 1,
                "unit_amount": "9000",
                "total_amount": "9000",
                "currency": "CNY"
              }
            ]
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
        .find(|lead| lead["claim_id"] == "CLM-CASE-LABEL-1")
        .unwrap()["lead_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, triage) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "open_case",
          "assignee": "siu-case-label-owner",
          "reviewer": "medical-case-label-owner",
          "priority": "high",
          "notes": "Open case for terminal status label generation.",
          "evidence_refs": ["triage_decisions:case_label_generation"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let case_id = triage["case"]["case_id"].as_str().unwrap();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "confirmed",
          "actor_id": "siu-case-label-owner",
          "notes": "Case reviewer confirmed FWA.",
          "evidence_refs": ["case_workflow:confirmed"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, labels) = json_request(app.clone(), "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert!(labels["labels"].as_array().unwrap().iter().any(|label| {
        label["claim_id"] == "CLM-CASE-LABEL-1"
            && label["label_name"] == "confirmed_fwa"
            && label["label_value"] == "true"
            && label["source_type"] == "case_status"
            && label["source_id"] == case_id
            && label["governance_status"] == "approved_for_training"
            && label["feedback_target"] == "model"
            && label["evidence_refs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|reference| {
                    reference == &serde_json::json!(format!("investigation_cases:{case_id}"))
                })
    }));

    let (status, _) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "rejected",
          "actor_id": "siu-case-label-owner",
          "notes": "Case reviewer rejected the lead after investigation.",
          "evidence_refs": ["case_workflow:rejected"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, labels) = json_request(app, "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let labels = labels["labels"].as_array().unwrap();
    assert!(labels.iter().any(|label| {
        label["claim_id"] == "CLM-CASE-LABEL-1"
            && label["label_name"] == "confirmed_fwa"
            && label["label_value"] == "false"
            && label["source_type"] == "case_status"
            && label["source_id"] == case_id
            && label["governance_status"] == "needs_review"
            && label["feedback_target"] == "model"
    }));
    assert!(labels.iter().any(|label| {
        label["claim_id"] == "CLM-CASE-LABEL-1"
            && label["label_name"] == "false_positive"
            && label["label_value"] == "true"
            && label["source_type"] == "case_status"
            && label["source_id"] == case_id
            && label["governance_status"] == "needs_review"
            && label["feedback_target"] == "rules"
    }));
}

#[tokio::test]
async fn returns_member_profile_summary_from_scored_claims() {
    let app = build_app(test_config());

    for (claim_id, policy_id, amount, limit) in [
        ("CLM-MEMBER-1001", "POL-MEMBER-1001", "9200.00", "10000.00"),
        ("CLM-MEMBER-1002", "POL-MEMBER-1002", "1800.00", "12000.00"),
    ] {
        let (status, _) = json_request(
            app.clone(),
            "POST",
            "/api/v1/claims/score",
            &format!(
                r#"{{
                  "source_system": "tpa-demo",
                  "claim": {{
                    "external_claim_id": "{claim_id}",
                    "claim_amount": "{amount}",
                    "currency": "CNY",
                    "service_date": "2026-02-05",
                    "diagnosis_code": "J10",
                    "member": {{
                      "external_member_id": "MBR-PROFILE-1"
                    }},
                    "policy": {{
                      "external_policy_id": "{policy_id}",
                      "product_code": "MED",
                      "coverage_start_date": "2026-01-01",
                      "coverage_end_date": "2026-12-31",
                      "coverage_limit": "{limit}",
                      "currency": "CNY"
                    }},
                    "provider": {{
                      "external_provider_id": "PRV-PROFILE-1",
                      "name": "Profile Hospital",
                      "provider_type": "hospital",
                      "region": "Shanghai",
                      "risk_tier": "Medium"
                    }},
                    "items": [
                      {{
                        "item_code": "PROC-1",
                        "item_type": "procedure",
                        "description": "Procedure",
                        "quantity": 1,
                        "unit_amount": "{amount}",
                        "total_amount": "{amount}",
                        "currency": "CNY"
                      }}
                    ]
                  }}
                }}"#
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    let (status, profile) = json_request(
        app,
        "GET",
        "/api/v1/members/MBR-PROFILE-1/profile-summary",
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(profile["member_id"], "MBR-PROFILE-1");
    assert_eq!(profile["claim_count"], 2);
    assert_eq!(profile["policy_count"], 2);
    assert_eq!(profile["currency"], "CNY");
    assert_eq!(profile["total_claim_amount"], "11000.00");
    assert_eq!(profile["latest_claim_id"], "CLM-MEMBER-1002");
    assert!(profile["high_risk_claim_count"].as_u64().unwrap() >= 1);
    assert!(profile["profile_summary"]
        .as_str()
        .unwrap()
        .contains("2 笔历史理赔"));
    assert!(profile["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("members:MBR-PROFILE-1")));
}

#[tokio::test]
async fn member_profile_summary_is_scoped_to_authenticated_customer() {
    let repository = InMemoryScoringRepository::shared();
    let alpha_app = build_app_with_parts(
        scoped_config("customer-alpha"),
        Arc::new(HeuristicModelScorer),
        repository.clone(),
    );
    let beta_app = build_app_with_parts(
        scoped_config("customer-beta"),
        Arc::new(HeuristicModelScorer),
        repository,
    );

    let (status, _) = json_request(
        alpha_app,
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-MEMBER-SCOPE-1",
            "claim_amount": "9200.00",
            "currency": "CNY",
            "service_date": "2026-02-05",
            "diagnosis_code": "J10",
            "member": {
              "external_member_id": "MBR-SCOPE-PROFILE-1"
            },
            "policy": {
              "external_policy_id": "POL-MEMBER-SCOPE-1",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": "10000.00",
              "currency": "CNY"
            },
            "provider": {
              "external_provider_id": "PRV-MEMBER-SCOPE-1",
              "name": "Profile Hospital",
              "provider_type": "hospital",
              "region": "Shanghai",
              "risk_tier": "Medium"
            },
            "items": [
              {
                "item_code": "PROC-1",
                "item_type": "procedure",
                "description": "Procedure",
                "quantity": 1,
                "unit_amount": "9200.00",
                "total_amount": "9200.00",
                "currency": "CNY"
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/members/MBR-SCOPE-PROFILE-1/profile-summary")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let response = beta_app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn pilot_loop_endpoints_require_api_key() {
    for (method, uri, body) in [
        (
            "POST",
            "/api/v1/investigations/results",
            r#"{
              "claim_id": "CLM-0287",
              "investigation_id": "INV-1001",
              "outcome": "confirmed_fwa",
              "confirmed_fwa": true,
              "saving_amount": "8200.00",
              "currency": "CNY",
              "notes": "missing key",
              "evidence_refs": ["agent_run:agent_CLM-0287"]
            }"#,
        ),
        (
            "POST",
            "/api/v1/qa/results",
            r#"{
              "qa_case_id": "QA-9001",
              "claim_id": "CLM-0287",
              "qa_conclusion": "issue_found_escalate",
              "issue_type": "alert_handling_incomplete",
              "feedback_target": "rules",
              "notes": "missing key",
              "evidence_refs": ["rule_runs:EARLY_CLAIM"]
            }"#,
        ),
        ("GET", "/api/v1/audit/claims/CLM-0287", "{}"),
        ("GET", "/api/v1/members/MBR-PROFILE-1/profile-summary", "{}"),
        ("GET", "/api/v1/ops/webhook-events", "{}"),
        (
            "POST",
            "/api/v1/ops/webhook-events/webhook_audit_1/delivery-attempts",
            r#"{"delivery_status":"failed"}"#,
        ),
    ] {
        let status = unauthenticated_request(method, uri, body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
