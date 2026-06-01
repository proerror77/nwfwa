use api_server::{
    app::{build_app, build_app_with_parts},
    config::AppConfig,
    repository::{InMemoryScoringRepository, ModelVersionRecord, SharedRepository},
};
use async_trait::async_trait;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use fwa_core::RecommendedAction;
use fwa_ml_runtime::{ModelRuntimeError, ModelScore, ModelScoreRequest, ModelScorer};
use fwa_rules::{Condition, Rule, RuleAction};
use fwa_scoring::{ConfidenceThresholds, RiskThresholds, RoutingPolicy};
use std::sync::Arc;
use tower::ServiceExt;

fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        source_system: "tpa-demo".into(),
        database_url: "postgres://unused".into(),
        model_service_url: "heuristic://local".into(),
    }
}

async fn activate_candidate_model(repository: SharedRepository) {
    repository
        .update_model_status("baseline_fwa", "0.1.0", "approved")
        .await
        .expect("default model status update");
    repository
        .save_model_version(ModelVersionRecord {
            model_key: "baseline_fwa".into(),
            version: "0.2.0-active".into(),
            model_type: "baseline_classifier".into(),
            runtime_kind: "python_http".into(),
            execution_provider: "cpu".into(),
            status: "active".into(),
            review_mode: "both".into(),
            artifact_uri: Some("s3://fwa-models/baseline_fwa/0.2.0-active/model.onnx".into()),
            endpoint_url: Some("http://127.0.0.1:8001/score/baseline_fwa/0.2.0-active".into()),
        })
        .await
        .expect("candidate model save");
}

async fn activate_post_payment_model(repository: SharedRepository) {
    repository
        .update_model_status("baseline_fwa", "0.1.0", "approved")
        .await
        .expect("default model status update");
    repository
        .save_model_version(ModelVersionRecord {
            model_key: "baseline_fwa".into(),
            version: "0.2.0-post-payment".into(),
            model_type: "baseline_classifier".into(),
            runtime_kind: "python_http".into(),
            execution_provider: "cpu".into(),
            status: "active".into(),
            review_mode: "post_payment".into(),
            artifact_uri: Some("s3://fwa-models/baseline_fwa/0.2.0-post-payment/model.onnx".into()),
            endpoint_url: Some(
                "http://127.0.0.1:8001/score/baseline_fwa/0.2.0-post-payment".into(),
            ),
        })
        .await
        .expect("post-payment model save");
}

async fn activate_post_payment_rule(repository: SharedRepository) {
    let rule_id = "rule_post_payment_limit_usage";
    repository
        .save_rule_candidate(
            Rule {
                rule_id: rule_id.into(),
                version: 1,
                name: "Post-payment limit usage".into(),
                review_mode: "post_payment".into(),
                scheme_family: Some("early_high_value_claim".into()),
                conditions: vec![Condition {
                    field: "claim_amount_to_limit_ratio".into(),
                    operator: ">=".into(),
                    value: serde_json::json!(0.7),
                }],
                action: RuleAction {
                    score: 30,
                    alert_code: "POST_PAYMENT_LIMIT_USAGE".into(),
                    recommended_action: RecommendedAction::PostPaymentAudit,
                    reason: "赔后规则仅用于高额度使用审计".into(),
                },
            },
            "rules-ops".into(),
        )
        .await
        .expect("post-payment rule save");
    repository
        .update_rule_status(rule_id, "active")
        .await
        .expect("post-payment rule activation");
}

struct HighRiskScorer;

#[async_trait]
impl ModelScorer for HighRiskScorer {
    async fn score(&self, request: ModelScoreRequest) -> Result<ModelScore, ModelRuntimeError> {
        Ok(ModelScore {
            model_key: request.model_key,
            model_version: request.model_version,
            runtime_kind: "test".into(),
            execution_provider: "cpu".into(),
            score: 95,
            label: "HIGH_RISK".into(),
            explanations: vec![],
            metadata: serde_json::json!({}),
            latency_ms: 0,
        })
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
    assert_eq!(body["model_score"]["model_key"], "baseline_fwa");
    assert_eq!(body["model_score"]["model_version"], "0.1.0");
    assert!(!body["model_score"]["explanations"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(body["model_score"]["metadata"]["fraud_probability"].is_number());
    assert!(body["model_score"]["metadata"]["abuse_probability"].is_number());
    assert!(body["model_score"]["metadata"]["waste_probability"].is_number());
    assert_eq!(body["risk_level"], "Critical");
    assert!(body["confidence_score"].as_u64().unwrap() >= 80);
    assert_eq!(body["confidence"], "High");
    assert!(body["routing_reason"]
        .as_str()
        .unwrap()
        .contains("医务复核"));
    assert_eq!(
        body["routing_policy"]["policy_id"],
        "fwa_risk_fusion_routing"
    );
    assert_eq!(body["routing_policy"]["version"], 1);
    assert_eq!(body["routing_policy"]["review_mode"], "pre_payment");
    assert_eq!(
        body["routing_policy"]["risk_thresholds"]["critical_min"],
        85
    );
    assert_eq!(
        body["routing_policy"]["confidence_thresholds"]["low_confidence_below"],
        60
    );
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
    let evidence_refs = body["evidence_refs"]
        .as_array()
        .expect("response should include evidence refs");
    assert!(evidence_refs.contains(&serde_json::json!("rule_runs:EARLY_HIGH_AMOUNT")));
    assert!(evidence_refs.contains(&serde_json::json!("model_scores:baseline_fwa")));
    assert!(evidence_refs.contains(&serde_json::json!("model_versions:baseline_fwa:0.1.0")));
    let feature_values = body["feature_values"]
        .as_array()
        .expect("response should include feature values");
    let amount_ratio_feature = feature_values
        .iter()
        .find(|feature| feature["name"] == "claim_amount_to_limit_ratio")
        .expect("feature trace should include claim amount ratio");
    assert_eq!(amount_ratio_feature["version"], 1);
    assert_eq!(amount_ratio_feature["value"], serde_json::json!(0.8));
    assert!(amount_ratio_feature["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "entity_type": "claim",
            "entity_id": "CLM-TOP-LEVEL",
            "field": "claim_amount"
        })));

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
    assert_eq!(
        scoring_event["payload"]["routing_policy"],
        body["routing_policy"]
    );
    assert_eq!(
        scoring_event["payload"]["feature_values"],
        body["feature_values"]
    );
    assert!(scoring_event["payload"]["routing_reason"]
        .as_str()
        .unwrap()
        .contains("医务复核"));
    assert!(scoring_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("rule_runs:EARLY_HIGH_AMOUNT")));
    assert!(scoring_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("model_scores:baseline_fwa")));
    assert!(scoring_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("model_versions:baseline_fwa:0.1.0")));
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
    assert_eq!(body["recommended_action"], "PostPaymentAudit");
    assert!(body["routing_reason"].as_str().unwrap().contains("赔后"));

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
    assert_eq!(
        scoring_event["payload"]["recommended_action"],
        "PostPaymentAudit"
    );
}

#[tokio::test]
async fn scoring_uses_active_routing_policy_from_registry() {
    let repository = InMemoryScoringRepository::shared_with_routing_policies(vec![RoutingPolicy {
        policy_id: "custom_prepay_routing".into(),
        version: 3,
        review_mode: "pre_payment".into(),
        risk_thresholds: RiskThresholds {
            low_max: 0,
            medium_min: 1,
            high_min: 1,
            critical_min: 1,
        },
        confidence_thresholds: ConfidenceThresholds {
            low_confidence_below: 60,
            high_confidence_min: 80,
        },
        provider_review_threshold: 70,
    }]);
    let app = build_app_with_parts(test_config(), Arc::new(HighRiskScorer), repository);

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "review_mode": "pre_payment",
              "claim": {
                "external_claim_id": "CLM-CUSTOM-POLICY",
                "claim_amount": "8000",
                "currency": "CNY"
              }
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["routing_policy"]["policy_id"], "custom_prepay_routing");
    assert_eq!(body["routing_policy"]["version"], 3);
    assert_eq!(body["risk_level"], "Critical");
    assert_eq!(body["recommended_action"], "EscalateInvestigation");
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
    assert_eq!(body["scores"]["medical_reasonableness_score"], 100);

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
    assert_eq!(
        scoring_event["payload"]["scores"]["medical_reasonableness_score"],
        body["scores"]["medical_reasonableness_score"]
    );
    assert!(scoring_event["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item.as_str().unwrap().contains("clinical_evidence")));
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
        .body(Body::from(
            r#"{"evidence_refs":["rules:rule_early_claim:v1"]}"#,
        ))
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
async fn scoring_filters_active_rules_by_review_mode() {
    let repository = InMemoryScoringRepository::shared();
    activate_post_payment_rule(repository.clone()).await;
    let app = build_app_with_parts(test_config(), Arc::new(RequestEchoScorer), repository);

    let pre_payment_request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-PRE-PAYMENT-RULE",
                "claim_amount": "8000",
                "currency": "CNY"
              },
              "policy": {
                "external_policy_id": "POL-PRE-PAYMENT-RULE",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              },
              "member": {
                "external_member_id": "MBR-PRE-PAYMENT-RULE"
              },
              "provider": {
                "external_provider_id": "PRV-PRE-PAYMENT-RULE",
                "name": "Northwind Hospital",
                "provider_type": "hospital",
                "region": "SH"
              }
            }"#,
        ))
        .unwrap();

    let pre_payment_response = app.clone().oneshot(pre_payment_request).await.unwrap();
    assert_eq!(pre_payment_response.status(), StatusCode::OK);
    let pre_payment_body = to_bytes(pre_payment_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let pre_payment_body: serde_json::Value = serde_json::from_slice(&pre_payment_body).unwrap();
    let pre_payment_alert_codes = pre_payment_body["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|alert| alert["alert_code"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(!pre_payment_alert_codes.contains("POST_PAYMENT_LIMIT_USAGE"));

    let post_payment_request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "review_mode": "post_payment",
              "claim": {
                "external_claim_id": "CLM-POST-PAYMENT-RULE",
                "claim_amount": "8000",
                "currency": "CNY"
              },
              "policy": {
                "external_policy_id": "POL-POST-PAYMENT-RULE",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              },
              "member": {
                "external_member_id": "MBR-POST-PAYMENT-RULE"
              },
              "provider": {
                "external_provider_id": "PRV-POST-PAYMENT-RULE",
                "name": "Northwind Hospital",
                "provider_type": "hospital",
                "region": "SH"
              }
            }"#,
        ))
        .unwrap();

    let post_payment_response = app.oneshot(post_payment_request).await.unwrap();
    assert_eq!(post_payment_response.status(), StatusCode::OK);
    let post_payment_body = to_bytes(post_payment_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let post_payment_body: serde_json::Value = serde_json::from_slice(&post_payment_body).unwrap();
    let post_payment_alert_codes = post_payment_body["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|alert| alert["alert_code"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(post_payment_alert_codes.contains("POST_PAYMENT_LIMIT_USAGE"));
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
async fn rejects_both_review_mode_for_scoring_contract() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "review_mode": "both",
              "claim": {
                "external_claim_id": "CLM-BOTH-REVIEW-MODE",
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
    assert!(body.contains("pre_payment, post_payment"));
}

#[tokio::test]
async fn rejects_source_system_mismatch_for_authenticated_scoring() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "untrusted-tpa",
              "claim_id": "CLM-LOAD"
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("SOURCE_SYSTEM_MISMATCH"));
}

#[tokio::test]
async fn rejects_blank_scoring_identity_fields() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": " ",
              "claim_id": "CLM-LOAD"
            }"#,
        ))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("INVALID_SCORE_REQUEST"));
    assert!(body.contains("source_system"));

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim_id": " "
            }"#,
        ))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("INVALID_SCORE_REQUEST"));
    assert!(body.contains("claim_id"));

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": " ",
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
    assert!(body.contains("INVALID_SCORE_REQUEST"));
    assert!(body.contains("claim.external_claim_id"));
}

#[tokio::test]
async fn rejects_invalid_provider_risk_signal_ranges() {
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
                "external_claim_id": "CLM-BAD-PROVIDER-PROFILE",
                "claim_amount": "8000",
                "currency": "CNY"
              },
              "provider_profile": {
                "windows": [
                  {
                    "window_days": 7,
                    "claim_count": 12,
                    "total_claim_amount": "42000",
                    "high_cost_item_ratio": 0.4,
                    "diagnosis_procedure_mismatch_rate": 0.2,
                    "peer_amount_percentile": 80,
                    "peer_frequency_percentile": 75,
                    "review_failure_count": 0,
                    "confirmed_fwa_count": 0,
                    "false_positive_count": 0
                  }
                ]
              }
            }"#,
        ))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("INVALID_SCORE_REQUEST"));
    assert!(body.contains("provider_profile.windows.window_days"));

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-BAD-PROVIDER-RATIO",
                "claim_amount": "8000",
                "currency": "CNY"
              },
              "provider_profile": {
                "windows": [
                  {
                    "window_days": 90,
                    "claim_count": 12,
                    "total_claim_amount": "42000",
                    "high_cost_item_ratio": 1.2,
                    "diagnosis_procedure_mismatch_rate": 0.2,
                    "peer_amount_percentile": 80,
                    "peer_frequency_percentile": 75,
                    "review_failure_count": 0,
                    "confirmed_fwa_count": 0,
                    "false_positive_count": 0
                  }
                ]
              }
            }"#,
        ))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("provider_profile.windows.high_cost_item_ratio"));

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-BAD-GRAPH-SIGNAL",
                "claim_amount": "8000",
                "currency": "CNY"
              },
              "provider_relationships": {
                "high_risk_neighbor_ratio": 0.2,
                "provider_patient_overlap_score": 0.3,
                "referral_concentration_score": 1.4,
                "connected_confirmed_fwa_count": 2,
                "network_component_risk_score": 101,
                "evidence_refs": ["relationship_edges:PRV-1"]
              }
            }"#,
        ))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("provider_relationships.referral_concentration_score"));

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-BAD-GRAPH-EVIDENCE",
                "claim_amount": "8000",
                "currency": "CNY"
              },
              "provider_relationships": {
                "high_risk_neighbor_ratio": 0.2,
                "provider_patient_overlap_score": 0.3,
                "connected_confirmed_fwa_count": 2,
                "evidence_refs": [" "]
              }
            }"#,
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("provider_relationships.evidence_refs"));
}

#[tokio::test]
async fn rejects_invalid_scoring_business_fields() {
    let app = build_app(test_config());
    let cases = [
        (
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-ZERO-AMOUNT",
                "claim_amount": "0",
                "currency": "CNY"
              }
            }"#,
            "claim.claim_amount",
        ),
        (
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-ZERO-QUANTITY",
                "claim_amount": "8000",
                "currency": "CNY",
                "items": [
                  {
                    "item_code": "MRI",
                    "item_type": "procedure",
                    "description": "MRI scan",
                    "quantity": 0,
                    "unit_amount": "8000",
                    "total_amount": "8000",
                    "currency": "CNY"
                  }
                ]
              }
            }"#,
            "item.quantity",
        ),
        (
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-NEGATIVE-ITEM",
                "claim_amount": "8000",
                "currency": "CNY",
                "items": [
                  {
                    "item_code": "MRI",
                    "item_type": "procedure",
                    "description": "MRI scan",
                    "quantity": 1,
                    "unit_amount": "-1",
                    "total_amount": "8000",
                    "currency": "CNY"
                  }
                ]
              }
            }"#,
            "item.unit_amount",
        ),
        (
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-NEGATIVE-TOTAL",
                "claim_amount": "8000",
                "currency": "CNY",
                "items": [
                  {
                    "item_code": "MRI",
                    "item_type": "procedure",
                    "description": "MRI scan",
                    "quantity": 1,
                    "unit_amount": "8000",
                    "total_amount": "-1",
                    "currency": "CNY"
                  }
                ]
              }
            }"#,
            "item.total_amount",
        ),
        (
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-ZERO-LIMIT",
                "claim_amount": "8000",
                "currency": "CNY",
                "policy": {
                  "external_policy_id": "POL-ZERO-LIMIT",
                  "coverage_start_date": "2026-01-01",
                  "coverage_end_date": "2026-12-31",
                  "coverage_limit": "0"
                }
              }
            }"#,
            "policy.coverage_limit",
        ),
        (
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-BAD-DATES",
                "claim_amount": "8000",
                "currency": "CNY",
                "policy": {
                  "external_policy_id": "POL-BAD-DATES",
                  "coverage_start_date": "2026-12-31",
                  "coverage_end_date": "2026-01-01",
                  "coverage_limit": "10000"
                }
              }
            }"#,
            "policy.coverage_end_date",
        ),
        (
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-NEGATIVE-PROVIDER-TOTAL",
                "claim_amount": "8000",
                "currency": "CNY"
              },
              "provider_profile": {
                "windows": [
                  {
                    "window_days": 90,
                    "claim_count": 12,
                    "total_claim_amount": "-1",
                    "high_cost_item_ratio": 0.4,
                    "diagnosis_procedure_mismatch_rate": 0.2,
                    "peer_amount_percentile": 80,
                    "peer_frequency_percentile": 75,
                    "review_failure_count": 0,
                    "confirmed_fwa_count": 0,
                    "false_positive_count": 0
                  }
                ]
              }
            }"#,
            "provider_profile.windows.total_claim_amount",
        ),
        (
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-EMPTY-PROVIDER-WINDOWS",
                "claim_amount": "8000",
                "currency": "CNY"
              },
              "provider_profile": {
                "windows": []
              }
            }"#,
            "provider_profile.windows",
        ),
    ];

    for (payload, field) in cases {
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/claims/score")
            .header("content-type", "application/json")
            .header("x-api-key", "dev-secret")
            .body(Body::from(payload))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{field}");
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("INVALID_SCORE_REQUEST"), "{field}: {body}");
        assert!(body.contains(field), "{field}: {body}");
    }
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
        serde_json::json!(["pre_payment", "post_payment"])
    );
    assert_eq!(claim_id_mode["properties"]["source_system"]["minLength"], 1);
    assert!(claim_id_mode["properties"]["source_system"]["description"]
        .as_str()
        .unwrap()
        .contains("authenticated API key"));
    assert_eq!(claim_id_mode["properties"]["claim_id"]["minLength"], 1);
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
    let full_payload_mode = &schema["components"]["schemas"]["FullPayloadScoreClaimRequest"];
    assert_eq!(
        full_payload_mode["properties"]["review_mode"]["enum"],
        serde_json::json!(["pre_payment", "post_payment"])
    );
    assert_eq!(
        full_payload_mode["properties"]["source_system"]["minLength"],
        1
    );
    assert!(
        full_payload_mode["properties"]["source_system"]["description"]
            .as_str()
            .unwrap()
            .contains("authenticated API key")
    );
    for (schema_name, fields) in [
        (
            "FullClaimPayload",
            &["external_claim_id", "currency", "diagnosis_code"][..],
        ),
        (
            "ClaimItemPayload",
            &["item_code", "item_type", "description", "currency"][..],
        ),
        ("MemberPayload", &["external_member_id", "gender"][..]),
        (
            "PolicyPayload",
            &["external_policy_id", "product_code", "currency"][..],
        ),
        (
            "ProviderPayload",
            &["external_provider_id", "name", "provider_type", "region"][..],
        ),
        (
            "DocumentPayload",
            &["external_document_id", "document_type"][..],
        ),
    ] {
        for field in fields {
            assert_eq!(
                schema["components"]["schemas"][schema_name]["properties"][*field]["minLength"], 1,
                "missing {schema_name}.{field} minLength"
            );
        }
    }
    assert_eq!(
        schema["components"]["schemas"]["DocumentPayload"]["properties"]["linked_item_codes"]
            ["items"]["minLength"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["ClaimItemPayload"]["properties"]["quantity"]["minimum"],
        1
    );
    assert!(
        schema["components"]["schemas"]["FullClaimPayload"]["properties"]["claim_amount"]
            ["description"]
            .as_str()
            .unwrap()
            .contains("Positive decimal")
    );
    assert!(
        schema["components"]["schemas"]["PolicyPayload"]["properties"]["coverage_limit"]
            ["description"]
            .as_str()
            .unwrap()
            .contains("Positive decimal")
    );
    assert!(
        schema["components"]["schemas"]["ProviderProfileWindowPayload"]["properties"]
            ["total_claim_amount"]["description"]
            .as_str()
            .unwrap()
            .contains("Non-negative decimal")
    );
    assert_eq!(
        schema["components"]["schemas"]["ProviderProfilePayload"]["properties"]["windows"]
            ["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["ProviderProfileWindowPayload"]["properties"]
            ["window_days"]["enum"],
        serde_json::json!([30, 90, 180])
    );
    for field in ["high_cost_item_ratio", "diagnosis_procedure_mismatch_rate"] {
        assert_eq!(
            schema["components"]["schemas"]["ProviderProfileWindowPayload"]["properties"][field]
                ["minimum"],
            0,
            "missing ProviderProfileWindowPayload.{field} minimum"
        );
        assert_eq!(
            schema["components"]["schemas"]["ProviderProfileWindowPayload"]["properties"][field]
                ["maximum"],
            1,
            "missing ProviderProfileWindowPayload.{field} maximum"
        );
    }
    for field in ["peer_amount_percentile", "peer_frequency_percentile"] {
        assert_eq!(
            schema["components"]["schemas"]["ProviderProfileWindowPayload"]["properties"][field]
                ["maximum"],
            100,
            "missing ProviderProfileWindowPayload.{field} maximum"
        );
    }
    for field in [
        "high_risk_neighbor_ratio",
        "provider_patient_overlap_score",
        "referral_concentration_score",
    ] {
        assert_eq!(
            schema["components"]["schemas"]["ProviderRelationshipGraphPayload"]["properties"]
                [field]["minimum"],
            0,
            "missing ProviderRelationshipGraphPayload.{field} minimum"
        );
        assert_eq!(
            schema["components"]["schemas"]["ProviderRelationshipGraphPayload"]["properties"]
                [field]["maximum"],
            1,
            "missing ProviderRelationshipGraphPayload.{field} maximum"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["ProviderRelationshipGraphPayload"]["properties"]
            ["network_component_risk_score"]["maximum"],
        100
    );

    let response_properties = &schema["components"]["schemas"]["ScoreClaimResponse"]["properties"];
    for field in [
        "run_id",
        "audit_id",
        "claim_id",
        "review_mode",
        "risk_score",
        "rag",
        "risk_level",
        "recommended_action",
        "confidence_score",
        "confidence",
        "routing_reason",
        "routing_policy",
        "scores",
        "model_score",
        "top_reasons",
        "evidence_refs",
        "clinical_evidence",
        "provider_profile",
        "provider_relationships",
        "similar_cases",
        "feature_values",
        "layers",
    ] {
        assert!(response_properties[field].is_object(), "missing {field}");
    }
    assert_eq!(
        response_properties["review_mode"]["enum"],
        serde_json::json!(["pre_payment", "post_payment"])
    );
    assert_eq!(
        response_properties["recommended_action"]["enum"],
        serde_json::json!([
            "StandardProcessing",
            "QaSample",
            "ManualReview",
            "RequestEvidence",
            "EscalateInvestigation",
            "PostPaymentAudit",
            "ProviderReview",
            "RecoveryReview"
        ])
    );
    assert_eq!(
        response_properties["routing_policy"]["$ref"],
        "#/components/schemas/RoutingPolicy"
    );
    assert_eq!(
        response_properties["model_score"]["$ref"],
        "#/components/schemas/ModelScore"
    );
    assert_eq!(
        schema["components"]["schemas"]["ModelScore"]["properties"]["metadata"]["properties"]
            ["fraud_probability"]["maximum"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["ModelScore"]["properties"]["explanations"]["items"]
            ["$ref"],
        "#/components/schemas/ModelExplanation"
    );
    assert_eq!(
        schema["components"]["schemas"]["RoutingPolicy"]["required"],
        serde_json::json!([
            "policy_id",
            "version",
            "review_mode",
            "risk_thresholds",
            "confidence_thresholds",
            "provider_review_threshold"
        ])
    );
    let response_required = schema["components"]["schemas"]["ScoreClaimResponse"]["required"]
        .as_array()
        .expect("score response required fields");
    for field in [
        "run_id",
        "audit_id",
        "claim_id",
        "risk_score",
        "rag",
        "recommended_action",
        "scores",
        "model_score",
        "top_reasons",
        "layers",
        "evidence_refs",
    ] {
        assert!(
            response_required.iter().any(|required| required == field),
            "ScoreClaimResponse should require {field}"
        );
    }
    assert_eq!(response_properties["layers"]["minItems"], 7);
    assert_eq!(response_properties["layers"]["maxItems"], 7);
    assert_eq!(response_properties["evidence_refs"]["minItems"], 1);
    assert_eq!(response_properties["top_reasons"]["items"]["minLength"], 1);
    assert_eq!(
        response_properties["layers"]["items"]["$ref"],
        "#/components/schemas/DetectionLayerScore"
    );
    assert!(schema["components"]["schemas"]["ClinicalEvidenceAssessment"].is_object());
    assert!(schema["components"]["schemas"]["ProviderProfileAssessment"].is_object());
    assert!(schema["components"]["schemas"]["ProviderRelationshipGraphAssessment"].is_object());
    assert_eq!(
        response_properties["feature_values"]["items"]["$ref"],
        "#/components/schemas/FeatureValue"
    );
    assert_eq!(
        schema["components"]["schemas"]["FeatureValue"]["properties"]["evidence_refs"]["items"]
            ["$ref"],
        "#/components/schemas/EvidenceRef"
    );
    let layer_schema = &schema["components"]["schemas"]["DetectionLayerScore"];
    assert_eq!(
        layer_schema["required"],
        serde_json::json!(["layer_id", "name", "score", "status", "reason"])
    );
    assert_eq!(
        layer_schema["properties"]["layer_id"]["enum"],
        serde_json::json!([
            "L1_PEER_BENCHMARK",
            "L2_RULE_DETECTION",
            "L3_UNSUPERVISED_ANOMALY",
            "L4_SUPERVISED_ML",
            "L5_MEDICAL_REASONABLENESS",
            "L6_PROVIDER_GRAPH_RISK",
            "L7_RISK_FUSION_ROUTING"
        ])
    );
    assert_eq!(layer_schema["properties"]["score"]["minimum"], 0);
    assert_eq!(layer_schema["properties"]["score"]["maximum"], 100);

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
struct RequestEchoScorer;

#[async_trait]
impl ModelScorer for RequestEchoScorer {
    async fn score(&self, request: ModelScoreRequest) -> Result<ModelScore, ModelRuntimeError> {
        Ok(ModelScore {
            model_key: request.model_key,
            model_version: request.model_version,
            runtime_kind: "test_echo".into(),
            execution_provider: "cpu".into(),
            score: 72,
            label: "ACTIVE_MODEL_USED".into(),
            explanations: vec![],
            metadata: serde_json::json!({
                "endpoint_url": request.endpoint_url,
            }),
            latency_ms: 0,
        })
    }
}

#[tokio::test]
async fn scoring_uses_active_model_version_from_model_registry() {
    let repository = InMemoryScoringRepository::shared();
    activate_candidate_model(repository.clone()).await;
    let app = build_app_with_parts(test_config(), Arc::new(RequestEchoScorer), repository);

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-ACTIVE-MODEL",
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
    assert_eq!(body["scores"]["ml_score"], 72);
    assert_eq!(body["model_score"]["score"], 72);
    assert_eq!(body["model_score"]["model_version"], "0.2.0-active");
    assert_eq!(body["model_score"]["runtime_kind"], "test_echo");
    assert!(body["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "model_versions:baseline_fwa:0.2.0-active"
        )));

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-ACTIVE-MODEL")
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
        scoring_event["payload"]["model_score"]["model_version"],
        "0.2.0-active"
    );
    assert_eq!(
        scoring_event["payload"]["model_score"]["metadata"]["endpoint_url"],
        "http://127.0.0.1:8001/score/baseline_fwa/0.2.0-active"
    );
    assert!(scoring_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "model_versions:baseline_fwa:0.2.0-active"
        )));
}

#[tokio::test]
async fn scoring_filters_active_model_by_review_mode() {
    let repository = InMemoryScoringRepository::shared();
    activate_post_payment_model(repository.clone()).await;
    let app = build_app_with_parts(test_config(), Arc::new(RequestEchoScorer), repository);

    let pre_payment_request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-PRE-PAYMENT-MODEL",
                "claim_amount": "8000",
                "currency": "CNY"
              }
            }"#,
        ))
        .unwrap();

    let pre_payment_response = app.clone().oneshot(pre_payment_request).await.unwrap();
    assert_eq!(pre_payment_response.status(), StatusCode::CONFLICT);

    let post_payment_request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "review_mode": "post_payment",
              "claim": {
                "external_claim_id": "CLM-POST-PAYMENT-MODEL",
                "claim_amount": "8000",
                "currency": "CNY"
              }
            }"#,
        ))
        .unwrap();

    let post_payment_response = app.clone().oneshot(post_payment_request).await.unwrap();
    assert_eq!(post_payment_response.status(), StatusCode::OK);
    let body = to_bytes(post_payment_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["scores"]["ml_score"], 72);

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-POST-PAYMENT-MODEL")
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
        scoring_event["payload"]["model_score"]["model_version"],
        "0.2.0-post-payment"
    );
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

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("MODEL_RESPONSE_INVALID"));

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-BAD-MODEL")
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let audit_response = app.oneshot(audit_request).await.unwrap();
    assert_eq!(audit_response.status(), StatusCode::OK);
    let audit_body = to_bytes(audit_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let audit_body: serde_json::Value = serde_json::from_slice(&audit_body).unwrap();
    let failed_event = audit_body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "scoring.failed")
        .expect("failed model scoring should be audited");
    assert_eq!(failed_event["event_status"], "failed");
    assert_eq!(failed_event["summary"], "model scoring failed");
    assert_eq!(failed_event["payload"]["claim_id"], "CLM-BAD-MODEL");
    assert_eq!(failed_event["payload"]["review_mode"], "pre_payment");
    assert!(failed_event["payload"]["error"]
        .as_str()
        .unwrap()
        .contains("missing score"));
    assert!(!failed_event["evidence_refs"].as_array().unwrap().is_empty());
}
