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

async fn score_high_risk_claim(app: axum::Router, claim_id: &str, amount: &str) {
    let body = format!(
        r#"{{
          "source_system": "tpa-demo",
          "claim": {{
            "external_claim_id": "{claim_id}",
            "claim_amount": "{amount}",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          }},
          "items": [
            {{
              "item_code": "PROC-001",
              "item_type": "procedure",
              "description": "Imaging",
              "quantity": 1,
              "unit_amount": "{amount}",
              "total_amount": "{amount}"
            }}
          ],
          "member": {{ "external_member_id": "MBR-{claim_id}" }},
          "policy": {{
            "external_policy_id": "POL-{claim_id}",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000",
            "currency": "CNY"
          }},
          "provider": {{
            "external_provider_id": "PRV-{claim_id}",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "High"
          }}
        }}"#
    );
    let (status, response) = json_request(app, "POST", "/api/v1/claims/score", &body).await;
    assert_eq!(status, StatusCode::OK);
    assert!(response["risk_score"].as_u64().unwrap() >= 70);
}

#[tokio::test]
async fn creates_audit_sample_from_ranked_leads() {
    let app = build_app(test_config());

    score_high_risk_claim(app.clone(), "CLM-SAMPLE-1", "9000").await;
    score_high_risk_claim(app.clone(), "CLM-SAMPLE-2", "8200").await;

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/audit-samples",
        r#"{
          "sample_mode": "unknown",
          "population_definition": "RED and high risk leads for weekly QA",
          "inclusion_criteria": {
            "min_risk_score": 70
          },
          "sample_size": 1,
          "reviewer": "qa-reviewer-1",
          "assignment_queue": "QA Review"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_SAMPLE_MODE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/audit-samples",
        r#"{
          "sample_mode": "risk_ranked",
          "population_definition": " ",
          "inclusion_criteria": {
            "min_risk_score": 70
          },
          "sample_size": 1,
          "reviewer": "qa-reviewer-1",
          "assignment_queue": "QA Review"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_POPULATION_DEFINITION");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/audit-samples",
        r#"{
          "sample_mode": "risk_ranked",
          "population_definition": "RED and high risk leads for weekly QA",
          "inclusion_criteria": {
            "min_risk_score": 70
          },
          "sample_size": 1,
          "reviewer": " ",
          "assignment_queue": "QA Review"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_SAMPLE_REVIEWER");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/audit-samples",
        r#"{
          "sample_mode": "risk_ranked",
          "population_definition": "RED and high risk leads for weekly QA",
          "inclusion_criteria": {
            "min_risk_score": 70
          },
          "sample_size": 1,
          "reviewer": "qa-reviewer-1",
          "assignment_queue": " "
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_ASSIGNMENT_QUEUE");

    let (status, sample) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/audit-samples",
        r#"{
          "sample_mode": "risk_ranked",
          "population_definition": "RED and high risk leads for weekly QA",
          "inclusion_criteria": {
            "min_risk_score": 70
          },
          "deterministic_seed": "pilot-week-1",
          "sample_size": 1,
          "reviewer": "qa-reviewer-1",
          "assignment_queue": "QA Review"
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(sample["sample_id"].as_str().unwrap().starts_with("sample_"));
    assert_eq!(sample["sample_mode"], "risk_ranked");
    assert_eq!(sample["selection_method"], "risk_score_desc");
    assert_eq!(sample["sample_size"], 1);
    assert_eq!(sample["reviewer"], "qa-reviewer-1");
    assert_eq!(sample["selected_leads"].as_array().unwrap().len(), 1);
    assert_eq!(sample["outcome_distribution"]["selected_count"], 1);
    assert_eq!(sample["outcome_distribution"]["reviewed_count"], 0);
    assert_eq!(sample["outcome_distribution"]["open_count"], 1);

    let sample_id = sample["sample_id"].as_str().unwrap();
    let lead_id = sample["selected_leads"][0]["lead_id"].as_str().unwrap();
    let qa_case_id = format!("qa_{sample_id}_{lead_id}");
    let claim_id = sample["selected_leads"][0]["claim_id"].as_str().unwrap();
    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        &format!(
            r#"{{
              "qa_case_id": "{qa_case_id}",
              "claim_id": "{claim_id}",
              "qa_conclusion": "issue_found_escalate",
              "issue_type": "medical_necessity_issue",
              "feedback_target": "rules",
              "notes": "QA completed sampled case review.",
              "evidence_refs": ["qa_queue:{qa_case_id}", "audit:scoring.completed"]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, samples) = json_request(app, "GET", "/api/v1/ops/audit-samples", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(samples["samples"].as_array().unwrap().len(), 1);
    assert_eq!(samples["samples"][0]["sample_id"], sample["sample_id"]);
    let distribution = &samples["samples"][0]["outcome_distribution"];
    assert_eq!(distribution["selected_count"], 1);
    assert_eq!(distribution["reviewed_count"], 1);
    assert_eq!(distribution["open_count"], 0);
    assert_eq!(distribution["qa_conclusions"]["issue_found_escalate"], 1);
    assert_eq!(distribution["issue_types"]["medical_necessity_issue"], 1);
    assert_eq!(distribution["feedback_targets"]["rules"], 1);
}
