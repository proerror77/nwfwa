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

#[path = "claims_score/canonical_inbox.rs"]
mod canonical_inbox;
#[path = "claims_score/evidence_signals.rs"]
mod evidence_signals;
#[path = "claims_score/model_registry.rs"]
mod model_registry;
#[path = "claims_score/openapi_contract.rs"]
mod openapi_contract;
#[path = "claims_score/payload_contracts.rs"]
mod payload_contracts;
#[path = "claims_score/routing_decisions.rs"]
mod routing_decisions;
#[path = "claims_score/support.rs"]
mod support;

use support::{activate_post_payment_rule, test_config, RequestEchoScorer};

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
