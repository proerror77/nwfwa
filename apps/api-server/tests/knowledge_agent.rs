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
    assert!(body["results"][0]["similarity_score"].as_f64().unwrap() > 0.0);
    assert!(!body["results"][0]["matched_signals"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!body["results"][0]["evidence_refs"]
        .as_array()
        .unwrap()
        .is_empty());
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

    let (status, body) = json_request(app, "GET", "/api/v1/audit/claims/CLM-KB-1", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(body["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event_type"] == "knowledge.case.published"));
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
    assert!(body["risk_summary"].as_str().unwrap().contains("CLM-0287"));
    assert!(body["investigation_checklist"].as_array().unwrap().len() >= 3);
    assert!(!body["similar_cases"].as_array().unwrap().is_empty());
    assert!(body["findings"]
        .as_array()
        .unwrap()
        .iter()
        .all(|finding| !finding["evidence_refs"].as_array().unwrap().is_empty()));
    assert!(!body["evidence_refs"].as_array().unwrap().is_empty());
}
