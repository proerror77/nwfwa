use api_server::app::build_app;
use axum::http::StatusCode;

use super::{json_request, test_config};

#[tokio::test]
async fn submits_agent_approval_decision_for_governance_review() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-APPROVAL",
          "risk_score": 94,
          "rag": "RED",
          "top_reasons": ["Agent 建议升级人工审核"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["provider_outlier"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let investigation: serde_json::Value = serde_json::from_str(&body).unwrap();
    let agent_run_id = investigation["agent_run_id"].as_str().unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/agent-runs/{agent_run_id}/approvals"),
        &format!(
            r#"{{
          "decision": "approved",
          "approver": "qa-lead",
          "reason": "Evidence package is sufficient for manual review routing.",
          "evidence_refs": ["agent_run:{agent_run_id}", "agent_approval:manual_review_required"]
        }}"#,
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["approval"]["agent_run_id"], agent_run_id);
    assert_eq!(body["approval"]["decision"], "approved");
    assert_eq!(body["approval"]["approver"], "qa-lead");
    let approval_evidence_refs = body["approval"]["evidence_refs"].clone();
    assert!(body["approval"]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(format!("agent_run:{agent_run_id}"))));
    assert!(body["approval"]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("policy:demo-agent-policy")));
    assert!(body["audit_id"].as_str().unwrap().starts_with("aud_"));

    let (status, body) = json_request(app.clone(), "GET", "/api/v1/ops/agent-runs", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let run = body["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|run| run["agent_run_id"] == agent_run_id)
        .expect("agent run should be listed after approval");
    let approval = run["approvals"]
        .as_array()
        .unwrap()
        .first()
        .expect("submitted approval should be included in agent governance logs");
    assert_eq!(approval["decision"], "approved");
    assert_eq!(approval["approver"], "qa-lead");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/agent-runs/{agent_run_id}/approvals"),
        &format!(
            r#"{{
          "decision": "rejected",
          "approver": "qa-lead",
          "reason": "Attempt to change a completed approval decision.",
          "evidence_refs": ["agent_run:{agent_run_id}", "agent_approval:manual_review_required"]
        }}"#,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "AGENT_APPROVAL_NOT_PENDING");

    let (status, body) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?event_group=governance&event_type=agent.approval.decided&actor_id=qa-lead&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let events = body["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event_type"], "agent.approval.decided");
    assert_eq!(events[0]["payload"]["agent_run_id"], agent_run_id);
    assert_eq!(events[0]["payload"]["customer_scope_id"], "demo-customer");
    assert_eq!(events[0]["payload"]["agent_policy_id"], "demo-agent-policy");
    assert_eq!(
        events[0]["payload"]["evidence_refs"],
        approval_evidence_refs
    );
    assert!(events[0]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("policy:demo-agent-policy")));

    let (status, body) = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/ops/audit-events?agent_run_id={agent_run_id}&limit=10"),
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let events = body["events"].as_array().unwrap();
    assert_eq!(events.len(), 2);
    assert!(events.iter().any(
        |event| event["event_type"] == "agent.investigation.completed"
            && event["payload"]["agent_run_id"] == agent_run_id
    ));
    assert!(events
        .iter()
        .any(|event| event["event_type"] == "agent.approval.decided"
            && event["payload"]["agent_run_id"] == agent_run_id));

    let (status, body) = json_request(
        app.clone(),
        "GET",
        "/api/v1/audit/claims/CLM-AGENT-APPROVAL",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let claim_events = body["events"].as_array().unwrap();
    assert!(claim_events.iter().any(|event| event["event_type"]
        == "agent.investigation.completed"
        && event["payload"]["agent_run_id"] == agent_run_id));
    assert!(claim_events
        .iter()
        .any(|event| event["event_type"] == "agent.approval.decided"
            && event["payload"]["agent_run_id"] == agent_run_id));

    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/ops/audit-events?agent_run_id=missing-agent-run&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(body["events"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn rejects_agent_approval_without_evidence_or_reviewer_context() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-APPROVAL-GUARD",
          "risk_score": 91,
          "rag": "RED",
          "top_reasons": ["Agent 建议必须经过有证据的人审"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["provider_outlier"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let investigation: serde_json::Value = serde_json::from_str(&body).unwrap();
    let agent_run_id = investigation["agent_run_id"].as_str().unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/agent-runs/{agent_run_id}/approvals"),
        r#"{
          "decision": "approved",
          "approver": "qa-lead",
          "reason": "Evidence package is sufficient for manual review routing.",
          "evidence_refs": []
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "MISSING_AGENT_APPROVAL_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/agent-runs/{agent_run_id}/approvals"),
        r#"{
          "decision": "approved",
          "approver": "qa-lead",
          "reason": "Evidence package is sufficient for manual review routing.",
          "evidence_refs": ["agent_approval:manual_review_required"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "MISSING_AGENT_APPROVAL_RUN_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/agent-runs/{agent_run_id}/approvals"),
        r#"{
          "decision": "approved",
          "approver": "qa-lead",
          "reason": "Evidence package is sufficient for manual review routing.",
          "evidence_refs": ["agent_approval:manual_review_required", " "]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "MISSING_AGENT_APPROVAL_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/agent-runs/{agent_run_id}/approvals"),
        r#"{
          "decision": "approved",
          "approver": " ",
          "reason": " ",
          "evidence_refs": ["agent_approval:manual_review_required"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "MISSING_AGENT_APPROVER");

    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/ops/agent-runs/{agent_run_id}/approvals"),
        r#"{
          "decision": "approved",
          "approver": "qa-lead",
          "reason": " ",
          "evidence_refs": ["agent_approval:manual_review_required"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "MISSING_AGENT_APPROVAL_REASON");
}

#[tokio::test]
async fn rejects_agent_approval_with_pii_in_reason_or_evidence_refs() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-APPROVAL-PII",
          "risk_score": 91,
          "rag": "RED",
          "top_reasons": ["Agent approval must remain PII controlled"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["provider_outlier"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let investigation: serde_json::Value = serde_json::from_str(&body).unwrap();
    let agent_run_id = investigation["agent_run_id"].as_str().unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/agent-runs/{agent_run_id}/approvals"),
        &format!(
            r#"{{
          "decision": "approved",
          "approver": "qa-lead",
          "reason": "Reviewer copied member email alice@example.com into approval reason.",
          "evidence_refs": ["agent_run:{agent_run_id}"]
        }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_AGENT_APPROVAL");

    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/ops/agent-runs/{agent_run_id}/approvals"),
        &format!(
            r#"{{
          "decision": "approved",
          "approver": "qa-lead",
          "reason": "Evidence package is sufficient for manual review routing.",
          "evidence_refs": ["agent_run:{agent_run_id}", "phone:13800138000"]
        }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_AGENT_APPROVAL");
}

#[tokio::test]
async fn lists_agent_approval_alert_until_decision_is_recorded() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-APPROVAL-ALERT",
          "risk_score": 93,
          "rag": "RED",
          "top_reasons": ["Agent output requires human approval before action"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["provider_outlier"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let investigation: serde_json::Value = serde_json::from_str(&body).unwrap();
    let agent_run_id = investigation["agent_run_id"].as_str().unwrap();

    let (status, body) = json_request(app.clone(), "GET", "/api/v1/ops/alerts", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let alert = body["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|alert| alert["alert_type"] == "agent_approval_pending")
        .expect("pending agent approval should create an operations alert");
    assert_eq!(alert["claim_id"], "CLM-AGENT-APPROVAL-ALERT");
    assert!(alert["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(format!("agent_run:{agent_run_id}"))));

    let (status, _) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/agent-runs/{agent_run_id}/approvals"),
        &format!(
            r#"{{
          "decision": "approved",
          "approver": "qa-lead",
          "reason": "Evidence package is sufficient for manual review routing.",
          "evidence_refs": ["agent_run:{agent_run_id}", "agent_approval:manual_review_required"]
        }}"#,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(app, "GET", "/api/v1/ops/alerts", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(!body["alerts"].as_array().unwrap().iter().any(|alert| {
        alert["alert_type"] == "agent_approval_pending"
            && alert["claim_id"] == "CLM-AGENT-APPROVAL-ALERT"
    }));
}
