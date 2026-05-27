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

#[tokio::test]
async fn openapi_includes_operations_paths() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("GET")
        .uri("/api/openapi.json")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let schema: serde_json::Value = serde_json::from_slice(&body).unwrap();
    for path in [
        "/api/v1/ops/rules",
        "/api/v1/ops/rules/{rule_id}",
        "/api/v1/ops/rules/backtest",
        "/api/v1/ops/rules/performance",
        "/api/v1/ops/rules/{rule_id}/promotion-gates",
        "/api/v1/ops/rules/{rule_id}/promotion-reviews",
        "/api/v1/ops/rules/candidates",
        "/api/v1/ops/rules/discover",
        "/api/v1/ops/models",
        "/api/v1/ops/models/{model_key}/performance",
        "/api/v1/ops/models/{model_key}/promotion-gates",
        "/api/v1/ops/models/{model_key}/promotion-reviews",
        "/api/v1/ops/datasets",
        "/api/v1/ops/datasets/{dataset_id}",
        "/api/v1/ops/datasets/{dataset_id}/mappings",
        "/api/v1/ops/feature-sets",
        "/api/v1/ops/model-datasets",
        "/api/v1/ops/model-evaluations",
        "/api/v1/ops/model-evaluations/{evaluation_run_id}",
        "/api/v1/ops/dashboard/summary",
        "/api/v1/ops/leads",
        "/api/v1/ops/leads/{lead_id}/triage",
        "/api/v1/ops/cases",
        "/api/v1/ops/audit-samples",
        "/api/v1/ops/knowledge/cases",
        "/api/v1/knowledge/search-similar",
        "/api/v1/agent/cases/investigate",
        "/api/v1/investigations/results",
        "/api/v1/qa/results",
        "/api/v1/audit/claims/{claim_id}",
    ] {
        assert!(schema["paths"][path].is_object(), "missing {path}");
    }
    assert!(schema["components"]["schemas"]["RuleDiscoveryResponse"].is_object());
    assert!(schema["components"]["schemas"]["RulePerformanceResponse"].is_object());
    assert!(schema["components"]["schemas"]["LeadListResponse"].is_object());
    assert!(schema["components"]["schemas"]["CaseListResponse"].is_object());
    assert!(schema["components"]["schemas"]["AuditSampleRecord"].is_object());
    assert!(schema["paths"]["/api/v1/ops/model-evaluations"]["get"].is_object());
    assert!(schema["paths"]["/api/v1/ops/model-evaluations"]["post"].is_object());
}
