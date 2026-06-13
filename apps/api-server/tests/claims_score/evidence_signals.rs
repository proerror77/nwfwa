use api_server::app::build_app;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use super::support::test_config;

#[tokio::test]
async fn returns_clinical_evidence_gaps_for_medical_necessity_review() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-CLINICAL-1",
                "claim_amount": "12000",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "diagnosis_code": "J10"
              },
              "items": [
                {
                  "item_code": "IMG-900",
                  "item_type": "procedure",
                  "description": "High cost imaging",
                  "quantity": 1,
                  "unit_amount": "12000",
                  "total_amount": "12000"
                }
              ],
              "member": {
                "external_member_id": "MBR-CLINICAL-1"
              },
              "policy": {
                "external_policy_id": "POL-CLINICAL-1",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "15000",
                "currency": "CNY"
              },
              "provider": {
                "external_provider_id": "PRV-CLINICAL-1",
                "name": "Northwind Hospital",
                "provider_type": "hospital",
                "region": "SH",
                "risk_tier": "High"
              }
            }"#,
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let clinical = &body["clinical_evidence"];
    assert_eq!(clinical["review_required"], true);
    assert_eq!(clinical["review_route"], "medical_review");
    assert_eq!(clinical["evidence_status"], "missing_required_evidence");
    assert_eq!(body["decision_outcome"], "pending_evidence");
    assert_eq!(body["decision_authority"], "clinical_policy_rule");
    assert_eq!(body["reason_code"], "MEDICALLY_UNNECESSARY_SERVICE");
    assert!(clinical["missing_evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "medical_record"));
    let clinical_missing_evidence = clinical["missing_evidence"].as_array().unwrap();
    assert!(clinical_missing_evidence
        .iter()
        .any(|item| item == "radiology_report"));
    assert!(!clinical_missing_evidence
        .iter()
        .any(|item| item == "prescription"));
    let clinical_rule_alert = body["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|alert| alert["alert_code"] == "MEDICALLY_UNNECESSARY_SERVICE")
        .expect("clinical pending evidence rule should trigger");
    let alert_required_evidence = clinical_rule_alert["required_evidence"].as_array().unwrap();
    assert!(alert_required_evidence
        .iter()
        .any(|item| item["evidence_type"] == "radiology_report"));
    assert!(alert_required_evidence
        .iter()
        .any(|item| item["evidence_type"] == "medical_record"));
    assert!(!alert_required_evidence
        .iter()
        .any(|item| item["evidence_type"] == "prescription"));
    assert_eq!(
        alert_required_evidence[0]["policy_authority_ref"],
        "policy:clinical-evidence:v1"
    );
    assert_eq!(
        clinical["item_findings"][0]["issue_type"],
        "medical_necessity_review_required"
    );
    assert_eq!(clinical["item_findings"][0]["item_code"], "IMG-900");
    assert!(clinical["item_findings"][0]["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "claim_items:IMG-900"));
    assert_eq!(body["scores"]["medical_reasonableness_score"], 100);

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-CLINICAL-1")
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let audit_response = app.clone().oneshot(audit_request).await.unwrap();
    assert_eq!(audit_response.status(), StatusCode::OK);
    let audit_body = to_bytes(audit_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let audit_body: serde_json::Value = serde_json::from_slice(&audit_body).unwrap();
    let scoring_event = audit_body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "scoring.completed")
        .expect("audit history should include scoring.completed");
    assert_eq!(
        scoring_event["payload"]["clinical_evidence"]["review_route"],
        "medical_review"
    );
    assert_eq!(
        scoring_event["payload"]["scores"]["medical_reasonableness_score"],
        body["scores"]["medical_reasonableness_score"]
    );
    assert!(scoring_event["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item.as_str().unwrap().contains("clinical_evidence")));

    let evidence_request = Request::builder()
        .method("GET")
        .uri("/api/v1/ops/evidence-requests")
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let evidence_response = app.oneshot(evidence_request).await.unwrap();
    assert_eq!(evidence_response.status(), StatusCode::OK);
    let evidence_body = to_bytes(evidence_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let evidence_body: serde_json::Value = serde_json::from_slice(&evidence_body).unwrap();
    let request = evidence_body["requests"]
        .as_array()
        .unwrap()
        .iter()
        .find(|request| request["claim_id"] == "CLM-CLINICAL-1")
        .expect("clinical pending rule should create an evidence request");
    assert_eq!(request["request_reason"], "rule_required_evidence");
    assert!(request["missing_evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "radiology_report"));
    assert!(!request["missing_evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "prescription"));
    assert!(request["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["document_type"] == "medical_record"
            && item["blocking"] == true
            && item["policy_authority_ref"] == "policy:clinical-evidence:v1"
            && item["exception_check"] == "required_clinical_documents_not_present"));
}

#[tokio::test]
async fn returns_provider_profile_outlier_evidence_for_network_risk() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-PROVIDER-1",
                "claim_amount": "18000",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "diagnosis_code": "J10"
              },
              "items": [
                {
                  "item_code": "IMG-901",
                  "item_type": "procedure",
                  "description": "High cost imaging",
                  "quantity": 1,
                  "unit_amount": "18000",
                  "total_amount": "18000"
                }
              ],
              "member": {
                "external_member_id": "MBR-PROVIDER-1"
              },
              "policy": {
                "external_policy_id": "POL-PROVIDER-1",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "20000",
                "currency": "CNY"
              },
              "provider": {
                "external_provider_id": "PRV-PROVIDER-1",
                "name": "Northwind Hospital",
                "provider_type": "hospital",
                "region": "SH",
                "risk_tier": "Medium"
              },
              "provider_profile": {
                "specialty": "imaging",
                "network_status": "in_network",
                "windows": [
                  {
                    "window_days": 90,
                    "claim_count": 126,
                    "total_claim_amount": "420000",
                    "high_cost_item_ratio": 0.72,
                    "diagnosis_procedure_mismatch_rate": 0.38,
                    "peer_amount_percentile": 97,
                    "peer_frequency_percentile": 96,
                    "review_failure_count": 3,
                    "confirmed_fwa_count": 4,
                    "false_positive_count": 1
                  }
                ]
              }
            }"#,
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let profile = &body["provider_profile"];
    assert_eq!(profile["provider_id"], "PRV-PROVIDER-1");
    assert_eq!(profile["review_required"], true);
    assert_eq!(profile["review_route"], "provider_review");
    assert!(profile["risk_score"].as_u64().unwrap() >= 80);
    assert_eq!(
        body["scores"]["provider_network_score"],
        profile["risk_score"]
    );
    assert!(profile["outlier_flags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "peer_amount_p97"));
    assert_eq!(profile["review_failure_count"], 3);
    assert_eq!(profile["confirmed_fwa_count"], 4);
    assert_eq!(profile["false_positive_count"], 1);
    assert!(profile["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "provider_profile:PRV-PROVIDER-1:90d"));

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-PROVIDER-1")
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let audit_response = app.oneshot(audit_request).await.unwrap();
    assert_eq!(audit_response.status(), StatusCode::OK);
    let audit_body = to_bytes(audit_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let audit_body: serde_json::Value = serde_json::from_slice(&audit_body).unwrap();
    let scoring_event = audit_body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "scoring.completed")
        .expect("audit history should include scoring.completed");
    assert_eq!(
        scoring_event["payload"]["provider_profile"]["review_route"],
        "provider_review"
    );
}

#[tokio::test]
async fn returns_provider_sanctions_evidence_for_excluded_provider() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-PROVIDER-SANCTIONS-1",
                "claim_amount": "2500",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "diagnosis_code": "J10"
              },
              "items": [
                {
                  "item_code": "CONSULT-001",
                  "item_type": "procedure",
                  "description": "Consultation",
                  "quantity": 1,
                  "unit_amount": "2500",
                  "total_amount": "2500"
                }
              ],
              "member": {
                "external_member_id": "MBR-PROVIDER-SANCTIONS-1"
              },
              "policy": {
                "external_policy_id": "POL-PROVIDER-SANCTIONS-1",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "20000",
                "currency": "CNY"
              },
              "provider": {
                "external_provider_id": "PRV-SANCTIONED-1",
                "name": "Sanctioned Provider",
                "provider_type": "clinic",
                "region": "SH",
                "risk_tier": "Low"
              },
              "provider_profile": {
                "specialty": "primary_care",
                "network_status": "in_network",
                "oig_excluded": true,
                "sam_debarred": true,
                "windows": [
                  {
                    "window_days": 90,
                    "claim_count": 2,
                    "total_claim_amount": "2500",
                    "high_cost_item_ratio": 0.10,
                    "diagnosis_procedure_mismatch_rate": 0.0,
                    "peer_amount_percentile": 40,
                    "peer_frequency_percentile": 35,
                    "review_failure_count": 0,
                    "confirmed_fwa_count": 0,
                    "false_positive_count": 0
                  }
                ]
              }
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let profile = &body["provider_profile"];

    assert_eq!(profile["risk_score"], 100);
    assert_eq!(profile["risk_tier"], "high");
    assert_eq!(profile["review_required"], true);
    assert_eq!(profile["review_route"], "provider_sanctions_review");
    assert_eq!(profile["oig_excluded"], true);
    assert_eq!(profile["sam_debarred"], true);
    assert!(profile["outlier_flags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "oig_excluded"));
    assert!(profile["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "provider_sanctions:PRV-SANCTIONED-1:oig"));
}

#[tokio::test]
async fn returns_provider_relationship_graph_evidence_for_l6_network_risk() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-GRAPH-1",
                "claim_amount": "9000",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "diagnosis_code": "J10"
              },
              "items": [
                {
                  "item_code": "IMG-910",
                  "item_type": "procedure",
                  "description": "High cost imaging",
                  "quantity": 1,
                  "unit_amount": "9000",
                  "total_amount": "9000"
                }
              ],
              "member": {
                "external_member_id": "MBR-GRAPH-1"
              },
              "policy": {
                "external_policy_id": "POL-GRAPH-1",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000",
                "currency": "CNY"
              },
              "provider": {
                "external_provider_id": "PRV-GRAPH-1",
                "name": "Northwind Hospital",
                "provider_type": "hospital",
                "region": "SH",
                "risk_tier": "Medium"
              },
              "provider_relationships": {
                "high_risk_neighbor_ratio": 0.34,
                "provider_patient_overlap_score": 0.68,
                "referral_concentration_score": 0.72,
                "connected_confirmed_fwa_count": 2,
                "network_component_risk_score": 82,
                "evidence_refs": ["relationship_edges:PRV-GRAPH-1"]
              }
            }"#,
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let graph = &body["provider_relationships"];
    assert_eq!(graph["provider_id"], "PRV-GRAPH-1");
    assert_eq!(graph["review_required"], true);
    assert_eq!(graph["review_route"], "provider_graph_review");
    assert!(graph["risk_score"].as_u64().unwrap() >= 90);
    assert_eq!(
        body["scores"]["provider_network_score"],
        graph["risk_score"]
    );
    assert!(graph["graph_reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item.as_str().unwrap().contains("关系邻居")));
    assert!(graph["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("relationship_edges:PRV-GRAPH-1")));

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-GRAPH-1")
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let audit_response = app.oneshot(audit_request).await.unwrap();
    assert_eq!(audit_response.status(), StatusCode::OK);
    let audit_body = to_bytes(audit_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let audit_body: serde_json::Value = serde_json::from_slice(&audit_body).unwrap();
    let scoring_event = audit_body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "scoring.completed")
        .expect("audit history should include scoring.completed");
    assert_eq!(
        scoring_event["payload"]["provider_relationships"]["review_route"],
        "provider_graph_review"
    );
    assert!(scoring_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("relationship_edges:PRV-GRAPH-1")));
}

#[tokio::test]
async fn scoring_includes_similar_case_signal_from_knowledge_base() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-SIMILAR-CASE",
                "claim_amount": "9000",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "diagnosis_code": "J10"
              },
              "items": [
                {
                  "item_code": "IMG-902",
                  "item_type": "procedure",
                  "description": "High cost imaging",
                  "quantity": 1,
                  "unit_amount": "9000",
                  "total_amount": "9000"
                }
              ],
              "member": {
                "external_member_id": "MBR-SIMILAR-CASE"
              },
              "policy": {
                "external_policy_id": "POL-SIMILAR-CASE",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000",
                "currency": "CNY"
              },
              "provider": {
                "external_provider_id": "PRV-SIMILAR-CASE",
                "name": "Northwind Hospital",
                "provider_type": "hospital",
                "region": "Shanghai",
                "risk_tier": "High"
              }
            }"#,
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["scores"]["similar_case_score"].as_u64().unwrap() >= 90);
    let similar_cases = body["similar_cases"]
        .as_array()
        .expect("scoring response should include similar cases");
    assert_eq!(similar_cases[0]["case_id"], "KC-1001");
    assert!(similar_cases[0]["provenance_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("knowledge_cases:KC-1001")));
    let agent_prefill = &body["agent_investigation_prefill"];
    assert_eq!(agent_prefill["claim_id"], "CLM-SIMILAR-CASE");
    assert_eq!(agent_prefill["risk_score"], body["risk_score"]);
    assert_eq!(agent_prefill["rag"], "RED");
    assert_eq!(agent_prefill["top_reasons"], body["top_reasons"]);
    assert_eq!(agent_prefill["similar_case_query"]["diagnosis_code"], "J10");
    assert_eq!(
        agent_prefill["similar_case_query"]["provider_region"],
        "Shanghai"
    );
    assert!(agent_prefill["similar_case_query"]["tags"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("early_claim")));
    assert_eq!(
        agent_prefill["scheme_family"],
        "diagnosis_procedure_mismatch"
    );
    assert!(agent_prefill["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("knowledge_cases:KC-1001")));

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-SIMILAR-CASE")
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let audit_response = app.oneshot(audit_request).await.unwrap();
    assert_eq!(audit_response.status(), StatusCode::OK);
    let audit_body = to_bytes(audit_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let audit_body: serde_json::Value = serde_json::from_slice(&audit_body).unwrap();
    let scoring_event = audit_body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "scoring.completed")
        .expect("audit history should include scoring.completed");
    assert_eq!(
        scoring_event["payload"]["similar_cases"][0]["case_id"],
        "KC-1001"
    );
    assert!(scoring_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("knowledge_cases:KC-1001")));
    assert_eq!(
        scoring_event["payload"]["agent_investigation_prefill"]["claim_id"],
        "CLM-SIMILAR-CASE"
    );
}
