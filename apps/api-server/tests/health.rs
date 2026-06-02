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
