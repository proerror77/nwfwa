use api_server::{app::build_app, config::AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        source_system: "tpa-demo".into(),
        database_url: "postgres://postgres:postgres@localhost:5432/fwa".into(),
        model_service_url: "http://127.0.0.1:8001".into(),
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
async fn health_returns_service_metadata_and_checks() {
    let app = build_app(test_config());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "api-server");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "http_router",
            "status": "ok"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "openapi_contract",
            "status": "ok"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "model_scorer",
            "status": "ok",
            "runtime_kind": "python_http"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "model_service_configuration",
            "status": "local_dev_model_service"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "api_key_configuration",
            "status": "local_dev_key"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "source_system_configuration",
            "status": "local_demo_source"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "database_configuration",
            "status": "local_dev_database"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "object_storage_configuration",
            "status": "local_demo_object_storage"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "customer_scope_configuration",
            "status": "local_demo_customer_scope"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "retention_policy_configuration",
            "status": "local_demo_retention_policy"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "backup_restore_configuration",
            "status": "local_demo_backup_restore"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "pii_masking_configuration",
            "status": "local_demo_pii_masking"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "key_rotation_configuration",
            "status": "local_demo_key_rotation"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "network_allowlist_configuration",
            "status": "local_demo_network_allowlist"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "alert_routing_configuration",
            "status": "local_demo_alert_routing"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "observability_exporter_configuration",
            "status": "local_demo_observability_exporter"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "agent_policy_configuration",
            "status": "local_demo_agent_policy"
        })));
    assert!(
        !body.to_string().contains("127.0.0.1:8001"),
        "health response must not expose internal model service URLs"
    );
    assert!(
        !body.to_string().contains("dev-secret"),
        "health response must not expose API key values"
    );
    assert!(
        !body
            .to_string()
            .contains("postgres://postgres:postgres@localhost:5432/fwa"),
        "health response must not expose database URLs"
    );
    assert!(
        !body.to_string().contains("local://demo-artifacts"),
        "health response must not expose object storage URIs"
    );
    assert!(
        !body.to_string().contains("demo-customer"),
        "health response must not expose customer scope ids"
    );
    assert!(
        !body.to_string().contains("demo-retention-policy"),
        "health response must not expose retention policy ids"
    );
    assert!(
        !body.to_string().contains("demo-backup-restore-plan"),
        "health response must not expose backup restore plan ids"
    );
    assert!(
        !body.to_string().contains("demo-pii-masking-policy"),
        "health response must not expose PII masking policy ids"
    );
    assert!(
        !body.to_string().contains("demo-key-rotation-policy"),
        "health response must not expose key rotation policy ids"
    );
    assert!(
        !body.to_string().contains("demo-network-allowlist"),
        "health response must not expose network allowlist ids"
    );
    assert!(
        !body.to_string().contains("demo-alert-routing-policy"),
        "health response must not expose alert routing policy ids"
    );
    assert!(
        !body.to_string().contains("local://demo-observability"),
        "health response must not expose observability exporter endpoints"
    );
    assert!(
        !body.to_string().contains("demo-agent-policy"),
        "health response must not expose agent policy ids"
    );
}

#[tokio::test]
async fn health_reports_explicit_heuristic_model_scorer_mode() {
    let mut config = test_config();
    config.model_service_url = "heuristic://local".into();
    let app = build_app(config);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "model_scorer",
            "status": "ok",
            "runtime_kind": "heuristic"
        })));
    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "model_service_configuration",
            "status": "heuristic_model_scorer"
        })));
}

#[tokio::test]
async fn health_reports_configured_api_key_without_exposing_value() {
    let mut config = test_config();
    config.api_key = "customer-pilot-secret".into();
    let app = build_app(config);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "api_key_configuration",
            "status": "configured"
        })));
    assert!(
        !body.to_string().contains("customer-pilot-secret"),
        "health response must not expose configured API key values"
    );
}

#[tokio::test]
async fn health_reports_configured_source_system_without_exposing_value() {
    let mut config = test_config();
    config.source_system = "customer-claims-system".into();
    let app = build_app(config);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "source_system_configuration",
            "status": "configured"
        })));
    assert!(
        !body.to_string().contains("customer-claims-system"),
        "health response must not expose configured source system values"
    );
}

#[tokio::test]
async fn health_reports_configured_database_without_exposing_value() {
    let mut config = test_config();
    config.database_url = "postgres://customer-db.internal:5432/fwa".into();
    let app = build_app(config);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "database_configuration",
            "status": "configured"
        })));
    assert!(
        !body.to_string().contains("customer-db.internal"),
        "health response must not expose configured database URL values"
    );
}

#[tokio::test]
async fn health_reports_configured_model_service_without_exposing_value() {
    let mut config = test_config();
    config.model_service_url = "https://models.customer.internal".into();
    let app = build_app(config);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "model_service_configuration",
            "status": "configured"
        })));
    assert!(
        !body.to_string().contains("models.customer.internal"),
        "health response must not expose configured model service URL values"
    );
}

#[tokio::test]
async fn health_reports_configured_object_storage_without_exposing_value() {
    let mut config = test_config();
    config.object_storage_uri = "s3://customer-fwa-artifacts".into();
    let app = build_app(config);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "object_storage_configuration",
            "status": "configured"
        })));
    assert!(
        !body.to_string().contains("customer-fwa-artifacts"),
        "health response must not expose configured object storage URI values"
    );
}

#[tokio::test]
async fn health_reports_configured_customer_scope_without_exposing_value() {
    let mut config = test_config();
    config.customer_scope_id = "customer-alpha-prod".into();
    let app = build_app(config);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "customer_scope_configuration",
            "status": "configured"
        })));
    assert!(
        !body.to_string().contains("customer-alpha-prod"),
        "health response must not expose configured customer scope ids"
    );
}

#[tokio::test]
async fn health_reports_configured_retention_policy_without_exposing_value() {
    let mut config = test_config();
    config.retention_policy_id = "customer-alpha-retention-v1".into();
    let app = build_app(config);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "retention_policy_configuration",
            "status": "configured"
        })));
    assert!(
        !body.to_string().contains("customer-alpha-retention-v1"),
        "health response must not expose configured retention policy ids"
    );
}

#[tokio::test]
async fn health_reports_configured_backup_restore_without_exposing_value() {
    let mut config = test_config();
    config.backup_restore_plan_id = "customer-alpha-backup-restore-v1".into();
    let app = build_app(config);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "backup_restore_configuration",
            "status": "configured"
        })));
    assert!(
        !body
            .to_string()
            .contains("customer-alpha-backup-restore-v1"),
        "health response must not expose configured backup restore plan ids"
    );
}

#[tokio::test]
async fn health_reports_configured_pii_masking_without_exposing_value() {
    let mut config = test_config();
    config.pii_masking_policy_id = "customer-alpha-pii-masking-v1".into();
    let app = build_app(config);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "pii_masking_configuration",
            "status": "configured"
        })));
    assert!(
        !body.to_string().contains("customer-alpha-pii-masking-v1"),
        "health response must not expose configured PII masking policy ids"
    );
}

#[tokio::test]
async fn health_reports_configured_key_rotation_without_exposing_value() {
    let mut config = test_config();
    config.key_rotation_policy_id = "customer-alpha-key-rotation-v1".into();
    let app = build_app(config);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "key_rotation_configuration",
            "status": "configured"
        })));
    assert!(
        !body.to_string().contains("customer-alpha-key-rotation-v1"),
        "health response must not expose configured key rotation policy ids"
    );
}

#[tokio::test]
async fn health_reports_configured_network_allowlist_without_exposing_value() {
    let mut config = test_config();
    config.network_allowlist_id = "customer-alpha-network-allowlist-v1".into();
    let app = build_app(config);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "network_allowlist_configuration",
            "status": "configured"
        })));
    assert!(
        !body
            .to_string()
            .contains("customer-alpha-network-allowlist-v1"),
        "health response must not expose configured network allowlist ids"
    );
}

#[tokio::test]
async fn health_reports_configured_alert_routing_without_exposing_value() {
    let mut config = test_config();
    config.alert_routing_policy_id = "customer-alpha-alert-routing-v1".into();
    let app = build_app(config);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "alert_routing_configuration",
            "status": "configured"
        })));
    assert!(
        !body.to_string().contains("customer-alpha-alert-routing-v1"),
        "health response must not expose configured alert routing policy ids"
    );
}

#[tokio::test]
async fn health_reports_configured_observability_exporter_without_exposing_value() {
    let mut config = test_config();
    config.observability_exporter_endpoint = "https://otel.customer-alpha.example".into();
    let app = build_app(config);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "observability_exporter_configuration",
            "status": "configured"
        })));
    assert!(
        !body
            .to_string()
            .contains("https://otel.customer-alpha.example"),
        "health response must not expose configured observability exporter endpoints"
    );
}

#[tokio::test]
async fn health_reports_configured_agent_policy_without_exposing_value() {
    let mut config = test_config();
    config.agent_policy_id = "customer-alpha-agent-policy-v1".into();
    let app = build_app(config);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["checks"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!({
            "name": "agent_policy_configuration",
            "status": "configured"
        })));
    assert!(
        !body.to_string().contains("customer-alpha-agent-policy-v1"),
        "health response must not expose configured agent policy ids"
    );
}
