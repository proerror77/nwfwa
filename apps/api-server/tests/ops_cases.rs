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
async fn creates_lead_from_high_risk_scoring_and_triages_to_case() {
    let app = build_app(test_config());

    let (status, score) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-LEAD-1001",
            "claim_amount": "9000",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "items": [
            {
              "item_code": "PROC-001",
              "item_type": "procedure",
              "description": "Imaging",
              "quantity": 1,
              "unit_amount": "9000",
              "total_amount": "9000"
            }
          ],
          "member": {
            "external_member_id": "MBR-LEAD-1001"
          },
          "policy": {
            "external_policy_id": "POL-LEAD-1001",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-LEAD-1001",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "High"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(score["risk_score"].as_u64().unwrap() >= 70);

    let (status, leads) = json_request(app.clone(), "GET", "/api/v1/ops/leads", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let lead = leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lead| lead["claim_id"] == "CLM-LEAD-1001")
        .expect("lead generated from high risk scoring");
    assert_eq!(lead["lead_source"], "scoring_run");
    assert_eq!(lead["status"], "new");
    assert_eq!(lead["disposition"], "pending_triage");
    assert!(lead["scheme_family"].as_str().unwrap().len() > 3);
    assert!(!lead["evidence_refs"].as_array().unwrap().is_empty());
    let lead_id = lead["lead_id"].as_str().unwrap();

    let (status, triage) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "open_case",
          "assignee": "siu-reviewer-1",
          "reviewer": "medical-reviewer-1",
          "priority": "high",
          "notes": "Open investigation from high-risk FWA lead."
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(triage["case"]["lead_id"], lead_id);
    assert_eq!(triage["case"]["claim_id"], "CLM-LEAD-1001");
    assert_eq!(triage["case"]["status"], "triage");
    assert_eq!(triage["case"]["assignee"], "siu-reviewer-1");
    assert_eq!(triage["case"]["reviewer"], "medical-reviewer-1");
    assert_eq!(triage["case"]["priority"], "high");
    assert!(triage["audit_id"].as_str().unwrap().starts_with("aud_"));

    let (status, cases) = json_request(app, "GET", "/api/v1/ops/cases", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert!(cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .any(|case| case["lead_id"] == lead_id));
}
