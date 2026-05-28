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
) -> (StatusCode, String) {
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
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn lists_knowledge_cases() {
    let app = build_app(test_config());

    let (status, body) = json_request(app, "GET", "/api/v1/ops/knowledge/cases", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["cases"][0]["case_id"], "KC-1001");
    assert_eq!(body["cases"][0]["fwa_type"], "Abuse");
    assert_eq!(
        body["cases"][0]["scheme_family"],
        "diagnosis_procedure_mismatch"
    );
    assert!(!body["cases"][0]["evidence_refs"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn searches_similar_knowledge_cases_with_evidence() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/knowledge/search-similar",
        r#"{
          "claim_id": "CLM-0287",
          "diagnosis_code": "J10",
          "provider_region": "Shanghai",
          "tags": ["early_claim", "high_amount"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["results"][0]["case_id"], "KC-1001");
    assert_eq!(
        body["results"][0]["scheme_family"],
        "diagnosis_procedure_mismatch"
    );
    assert!(body["results"][0]["similarity_score"].as_f64().unwrap() > 0.0);
    assert!(!body["results"][0]["matched_signals"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!body["results"][0]["evidence_refs"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        body["results"][0]["retrieval_method"],
        "structured_signal_overlap"
    );
    assert!(body["results"][0]["provenance_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("knowledge_cases:KC-1001")));
}

#[tokio::test]
async fn publishes_confirmed_knowledge_case_for_similarity_and_audit() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/knowledge/cases",
        r#"{
          "case_id": "KC-PUBLISHED-1",
          "title": "Published provider lab overuse case",
          "fwa_type": "Waste",
          "scheme_family": "laboratory_testing_abuse",
          "diagnosis_code": "E11",
          "provider_region": "Guangzhou",
          "provider_type": "lab",
          "summary": "Confirmed repeated lab testing overuse pattern.",
          "outcome": "Confirmed waste; provider education and post-payment audit opened.",
          "tags": ["lab_overuse", "provider_pattern"],
          "evidence_refs": ["investigation_results:INV-KB-1", "qa_reviews:QA-KB-1"],
          "source_claim_id": "CLM-KB-1"
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["case"]["case_id"], "KC-PUBLISHED-1");
    assert_eq!(body["case"]["scheme_family"], "laboratory_testing_abuse");
    assert!(body["audit_id"].as_str().unwrap().starts_with("aud_"));

    let (status, body) =
        json_request(app.clone(), "GET", "/api/v1/ops/knowledge/cases", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(body["cases"]
        .as_array()
        .unwrap()
        .iter()
        .any(|case| case["case_id"] == "KC-PUBLISHED-1"));

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/knowledge/search-similar",
        r#"{
          "claim_id": "CLM-KB-SEARCH",
          "diagnosis_code": "E11",
          "provider_region": "Guangzhou",
          "tags": ["lab_overuse"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["results"][0]["case_id"], "KC-PUBLISHED-1");
    assert_eq!(
        body["results"][0]["scheme_family"],
        "laboratory_testing_abuse"
    );

    let (status, body) = json_request(app, "GET", "/api/v1/audit/claims/CLM-KB-1", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let publish_event = body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "knowledge.case.published")
        .expect("knowledge case publish should be audited");
    assert_eq!(
        publish_event["payload"]["scheme_family"],
        "laboratory_testing_abuse"
    );
}

#[tokio::test]
async fn investigates_case_as_assistive_agent_with_evidence_refs() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-0287",
          "risk_score": 87,
          "rag": "RED",
          "scheme_family": "provider_peer_outlier",
          "top_reasons": [
            "金额高于同病种同地区 P99",
            "诊断-项目匹配度偏低"
          ],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim", "high_amount"]
          }
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["decision_boundary"], "assistive_only");
    assert!(body["agent_run_id"].as_str().unwrap().starts_with("agent_"));
    assert!(!body["risk_summary"].as_str().unwrap().contains("CLM-0287"));
    assert!(body["risk_summary"]
        .as_str()
        .unwrap()
        .contains("masked:claim:"));
    assert!(body["investigation_checklist"].as_array().unwrap().len() >= 3);
    assert!(!body["similar_cases"].as_array().unwrap().is_empty());
    let similar_case = &body["similar_cases"][0];
    assert!(similar_case["provenance_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("knowledge_cases:KC-1001")));
    assert!(similar_case["provenance_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference.as_str().unwrap().starts_with("matched_signal:")));
    assert!(body["findings"]
        .as_array()
        .unwrap()
        .iter()
        .all(|finding| !finding["evidence_refs"].as_array().unwrap().is_empty()));
    assert_eq!(
        body["evidence_sufficiency"]["scheme_family"],
        "provider_peer_outlier"
    );
    assert!(body["evidence_sufficiency"]["minimum_evidence"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("peer_group_definition")));
    assert!(body["evidence_sufficiency"]["missing_evidence"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("specialty")));
    assert!(body["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("knowledge_cases:KC-1001")));
    assert!(body["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference.as_str().unwrap().starts_with("matched_signal:")));
    assert!(!body["evidence_refs"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn lists_agent_run_logs_for_governance_review() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-LOGS",
          "risk_score": 91,
          "rag": "RED",
          "top_reasons": ["Provider 风险画像偏高"],
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

    let (status, body) = json_request(app, "GET", "/api/v1/ops/agent-runs", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let run = body["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|run| run["agent_run_id"] == agent_run_id)
        .expect("agent run should be listed for governance review");
    assert_eq!(run["claim_id"], "CLM-AGENT-LOGS");
    assert_eq!(run["status"], "succeeded");
    assert_eq!(run["decision_boundary"], "assistive_only");
    assert!(!run["agent_run_id"]
        .as_str()
        .unwrap()
        .contains("CLM-AGENT-LOGS"));
    assert!(!run["steps"].as_array().unwrap().is_empty());
    let context_snapshot = run["context_snapshots"]
        .as_array()
        .unwrap()
        .first()
        .expect("agent context snapshot should be audited");
    assert_eq!(context_snapshot["redaction_status"], "pii_masked");
    assert!(context_snapshot["checksum"]
        .as_str()
        .unwrap()
        .starts_with("snapshot:"));
    assert!(context_snapshot["context_json"]["claim_id"].is_string());
    assert_ne!(
        context_snapshot["context_json"]["claim_id"],
        "CLM-AGENT-LOGS"
    );
    assert!(context_snapshot["context_json"]["claim_id"]
        .as_str()
        .unwrap()
        .starts_with("masked:claim:"));
    assert!(!context_snapshot["context_json"]
        .to_string()
        .contains("CLM-AGENT-LOGS"));
    assert!(!context_snapshot["source_refs"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!context_snapshot["source_refs"]
        .to_string()
        .contains("CLM-AGENT-LOGS"));
    let tool_call = run["tool_calls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|call| call["tool_name"] == "knowledge.search_similar")
        .expect("similar-case search tool call should be audited");
    assert_eq!(tool_call["status"], "succeeded");
    assert!(!tool_call["input_json"].as_object().unwrap().is_empty());
    assert!(!tool_call["input_json"]
        .to_string()
        .contains("CLM-AGENT-LOGS"));
    assert!(!tool_call["evidence_refs"].as_array().unwrap().is_empty());
    let policy_check = run["policy_checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["tool_name"] == "knowledge.search_similar")
        .expect("tool policy check should be audited before tool activity");
    assert_eq!(policy_check["decision"], "allowed");
    assert_eq!(policy_check["policy_name"], "agent_tool_allowlist");
    assert_eq!(policy_check["tool_call_id"], tool_call["tool_call_id"]);
    assert!(!policy_check["evidence_refs"].as_array().unwrap().is_empty());
    let tool_result = run["tool_results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|result| result["tool_name"] == "knowledge.search_similar")
        .expect("similar-case search tool result should be audited");
    assert_eq!(tool_result["status"], "succeeded");
    assert!(tool_result["output_json"]["result_count"].as_u64().unwrap() > 0);
    assert!(tool_result["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference.as_str().unwrap().starts_with("matched_signal:")));
    let approval = run["approvals"]
        .as_array()
        .unwrap()
        .first()
        .expect("agent run should create a pending human approval gate");
    assert_eq!(approval["decision"], "pending");
    assert_eq!(approval["proposed_action"], "manual_review_required");
    assert!(!approval["evidence_refs"].as_array().unwrap().is_empty());
    assert!(!run["evidence_refs"].as_array().unwrap().is_empty());
    assert!(run["output_json"]["evidence_sufficiency"].is_object());
    assert!(!run["output_json"].to_string().contains("CLM-AGENT-LOGS"));
}

#[tokio::test]
async fn submits_agent_approval_decision_for_governance_review() {
    let app = build_app(test_config());

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
        r#"{
          "decision": "approved",
          "approver": "qa-lead",
          "reason": "Evidence package is sufficient for manual review routing.",
          "evidence_refs": ["agent_approval:manual_review_required"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["approval"]["agent_run_id"], agent_run_id);
    assert_eq!(body["approval"]["decision"], "approved");
    assert_eq!(body["approval"]["approver"], "qa-lead");
    assert!(body["audit_id"].as_str().unwrap().starts_with("aud_"));

    let (status, body) = json_request(app, "GET", "/api/v1/ops/agent-runs", "{}").await;
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
}
