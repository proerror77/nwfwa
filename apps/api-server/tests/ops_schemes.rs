use api_server::{app::build_app, config::AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        api_key_principals: vec![],
        source_system: "tpa-demo".into(),
        database_url: "postgres://unused".into(),
        model_service_url: "heuristic://local".into(),
        object_storage_uri: "local://demo-artifacts".into(),
        customer_scope_id: "demo-customer".into(),
        retention_policy_id: "demo-retention-policy".into(),
        backup_restore_plan_id: "demo-backup-restore-plan".into(),
        pii_masking_policy_id: "demo-pii-masking-policy".into(),
        key_rotation_policy_id: "demo-key-rotation-policy".into(),
        network_allowlist_id: "demo-network-allowlist".into(),
        alert_routing_policy_id: "demo-alert-routing-policy".into(),
        observability_exporter_endpoint: "local://demo-observability".into(),
        agent_policy_id: "demo-agent-policy".into(),
    }
}

#[tokio::test]
async fn lists_governed_fwa_scheme_taxonomy() {
    let app = build_app(test_config());
    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/ops/fwa-schemes")
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["scheme_count"].as_u64().unwrap() >= 15);
    let schemes = body["schemes"].as_array().unwrap();
    let provider_scheme = schemes
        .iter()
        .find(|scheme| scheme["scheme_family"] == "provider_peer_outlier")
        .expect("provider peer outlier scheme should be governed");
    assert_eq!(provider_scheme["default_review_route"], "provider_review");
    assert!(provider_scheme["minimum_evidence"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("peer_group_definition")));
    assert!(provider_scheme["primary_layers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("L6_PROVIDER_GRAPH_RISK")));
}

#[tokio::test]
async fn scheme_taxonomy_requires_api_key() {
    let app = build_app(test_config());
    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/ops/fwa-schemes")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
