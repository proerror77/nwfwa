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
    ] {
        let status = unauthenticated_request(method, uri, body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
