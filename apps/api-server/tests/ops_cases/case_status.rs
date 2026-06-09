use api_server::app::build_app;
use axum::http::StatusCode;

use super::{json_request, test_config};

#[tokio::test]
async fn updates_case_status_with_audit_trail() {
    let app = build_app(test_config());

    let (status, score) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-CASE-STATUS",
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
            "external_member_id": "MBR-CASE-STATUS"
          },
          "policy": {
            "external_policy_id": "POL-CASE-STATUS",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-CASE-STATUS",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "High"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let scoring_run_id = score["run_id"].as_str().unwrap();

    let (status, leads) = json_request(app.clone(), "GET", "/api/v1/ops/leads", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let lead_id = leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lead| lead["claim_id"] == "CLM-CASE-STATUS")
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
          "assignee": "siu-reviewer-2",
          "reviewer": "medical-reviewer-2",
          "priority": "high",
          "notes": "Open investigation from high-risk FWA lead.",
          "evidence_refs": ["triage_decisions:open_case_status"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let case_id = triage["case"]["case_id"].as_str().unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "investigating",
          "actor_id": "siu-reviewer-2",
          "notes": " ",
          "evidence_refs": ["case_workflow:investigation_started"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_CASE_STATUS_NOTES");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "investigating",
          "actor_id": "siu-reviewer-2",
          "notes": "Investigation started with provider history review.",
          "evidence_refs": []
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_CASE_STATUS_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "investigating",
          "actor_id": "siu-reviewer-2",
          "notes": "Investigation started with provider history review.",
          "evidence_refs": ["case_workflow:investigation_started", " "]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_CASE_STATUS_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "investigating",
          "actor_id": "siu-reviewer-2",
          "notes": "Investigation started for ID 11010519491231002X.",
          "evidence_refs": ["case_workflow:investigation_started"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_CASE_WORKFLOW");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "investigating",
          "actor_id": "siu-reviewer-2",
          "notes": "Investigation started with provider history review.",
          "evidence_refs": ["phone:13800138000"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_CASE_WORKFLOW");

    let (status, update) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "investigating",
          "actor_id": "siu-reviewer-2",
          "notes": "Investigation started with provider history review.",
          "evidence_refs": ["case_workflow:investigation_started"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(update["case"]["case_id"], case_id);
    assert_eq!(update["case"]["status"], "investigating");
    assert!(update["audit_id"].as_str().unwrap().starts_with("aud_"));

    let (status, cases) = json_request(app.clone(), "GET", "/api/v1/ops/cases", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["case_id"] == case_id)
        .unwrap();
    assert_eq!(case["status"], "investigating");

    let (status, audit) =
        json_request(app, "GET", "/api/v1/audit/claims/CLM-CASE-STATUS", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let status_event = audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "case.status.updated")
        .expect("case status update should be audited");
    assert_eq!(status_event["payload"]["case_id"], case_id);
    assert_eq!(status_event["run_id"], scoring_run_id);
    assert_eq!(status_event["payload"]["to_status"], "investigating");
    assert_eq!(
        status_event["payload"]["customer_scope_id"],
        "demo-customer"
    );
    assert!(status_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("case_workflow:investigation_started")));
}
