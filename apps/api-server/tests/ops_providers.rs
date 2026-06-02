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

async fn json_request(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: &str,
) -> (StatusCode, serde_json::Value) {
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
    let body = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}));
    (status, body)
}

#[tokio::test]
async fn returns_provider_risk_summary_from_scoring_profiles() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-PROVIDER-SUMMARY-1",
            "claim_amount": "18000",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "items": [
            {
              "item_code": "IMG-901",
              "item_type": "procedure",
              "description": "High cost imaging",
              "quantity": 1,
              "unit_amount": "18000",
              "total_amount": "18000"
            }
          ],
          "member": {
            "external_member_id": "MBR-PROVIDER-SUMMARY-1"
          },
          "policy": {
            "external_policy_id": "POL-PROVIDER-SUMMARY-1",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "20000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-PROVIDER-SUMMARY-1",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "Medium"
          },
          "provider_profile": {
            "specialty": "imaging",
            "network_status": "in_network",
            "windows": [
              {
                "window_days": 90,
                "claim_count": 126,
                "total_claim_amount": "420000",
                "high_cost_item_ratio": 0.72,
                "diagnosis_procedure_mismatch_rate": 0.38,
                "peer_amount_percentile": 97,
                "peer_frequency_percentile": 96,
                "review_failure_count": 3,
                "confirmed_fwa_count": 4,
                "false_positive_count": 1
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, summary) =
        json_request(app, "GET", "/api/v1/ops/providers/risk-summary", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary["provider_count"], 1);
    assert_eq!(summary["review_required_count"], 1);
    assert_eq!(summary["high_risk_count"], 1);
    assert_eq!(
        summary["providers"][0]["provider_id"],
        "PRV-PROVIDER-SUMMARY-1"
    );
    assert_eq!(summary["providers"][0]["claim_count"], 1);
    assert_eq!(summary["providers"][0]["specialty"], "imaging");
    assert_eq!(summary["providers"][0]["network_status"], "in_network");
    assert_eq!(summary["providers"][0]["review_failure_count"], 3);
    assert_eq!(summary["providers"][0]["confirmed_fwa_count"], 4);
    assert_eq!(summary["providers"][0]["false_positive_count"], 1);
    assert_eq!(summary["providers"][0]["review_required"], true);
    assert_eq!(summary["providers"][0]["review_route"], "provider_review");
    assert!(summary["providers"][0]["risk_score"].as_u64().unwrap() >= 80);
    assert!(summary["providers"][0]["outlier_flags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|flag| flag == "peer_amount_p97"));
    assert!(summary["providers"][0]["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|evidence| evidence == "provider_profile:PRV-PROVIDER-SUMMARY-1:90d"));
}

#[tokio::test]
async fn returns_provider_graph_risk_summary_from_relationships() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-PROVIDER-GRAPH-SUMMARY-1",
            "claim_amount": "9000",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "items": [
            {
              "item_code": "IMG-910",
              "item_type": "procedure",
              "description": "High cost imaging",
              "quantity": 1,
              "unit_amount": "9000",
              "total_amount": "9000"
            }
          ],
          "member": {
            "external_member_id": "MBR-PROVIDER-GRAPH-SUMMARY-1"
          },
          "policy": {
            "external_policy_id": "POL-PROVIDER-GRAPH-SUMMARY-1",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-PROVIDER-GRAPH-SUMMARY-1",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "Medium"
          },
          "provider_relationships": {
            "high_risk_neighbor_ratio": 0.34,
            "provider_patient_overlap_score": 0.68,
            "referral_concentration_score": 0.72,
            "connected_confirmed_fwa_count": 2,
            "network_component_risk_score": 82,
            "evidence_refs": ["relationship_edges:PRV-PROVIDER-GRAPH-SUMMARY-1"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, summary) =
        json_request(app, "GET", "/api/v1/ops/providers/risk-summary", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary["provider_count"], 1);
    assert_eq!(summary["review_required_count"], 1);
    assert_eq!(summary["high_risk_count"], 1);
    assert_eq!(
        summary["providers"][0]["provider_id"],
        "PRV-PROVIDER-GRAPH-SUMMARY-1"
    );
    assert_eq!(
        summary["providers"][0]["review_route"],
        "provider_graph_review"
    );
    assert!(summary["providers"][0]["risk_score"].as_u64().unwrap() >= 90);
    assert!(
        summary["providers"][0]["network_risk_score"]
            .as_u64()
            .unwrap()
            >= 90
    );
    assert!(summary["providers"][0]["graph_reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason.as_str().unwrap().contains("关系邻居")));
    assert!(summary["providers"][0]["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|evidence| evidence == "relationship_edges:PRV-PROVIDER-GRAPH-SUMMARY-1"));
}
