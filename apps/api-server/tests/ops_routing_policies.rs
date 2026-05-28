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
