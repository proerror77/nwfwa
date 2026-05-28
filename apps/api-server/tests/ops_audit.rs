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
async fn lists_global_audit_events_for_governance_review() {
    let app = build_app(test_config());

    let save_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/ops/routing-policies")
                .header("content-type", "application/json")
                .header("x-api-key", "dev-secret")
                .body(Body::from(
                    r#"{
                      "owner": "policy-ops",
                      "policy": {
                        "policy_id": "audit_visible_policy",
                        "version": 1,
                        "review_mode": "pre_payment",
                        "risk_thresholds": {
                          "low_max": 24,
                          "medium_min": 25,
                          "high_min": 65,
                          "critical_min": 88
                        },
                        "confidence_thresholds": {
                          "low_confidence_below": 55,
                          "high_confidence_min": 85
                        },
                        "provider_review_threshold": 72
                      }
                    }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(save_response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/ops/audit-events?limit=5")
                .header("x-api-key", "dev-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let event = body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "routing_policy.candidate.saved")
        .expect("global audit log should include routing policy lifecycle events");
    assert_eq!(event["payload"]["policy_id"], "audit_visible_policy");
    assert_eq!(event["payload"]["to_status"], "draft");
    assert_eq!(
        event["evidence_refs"][0],
        "routing_policies:audit_visible_policy:v1:pre_payment"
    );
}

#[tokio::test]
async fn global_audit_events_require_api_key() {
    let app = build_app(test_config());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/ops/audit-events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
