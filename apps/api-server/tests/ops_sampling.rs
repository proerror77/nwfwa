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

struct SampleClaimProfile<'a> {
    product_code: &'a str,
    provider_type: &'a str,
    provider_region: &'a str,
    provider_name: &'a str,
    review_mode: Option<&'a str>,
}

async fn score_high_risk_claim(app: axum::Router, claim_id: &str, amount: &str) {
    score_high_risk_claim_with_context(
        app,
        claim_id,
        amount,
        "MED",
        "hospital",
        "SH",
        "Northwind Hospital",
    )
    .await;
}

async fn score_high_risk_claim_with_context(
    app: axum::Router,
    claim_id: &str,
    amount: &str,
    product_code: &str,
    provider_type: &str,
    provider_region: &str,
    provider_name: &str,
) {
    score_high_risk_claim_with_review_mode(
        app,
        claim_id,
        amount,
        SampleClaimProfile {
            product_code,
            provider_type,
            provider_region,
            provider_name,
            review_mode: None,
        },
    )
    .await;
}

async fn score_high_risk_claim_with_review_mode(
    app: axum::Router,
    claim_id: &str,
    amount: &str,
    profile: SampleClaimProfile<'_>,
) {
    let review_mode_field = profile
        .review_mode
        .map(|value| format!(r#""review_mode": "{value}","#))
        .unwrap_or_default();
    let product_code = profile.product_code;
    let provider_type = profile.provider_type;
    let provider_region = profile.provider_region;
    let provider_name = profile.provider_name;
    let body = format!(
        r#"{{
          "source_system": "tpa-demo",
          {review_mode_field}
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
            "product_code": "{product_code}",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000",
            "currency": "CNY"
          }},
          "provider": {{
            "external_provider_id": "PRV-{claim_id}",
            "name": "{provider_name}",
            "provider_type": "{provider_type}",
            "region": "{provider_region}",
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

#[tokio::test]
async fn stratified_sample_spans_operational_strata() {
    let app = build_app(test_config());

    score_high_risk_claim_with_context(
        app.clone(),
        "CLM-STRATA-A1",
        "9900",
        "MED",
        "hospital",
        "SH",
        "Northwind Hospital",
    )
    .await;
    score_high_risk_claim_with_context(
        app.clone(),
        "CLM-STRATA-A2",
        "9800",
        "MED",
        "hospital",
        "SH",
        "Northwind Hospital",
    )
    .await;
    score_high_risk_claim_with_context(
        app.clone(),
        "CLM-STRATA-Z1",
        "9000",
        "DENTAL",
        "clinic",
        "BJ",
        "Capital Clinic",
    )
    .await;
    score_high_risk_claim_with_context(
        app.clone(),
        "CLM-STRATA-Z2",
        "8900",
        "DENTAL",
        "clinic",
        "BJ",
        "Capital Clinic",
    )
    .await;

    let (status, sample) = json_request(
        app,
        "POST",
        "/api/v1/ops/audit-samples",
        r#"{
          "sample_mode": "stratified",
          "population_definition": "Stratified FWA QA sample by scheme, provider, region, policy, and risk band",
          "inclusion_criteria": {
            "min_risk_score": 70
          },
          "deterministic_seed": "strata-week-1",
          "sample_size": 2,
          "reviewer": "qa-reviewer-2",
          "assignment_queue": "QA Review"
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(sample["sample_mode"], "stratified");
    assert_eq!(sample["selection_method"], "stratified_round_robin");

    let selected = sample["selected_leads"].as_array().unwrap();
    assert_eq!(selected.len(), 2);
    assert!(selected
        .iter()
        .all(|lead| lead["scheme_family"].as_str().is_some()));
    assert!(selected
        .iter()
        .all(|lead| lead["risk_band"].as_str().is_some()));
    assert!(selected
        .iter()
        .any(|lead| lead["provider_type"] == "hospital"
            && lead["provider_region"] == "SH"
            && lead["policy_type"] == "MED"));
    assert!(selected.iter().any(|lead| lead["provider_type"] == "clinic"
        && lead["provider_region"] == "BJ"
        && lead["policy_type"] == "DENTAL"));
    assert!(selected
        .iter()
        .all(|lead| lead["strata_key"].as_str().unwrap().contains("scheme=")));
    assert_eq!(sample["outcome_distribution"]["selected_count"], 2);
    assert!(sample["outcome_distribution"]["strata_distribution"]
        .as_object()
        .unwrap()
        .keys()
        .any(|key| key.contains("provider_type=hospital")));
    assert!(sample["outcome_distribution"]["strata_distribution"]
        .as_object()
        .unwrap()
        .keys()
        .any(|key| key.contains("provider_type=clinic")));
}

#[tokio::test]
async fn post_payment_audit_samples_only_post_payment_leads() {
    let app = build_app(test_config());

    score_high_risk_claim_with_review_mode(
        app.clone(),
        "CLM-PREPAY-HIGH",
        "9900",
        SampleClaimProfile {
            product_code: "MED",
            provider_type: "hospital",
            provider_region: "SH",
            provider_name: "Northwind Hospital",
            review_mode: Some("pre_payment"),
        },
    )
    .await;
    score_high_risk_claim_with_review_mode(
        app.clone(),
        "CLM-POSTPAY-RECOVERY",
        "9000",
        SampleClaimProfile {
            product_code: "MED",
            provider_type: "hospital",
            provider_region: "SH",
            provider_name: "Northwind Hospital",
            review_mode: Some("post_payment"),
        },
    )
    .await;

    let (status, sample) = json_request(
        app,
        "POST",
        "/api/v1/ops/audit-samples",
        r#"{
          "sample_mode": "post_payment_audit",
          "population_definition": "Post-payment recovery and rule discovery sample",
          "inclusion_criteria": {
            "min_risk_score": 70
          },
          "deterministic_seed": "post-payment-week-1",
          "sample_size": 2,
          "reviewer": "recovery-auditor-1",
          "assignment_queue": "Post-payment Audit"
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(sample["sample_mode"], "post_payment_audit");
    assert_eq!(sample["selection_method"], "risk_score_desc_post_payment");
    let selected = sample["selected_leads"].as_array().unwrap();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0]["claim_id"], "CLM-POSTPAY-RECOVERY");
    assert_eq!(selected[0]["review_mode"], "post_payment");
    assert_eq!(sample["outcome_distribution"]["selected_count"], 1);
    assert_eq!(
        sample["outcome_distribution"]["review_mode_distribution"]["post_payment"],
        1
    );
}

#[tokio::test]
async fn qa_calibration_rotates_away_from_reviewer_repeat_leads() {
    let app = build_app(test_config());

    score_high_risk_claim(app.clone(), "CLM-QA-CAL-HIGH", "9900").await;
    score_high_risk_claim(app.clone(), "CLM-QA-CAL-LOWER", "8000").await;

    let (status, first_sample) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/audit-samples",
        r#"{
          "sample_mode": "risk_ranked",
          "population_definition": "Initial reviewer calibration baseline",
          "inclusion_criteria": {
            "min_risk_score": 70
          },
          "deterministic_seed": "qa-calibration-initial",
          "sample_size": 1,
          "reviewer": "qa-calibrator-1",
          "assignment_queue": "QA Calibration"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        first_sample["selected_leads"][0]["claim_id"],
        "CLM-QA-CAL-HIGH"
    );

    let (status, calibration_sample) = json_request(
        app,
        "POST",
        "/api/v1/ops/audit-samples",
        r#"{
          "sample_mode": "qa_calibration",
          "population_definition": "Reviewer consistency rotation sample",
          "inclusion_criteria": {
            "min_risk_score": 70
          },
          "deterministic_seed": "qa-calibration-week-2",
          "sample_size": 1,
          "reviewer": "qa-calibrator-1",
          "assignment_queue": "QA Calibration"
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(calibration_sample["sample_mode"], "qa_calibration");
    assert_eq!(
        calibration_sample["selection_method"],
        "reviewer_consistency_rotation"
    );
    assert_eq!(
        calibration_sample["selected_leads"][0]["claim_id"],
        "CLM-QA-CAL-LOWER"
    );
    assert_eq!(
        calibration_sample["selected_leads"][0]["prior_reviewer_sample_count"],
        0
    );
    assert_eq!(
        calibration_sample["outcome_distribution"]["reviewer_history_distribution"]
            ["new_to_reviewer"],
        1
    );
}
