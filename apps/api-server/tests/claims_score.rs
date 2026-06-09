use api_server::{
    app::{build_app, build_app_with_parts},
    repository::InMemoryScoringRepository,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use fwa_scoring::{ConfidenceThresholds, RiskThresholds, RoutingPolicy};
use std::{fs, path::Path, sync::Arc};
use tower::ServiceExt;

#[path = "claims_score/canonical_inbox.rs"]
mod canonical_inbox;
#[path = "claims_score/model_registry.rs"]
mod model_registry;
#[path = "claims_score/openapi_contract.rs"]
mod openapi_contract;
#[path = "claims_score/payload_contracts.rs"]
mod payload_contracts;
#[path = "claims_score/support.rs"]
mod support;

use support::{
    activate_pending_evidence_rule, activate_post_payment_rule, test_config, HighRiskScorer,
    RequestEchoScorer,
};

#[tokio::test]
async fn approved_demo_hard_deny_rule_auto_denies_coverage_ineligible_claim() {
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
                "external_claim_id": "CLM-BEFORE-COVERAGE",
                "claim_amount": "1200",
                "currency": "CNY",
                "service_date": "2025-12-31",
                "diagnosis_code": "J10",
                "policy": {
                  "external_policy_id": "POL-BEFORE-COVERAGE",
                  "coverage_start_date": "2026-01-01",
                  "coverage_end_date": "2026-12-31",
                  "coverage_limit": "10000"
                }
              }
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["decision_outcome"], "auto_deny");
    assert_eq!(body["decision_authority"], "customer_policy_rule");
    assert_eq!(body["decision_confidence"], "deterministic");
    assert_eq!(body["appeal_or_review_required"], true);
    assert_eq!(body["reason_code"], "SERVICE_BEFORE_COVERAGE_START");
    assert!(body["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "rule_runs:SERVICE_BEFORE_COVERAGE_START"
        )));
    let alert = body["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|alert| alert["alert_code"] == "SERVICE_BEFORE_COVERAGE_START")
        .expect("approved hard-deny alert");
    assert_eq!(
        alert["required_evidence"][0]["policy_authority_ref"],
        "policy:coverage-eligibility:v1"
    );
    assert_eq!(
        alert["required_evidence"][0]["exception_check"],
        "no_retroactive_coverage_exception"
    );
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
    assert_eq!(body["decision_outcome"], "post_payment_audit");
    assert_eq!(body["decision_authority"], "customer_policy_rule");
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
    assert_eq!(
        scoring_event["payload"]["decision_outcome"],
        "post_payment_audit"
    );
}

#[tokio::test]
async fn pending_evidence_rule_returns_required_evidence() {
    let repository = InMemoryScoringRepository::shared();
    activate_pending_evidence_rule(repository.clone()).await;
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
                "external_claim_id": "CLM-PENDING-EVIDENCE",
                "claim_amount": "8000",
                "currency": "CNY"
              },
              "policy": {
                "external_policy_id": "POL-PENDING-EVIDENCE",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              },
              "member": {
                "external_member_id": "MBR-PENDING-EVIDENCE"
              },
              "provider": {
                "external_provider_id": "PRV-PENDING-EVIDENCE",
                "name": "Northwind Hospital",
                "provider_type": "hospital",
                "region": "SH"
              }
            }"#,
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["decision_outcome"], "pending_evidence");
    assert_eq!(body["decision_authority"], "customer_policy_rule");
    assert_eq!(body["decision_confidence"], "deterministic");
    assert_eq!(body["reason_code"], "DENTAL_XRAY_REQUIRED");
    let alert = body["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|alert| alert["alert_code"] == "DENTAL_XRAY_REQUIRED")
        .expect("pending evidence rule alert");
    assert_eq!(
        alert["required_evidence"][0]["evidence_type"],
        "dental_xray"
    );
    assert_eq!(
        alert["required_evidence"][0]["evidence_request_type"],
        "document_request"
    );
    assert_eq!(alert["required_evidence"][0]["blocking"], true);
    assert_eq!(
        alert["required_evidence"][0]["policy_authority_ref"],
        "policy:dental:evidence:v1"
    );
    assert_eq!(
        alert["required_evidence"][0]["exception_check"],
        "xray_waiver_not_present"
    );

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-PENDING-EVIDENCE")
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
    let triggered_rule = scoring_event["payload"]["triggered_rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|alert| alert["alert_code"] == "DENTAL_XRAY_REQUIRED")
        .expect("audit payload should include pending evidence rule");
    assert_eq!(
        triggered_rule["required_evidence"][0]["evidence_type"],
        "dental_xray"
    );

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
        .find(|request| request["claim_id"] == "CLM-PENDING-EVIDENCE")
        .expect("rule required evidence should create an evidence request");
    assert_eq!(request["request_reason"], "rule_required_evidence");
    assert_eq!(
        request["missing_evidence"],
        serde_json::json!(["dental_xray"])
    );
}

#[tokio::test]
async fn generated_tpa_rule_funnel_demo_payloads_match_expected_outcomes() {
    let app = build_app(test_config());
    let dataset_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/tpa-rule-funnel-demo");
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(dataset_root.join("manifest.json")).expect("TPA demo manifest"),
    )
    .expect("TPA demo manifest JSON");

    for case in manifest["cases"].as_array().expect("manifest cases") {
        let case_id = case["case_id"].as_str().expect("case_id");
        let expected = &case["expected"];

        if let Some(expected_outcome) = expected["decision_outcome"].as_str() {
            let payload_path = dataset_root.join(
                case["direct_scoring_payload"]
                    .as_str()
                    .expect("direct scoring payload path"),
            );
            let payload = fs::read_to_string(&payload_path).expect("direct scoring payload");
            let request = Request::builder()
                .method("POST")
                .uri("/api/v1/claims/score")
                .header("content-type", "application/json")
                .header("x-api-key", "dev-secret")
                .body(Body::from(payload))
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            let status = response.status();
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            assert_eq!(
                status,
                StatusCode::OK,
                "{case_id} should score successfully: {}",
                String::from_utf8_lossy(&body)
            );
            let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(
                body["decision_outcome"],
                expected_outcome,
                "{case_id} decision_outcome: risk_score={}, risk_level={}, recommended_action={}, alerts={}, clinical_evidence={}",
                body["risk_score"],
                body["risk_level"],
                body["recommended_action"],
                body["alerts"],
                body["clinical_evidence"]
            );

            let expected_evidence = expected["required_evidence"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let actual_evidence = body["alerts"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|alert| {
                    alert["required_evidence"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                })
                .filter_map(|item| item["evidence_type"].as_str().map(str::to_string))
                .collect::<std::collections::BTreeSet<_>>();
            for evidence in expected_evidence {
                let evidence = evidence.as_str().unwrap();
                assert!(
                    actual_evidence.contains(evidence),
                    "{case_id} should require {evidence}; actual {actual_evidence:?}"
                );
            }
        }

        if expected["normalize_scoring_ready"].as_bool() == Some(false) {
            let payload_path =
                dataset_root.join(case["inbox_payload"].as_str().expect("inbox payload path"));
            let payload = fs::read_to_string(&payload_path).expect("inbox payload");
            let request = Request::builder()
                .method("POST")
                .uri("/api/v1/inbox/claims/normalize")
                .header("content-type", "application/json")
                .header("x-api-key", "dev-secret")
                .body(Body::from(payload))
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "{case_id} should normalize with warnings"
            );
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(body["scoring_ready"], false, "{case_id} scoring_ready");
            let expected_path = expected["validation_field_path"].as_str().unwrap();
            assert!(
                body["validation_errors"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|error| error["field_path"] == expected_path),
                "{case_id} should report validation path {expected_path}"
            );
        }
    }
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
async fn rejects_canonical_context_with_full_payload_fields() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "canonical_claim_context": {},
              "claim": {
                "external_claim_id": "CLM-CANONICAL-AMBIGUOUS",
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
    assert!(body.contains("AMBIGUOUS_SCORE_REQUEST"));
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
