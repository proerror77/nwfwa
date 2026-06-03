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

fn customer_pilot_config() -> AppConfig {
    AppConfig {
        api_key: "customer-pilot-secret".into(),
        source_system: "customer-claims-system".into(),
        database_url: "postgres://customer-db.internal:5432/fwa".into(),
        model_service_url: "https://models.customer.internal".into(),
        object_storage_uri: "s3://customer-fwa-artifacts".into(),
        customer_scope_id: "customer-alpha-prod".into(),
        retention_policy_id: "customer-alpha-retention-v1".into(),
        backup_restore_plan_id: "customer-alpha-backup-restore-v1".into(),
        pii_masking_policy_id: "customer-alpha-pii-masking-v1".into(),
        key_rotation_policy_id: "customer-alpha-key-rotation-v1".into(),
        network_allowlist_id: "customer-alpha-network-allowlist-v1".into(),
        alert_routing_policy_id: "customer-alpha-alert-routing-v1".into(),
        observability_exporter_endpoint: "https://otel.customer-alpha.example".into(),
        agent_policy_id: "customer-alpha-agent-policy-v1".into(),
    }
}

fn health_check<'a>(body: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    body["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == name)
        .unwrap_or_else(|| panic!("missing health check {name}"))
}

fn blocking_check<'a>(body: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    body["pilot_readiness"]["blocking_checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == name)
        .unwrap_or_else(|| panic!("missing blocking check {name}"))
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
    assert_eq!(
        health_check(&body, "model_service_configuration")["status"],
        "local_dev_model_service"
    );
    assert_eq!(
        health_check(&body, "api_key_configuration")["status"],
        "local_dev_key"
    );
    assert_eq!(
        health_check(&body, "source_system_configuration")["status"],
        "local_demo_source"
    );
    assert_eq!(
        health_check(&body, "database_configuration")["status"],
        "local_dev_database"
    );
    assert_eq!(
        health_check(&body, "object_storage_configuration")["status"],
        "local_demo_object_storage"
    );
    assert_eq!(
        health_check(&body, "customer_scope_configuration")["status"],
        "local_demo_customer_scope"
    );
    assert_eq!(
        health_check(&body, "retention_policy_configuration")["status"],
        "local_demo_retention_policy"
    );
    assert_eq!(
        health_check(&body, "backup_restore_configuration")["status"],
        "local_demo_backup_restore"
    );
    assert_eq!(
        health_check(&body, "pii_masking_configuration")["status"],
        "local_demo_pii_masking"
    );
    assert_eq!(
        health_check(&body, "key_rotation_configuration")["status"],
        "local_demo_key_rotation"
    );
    assert_eq!(
        health_check(&body, "network_allowlist_configuration")["status"],
        "local_demo_network_allowlist"
    );
    assert_eq!(
        health_check(&body, "alert_routing_configuration")["status"],
        "local_demo_alert_routing"
    );
    assert_eq!(
        health_check(&body, "observability_exporter_configuration")["status"],
        "local_demo_observability_exporter"
    );
    assert_eq!(
        health_check(&body, "agent_policy_configuration")["status"],
        "local_demo_agent_policy"
    );
    assert!(
        health_check(&body, "api_key_configuration")["remediation"]
            .as_str()
            .unwrap()
            .contains("FWA_API_KEY_PRINCIPALS"),
        "blocking API key check should include non-secret remediation"
    );
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
    assert_eq!(body["pilot_readiness"]["status"], "not_ready");
    assert_eq!(body["pilot_readiness"]["ready_for_customer_pilot"], false);
    assert_eq!(body["pilot_readiness"]["required_check_count"], 14);
    let blocking_checks = body["pilot_readiness"]["blocking_checks"]
        .as_array()
        .expect("pilot readiness should list blocking checks");
    assert_eq!(
        body["pilot_readiness"]["blocking_check_count"],
        blocking_checks.len()
    );
    assert_eq!(body["pilot_readiness"]["ready_check_count"], 0);
    let required_check_names = body["pilot_readiness"]["required_check_names"]
        .as_array()
        .expect("pilot readiness should list required check names");
    assert!(required_check_names.contains(&serde_json::json!("api_key_configuration")));
    assert!(required_check_names.contains(&serde_json::json!("agent_policy_configuration")));
    assert_eq!(
        required_check_names.len(),
        body["pilot_readiness"]["required_check_count"]
            .as_u64()
            .unwrap() as usize
    );
    let blocking_check_names = body["pilot_readiness"]["blocking_check_names"]
        .as_array()
        .expect("pilot readiness should list compact blocking check names");
    assert_eq!(blocking_check_names.len(), blocking_checks.len());
    assert!(blocking_check_names.contains(&serde_json::json!("api_key_configuration")));
    let remediation_summary = body["pilot_readiness"]["remediation_summary"]
        .as_array()
        .expect("pilot readiness should list compact remediation hints");
    assert_eq!(remediation_summary.len(), blocking_checks.len());
    assert!(remediation_summary.iter().all(|item| item.is_string()));
    assert_eq!(
        blocking_check(&body, "api_key_configuration")["status"],
        "local_dev_key"
    );
    assert_eq!(
        blocking_check(&body, "model_service_configuration")["status"],
        "local_dev_model_service"
    );
    assert_eq!(
        blocking_check(&body, "agent_policy_configuration")["status"],
        "local_demo_agent_policy"
    );
    assert!(
        !body["pilot_readiness"].to_string().contains("dev-secret"),
        "pilot readiness must not expose secret values"
    );
}

#[tokio::test]
async fn health_reports_pilot_readiness_ready_when_all_pilot_configuration_is_set() {
    let app = build_app(customer_pilot_config());

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

    assert_eq!(body["pilot_readiness"]["status"], "ready");
    assert_eq!(body["pilot_readiness"]["ready_for_customer_pilot"], true);
    assert_eq!(body["pilot_readiness"]["required_check_count"], 14);
    assert_eq!(body["pilot_readiness"]["ready_check_count"], 14);
    assert_eq!(body["pilot_readiness"]["blocking_check_count"], 0);
    assert_eq!(
        body["pilot_readiness"]["blocking_check_names"],
        serde_json::json!([])
    );
    assert_eq!(
        body["pilot_readiness"]["remediation_summary"],
        serde_json::json!([])
    );
    assert_eq!(
        body["pilot_readiness"]["blocking_checks"],
        serde_json::json!([])
    );
    assert_eq!(
        body["pilot_readiness"]["ready_checks"]
            .as_array()
            .expect("ready pilot readiness should list ready checks")
            .len(),
        14
    );
    assert!(
        !body.to_string().contains("customer-pilot-secret"),
        "pilot readiness must not expose API key values"
    );
    assert!(
        !body.to_string().contains("models.customer.internal"),
        "pilot readiness must not expose model service URL values"
    );
    assert!(
        !body.to_string().contains("customer-alpha-agent-policy-v1"),
        "pilot readiness must not expose Agent policy ids"
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
    assert_eq!(
        health_check(&body, "model_service_configuration")["status"],
        "heuristic_model_scorer"
    );
    assert!(
        health_check(&body, "model_service_configuration")["remediation"]
            .as_str()
            .unwrap()
            .contains("FWA_MODEL_SERVICE_URL"),
        "heuristic scorer mode should include model runtime remediation"
    );
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
