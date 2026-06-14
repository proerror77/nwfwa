use api_server::config::AppConfig;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

pub(super) fn test_config() -> AppConfig {
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

pub(super) fn test_config_with_rule_actors() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        api_key_principals: vec![
            "dev-secret|test-operator|fwa_operator|tpa-demo|demo-customer|ops:*,tpa:*".into(),
            "submit-secret|rule-submitter|fwa_operator|ops-studio|demo-customer|ops:rules:write,ops:rules:approve,ops:rules:publish,ops:rules:review".into(),
            "approve-secret|rule-approver|fwa_operator|ops-studio|demo-customer|ops:rules:write,ops:rules:approve,ops:rules:publish,ops:rules:review".into(),
        ],
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

pub(super) async fn json_request(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: &str,
) -> (StatusCode, String) {
    json_request_with_key(app, method, uri, body, "dev-secret").await
}

pub(super) async fn json_request_with_key(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: &str,
    api_key: &str,
) -> (StatusCode, String) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("x-api-key", api_key)
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

pub(super) fn rule_lifecycle_payload(rule_id: &str, version: u32) -> String {
    format!(r#"{{"evidence_refs":["rules:{rule_id}:v{version}"]}}"#)
}

pub(super) async fn seed_rule_promotion_evidence(app: axum::Router) {
    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-RULE-PROMOTE",
            "claim_amount": "8000",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10",
            "policy": {
              "external_policy_id": "POL-RULE-PROMOTE",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": "10000",
              "currency": "CNY"
            }
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-RULE-PROMOTE",
          "investigation_id": "INV-RULE-PROMOTE",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "800.00",
          "currency": "CNY",
          "notes": "Confirmed FWA for rule promotion evidence.",
          "evidence_refs": ["rule_runs:EARLY_CLAIM"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/rules/backtest",
        r#"{
          "rule": {
            "rule_id": "rule_early_claim",
            "version": 1,
            "name": "Early claim after policy start",
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
              "external_claim_id": "CLM-PROMOTE-TP-1",
              "claim_amount": "8000",
              "currency": "CNY",
              "service_date": "2026-01-06",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-PROMOTE-TP-1",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-PROMOTE-TP-2",
              "claim_amount": "7000",
              "currency": "CNY",
              "service_date": "2026-01-07",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-PROMOTE-TP-2",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-PROMOTE-TN",
              "claim_amount": "500",
              "currency": "CNY",
              "service_date": "2026-03-01",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-PROMOTE-TN",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            }
          ],
          "expected_review_capacity": 5
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["promotion_recommendation"], "eligible_for_review");
}
