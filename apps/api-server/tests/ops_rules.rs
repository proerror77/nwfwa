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

async fn json_request(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: &str,
) -> (StatusCode, String) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn lists_rule_library() {
    let app = build_app(test_config());

    let (status, body) = json_request(app, "GET", "/api/v1/ops/rules", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["rules"][0]["rule_id"], "rule_early_claim");
    assert_eq!(body["rules"][0]["status"], "active");
    assert_eq!(body["rules"][0]["active_version"], 1);
}

#[tokio::test]
async fn returns_rule_detail_with_versions() {
    let app = build_app(test_config());

    let (status, body) = json_request(app, "GET", "/api/v1/ops/rules/rule_early_claim", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["summary"]["rule_id"], "rule_early_claim");
    assert_eq!(body["versions"][0]["version"], 1);
    assert!(body["versions"][0]["dsl"]["conditions"].is_array());
}

#[tokio::test]
async fn backtests_candidate_rule_against_samples() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/rules/backtest",
        r#"{
          "rule": {
            "rule_id": "candidate_early_claim",
            "version": 1,
            "name": "Candidate early claim",
            "conditions": [
              {
                "field": "days_since_policy_start",
                "operator": "<=",
                "value": 7
              }
            ],
            "action": {
              "score": 25,
              "alert_code": "EARLY_CLAIM",
              "recommended_action": "ManualReview",
              "reason": "保单生效后 7 天内发生理赔"
            }
          },
          "samples": [
            {
              "external_claim_id": "CLM-MATCH",
              "claim_amount": "8000",
              "currency": "CNY",
              "service_date": "2026-01-06",
              "policy": {
                "external_policy_id": "POL-MATCH",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-NO-MATCH",
              "claim_amount": "500",
              "currency": "CNY",
              "service_date": "2026-02-01",
              "policy": {
                "external_policy_id": "POL-NO-MATCH",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            }
          ]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["sample_count"], 2);
    assert_eq!(body["matched_count"], 1);
    assert_eq!(body["match_rate"], 0.5);
    assert_eq!(body["estimated_saving"], "800.00");
    assert_eq!(body["matched_claim_ids"][0], "CLM-MATCH");
}

#[tokio::test]
async fn discovers_candidate_rules_from_labeled_samples() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/rules/discover",
        r#"{
          "min_support": 1,
          "samples": [
            {
              "external_claim_id": "CLM-FWA-EARLY-HIGH",
              "claim_amount": "9000",
              "currency": "CNY",
              "service_date": "2026-01-05",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-FWA-EARLY-HIGH",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-NORMAL-LATE-LOW",
              "claim_amount": "500",
              "currency": "CNY",
              "service_date": "2026-03-01",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-NORMAL-LATE-LOW",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-NORMAL-LATE-HIGH",
              "claim_amount": "9000",
              "currency": "CNY",
              "service_date": "2026-03-01",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-NORMAL-LATE-HIGH",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            }
          ]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["sample_count"], 3);
    assert_eq!(body["positive_count"], 1);
    let candidate = &body["candidates"][0];
    assert_eq!(candidate["rule"]["rule_id"], "candidate_early_high_amount");
    assert_eq!(candidate["support"], 1);
    assert_eq!(candidate["precision"], 1.0);
    assert!(candidate["lift"].as_f64().unwrap() > 1.0);
    assert_eq!(candidate["false_positive_rate"], 0.0);
    assert_eq!(candidate["estimated_saving"], "900.00");
    assert!(candidate["explanation"]
        .as_str()
        .unwrap()
        .contains("保单生效"));
}

#[tokio::test]
async fn advances_rule_lifecycle() {
    let app = build_app(test_config());

    for (uri, expected_status) in [
        ("/api/v1/ops/rules/rule_early_claim/submit", "submitted"),
        ("/api/v1/ops/rules/rule_early_claim/approve", "approved"),
        ("/api/v1/ops/rules/rule_early_claim/publish", "active"),
    ] {
        let (status, body) = json_request(app.clone(), "POST", uri, "{}").await;

        assert_eq!(status, StatusCode::OK);
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body["rule_id"], "rule_early_claim");
        assert_eq!(body["status"], expected_status);
    }
}
