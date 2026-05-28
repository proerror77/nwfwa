use api_server::{
    app::{build_app, build_app_with_parts},
    config::AppConfig,
    repository::InMemoryScoringRepository,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use fwa_ml_runtime::HeuristicModelScorer;
use fwa_scoring::{ConfidenceThresholds, RiskThresholds, RoutingPolicy};
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
async fn lists_default_routing_policies_for_governance_visibility() {
    let app = build_app(test_config());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/ops/routing-policies")
                .header("x-api-key", "dev-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let policies = body["policies"].as_array().unwrap();
    assert_eq!(policies.len(), 3);
    assert!(policies
        .iter()
        .any(|policy| policy["review_mode"] == "pre_payment"));
    assert!(policies
        .iter()
        .all(|policy| policy["policy_id"] == "fwa_risk_fusion_routing"));
    assert!(policies.iter().all(|policy| policy["status"] == "active"));
    assert_eq!(policies[0]["risk_thresholds"]["critical_min"], 85);
    assert_eq!(
        policies[0]["confidence_thresholds"]["low_confidence_below"],
        60
    );
}

#[tokio::test]
async fn lists_injected_active_routing_policy_versions() {
    let repository = InMemoryScoringRepository::shared_with_routing_policies(vec![RoutingPolicy {
        policy_id: "custom_prepay_routing".into(),
        version: 4,
        review_mode: "pre_payment".into(),
        risk_thresholds: RiskThresholds {
            low_max: 24,
            medium_min: 25,
            high_min: 55,
            critical_min: 80,
        },
        confidence_thresholds: ConfidenceThresholds {
            low_confidence_below: 50,
            high_confidence_min: 90,
        },
        provider_review_threshold: 65,
    }]);
    let app = build_app_with_parts(test_config(), Arc::new(HeuristicModelScorer), repository);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/ops/routing-policies")
                .header("x-api-key", "dev-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["policies"][0]["policy_id"], "custom_prepay_routing");
    assert_eq!(body["policies"][0]["version"], 4);
    assert_eq!(body["policies"][0]["review_mode"], "pre_payment");
    assert_eq!(body["policies"][0]["risk_thresholds"]["high_min"], 55);
    assert_eq!(body["policies"][0]["provider_review_threshold"], 65);
}

#[tokio::test]
async fn saves_draft_routing_policy_candidate_without_affecting_scoring() {
    let app = build_app(test_config());

    let save_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/ops/routing-policies")
                .header("content-type", "application/json")
                .header("x-api-key", "dev-secret")
                .body(Body::from(
                    r#"{
                      "owner": "policy-ops",
                      "policy": {
                        "policy_id": "candidate_strict_prepay",
                        "version": 2,
                        "review_mode": "pre_payment",
                        "risk_thresholds": {
                          "low_max": 0,
                          "medium_min": 1,
                          "high_min": 1,
                          "critical_min": 1
                        },
                        "confidence_thresholds": {
                          "low_confidence_below": 60,
                          "high_confidence_min": 80
                        },
                        "provider_review_threshold": 70
                      }
                    }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(save_response.status(), StatusCode::OK);
    let save_body = to_bytes(save_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let save_body: serde_json::Value = serde_json::from_slice(&save_body).unwrap();
    assert_eq!(save_body["policy_id"], "candidate_strict_prepay");
    assert_eq!(save_body["status"], "draft");
    assert_eq!(save_body["owner"], "policy-ops");

    let score_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/claims/score")
                .header("content-type", "application/json")
                .header("x-api-key", "dev-secret")
                .body(Body::from(
                    r#"{
                      "source_system": "tpa-demo",
                      "review_mode": "pre_payment",
                      "claim": {
                        "external_claim_id": "CLM-DRAFT-POLICY",
                        "claim_amount": "8000",
                        "currency": "CNY"
                      }
                    }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(score_response.status(), StatusCode::OK);
    let score_body = to_bytes(score_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let score_body: serde_json::Value = serde_json::from_slice(&score_body).unwrap();
    assert_eq!(
        score_body["routing_policy"]["policy_id"],
        "fwa_risk_fusion_routing"
    );
}

#[tokio::test]
async fn advances_routing_policy_lifecycle_and_activated_policy_controls_scoring() {
    let app = build_app(test_config());

    let (status, saved) = post_json(
        app.clone(),
        "/api/v1/ops/routing-policies",
        r#"{
          "owner": "policy-ops",
          "policy": {
            "policy_id": "candidate_strict_prepay",
            "version": 2,
            "review_mode": "pre_payment",
            "risk_thresholds": {
              "low_max": 0,
              "medium_min": 1,
              "high_min": 1,
              "critical_min": 1
            },
            "confidence_thresholds": {
              "low_confidence_below": 60,
              "high_confidence_min": 80
            },
            "provider_review_threshold": 70
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["status"], "draft");

    let (status, blocked) = post_empty(
        app.clone(),
        "/api/v1/ops/routing-policies/candidate_strict_prepay/pre_payment/2/activate",
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(blocked["code"], "ROUTING_POLICY_APPROVAL_REQUIRED");

    let (status, submitted) = post_empty(
        app.clone(),
        "/api/v1/ops/routing-policies/candidate_strict_prepay/pre_payment/2/submit",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(submitted["status"], "submitted");

    let (status, approved) = post_empty(
        app.clone(),
        "/api/v1/ops/routing-policies/candidate_strict_prepay/pre_payment/2/approve",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(approved["status"], "approved");

    let (status, activated) = post_empty(
        app.clone(),
        "/api/v1/ops/routing-policies/candidate_strict_prepay/pre_payment/2/activate",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(activated["status"], "active");

    let (status, scored) = post_json(
        app.clone(),
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "review_mode": "pre_payment",
          "claim": {
            "external_claim_id": "CLM-ACTIVE-POLICY",
            "claim_amount": "8000",
            "currency": "CNY"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        scored["routing_policy"]["policy_id"],
        "candidate_strict_prepay"
    );
    assert_eq!(scored["routing_policy"]["risk_thresholds"]["high_min"], 1);

    let (status, rolled_back) = post_empty(
        app.clone(),
        "/api/v1/ops/routing-policies/candidate_strict_prepay/pre_payment/2/rollback",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(rolled_back["status"], "approved");

    let (status, scored_after_rollback) = post_json(
        app,
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "review_mode": "pre_payment",
          "claim": {
            "external_claim_id": "CLM-ROLLED-BACK-POLICY",
            "claim_amount": "8000",
            "currency": "CNY"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        scored_after_rollback["routing_policy"]["policy_id"],
        "fwa_risk_fusion_routing"
    );
}

#[tokio::test]
async fn routing_policy_list_requires_api_key() {
    let app = build_app(test_config());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/ops/routing-policies")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

async fn post_json(
    app: axum::Router,
    uri: &str,
    body: &'static str,
) -> (StatusCode, serde_json::Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .header("x-api-key", "dev-secret")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

async fn post_empty(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("x-api-key", "dev-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}
