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

use super::support::{
    activate_pending_evidence_rule, test_config, HighRiskScorer, RequestEchoScorer,
};

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
