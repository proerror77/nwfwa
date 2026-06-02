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
        model_service_url: "heuristic://local".into(),
        object_storage_uri: "local://demo-artifacts".into(),
        customer_scope_id: "demo-customer".into(),
        retention_policy_id: "demo-retention-policy".into(),
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
        app.clone(),
        "POST",
        "/api/v1/knowledge/search-similar",
        r#"{
          "claim_id": "CLM-BLANK-QUERY",
          "diagnosis_code": " ",
          "provider_region": "Shanghai",
          "tags": ["early_claim"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_SIMILAR_CASE_QUERY");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/knowledge/search-similar",
        r#"{
          "claim_id": "CLM-BLANK-TAGS",
          "diagnosis_code": "J10",
          "provider_region": "Shanghai",
          "tags": [" "]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_SIMILAR_CASE_QUERY");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/knowledge/search-similar",
        r#"{
          "claim_id": "CLM-BLANK-TAG",
          "diagnosis_code": "J10",
          "provider_region": "Shanghai",
          "tags": ["early_claim", " "]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_SIMILAR_CASE_QUERY");

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
          "case_id": "KC-MISSING-EVIDENCE",
          "title": "Missing evidence case",
          "fwa_type": "Waste",
          "scheme_family": "lab_overuse",
          "diagnosis_code": "E11",
          "provider_region": "Guangzhou",
          "provider_type": "lab",
          "summary": "Confirmed repeated lab testing overuse pattern.",
          "outcome": "Confirmed waste.",
          "tags": ["lab_overuse"],
          "evidence_refs": [" "],
          "source_claim_id": "CLM-KB-1"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_KNOWLEDGE_CASE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/knowledge/cases",
        r#"{
          "case_id": "KC-PII-SUMMARY",
          "title": "PII summary case",
          "fwa_type": "Waste",
          "scheme_family": "lab_overuse",
          "diagnosis_code": "E11",
          "provider_region": "Guangzhou",
          "provider_type": "lab",
          "summary": "Confirmed pattern after contacting alice@example.com.",
          "outcome": "Confirmed waste.",
          "tags": ["lab_overuse"],
          "evidence_refs": ["investigation_results:INV-KB-1"],
          "source_claim_id": "CLM-KB-1"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_KNOWLEDGE_CASE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/knowledge/cases",
        r#"{
          "case_id": "KC-PII-TAG",
          "title": "PII tag case",
          "fwa_type": "Waste",
          "scheme_family": "lab_overuse",
          "diagnosis_code": "E11",
          "provider_region": "Guangzhou",
          "provider_type": "lab",
          "summary": "Confirmed repeated lab testing overuse pattern.",
          "outcome": "Confirmed waste.",
          "tags": ["phone:13800138000"],
          "evidence_refs": ["investigation_results:INV-KB-1"],
          "source_claim_id": "CLM-KB-1"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_KNOWLEDGE_CASE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/knowledge/cases",
        r#"{
          "case_id": "KC-PII-EVIDENCE",
          "title": "PII evidence case",
          "fwa_type": "Waste",
          "scheme_family": "lab_overuse",
          "diagnosis_code": "E11",
          "provider_region": "Guangzhou",
          "provider_type": "lab",
          "summary": "Confirmed repeated lab testing overuse pattern.",
          "outcome": "Confirmed waste.",
          "tags": ["lab_overuse"],
          "evidence_refs": ["investigation_results:INV-KB-1", "id:11010519491231002X"],
          "source_claim_id": "CLM-KB-1"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_KNOWLEDGE_CASE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/knowledge/cases",
        r#"{
          "case_id": "KC-WEAK-EVIDENCE",
          "title": "Weak evidence case",
          "fwa_type": "Waste",
          "scheme_family": "lab_overuse",
          "diagnosis_code": "E11",
          "provider_region": "Guangzhou",
          "provider_type": "lab",
          "summary": "Confirmed repeated lab testing overuse pattern.",
          "outcome": "Confirmed waste.",
          "tags": ["lab_overuse"],
          "evidence_refs": ["claims:CLM-KB-1", "knowledge_cases:KC-WEAK-EVIDENCE"],
          "source_claim_id": "CLM-KB-1"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_KNOWLEDGE_CASE");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("investigation_results or qa_reviews"));

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/knowledge/cases",
        r#"{
          "case_id": "KC-BLANK-TAG",
          "title": "Blank tag case",
          "fwa_type": "Waste",
          "scheme_family": "lab_overuse",
          "diagnosis_code": "E11",
          "provider_region": "Guangzhou",
          "provider_type": "lab",
          "summary": "Confirmed repeated lab testing overuse pattern.",
          "outcome": "Confirmed waste.",
          "tags": ["lab_overuse", " "],
          "evidence_refs": ["investigation_results:INV-KB-1"],
          "source_claim_id": "CLM-KB-1"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_KNOWLEDGE_CASE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/knowledge/cases",
        r#"{
          "case_id": "KC-BLANK-EVIDENCE-REF",
          "title": "Blank evidence ref case",
          "fwa_type": "Waste",
          "scheme_family": "lab_overuse",
          "diagnosis_code": "E11",
          "provider_region": "Guangzhou",
          "provider_type": "lab",
          "summary": "Confirmed repeated lab testing overuse pattern.",
          "outcome": "Confirmed waste.",
          "tags": ["lab_overuse"],
          "evidence_refs": ["investigation_results:INV-KB-1", " "],
          "source_claim_id": "CLM-KB-1"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_KNOWLEDGE_CASE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/knowledge/cases",
        r#"{
          "case_id": "KC-MISSING-SUMMARY",
          "title": "Missing summary case",
          "fwa_type": "Waste",
          "scheme_family": "lab_overuse",
          "diagnosis_code": "E11",
          "provider_region": "Guangzhou",
          "provider_type": "lab",
          "summary": " ",
          "outcome": "Confirmed waste.",
          "tags": ["lab_overuse"],
          "evidence_refs": ["investigation_results:INV-KB-1"],
          "source_claim_id": "CLM-KB-1"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_KNOWLEDGE_CASE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/knowledge/cases",
        r#"{
          "case_id": "KC-PUBLISHED-1",
          "title": "Published provider lab overuse case",
          "fwa_type": "Waste",
          "scheme_family": "lab_overuse",
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
async fn publish_knowledge_case_preserves_canonical_evidence_refs_from_scoring_audit() {
    let app = build_app(test_config());

    let (status, _body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "canonical_claim_context": {
            "claim_header": {
              "external_claim_id": "CLM-KB-CANONICAL",
              "total_amount": 9100,
              "currency": "CNY",
              "service_date": "2026-01-06"
            },
            "member_policy_snapshot": {
              "masked_member_id": "masked-member-kb",
              "masked_certificate_id": "masked-cert-kb",
              "member_birth_date": "1988-03-12",
              "member_gender": "F",
              "policy_id": "POL-KB-CANONICAL",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": 10000
            },
            "provider_snapshot": {
              "provider_id": "PRV-KB-CANONICAL",
              "name": "Knowledge Trace Hospital",
              "provider_type": "hospital",
              "region": "Guangzhou",
              "risk_tier": "High"
            },
            "itemized_bill_lines": [
              {
                "item_name": "Repeated lab panel",
                "fee_category": "lab",
                "amount": 9100,
                "diagnosis_list": [
                  { "code": "E11", "name": "Type 2 diabetes mellitus" }
                ],
                "source_path": "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]",
                "evidence_refs": ["invoice:INV-KB-CANONICAL:fee_detail:LINE-1"]
              }
            ],
            "document_evidence": [
              {
                "document_id": "MR-KB-CANONICAL-1",
                "medical_record_type": "outpatient_record",
                "source_refs": ["medical_record:MR-KB-CANONICAL-1"]
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/knowledge/cases",
        r#"{
          "case_id": "KC-CANONICAL-EVIDENCE",
          "title": "Published canonical trace case",
          "fwa_type": "Waste",
          "scheme_family": "lab_overuse",
          "diagnosis_code": "E11",
          "provider_region": "Guangzhou",
          "provider_type": "lab",
          "summary": "Confirmed repeated lab testing overuse pattern.",
          "outcome": "Confirmed waste; provider education opened.",
          "tags": ["lab_overuse", "provider_pattern"],
          "evidence_refs": ["investigation_results:INV-KB-CANONICAL"],
          "source_claim_id": "CLM-KB-CANONICAL"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(body["case"]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "invoice:INV-KB-CANONICAL:fee_detail:LINE-1"
        )));

    let (status, body) =
        json_request(app.clone(), "GET", "/api/v1/ops/knowledge/cases", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let published_case = body["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["case_id"] == "KC-CANONICAL-EVIDENCE")
        .expect("published knowledge case should be listed");
    assert!(published_case["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "invoice:INV-KB-CANONICAL:fee_detail:LINE-1"
        )));

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/knowledge/search-similar",
        r#"{
          "claim_id": "CLM-KB-CANONICAL-SEARCH",
          "diagnosis_code": "E11",
          "provider_region": "Guangzhou",
          "tags": ["lab_overuse"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let published_result = body["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["case_id"] == "KC-CANONICAL-EVIDENCE")
        .expect("published knowledge case should be searchable");
    assert!(published_result["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "invoice:INV-KB-CANONICAL:fee_detail:LINE-1"
        )));

    let (status, body) =
        json_request(app, "GET", "/api/v1/audit/claims/CLM-KB-CANONICAL", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let publish_event = body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "knowledge.case.published")
        .expect("knowledge case publish should be audited");
    assert!(publish_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "invoice:INV-KB-CANONICAL:fee_detail:LINE-1"
        )));
}

#[tokio::test]
async fn investigates_case_as_assistive_agent_with_evidence_refs() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": " ",
          "risk_score": 87,
          "rag": "RED",
          "top_reasons": ["金额高于同病种同地区 P99"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_CLAIM_ID");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-BAD-SCORE",
          "risk_score": 101,
          "rag": "RED",
          "top_reasons": ["金额高于同病种同地区 P99"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_RISK_SCORE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-BAD-RAG",
          "risk_score": 87,
          "rag": "BLUE",
          "top_reasons": ["金额高于同病种同地区 P99"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_RAG");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-NO-REASON",
          "risk_score": 87,
          "rag": "RED",
          "top_reasons": [" "],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_TOP_REASONS");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-BLANK-REASON",
          "risk_score": 87,
          "rag": "RED",
          "top_reasons": ["金额高于同病种同地区 P99", " "],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_TOP_REASONS");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-BAD-SIMILAR",
          "risk_score": 87,
          "rag": "RED",
          "top_reasons": ["金额高于同病种同地区 P99"],
          "similar_case_query": {
            "diagnosis_code": " ",
            "provider_region": "Shanghai",
            "tags": ["early_claim"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_SIMILAR_CASE_QUERY");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-BAD-TAGS",
          "risk_score": 87,
          "rag": "RED",
          "top_reasons": ["金额高于同病种同地区 P99"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": [" "]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_SIMILAR_CASE_QUERY");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-BLANK-TAG",
          "risk_score": 87,
          "rag": "RED",
          "top_reasons": ["金额高于同病种同地区 P99"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim", " "]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_SIMILAR_CASE_QUERY");

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
    assert!(body["evidence_refs_by_type"]["claim"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .starts_with("claim:masked:claim:")));
    assert!(body["evidence_refs_by_type"]["similar_case"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("knowledge_cases:KC-1001")));
}

#[tokio::test]
async fn downgrades_unconfirmed_fraud_language_in_agent_outputs() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-LANGUAGE-GUARD",
          "risk_score": 92,
          "rag": "RED",
          "top_reasons": [
            "Confirmed fraud ring pattern in provider billing",
            "已确认欺诈，需要人工调查"
          ],
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
    let output_text = investigation.to_string().to_ascii_lowercase();
    assert!(!output_text.contains("confirmed fraud"));
    assert!(!investigation.to_string().contains("已确认欺诈"));
    assert!(output_text.contains("suspected fwa risk"));
    assert!(investigation.to_string().contains("疑似 FWA 风险"));

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
    let run_text = run.to_string().to_ascii_lowercase();
    assert!(!run_text.contains("confirmed fraud"));
    assert!(!run.to_string().contains("已确认欺诈"));
}

#[tokio::test]
async fn redacts_pii_from_agent_free_text_outputs_and_logs() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-PII-GUARD",
          "risk_score": 89,
          "rag": "RED",
          "top_reasons": [
            "Member email alice@example.com appears in notes",
            "Phone 13800138000 and ID 11010519491231002X were attached to the risk reason"
          ],
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
    let response_text = investigation.to_string();
    assert!(!response_text.contains("alice@example.com"));
    assert!(!response_text.contains("13800138000"));
    assert!(!response_text.contains("11010519491231002X"));
    assert!(response_text.contains("[REDACTED_EMAIL]"));
    assert!(response_text.contains("[REDACTED_PHONE]"));
    assert!(response_text.contains("[REDACTED_ID]"));

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
    let run_text = run.to_string();
    assert!(!run_text.contains("alice@example.com"));
    assert!(!run_text.contains("13800138000"));
    assert!(!run_text.contains("11010519491231002X"));
}

#[tokio::test]
async fn redacts_structured_pii_tags_from_agent_context_and_logs() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-TAG-PII-GUARD",
          "risk_score": 88,
          "rag": "RED",
          "top_reasons": ["Provider risk review requested"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["provider_outlier", "email:alice@example.com", "phone:13800138000"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let investigation: serde_json::Value = serde_json::from_str(&body).unwrap();
    let response_text = investigation.to_string();
    assert!(!response_text.contains("alice@example.com"));
    assert!(!response_text.contains("13800138000"));

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
    let run_text = run.to_string();
    assert!(!run_text.contains("alice@example.com"));
    assert!(!run_text.contains("13800138000"));
    assert!(run_text.contains("[REDACTED_EMAIL]"));
    assert!(run_text.contains("[REDACTED_PHONE]"));
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
async fn agent_context_uses_canonical_trace_from_prior_scoring_audit() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "canonical_claim_context": {
            "claim_header": {
              "external_claim_id": "CLM-AGENT-CANONICAL",
              "total_amount": 8800,
              "currency": "CNY",
              "service_date": "2026-01-06"
            },
            "member_policy_snapshot": {
              "masked_member_id": "masked-member-agent",
              "masked_certificate_id": "masked-cert-agent",
              "member_birth_date": "1988-03-12",
              "member_gender": "F",
              "policy_id": "POL-AGENT-CANONICAL",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": 10000
            },
            "provider_snapshot": {
              "provider_id": "PRV-AGENT-CANONICAL",
              "name": "Agent Trace Hospital",
              "provider_type": "hospital",
              "region": "SH",
              "risk_tier": "High"
            },
            "itemized_bill_lines": [
              {
                "item_name": "High cost imaging",
                "fee_category": "procedure",
                "amount": 8800,
                "diagnosis_list": [{ "code": "J10", "name": "Influenza" }],
                "source_path": "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]",
                "evidence_refs": ["invoice:INV-AGENT:fee_detail:LINE-1"]
              }
            ],
            "document_evidence": [
              {
                "document_id": "MR-AGENT-1",
                "medical_record_type": "outpatient_record",
                "source_refs": ["medical_record:MR-AGENT-1"]
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-CANONICAL",
          "risk_score": 87,
          "rag": "RED",
          "scheme_family": "diagnosis_procedure_mismatch",
          "top_reasons": ["诊断-项目匹配度偏低"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["diagnosis_mismatch"]
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
    let context_snapshot = run["context_snapshots"]
        .as_array()
        .unwrap()
        .first()
        .expect("agent context snapshot should be audited");
    assert!(
        context_snapshot["source_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]"
            )),
        "agent context source refs should include normalized bill-line source path"
    );
    assert!(
        context_snapshot["source_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("medical_record:MR-AGENT-1")),
        "agent context source refs should include normalized document source ref"
    );
    assert!(
        context_snapshot["context_json"]["canonical_claim_context_trace"]["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("invoice:INV-AGENT:fee_detail:LINE-1")),
        "agent context should carry canonical evidence refs for investigation grounding"
    );
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
    assert!(body["approval"]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(format!("agent_run:{agent_run_id}"))));
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
    let app = build_app(test_config());

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
    let app = build_app(test_config());

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
    let app = build_app(test_config());

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
