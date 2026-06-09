use api_server::{app::build_app_with_parts, repository::InMemoryScoringRepository};
use async_trait::async_trait;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use fwa_ml_runtime::{ModelRuntimeError, ModelScore, ModelScoreRequest, ModelScorer};
use std::sync::Arc;
use tower::ServiceExt;

use super::support::{
    activate_candidate_model, activate_post_payment_model, test_config, RequestEchoScorer,
};

#[tokio::test]
async fn scoring_uses_active_model_version_from_model_registry() {
    let repository = InMemoryScoringRepository::shared();
    activate_candidate_model(repository.clone()).await;
    let app = build_app_with_parts(test_config(), Arc::new(RequestEchoScorer), repository);

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-ACTIVE-MODEL",
                "claim_amount": "8000",
                "currency": "CNY"
              }
            }"#,
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["scores"]["ml_score"], 72);
    assert_eq!(body["model_score"]["score"], 72);
    assert_eq!(body["model_score"]["model_version"], "0.2.0-active");
    assert_eq!(body["model_score"]["runtime_kind"], "test_echo");
    assert!(body["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "model_versions:baseline_fwa:0.2.0-active"
        )));

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-ACTIVE-MODEL")
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let audit_response = app.oneshot(audit_request).await.unwrap();
    assert_eq!(audit_response.status(), StatusCode::OK);
    let audit_body = to_bytes(audit_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let audit_body: serde_json::Value = serde_json::from_slice(&audit_body).unwrap();
    let scoring_event = audit_body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "scoring.completed")
        .expect("audit history should include scoring.completed");
    assert_eq!(
        scoring_event["payload"]["model_score"]["model_version"],
        "0.2.0-active"
    );
    assert_eq!(
        scoring_event["payload"]["model_score"]["metadata"]["endpoint_url"],
        "http://127.0.0.1:8001/score/baseline_fwa/0.2.0-active"
    );
    assert!(scoring_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "model_versions:baseline_fwa:0.2.0-active"
        )));
}

#[tokio::test]
async fn scoring_filters_active_model_by_review_mode() {
    let repository = InMemoryScoringRepository::shared();
    activate_post_payment_model(repository.clone()).await;
    let app = build_app_with_parts(test_config(), Arc::new(RequestEchoScorer), repository);

    let pre_payment_request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-PRE-PAYMENT-MODEL",
                "claim_amount": "8000",
                "currency": "CNY"
              }
            }"#,
        ))
        .unwrap();

    let pre_payment_response = app.clone().oneshot(pre_payment_request).await.unwrap();
    assert_eq!(pre_payment_response.status(), StatusCode::CONFLICT);

    let post_payment_request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "review_mode": "post_payment",
              "claim": {
                "external_claim_id": "CLM-POST-PAYMENT-MODEL",
                "claim_amount": "8000",
                "currency": "CNY"
              }
            }"#,
        ))
        .unwrap();

    let post_payment_response = app.clone().oneshot(post_payment_request).await.unwrap();
    assert_eq!(post_payment_response.status(), StatusCode::OK);
    let body = to_bytes(post_payment_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["scores"]["ml_score"], 72);

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-POST-PAYMENT-MODEL")
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let audit_response = app.oneshot(audit_request).await.unwrap();
    assert_eq!(audit_response.status(), StatusCode::OK);
    let audit_body = to_bytes(audit_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let audit_body: serde_json::Value = serde_json::from_slice(&audit_body).unwrap();
    let scoring_event = audit_body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "scoring.completed")
        .expect("audit history should include scoring.completed");
    assert_eq!(
        scoring_event["payload"]["model_score"]["model_version"],
        "0.2.0-post-payment"
    );
}

#[derive(Debug)]
struct InvalidResponseScorer;

#[async_trait]
impl ModelScorer for InvalidResponseScorer {
    async fn score(&self, _request: ModelScoreRequest) -> Result<ModelScore, ModelRuntimeError> {
        Err(ModelRuntimeError::InvalidResponse("missing score".into()))
    }
}

#[tokio::test]
async fn returns_invalid_model_response_code() {
    let app = build_app_with_parts(
        test_config(),
        Arc::new(InvalidResponseScorer),
        InMemoryScoringRepository::shared(),
    );

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-BAD-MODEL",
                "claim_amount": "8000",
                "currency": "CNY"
              }
            }"#,
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("MODEL_RESPONSE_INVALID"));
    assert!(body.contains("model response invalid"));
    assert!(!body.contains("missing score"));

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-BAD-MODEL")
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let audit_response = app.oneshot(audit_request).await.unwrap();
    assert_eq!(audit_response.status(), StatusCode::OK);
    let audit_body = to_bytes(audit_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let audit_body: serde_json::Value = serde_json::from_slice(&audit_body).unwrap();
    let failed_event = audit_body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "scoring.failed")
        .expect("failed model scoring should be audited");
    assert_eq!(failed_event["event_status"], "failed");
    assert_eq!(failed_event["summary"], "model scoring failed");
    assert_eq!(failed_event["payload"]["claim_id"], "CLM-BAD-MODEL");
    assert_eq!(failed_event["payload"]["review_mode"], "pre_payment");
    assert!(failed_event["payload"]["error"]
        .as_str()
        .unwrap()
        .contains("missing score"));
    assert!(!failed_event["evidence_refs"].as_array().unwrap().is_empty());
}
