use api_server::{
    app::{build_app, build_app_with_parts},
    config::AppConfig,
    repository::InMemoryScoringRepository,
};
use async_trait::async_trait;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use fwa_ml_runtime::{ModelRuntimeError, ModelScore, ModelScoreRequest, ModelScorer};
use std::sync::Arc;
use tower::ServiceExt;

fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        source_system: "tpa-demo".into(),
        database_url: "postgres://unused".into(),
        model_service_url: "http://unused".into(),
    }
}

#[tokio::test]
async fn scores_full_payload_with_api_key() {
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
                "external_claim_id": "CLM-0287",
                "claim_amount": "8000",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "diagnosis_code": "J10",
                "items": [
                  {
                    "item_code": "PROC-001",
                    "item_type": "procedure",
                    "description": "Imaging",
                    "quantity": 1,
                    "unit_amount": "8000",
                    "total_amount": "8000"
                  }
                ],
                "member": {
                  "external_member_id": "MBR-0287",
                  "dob": "1988-03-12",
                  "gender": "F"
                },
                "policy": {
                  "external_policy_id": "POL-0287",
                  "product_code": "MED",
                  "coverage_start_date": "2026-01-01",
                  "coverage_end_date": "2026-12-31",
                  "coverage_limit": "10000",
                  "currency": "CNY"
                },
                "provider": {
                  "external_provider_id": "PRV-0287",
                  "name": "Northwind Hospital",
                  "provider_type": "hospital",
                  "region": "SH",
                  "risk_tier": "High"
                }
              }
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn scores_spec_style_top_level_full_payload() {
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
                "external_claim_id": "CLM-TOP-LEVEL",
                "claim_amount": "8000",
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
                  "unit_amount": "8000",
                  "total_amount": "8000"
                }
              ],
              "member": {
                "external_member_id": "MBR-TOP-LEVEL"
              },
              "policy": {
                "external_policy_id": "POL-TOP-LEVEL",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000",
                "currency": "CNY"
              },
              "provider": {
                "external_provider_id": "PRV-TOP-LEVEL",
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
    for score_field in [
        "peer_deviation_score",
        "rule_score",
        "anomaly_score",
        "ml_score",
        "medical_reasonableness_score",
        "provider_network_score",
        "similar_case_score",
        "final_score",
    ] {
        assert!(
            body["scores"][score_field].is_number(),
            "scores should include {score_field}"
        );
    }
    assert!(body["scores"]["anomaly_score"].as_u64().unwrap() > 0);
    assert_eq!(body["risk_level"], "Critical");
    assert!(body["confidence_score"].as_u64().unwrap() >= 80);
    assert_eq!(body["confidence"], "High");
    assert!(body["routing_reason"]
        .as_str()
        .unwrap()
        .contains("医务复核"));
    let layers = body["layers"]
        .as_array()
        .expect("response should include layers");
    assert_eq!(layers.len(), 7);
    assert_eq!(layers[0]["layer_id"], "L1_PEER_BENCHMARK");
    assert_eq!(layers[1]["layer_id"], "L2_RULE_DETECTION");
    assert_eq!(layers[2]["layer_id"], "L3_UNSUPERVISED_ANOMALY");
    assert_eq!(layers[3]["layer_id"], "L4_SUPERVISED_ML");
    assert_eq!(layers[4]["layer_id"], "L5_MEDICAL_REASONABLENESS");
    assert_eq!(layers[5]["layer_id"], "L6_PROVIDER_GRAPH_RISK");
    assert_eq!(layers[6]["layer_id"], "L7_RISK_FUSION_ROUTING");
    assert_eq!(layers[6]["score"], body["scores"]["final_score"]);

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-TOP-LEVEL")
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
    assert_eq!(scoring_event["run_id"], body["run_id"]);
    assert_eq!(scoring_event["payload"]["risk_level"], "Critical");
    assert_eq!(scoring_event["payload"]["confidence"], "High");
    assert_eq!(
        scoring_event["payload"]["scores"]["final_score"],
        body["scores"]["final_score"]
    );
    assert_eq!(scoring_event["payload"]["layers"][6], body["layers"][6]);
    assert!(scoring_event["payload"]["routing_reason"]
        .as_str()
        .unwrap()
        .contains("医务复核"));
}

#[tokio::test]
async fn scores_claim_with_review_mode_and_audits_routing_policy() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "review_mode": "post_payment",
              "claim": {
                "external_claim_id": "CLM-REVIEW-MODE",
                "claim_amount": "8000",
                "currency": "CNY"
              }
            }"#,
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["review_mode"], "post_payment");

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-REVIEW-MODE")
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
    assert_eq!(scoring_event["payload"]["review_mode"], "post_payment");
}

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
    assert!(clinical["missing_evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "medical_record"));
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

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-CLINICAL-1")
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
        scoring_event["payload"]["clinical_evidence"]["review_route"],
        "medical_review"
    );
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
}

#[tokio::test]
async fn scoring_uses_only_active_rule_versions() {
    let app = build_app(test_config());

    let submit_request = Request::builder()
        .method("POST")
        .uri("/api/v1/ops/rules/rule_early_claim/submit")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from("{}"))
        .unwrap();
    let submit_response = app.clone().oneshot(submit_request).await.unwrap();
    assert_eq!(submit_response.status(), StatusCode::OK);

    let score_request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-INACTIVE-RULE",
                "claim_amount": "8000",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "diagnosis_code": "J10"
              },
              "policy": {
                "external_policy_id": "POL-INACTIVE-RULE",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              },
              "member": {
                "external_member_id": "MBR-INACTIVE-RULE"
              },
              "provider": {
                "external_provider_id": "PRV-INACTIVE-RULE",
                "name": "Northwind Hospital",
                "provider_type": "hospital",
                "region": "SH"
              }
            }"#,
        ))
        .unwrap();

    let score_response = app.oneshot(score_request).await.unwrap();
    assert_eq!(score_response.status(), StatusCode::OK);
    let body = to_bytes(score_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let alert_codes = body["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|alert| alert["alert_code"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(!alert_codes.contains("EARLY_CLAIM"));
}

#[tokio::test]
async fn rejects_claim_id_with_top_level_payload_fields() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim_id": "CLM-LOAD",
              "member": {
                "external_member_id": "MBR-LOAD"
              }
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn rejects_duplicate_nested_and_top_level_payload_fields() {
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
                "external_claim_id": "CLM-DUPLICATE-MEMBER",
                "claim_amount": "8000",
                "currency": "CNY",
                "member": {
                  "external_member_id": "MBR-NESTED"
                }
              },
              "member": {
                "external_member_id": "MBR-TOP-LEVEL"
              }
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("DUPLICATE_SCORE_PAYLOAD"));
}

#[tokio::test]
async fn rejects_missing_api_key() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"source_system":"tpa-demo","claim_id":"CLM-1"}"#,
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rejects_invalid_review_mode() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "review_mode": "ad_hoc",
              "claim": {
                "external_claim_id": "CLM-BAD-REVIEW-MODE",
                "claim_amount": "8000",
                "currency": "CNY"
              }
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("INVALID_REVIEW_MODE"));
}

#[tokio::test]
async fn exposes_openapi_schema_for_scoring_contract() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("GET")
        .uri("/api/openapi.json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let schema: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(schema["openapi"], "3.1.0");
    assert!(schema["paths"]["/api/v1/claims/score"]["post"].is_object());
    assert_eq!(
        schema["components"]["securitySchemes"]["ApiKeyAuth"]["name"],
        "x-api-key"
    );
    let one_of = schema["components"]["schemas"]["ScoreClaimRequest"]["oneOf"]
        .as_array()
        .expect("request schema oneOf");
    assert_eq!(one_of.len(), 2);
    let claim_id_mode = &schema["components"]["schemas"]["ClaimIdScoreClaimRequest"];
    assert_eq!(
        claim_id_mode["properties"]["review_mode"]["enum"],
        serde_json::json!(["pre_payment", "post_payment", "both"])
    );
    for field in [
        "claim",
        "items",
        "member",
        "policy",
        "provider",
        "documents",
        "provider_profile",
        "provider_relationships",
    ] {
        assert!(
            claim_id_mode["not"]["anyOf"]
                .as_array()
                .expect("claim id mode forbidden payload fields")
                .iter()
                .any(|schema| schema["required"][0] == field),
            "claim_id mode should forbid {field}"
        );
    }

    let response_properties = &schema["components"]["schemas"]["ScoreClaimResponse"]["properties"];
    for field in [
        "run_id",
        "audit_id",
        "review_mode",
        "risk_score",
        "rag",
        "risk_level",
        "recommended_action",
        "confidence_score",
        "confidence",
        "routing_reason",
        "top_reasons",
        "evidence_refs",
        "clinical_evidence",
        "provider_profile",
        "provider_relationships",
        "similar_cases",
        "layers",
    ] {
        assert!(response_properties[field].is_object(), "missing {field}");
    }
    assert_eq!(
        response_properties["review_mode"]["enum"],
        serde_json::json!(["pre_payment", "post_payment", "both"])
    );
    assert!(schema["components"]["schemas"]["ClinicalEvidenceAssessment"].is_object());
    assert!(schema["components"]["schemas"]["ProviderProfileAssessment"].is_object());
    assert!(schema["components"]["schemas"]["ProviderRelationshipGraphAssessment"].is_object());

    let score_required = schema["components"]["schemas"]["ScoreBreakdown"]["required"]
        .as_array()
        .expect("score required fields");
    for score_field in [
        "peer_deviation_score",
        "rule_score",
        "anomaly_score",
        "ml_score",
        "medical_reasonableness_score",
        "provider_network_score",
        "similar_case_score",
        "final_score",
    ] {
        assert!(
            score_required.iter().any(|field| field == score_field),
            "ScoreBreakdown should require {score_field}"
        );
    }
}

#[tokio::test]
async fn scores_existing_claim_after_full_payload_upsert() {
    let app = build_app(test_config());

    let first = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-LOAD",
                "claim_amount": "8000",
                "currency": "CNY"
              }
            }"#,
        ))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(first).await.unwrap().status(),
        StatusCode::OK
    );

    let second = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{"source_system":"tpa-demo","claim_id":"CLM-LOAD"}"#,
        ))
        .unwrap();
    assert_eq!(app.oneshot(second).await.unwrap().status(), StatusCode::OK);
}

#[derive(Debug)]
struct InvalidResponseScorer;

#[async_trait]
impl ModelScorer for InvalidResponseScorer {
    async fn score(&self, _request: ModelScoreRequest) -> Result<ModelScore, ModelRuntimeError> {
        Err(ModelRuntimeError::InvalidResponse("missing score".into()))
    }
}

#[tokio::test]
async fn returns_invalid_model_response_code() {
    let app = build_app_with_parts(
        test_config(),
        Arc::new(InvalidResponseScorer),
        InMemoryScoringRepository::shared(),
    );

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-BAD-MODEL",
                "claim_amount": "8000",
                "currency": "CNY"
              }
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("MODEL_RESPONSE_INVALID"));
}
