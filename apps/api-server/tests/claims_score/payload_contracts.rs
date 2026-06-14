use api_server::{
    app::{build_app, build_app_with_parts},
    repository::InMemoryScoringRepository,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use std::sync::Arc;
use tower::ServiceExt;

use super::support::{scoped_config, test_config, HighRiskScorer};

#[tokio::test]
async fn scores_full_payload_with_api_key() {
    let app = build_app(test_config()).unwrap();

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
async fn claim_id_scoring_is_scoped_to_authenticated_customer() {
    let repository = InMemoryScoringRepository::shared();
    let alpha_app = build_app_with_parts(
        scoped_config("customer-alpha"),
        Arc::new(HighRiskScorer),
        repository.clone(),
    );
    let beta_app = build_app_with_parts(
        scoped_config("customer-beta"),
        Arc::new(HighRiskScorer),
        repository,
    );

    let alpha_score = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-SCOPE-CLAIM-1",
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
                  "external_member_id": "MBR-SCOPE-CLAIM-1"
                },
                "policy": {
                  "external_policy_id": "POL-SCOPE-CLAIM-1",
                  "product_code": "MED",
                  "coverage_start_date": "2026-01-01",
                  "coverage_end_date": "2026-12-31",
                  "coverage_limit": "10000",
                  "currency": "CNY"
                },
                "provider": {
                  "external_provider_id": "PRV-SCOPE-CLAIM-1",
                  "name": "Scope Hospital",
                  "provider_type": "hospital",
                  "region": "SH",
                  "risk_tier": "High"
                }
              }
            }"#,
        ))
        .unwrap();
    let alpha_response = alpha_app.oneshot(alpha_score).await.unwrap();
    assert_eq!(alpha_response.status(), StatusCode::OK);

    let beta_reload = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim_id": "CLM-SCOPE-CLAIM-1"
            }"#,
        ))
        .unwrap();
    let beta_response = beta_app.oneshot(beta_reload).await.unwrap();
    assert_eq!(beta_response.status(), StatusCode::NOT_FOUND);
    let body = to_bytes(beta_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["code"], "CLAIM_NOT_FOUND");
}

#[tokio::test]
async fn scores_spec_style_top_level_full_payload() {
    let app = build_app(test_config()).unwrap();

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim_amount_peer_percentile": 95,
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
    assert_eq!(body["decision_outcome"], "pending_evidence");
    assert_eq!(body["decision_authority"], "clinical_policy_rule");
    assert_eq!(body["decision_confidence"], "deterministic");
    assert_eq!(body["appeal_or_review_required"], true);
    assert_eq!(body["reason_code"], "MEDICALLY_UNNECESSARY_SERVICE");
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
    assert!(layers.iter().all(|layer| {
        layer["evidence_refs"]
            .as_array()
            .is_some_and(|refs| !refs.is_empty())
    }));
    assert!(layers[0]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "feature_values:claim_amount_peer_percentile:v1"
        )));
    assert!(layers[1]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("rules:rule_early_claim:v1")));
    assert!(layers[3]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("model_versions:baseline_fwa:0.1.0")));
    assert!(layers[6]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "routing_policies:fwa_risk_fusion_routing:v1:pre_payment"
        )));
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
    assert_eq!(amount_ratio_feature["is_proxy"], false);
    assert_eq!(amount_ratio_feature["data_source"], "claim");
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
    assert_eq!(scoring_event["run_id"], body["run_id"]);
    assert_eq!(scoring_event["payload"]["risk_level"], "Critical");
    assert_eq!(
        scoring_event["payload"]["decision_outcome"],
        body["decision_outcome"]
    );
    assert_eq!(
        scoring_event["payload"]["decision_authority"],
        body["decision_authority"]
    );
    assert_eq!(
        scoring_event["payload"]["decision_confidence"],
        body["decision_confidence"]
    );
    assert_eq!(
        scoring_event["payload"]["appeal_or_review_required"],
        body["appeal_or_review_required"]
    );
    assert_eq!(scoring_event["payload"]["reason_code"], body["reason_code"]);
    assert_eq!(scoring_event["payload"]["confidence"], "High");
    assert_eq!(
        scoring_event["payload"]["customer_scope_id"],
        "demo-customer"
    );
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
async fn scores_full_payload_with_materialized_worker_feature_context() {
    let app = build_app(test_config()).unwrap();

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-WORKER-CONTEXT",
                "claim_amount": "8000",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "diagnosis_code": "J10",
                "items": [
                  {
                    "item_code": "IMG-BUNDLE",
                    "item_type": "procedure",
                    "description": "Imaging bundle",
                    "quantity": 1,
                    "unit_amount": "8000",
                    "total_amount": "8000"
                  }
                ],
                "member": {
                  "external_member_id": "MBR-WORKER-CONTEXT"
                },
                "policy": {
                  "external_policy_id": "POL-WORKER-CONTEXT",
                  "product_code": "MED",
                  "coverage_start_date": "2026-01-01",
                  "coverage_end_date": "2026-12-31",
                  "coverage_limit": "10000",
                  "currency": "CNY"
                },
                "provider": {
                  "external_provider_id": "PRV-WORKER-CONTEXT",
                  "name": "Worker Context Hospital",
                  "provider_type": "hospital",
                  "region": "SH",
                  "risk_tier": "Medium"
                }
              },
              "scoring_feature_context": {
                "peer_context": {
                  "claim_amount_peer_percentile": 92
                },
                "clinical_compatibility_context": {
                  "diagnosis_procedure_match_score": 0.25,
                  "data_source": "worker.icd_cpt_compatibility_reference:clinical-ref-v1"
                },
                "episode_utilization_context": {
                  "member_provider_claim_count_30d": 3,
                  "duplicate_claim_similarity_score": 0.75,
                  "procedure_frequency_peer_percentile": 88,
                  "unbundling_candidate_count": 2,
                  "data_source": "worker.episode_utilization_rollup"
                },
                "evidence_refs": [
                  "scoring_feature_contexts:CLM-WORKER-CONTEXT",
                  "unbundling:UNB-IMG:MBR-WORKER-CONTEXT|PRV-WORKER-CONTEXT"
                ]
              }
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let feature_values = body["feature_values"]
        .as_array()
        .expect("response should include feature values");
    let feature = |name: &str| {
        feature_values
            .iter()
            .find(|feature| feature["name"] == name)
            .unwrap_or_else(|| panic!("missing feature {name}"))
    };

    assert_eq!(
        feature("claim_amount_peer_percentile")["value"],
        serde_json::json!(92)
    );
    assert_eq!(
        feature("claim_amount_peer_percentile")["data_source"],
        "worker.peer_percentile_benchmark_rollup"
    );
    assert_eq!(
        feature("diagnosis_procedure_match_score")["value"],
        serde_json::json!(0.25)
    );
    assert_eq!(
        feature("diagnosis_procedure_match_score")["is_proxy"],
        false
    );
    assert_eq!(
        feature("diagnosis_procedure_match_score")["data_source"],
        "worker.icd_cpt_compatibility_reference:clinical-ref-v1"
    );
    assert_eq!(
        feature("member_provider_claim_count_30d")["value"],
        serde_json::json!(3)
    );
    assert_eq!(
        feature("duplicate_claim_similarity_score")["value"],
        serde_json::json!(0.75)
    );
    assert_eq!(
        feature("procedure_frequency_peer_percentile")["value"],
        serde_json::json!(88)
    );
    assert_eq!(
        feature("unbundling_candidate_count")["value"],
        serde_json::json!(2)
    );
    let evidence_refs = body["evidence_refs"]
        .as_array()
        .expect("response should include evidence refs");
    assert!(evidence_refs.contains(&serde_json::json!(
        "scoring_feature_contexts:CLM-WORKER-CONTEXT"
    )));
    assert!(evidence_refs.contains(&serde_json::json!(
        "unbundling:UNB-IMG:MBR-WORKER-CONTEXT|PRV-WORKER-CONTEXT"
    )));
}

#[tokio::test]
async fn scores_full_payload_with_persisted_worker_feature_context() {
    let app = build_app(test_config()).unwrap();

    let materialization_request = Request::builder()
        .method("POST")
        .uri("/api/v1/ops/scoring-feature-context-materializations")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "materialization_id": "sfc-mat-claims-score-2026-06-13",
              "actor": "worker:scoring-feature-contexts",
              "notes": "claims scoring materialized context test",
              "report_uri": "local://artifacts/scoring/scoring_feature_context_report.json",
              "report_kind": "scoring_feature_context_materialization",
              "as_of_date": "2026-06-13",
              "source_uris": {
                "claims_uri": "local://inputs/scoring-claims.json",
                "episode_rollups_uri": "local://artifacts/episode/episode_aggregation_report.json",
                "peer_benchmarks_uri": "local://artifacts/peer/peer_percentile_benchmark.json",
                "clinical_compatibility_uri": "local://artifacts/clinical/clinical_compatibility_reference_report.json",
                "unbundling_candidates_uri": "local://artifacts/unbundling/unbundling_comparator_report.json"
              },
              "claim_count": 1,
              "context_count": 1,
              "contexts": [
                {
                  "claim_id": "CLM-PERSISTED-WORKER-CONTEXT",
                  "peer_context": {
                    "claim_amount_peer_percentile": 91
                  },
                  "clinical_compatibility_context": {
                    "diagnosis_procedure_match_score": 0.28,
                    "data_source": "worker.icd_cpt_compatibility_reference:clinical-ref-v1"
                  },
                  "episode_utilization_context": {
                    "member_provider_claim_count_30d": 4,
                    "duplicate_claim_similarity_score": 0.8,
                    "procedure_frequency_peer_percentile": 89,
                    "unbundling_candidate_count": 3,
                    "data_source": "worker.episode_utilization_rollup"
                  },
                  "evidence_refs": [
                    "claims:CLM-PERSISTED-WORKER-CONTEXT",
                    "scoring_feature_contexts:CLM-PERSISTED-WORKER-CONTEXT",
                    "unbundling:UNB-IMG:MBR-PERSISTED|PRV-PERSISTED"
                  ]
                }
              ],
              "evidence_refs": [
                "scoring_feature_contexts:local://artifacts/scoring/scoring_feature_context_report.json",
                "scoring_feature_context_claim_snapshot:local://inputs/scoring-claims.json",
                "episode_rollups:local://artifacts/episode/episode_aggregation_report.json",
                "peer_benchmarks:local://artifacts/peer/peer_percentile_benchmark.json",
                "clinical_compatibility:local://artifacts/clinical/clinical_compatibility_reference_report.json",
                "unbundling_candidates:local://artifacts/unbundling/unbundling_comparator_report.json"
              ],
              "governance_boundary": "materialization persists worker-owned context only; it must not assign fraud labels, deny claims, or alter scoring policy"
            }"#,
        ))
        .unwrap();
    let materialization_response = app.clone().oneshot(materialization_request).await.unwrap();
    assert_eq!(materialization_response.status(), StatusCode::OK);

    let score_request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-PERSISTED-WORKER-CONTEXT",
                "claim_amount": "8000",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "diagnosis_code": "J10"
              },
              "items": [
                {
                  "item_code": "IMG-900",
                  "item_type": "procedure",
                  "description": "Imaging bundle",
                  "quantity": 1,
                  "unit_amount": "8000",
                  "total_amount": "8000"
                }
              ],
              "member": {
                "external_member_id": "MBR-PERSISTED"
              },
              "policy": {
                "external_policy_id": "POL-PERSISTED",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000",
                "currency": "CNY"
              },
              "provider": {
                "external_provider_id": "PRV-PERSISTED",
                "name": "Persisted Worker Context Hospital",
                "provider_type": "hospital",
                "region": "SH",
                "risk_tier": "Medium"
              }
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(score_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let feature_values = body["feature_values"]
        .as_array()
        .expect("response should include feature values");
    let feature = |name: &str| {
        feature_values
            .iter()
            .find(|feature| feature["name"] == name)
            .unwrap_or_else(|| panic!("missing feature {name}"))
    };

    assert_eq!(
        feature("claim_amount_peer_percentile")["value"],
        serde_json::json!(91)
    );
    assert_eq!(
        feature("diagnosis_procedure_match_score")["value"],
        serde_json::json!(0.28)
    );
    assert_eq!(
        feature("member_provider_claim_count_30d")["value"],
        serde_json::json!(4)
    );
    assert_eq!(
        feature("unbundling_candidate_count")["value"],
        serde_json::json!(3)
    );
    let evidence_refs = body["evidence_refs"]
        .as_array()
        .expect("response should include evidence refs");
    assert!(evidence_refs.contains(&serde_json::json!(
        "scoring_feature_contexts:CLM-PERSISTED-WORKER-CONTEXT"
    )));
    assert!(evidence_refs.contains(&serde_json::json!(
        "scoring_feature_context_materializations:sfc-mat-claims-score-2026-06-13"
    )));
    assert!(evidence_refs.contains(&serde_json::json!(
        "scoring_feature_contexts:local://artifacts/scoring/scoring_feature_context_report.json"
    )));
    assert!(evidence_refs.contains(&serde_json::json!(
        "scoring_feature_context_claim_snapshot:local://inputs/scoring-claims.json"
    )));
    assert!(evidence_refs.contains(&serde_json::json!(
        "episode_rollups:local://artifacts/episode/episode_aggregation_report.json"
    )));
    assert!(evidence_refs.contains(&serde_json::json!(
        "peer_benchmarks:local://artifacts/peer/peer_percentile_benchmark.json"
    )));
    assert!(evidence_refs.contains(&serde_json::json!(
        "clinical_compatibility:local://artifacts/clinical/clinical_compatibility_reference_report.json"
    )));
    assert!(evidence_refs.contains(&serde_json::json!(
        "unbundling_candidates:local://artifacts/unbundling/unbundling_comparator_report.json"
    )));
}

#[tokio::test]
async fn scores_full_payload_with_persisted_clinical_compatibility_reference() {
    let app = build_app(test_config()).unwrap();

    let clinical_reference_request = Request::builder()
        .method("POST")
        .uri("/api/v1/ops/clinical-compatibility-references")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "actor": "worker:build-clinical-compatibility-reference",
              "notes": "customer policy board approved clinical reference",
              "source_report_uri": "local://artifacts/clinical/clinical_compatibility_reference_report.json",
              "report_kind": "clinical_compatibility_reference",
              "reference_version": "clinical-policy-2026-06",
              "effective_date": "2026-06-01",
              "source_authority": "customer-medical-policy-board",
              "source_uri": "local://inputs/clinical-reference.json",
              "record_count": 1,
              "records": [
                {
                  "compatibility_key": "J|IMG-900",
                  "diagnosis_code_prefix": "J",
                  "procedure_code": "IMG-900",
                  "diagnosis_procedure_match_score": 0.25,
                  "data_source": "worker.icd_cpt_compatibility_reference:clinical-policy-2026-06",
                  "policy_authority_ref": "policy:clinical:J:IMG-900",
                  "rationale": "Respiratory diagnosis requires additional support for this imaging procedure.",
                  "evidence_refs": ["policy:clinical:J:IMG-900", "medical_policy:v2026-06"]
                }
              ],
              "review_tasks": [
                {
                  "task_type": "clinical_policy_review_candidate",
                  "compatibility_key": "J|IMG-900",
                  "reason": "low compatibility score should be reviewed before production activation",
                  "evidence_refs": ["policy:clinical:J:IMG-900"]
                }
              ],
              "evidence_refs": [
                "clinical_compatibility_references:local://artifacts/clinical/clinical_compatibility_reference_report.json",
                "clinical_compatibility_reference:local://inputs/clinical-reference.json",
                "clinical_policy_authority:customer-medical-policy-board"
              ],
              "governance_boundary": "clinical compatibility reference data can feed ClinicalCompatibilityFeatureContext; it must not deny claims or replace medical review without customer-approved policy authority"
            }"#,
        ))
        .unwrap();
    let clinical_reference_response = app
        .clone()
        .oneshot(clinical_reference_request)
        .await
        .unwrap();
    assert_eq!(clinical_reference_response.status(), StatusCode::OK);

    let score_request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-PERSISTED-CLINICAL-REF",
                "claim_amount": "8000",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "diagnosis_code": "J10"
              },
              "items": [
                {
                  "item_code": "IMG-900",
                  "item_type": "procedure",
                  "description": "Imaging bundle",
                  "quantity": 1,
                  "unit_amount": "8000",
                  "total_amount": "8000"
                }
              ],
              "member": {
                "external_member_id": "MBR-PERSISTED-CLINICAL-REF"
              },
              "policy": {
                "external_policy_id": "POL-PERSISTED-CLINICAL-REF",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000",
                "currency": "CNY"
              },
              "provider": {
                "external_provider_id": "PRV-PERSISTED-CLINICAL-REF",
                "name": "Persisted Clinical Reference Hospital",
                "provider_type": "hospital",
                "region": "SH",
                "risk_tier": "Medium"
              }
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(score_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let feature_values = body["feature_values"]
        .as_array()
        .expect("response should include feature values");
    let clinical_feature = feature_values
        .iter()
        .find(|feature| feature["name"] == "diagnosis_procedure_match_score")
        .expect("clinical compatibility feature should be present");

    assert_eq!(clinical_feature["value"], serde_json::json!(0.25));
    assert_eq!(clinical_feature["is_proxy"], false);
    assert_eq!(
        clinical_feature["data_source"],
        "worker.icd_cpt_compatibility_reference:clinical-policy-2026-06"
    );
    let evidence_refs = body["evidence_refs"]
        .as_array()
        .expect("response should include evidence refs");
    assert!(evidence_refs.contains(&serde_json::json!(
        "clinical_compatibility_references:local://artifacts/clinical/clinical_compatibility_reference_report.json"
    )));
    assert!(evidence_refs.contains(&serde_json::json!(
        "clinical_compatibility_reference:J|IMG-900:clinical-policy-2026-06"
    )));
    assert!(evidence_refs.contains(&serde_json::json!("policy:clinical:J:IMG-900")));
}

#[tokio::test]
async fn scores_full_payload_with_persisted_peer_benchmark() {
    let app = build_app(test_config()).unwrap();

    let peer_benchmark_request = Request::builder()
        .method("POST")
        .uri("/api/v1/ops/providers/peer-benchmarks")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "actor": "worker:build-peer-benchmarks",
              "notes": "monthly peer percentile benchmark",
              "source_report_uri": "local://artifacts/peer/peer_percentile_benchmark.json",
              "report_kind": "peer_percentile_benchmark",
              "benchmark_month": "2026-06",
              "source_uri": "local://inputs/peer-claims.json",
              "claim_count": 5,
              "peer_group_count": 1,
              "peer_groups": [
                {
                  "peer_group_key": "dental|SH|outpatient",
                  "specialty": "dental",
                  "region": "SH",
                  "service_segment": "outpatient",
                  "claim_count": 5,
                  "p25": 200.0,
                  "p50": 300.0,
                  "p75": 400.0,
                  "p90": 500.0,
                  "p99": 500.0,
                  "evidence_refs": ["peer_benchmark_groups:dental|SH|outpatient", "claims:CLM-PEER-BENCHMARK-1"]
                }
              ],
              "evidence_refs": [
                "peer_benchmarks:local://artifacts/peer/peer_percentile_benchmark.json",
                "peer_benchmark_claim_snapshot:local://inputs/peer-claims.json"
              ],
              "governance_boundary": "benchmark computes peer percentile reference data only; it must not score claims, assign labels, or change routing policy"
            }"#,
        ))
        .unwrap();
    let peer_benchmark_response = app.clone().oneshot(peer_benchmark_request).await.unwrap();
    assert_eq!(peer_benchmark_response.status(), StatusCode::OK);

    let score_request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "service_segment": "outpatient",
              "claim": {
                "external_claim_id": "CLM-PERSISTED-PEER-BENCHMARK",
                "claim_amount": "450",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "diagnosis_code": "K02"
              },
              "items": [
                {
                  "item_code": "DEN-100",
                  "item_type": "procedure",
                  "description": "Dental procedure",
                  "quantity": 1,
                  "unit_amount": "450",
                  "total_amount": "450"
                }
              ],
              "member": {
                "external_member_id": "MBR-PERSISTED-PEER"
              },
              "policy": {
                "external_policy_id": "POL-PERSISTED-PEER",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000",
                "currency": "CNY"
              },
              "provider": {
                "external_provider_id": "PRV-PERSISTED-PEER",
                "name": "Persisted Peer Dental Clinic",
                "provider_type": "clinic",
                "region": "SH",
                "risk_tier": "Medium"
              },
              "provider_profile": {
                "specialty": "dental",
                "network_status": "in_network",
                "windows": [
                  {
                    "window_days": 30,
                    "claim_count": 5,
                    "total_claim_amount": "1500",
                    "high_cost_item_ratio": 0.2,
                    "diagnosis_procedure_mismatch_rate": 0.1,
                    "peer_amount_percentile": 50,
                    "peer_frequency_percentile": 50,
                    "review_failure_count": 0,
                    "confirmed_fwa_count": 0,
                    "false_positive_count": 0
                  }
                ]
              }
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(score_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let feature_values = body["feature_values"]
        .as_array()
        .expect("response should include feature values");
    let peer_feature = feature_values
        .iter()
        .find(|feature| feature["name"] == "claim_amount_peer_percentile")
        .expect("peer percentile feature should be present");

    assert_eq!(peer_feature["value"], serde_json::json!(90));
    assert_eq!(peer_feature["is_proxy"], false);
    assert_eq!(
        peer_feature["data_source"],
        "worker.peer_percentile_benchmark_rollup"
    );
    let evidence_refs = body["evidence_refs"]
        .as_array()
        .expect("response should include evidence refs");
    assert!(evidence_refs.contains(&serde_json::json!(
        "peer_benchmark_groups:dental|SH|outpatient"
    )));
    assert!(evidence_refs.contains(&serde_json::json!(
        "peer_benchmarks:local://artifacts/peer/peer_percentile_benchmark.json"
    )));
    assert!(evidence_refs.contains(&serde_json::json!(
        "peer_benchmark_group:dental|SH|outpatient:2026-06"
    )));
}

#[tokio::test]
async fn scores_full_payload_with_persisted_episode_rollup() {
    let app = build_app(test_config()).unwrap();

    let episode_rollup_request = Request::builder()
        .method("POST")
        .uri("/api/v1/ops/providers/episode-rollups")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "actor": "worker:build-episode-aggregation",
              "notes": "daily member-provider episode rollup",
              "source_report_uri": "local://artifacts/episode/episode_aggregation_report.json",
              "report_kind": "member_provider_episode_aggregation",
              "as_of_date": "2026-06-14",
              "source_uri": "local://inputs/episode-claims.json",
              "episode_count": 1,
              "claim_count": 3,
              "episodes": [
                {
                  "episode_key": "MBR-EPISODE-1|PRV-EPISODE-1",
                  "member_id": "MBR-EPISODE-1",
                  "provider_id": "PRV-EPISODE-1",
                  "windows": [
                    {
                      "window_days": 30,
                      "claim_count": 2,
                      "total_claim_amount": 200.0,
                      "unique_procedure_code_count": 2,
                      "max_procedure_code_frequency": 2,
                      "duplicate_amount_day_count": 1
                    },
                    {
                      "window_days": 90,
                      "claim_count": 3,
                      "total_claim_amount": 450.0,
                      "unique_procedure_code_count": 3,
                      "max_procedure_code_frequency": 2,
                      "duplicate_amount_day_count": 1
                    }
                  ],
                  "evidence_refs": ["claims:CLM-EPISODE-1", "claims:CLM-EPISODE-2"]
                }
              ],
              "evidence_refs": [
                "episode_rollups:local://artifacts/episode/episode_aggregation_report.json",
                "episode_claim_snapshot:local://inputs/episode-claims.json"
              ],
              "governance_boundary": "episode aggregation computes member-provider utilization evidence only; it must not assign fraud labels, deny claims, or write rules"
            }"#,
        ))
        .unwrap();
    let episode_rollup_response = app.clone().oneshot(episode_rollup_request).await.unwrap();
    assert_eq!(episode_rollup_response.status(), StatusCode::OK);

    let score_request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-PERSISTED-EPISODE-ROLLUP",
                "claim_amount": "8000",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "diagnosis_code": "J10"
              },
              "items": [
                {
                  "item_code": "IMG-900",
                  "item_type": "procedure",
                  "description": "Imaging bundle",
                  "quantity": 1,
                  "unit_amount": "8000",
                  "total_amount": "8000"
                }
              ],
              "member": {
                "external_member_id": "MBR-EPISODE-1"
              },
              "policy": {
                "external_policy_id": "POL-PERSISTED-EPISODE",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000",
                "currency": "CNY"
              },
              "provider": {
                "external_provider_id": "PRV-EPISODE-1",
                "name": "Persisted Episode Hospital",
                "provider_type": "hospital",
                "region": "SH",
                "risk_tier": "Medium"
              }
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(score_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let feature_values = body["feature_values"]
        .as_array()
        .expect("response should include feature values");
    let feature = |name: &str| {
        feature_values
            .iter()
            .find(|feature| feature["name"] == name)
            .unwrap_or_else(|| panic!("missing feature {name}"))
    };

    assert_eq!(
        feature("member_provider_claim_count_30d")["value"],
        serde_json::json!(2)
    );
    assert_eq!(
        feature("duplicate_claim_similarity_score")["value"],
        serde_json::json!(1.0)
    );
    assert_eq!(
        feature("unbundling_candidate_count")["value"],
        serde_json::json!(0)
    );
    assert_eq!(
        feature("member_provider_claim_count_30d")["data_source"],
        "worker.episode_utilization_rollup"
    );
    let evidence_refs = body["evidence_refs"]
        .as_array()
        .expect("response should include evidence refs");
    assert!(evidence_refs.contains(&serde_json::json!("claims:CLM-EPISODE-1")));
    assert!(evidence_refs.contains(&serde_json::json!(
        "episode_rollups:local://artifacts/episode/episode_aggregation_report.json"
    )));
    assert!(evidence_refs.contains(&serde_json::json!(
        "episode_rollup:MBR-EPISODE-1|PRV-EPISODE-1:2026-06-14"
    )));
}

#[tokio::test]
async fn scores_full_payload_with_persisted_unbundling_candidates() {
    let app = build_app(test_config()).unwrap();

    let unbundling_request = Request::builder()
        .method("POST")
        .uri("/api/v1/ops/unbundling-comparator-candidates")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "actor": "worker:build-unbundling-comparator",
              "notes": "customer-approved unbundling comparator candidates",
              "source_report_uri": "local://artifacts/unbundling/unbundling_comparator_report.json",
              "report_kind": "unbundling_comparator",
              "as_of_date": "2026-06-13",
              "source_uri": "local://inputs/unbundling-reference.json",
              "rule_count": 1,
              "episode_count": 1,
              "candidate_count": 1,
              "candidates": [
                {
                  "candidate_id": "unbundling:UNBUNDLE-KNEE-001:MBR-UNB-1|PRV-UNB-1",
                  "rule_id": "UNBUNDLE-KNEE-001",
                  "episode_key": "MBR-UNB-1|PRV-UNB-1",
                  "member_id": "MBR-UNB-1",
                  "provider_id": "PRV-UNB-1",
                  "window_days": 30,
                  "bundled_code": "BUNDLE-900",
                  "matched_component_codes": ["COMP-100", "COMP-200"],
                  "claim_ids": ["CLM-UNB-1", "CLM-UNB-2"],
                  "policy_authority_ref": "policy:unbundling:BUNDLE-900",
                  "evidence_refs": ["policy:unbundling:BUNDLE-900", "claims:CLM-UNB-1", "claims:CLM-UNB-2"],
                  "recommended_review": "medical_review_candidate"
                }
              ],
              "evidence_refs": [
                "unbundling_comparator_candidates:local://artifacts/unbundling/unbundling_comparator_report.json",
                "unbundling_comparator_input:local://inputs/unbundling-reference.json"
              ],
              "governance_boundary": "unbundling comparator emits medical-review candidates from governed bundled/component code references; it must not assign fraud labels or deny claims"
            }"#,
        ))
        .unwrap();
    let unbundling_response = app.clone().oneshot(unbundling_request).await.unwrap();
    assert_eq!(unbundling_response.status(), StatusCode::OK);

    let score_request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-PERSISTED-UNBUNDLING",
                "claim_amount": "8000",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "diagnosis_code": "M17"
              },
              "items": [
                {
                  "item_code": "COMP-100",
                  "item_type": "procedure",
                  "description": "Component procedure",
                  "quantity": 1,
                  "unit_amount": "4000",
                  "total_amount": "4000"
                },
                {
                  "item_code": "COMP-200",
                  "item_type": "procedure",
                  "description": "Component procedure",
                  "quantity": 1,
                  "unit_amount": "4000",
                  "total_amount": "4000"
                }
              ],
              "member": {
                "external_member_id": "MBR-UNB-1"
              },
              "policy": {
                "external_policy_id": "POL-PERSISTED-UNBUNDLING",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000",
                "currency": "CNY"
              },
              "provider": {
                "external_provider_id": "PRV-UNB-1",
                "name": "Persisted Unbundling Hospital",
                "provider_type": "hospital",
                "region": "SH",
                "risk_tier": "Medium"
              }
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(score_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let feature_values = body["feature_values"]
        .as_array()
        .expect("response should include feature values");
    let unbundling_feature = feature_values
        .iter()
        .find(|feature| feature["name"] == "unbundling_candidate_count")
        .expect("unbundling feature should be present");

    assert_eq!(unbundling_feature["value"], serde_json::json!(1));
    assert_eq!(
        unbundling_feature["data_source"],
        "worker.unbundling_comparator"
    );
    let evidence_refs = body["evidence_refs"]
        .as_array()
        .expect("response should include evidence refs");
    assert!(evidence_refs.contains(&serde_json::json!("unbundling:MBR-UNB-1|PRV-UNB-1")));
    assert!(evidence_refs.contains(&serde_json::json!(
        "unbundling_comparator_candidates:local://artifacts/unbundling/unbundling_comparator_report.json"
    )));
    assert!(evidence_refs.contains(&serde_json::json!(
        "unbundling_comparator_candidate:unbundling:UNBUNDLE-KNEE-001:MBR-UNB-1|PRV-UNB-1:2026-06-13"
    )));
}
