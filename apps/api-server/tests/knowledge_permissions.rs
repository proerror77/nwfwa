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

async fn search_similar_with_key(api_key: &str) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/knowledge/search-similar")
        .header("content-type", "application/json")
        .header("x-api-key", api_key)
        .body(Body::from(
            r#"{
              "claim_id": "CLM-0287",
              "diagnosis_code": "J10",
              "provider_region": "Shanghai",
              "tags": ["early_claim", "high_amount"]
            }"#,
        ))
        .unwrap();
    let response = build_app(test_config())
        .unwrap()
        .oneshot(request)
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}));
    (status, body)
}

#[tokio::test]
async fn similar_case_search_requires_tpa_knowledge_read_permission() {
    let previous_principals = std::env::var_os("FWA_API_KEY_PRINCIPALS");
    std::env::set_var(
        "FWA_API_KEY_PRINCIPALS",
        [
            "claims-key|claims-service|tpa_system|customer-tpa|customer-alpha|tpa:claims:score",
            "knowledge-key|tpa-panel|tpa_system|customer-tpa|customer-alpha|tpa:knowledge:read",
            "wildcard-key|pilot-ops|fwa_operator|customer-tpa|customer-alpha|tpa:*",
        ]
        .join(";"),
    );

    let (status, body) = search_similar_with_key("claims-key").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "FORBIDDEN");
    assert_eq!(body["message"], "missing permission: tpa:knowledge:read");

    let (status, body) = search_similar_with_key("knowledge-key").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["results"][0]["case_id"], "KC-1001");

    let (status, body) = search_similar_with_key("wildcard-key").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["results"][0]["case_id"], "KC-1001");

    if let Some(previous_principals) = previous_principals {
        std::env::set_var("FWA_API_KEY_PRINCIPALS", previous_principals);
    } else {
        std::env::remove_var("FWA_API_KEY_PRINCIPALS");
    }
}
