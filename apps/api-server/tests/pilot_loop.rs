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
          "evidence_refs": ["agent_run:agent_CLM-0287", "knowledge_cases:KC-1001"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(investigation["claim_id"], "CLM-0287");
    assert_eq!(investigation["event_type"], "investigation.result.received");

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
          "evidence_refs": ["audit:investigation.result.received", "rule_runs:EARLY_CLAIM"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(qa["claim_id"], "CLM-0287");
    assert_eq!(qa["event_type"], "qa.result.received");

    let (status, audit) = json_request(app, "GET", "/api/v1/audit/claims/CLM-0287", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(audit["claim_id"], "CLM-0287");
    let events = audit["events"].as_array().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["event_type"], "investigation.result.received");
    assert_eq!(events[1]["event_type"], "qa.result.received");
    assert!(events
        .iter()
        .all(|event| !event["evidence_refs"].as_array().unwrap().is_empty()));
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
          "notes": "Open case for webhook contract coverage."
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
          "notes": "Alert accepted into investigation workflow."
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
            && alert["alert_type"] == "high_risk_routing"));
}

#[tokio::test]
async fn lists_qa_feedback_items_for_rule_and_model_operators() {
    let app = build_app(test_config());

    for (qa_case_id, feedback_target, issue_type) in [
        ("QA-RULE-1001", "rules", "alert_handling_incomplete"),
        (
            "QA-MODEL-1001",
            "models",
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

    let (status, feedback) = json_request(app, "GET", "/api/v1/ops/qa/feedback-items", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let items = feedback["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["feedback_id"], "qa_feedback_QA-RULE-1001");
    assert_eq!(items[0]["feedback_target"], "rules");
    assert_eq!(items[0]["status"], "open");
    assert_eq!(items[0]["source"], "qa_review");
    assert_eq!(items[0]["note_present"], true);
    assert!(items[0]["summary"]
        .as_str()
        .unwrap()
        .contains("QA-RULE-1001"));
    assert!(items[0].get("notes").is_none());
    assert_eq!(items[1]["feedback_target"], "models");
    assert!(items
        .iter()
        .all(|item| !item["evidence_refs"].as_array().unwrap().is_empty()));
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
            "models",
            "model_under_scored_confirmed_issue",
            "issue_found_return",
        ),
        (
            "QA-QUEUE-TPA-1001",
            "tpa",
            "workflow_missing_evidence",
            "issue_found_escalate",
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

    let (status, summary) = json_request(app, "GET", "/api/v1/ops/qa/queue-summary", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary["open_count"], 3);
    assert_eq!(summary["rules_feedback_count"], 1);
    assert_eq!(summary["models_feedback_count"], 1);
    assert_eq!(summary["tpa_feedback_count"], 1);
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
          "feedback_target": "models",
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

    let (status, labels) = json_request(app, "GET", "/api/v1/ops/labels", "{}").await;

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
            && label["feedback_target"] == "models"
            && label["governance_status"] == "needs_review"
    }));
    assert!(labels
        .iter()
        .all(|label| !label["evidence_refs"].as_array().unwrap().is_empty()));
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
