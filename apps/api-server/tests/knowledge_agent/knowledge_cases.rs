use api_server::{
    app::{build_app, build_app_with_parts},
    repository::InMemoryScoringRepository,
};
use axum::http::StatusCode;
use fwa_ml_runtime::HeuristicModelScorer;
use std::sync::Arc;

use super::{json_request, scoped_config, test_config};

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
    assert_eq!(
        publish_event["payload"]["customer_scope_id"],
        "demo-customer"
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
async fn publish_knowledge_case_does_not_merge_cross_customer_canonical_evidence_refs() {
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

    let (status, _body) = json_request(
        alpha_app,
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "canonical_claim_context": {
            "claim_header": {
              "external_claim_id": "CLM-KB-CROSS-SCOPE",
              "total_amount": 9300,
              "currency": "CNY",
              "service_date": "2026-01-06"
            },
            "member_policy_snapshot": {
              "masked_member_id": "masked-member-alpha",
              "masked_certificate_id": "masked-cert-alpha",
              "member_birth_date": "1988-03-12",
              "member_gender": "F",
              "policy_id": "POL-KB-CROSS-SCOPE",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": 10000
            },
            "provider_snapshot": {
              "provider_id": "PRV-KB-CROSS-SCOPE",
              "name": "Cross Scope Hospital",
              "provider_type": "hospital",
              "region": "Guangzhou",
              "risk_tier": "High"
            },
            "itemized_bill_lines": [
              {
                "item_name": "Alpha-only invoice line",
                "fee_category": "lab",
                "amount": 9300,
                "diagnosis_list": [
                  { "code": "E11", "name": "Type 2 diabetes mellitus" }
                ],
                "source_path": "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]",
                "evidence_refs": ["invoice:ALPHA-ONLY:fee_detail:LINE-1"]
              }
            ],
            "document_evidence": [
              {
                "document_id": "MR-ALPHA-ONLY",
                "medical_record_type": "outpatient_record",
                "source_refs": ["medical_record:MR-ALPHA-ONLY"]
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(
        beta_app,
        "POST",
        "/api/v1/ops/knowledge/cases",
        r#"{
          "case_id": "KC-CROSS-SCOPE",
          "title": "Published beta scope case",
          "fwa_type": "Waste",
          "scheme_family": "lab_overuse",
          "diagnosis_code": "E11",
          "provider_region": "Guangzhou",
          "provider_type": "lab",
          "summary": "Confirmed repeated lab testing overuse pattern.",
          "outcome": "Confirmed waste; provider education opened.",
          "tags": ["lab_overuse", "provider_pattern"],
          "evidence_refs": ["investigation_results:INV-BETA-SCOPE"],
          "source_claim_id": "CLM-KB-CROSS-SCOPE"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(!body["case"]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("invoice:ALPHA-ONLY:fee_detail:LINE-1")));
}
