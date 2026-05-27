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
        database_url: "postgres://unused".into(),
        model_service_url: "http://unused".into(),
    }
}

async fn get_json(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}));
    (status, body)
}

#[tokio::test]
async fn lists_baseline_model_versions() {
    let app = build_app(test_config());

    let (status, body) = get_json(app, "/api/v1/ops/models").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["models"][0]["model_key"], "baseline_fwa");
    assert_eq!(body["models"][0]["version"], "0.1.0");
    assert_eq!(body["models"][0]["runtime_kind"], "python_http");
    assert_eq!(body["models"][0]["status"], "active");
}

#[tokio::test]
async fn returns_empty_model_performance_without_scores() {
    let app = build_app(test_config());

    let (status, body) = get_json(app, "/api/v1/ops/models/baseline_fwa/performance").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["model_key"], "baseline_fwa");
    assert_eq!(body["data_status"], "empty");
    assert_eq!(body["scored_runs"], 0);
    assert_eq!(body["average_score"], 0.0);
    assert_eq!(body["high_risk_count"], 0);
}

#[tokio::test]
async fn rejects_missing_api_key_for_model_ops() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/ops/models")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
