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
async fn returns_dashboard_summary_from_scoring_and_pilot_events() {
    let app = build_app(test_config());

    let (status, score) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-0287",
            "claim_amount": "8000",
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
              "unit_amount": "8000",
              "total_amount": "8000"
            }
          ],
          "member": {
            "external_member_id": "MBR-0287"
          },
          "policy": {
            "external_policy_id": "POL-0287",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-0287",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "High"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(score["rag"], "Red");

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-0287",
          "investigation_id": "INV-1001",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "TPA investigation confirmed over-treatment signals.",
          "evidence_refs": ["agent_run:agent_CLM-0287", "rule_runs:EARLY_CLAIM", "knowledge_cases:KC-1001"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, sample) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/audit-samples",
        r#"{
          "sample_mode": "qa_calibration",
          "population_definition": "High risk claims for QA dashboard",
          "inclusion_criteria": { "min_risk_score": 70 },
          "deterministic_seed": "dashboard-qa",
          "sample_size": 1,
          "reviewer": "qa-reviewer-1",
          "assignment_queue": "QA Review"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let selected_lead = &sample["selected_leads"].as_array().unwrap()[0];
    let qa_case_id = format!(
        "qa_{}_{}",
        sample["sample_id"].as_str().unwrap(),
        selected_lead["lead_id"].as_str().unwrap()
    );

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        &format!(
            r#"{{
          "qa_case_id": "{qa_case_id}",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Reviewer should attach provider history evidence.",
          "evidence_refs": ["audit:investigation.result.received", "rule_runs:EARLY_CLAIM"]
        }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, dashboard) = json_request(app, "GET", "/api/v1/ops/dashboard/summary", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(dashboard["suspected_claims"], 1);
    assert_eq!(dashboard["confirmed_fwa"], 1);
    assert_eq!(dashboard["qa_reviews"], 1);
    assert_eq!(dashboard["investigation_results"], 1);
    assert_eq!(dashboard["qa_queue"]["sampled_cases"], 1);
    assert_eq!(dashboard["qa_queue"]["open_cases"], 0);
    assert_eq!(dashboard["qa_queue"]["reviewed_cases"], 1);
    assert_eq!(dashboard["risk_amount"], "8000");
    assert_eq!(dashboard["saving_amount"], "8200.00");
    let attributions = dashboard["saving_attributions"].as_array().unwrap();
    assert_eq!(attributions.len(), 2);
    assert!(attributions.iter().any(|attribution| {
        attribution["source_type"] == "agent"
            && attribution["source_id"] == "agent_CLM-0287"
            && attribution["saving_amount"] == "4100.00"
    }));
    assert!(attributions.iter().any(|attribution| {
        attribution["source_type"] == "rule"
            && attribution["source_id"] == "EARLY_CLAIM"
            && attribution["saving_amount"] == "4100.00"
    }));
    assert_eq!(dashboard["rag_distribution"]["Red"], 1);
    assert!(dashboard["rule_hits"].as_u64().unwrap() >= 1);
    assert_eq!(dashboard["model_scores"]["baseline_fwa"]["scored_runs"], 1);
    assert!(
        dashboard["model_scores"]["baseline_fwa"]["average_score"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    assert_eq!(
        dashboard["layer_scores"]["L1_PEER_BENCHMARK"]["scored_runs"],
        1
    );
    assert_eq!(
        dashboard["layer_scores"]["L7_RISK_FUSION_ROUTING"]["scored_runs"],
        1
    );
    assert!(
        dashboard["layer_scores"]["L7_RISK_FUSION_ROUTING"]["average_score"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    assert!(
        dashboard["layer_scores"]["L7_RISK_FUSION_ROUTING"]["high_risk_count"]
            .as_u64()
            .unwrap()
            >= 1
    );
    assert_eq!(dashboard["label_pool"]["total_labels"], 3);
    assert_eq!(dashboard["label_pool"]["approved_for_training"], 2);
    assert_eq!(dashboard["label_pool"]["needs_review"], 1);
    assert_eq!(dashboard["label_pool"]["rule_feedback"], 1);
    assert_eq!(dashboard["label_pool"]["model_feedback"], 1);
    assert_eq!(dashboard["label_pool"]["workflow_feedback"], 1);
}

#[tokio::test]
async fn dashboard_summary_requires_api_key() {
    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/ops/dashboard/summary")
        .body(Body::empty())
        .unwrap();
    let response = build_app(test_config()).oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
