#![recursion_limit = "256"]

use api_server::app::build_app;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[path = "ops_models/lifecycle.rs"]
mod lifecycle;
#[path = "ops_models/mlops_monitoring.rs"]
mod mlops_monitoring;
#[path = "ops_models/performance.rs"]
mod performance;
#[path = "ops_models/promotion_gates.rs"]
mod promotion_gates;
#[path = "ops_models/retraining_jobs.rs"]
mod retraining_jobs;
#[path = "ops_models/retraining_output_validation.rs"]
mod retraining_output_validation;
#[path = "ops_models/retraining_readiness.rs"]
mod retraining_readiness;
#[path = "ops_models/support.rs"]
mod support;

use support::test_config;

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

#[tokio::test]
async fn rejects_missing_api_key_for_model_promotion_gates() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/ops/models/baseline_fwa/promotion-gates")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
