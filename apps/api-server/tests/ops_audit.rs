use api_server::{
    app::{build_app, build_app_with_parts},
    repository::InMemoryScoringRepository,
};
use axum::http::StatusCode;
use fwa_ml_runtime::HeuristicModelScorer;
use std::sync::Arc;

#[path = "ops_audit/filters.rs"]
mod filters;
#[path = "ops_audit/support.rs"]
mod support;

use support::{json_request, scoped_config, test_config};

#[tokio::test]
async fn audit_queries_are_scoped_to_authenticated_customer() {
    let repository = InMemoryScoringRepository::shared();
    let alpha_app = build_app_with_parts(
        scoped_config("customer-alpha"),
        Arc::new(HeuristicModelScorer),
        repository.clone(),
    );
    let beta_app = build_app_with_parts(
        scoped_config("customer-beta"),
        Arc::new(HeuristicModelScorer),
        repository,
    );

    let (status, score) = json_request(
        alpha_app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-SCOPE-ISOLATION",
            "claim_amount": "9000.00",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "items": [
            {
              "item_code": "PROC-SCOPE",
              "item_type": "procedure",
              "description": "Scope isolation procedure",
              "quantity": 1,
              "unit_amount": "9000.00",
              "total_amount": "9000.00"
            }
          ],
          "member": { "external_member_id": "MBR-SCOPE-ISOLATION" },
          "policy": {
            "external_policy_id": "POL-SCOPE-ISOLATION",
            "product_code": "HEALTH",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000.00",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-SCOPE-ISOLATION",
            "name": "Scope Isolation Clinic",
            "provider_type": "clinic",
            "region": "Shanghai",
            "risk_tier": "High"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(score["claim_id"], serde_json::json!("CLM-SCOPE-ISOLATION"));

    let (status, alpha_audit) = json_request(
        alpha_app.clone(),
        "GET",
        "/api/v1/audit/claims/CLM-SCOPE-ISOLATION",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(alpha_audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["payload"]["customer_scope_id"] == "customer-alpha"));

    let (status, beta_audit) = json_request(
        beta_app.clone(),
        "GET",
        "/api/v1/audit/claims/CLM-SCOPE-ISOLATION",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(beta_audit["events"].as_array().unwrap().is_empty());

    let (status, beta_audit_events) = json_request(
        beta_app.clone(),
        "GET",
        "/api/v1/ops/audit-events?claim_id=CLM-SCOPE-ISOLATION&limit=20",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(beta_audit_events["events"].as_array().unwrap().is_empty());

    let (status, alpha_api_calls) = json_request(
        alpha_app.clone(),
        "GET",
        "/api/v1/ops/api-calls?limit=20",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(alpha_api_calls["calls"]
        .as_array()
        .unwrap()
        .iter()
        .any(|call| {
            call["claim_id"] == "CLM-SCOPE-ISOLATION"
                && call["customer_scope_id"] == "customer-alpha"
        }));

    let (status, beta_api_calls) = json_request(
        beta_app.clone(),
        "GET",
        "/api/v1/ops/api-calls?limit=20",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!beta_api_calls["calls"]
        .as_array()
        .unwrap()
        .iter()
        .any(|call| call["claim_id"] == "CLM-SCOPE-ISOLATION"));

    let (status, alpha_webhooks) =
        json_request(alpha_app, "GET", "/api/v1/ops/webhook-events", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert!(alpha_webhooks["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["claim_id"] == "CLM-SCOPE-ISOLATION"
            && event["customer_scope_id"] == "customer-alpha"));

    let (status, beta_webhooks) =
        json_request(beta_app, "GET", "/api/v1/ops/webhook-events", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!beta_webhooks["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["claim_id"] == "CLM-SCOPE-ISOLATION"));
}

#[tokio::test]
async fn lists_global_audit_events_for_governance_review() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/routing-policies",
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
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(app, "GET", "/api/v1/ops/audit-events?limit=5", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let event = body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "routing_policy.candidate.saved")
        .expect("global audit log should include routing policy lifecycle events");
    assert_eq!(event["actor_role"], "tpa_system");
    assert_eq!(event["payload"]["policy_id"], "audit_visible_policy");
    assert_eq!(event["payload"]["to_status"], "draft");
    assert_eq!(
        event["evidence_refs"][0],
        "routing_policies:audit_visible_policy:v1:pre_payment"
    );
}

#[tokio::test]
async fn lists_audit_backed_tpa_api_calls() {
    let app = build_app(test_config());

    let (status, score) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-API-CALLS",
            "claim_amount": "9000.00",
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
              "unit_amount": "9000.00",
              "total_amount": "9000.00"
            }
          ],
          "member": { "external_member_id": "MBR-API-CALLS" },
          "policy": {
            "external_policy_id": "POL-API-CALLS",
            "product_code": "HEALTH",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000.00",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-API-CALLS",
            "name": "API Call Clinic",
            "provider_type": "clinic",
            "region": "Shanghai",
            "risk_tier": "High"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, investigation) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-API-CALLS",
          "investigation_id": "INV-API-CALLS",
          "outcome": "confirmed_fwa_review_needed",
          "confirmed_fwa": true,
          "financial_impact_type": "estimated_impact",
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "API call observability test writeback.",
          "evidence_refs": ["audit:score", "investigation_results:INV-API-CALLS"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, qa) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-API-CALLS",
          "claim_id": "CLM-API-CALLS",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "API call observability test QA writeback.",
          "evidence_refs": ["audit:score", "qa_reviews:QA-API-CALLS"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(app, "GET", "/api/v1/ops/api-calls?limit=20", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let calls = body["calls"].as_array().unwrap();
    let scoring_call = calls
        .iter()
        .find(|call| call["event_type"] == "scoring.completed")
        .expect("scoring API call should be visible");
    assert_eq!(scoring_call["endpoint"], "/api/v1/claims/score");
    assert_eq!(scoring_call["method"], "POST");
    assert_eq!(scoring_call["status_code"], 200);
    assert_eq!(scoring_call["result"], "succeeded");
    assert_eq!(scoring_call["source_system"], "tpa-demo");
    assert_eq!(scoring_call["actor_role"], "tpa_system");
    assert_eq!(scoring_call["customer_scope_id"], "demo-customer");
    assert_eq!(scoring_call["claim_id"], "CLM-API-CALLS");
    assert_eq!(scoring_call["run_id"], score["run_id"]);
    assert_eq!(scoring_call["audit_id"], score["audit_id"]);
    assert!(!scoring_call["evidence_refs"].as_array().unwrap().is_empty());

    let investigation_call = calls
        .iter()
        .find(|call| call["event_type"] == "investigation.result.received")
        .expect("investigation writeback API call should be visible");
    assert_eq!(
        investigation_call["endpoint"],
        "/api/v1/investigations/results"
    );
    assert_eq!(investigation_call["method"], "POST");
    assert_eq!(investigation_call["status_code"], 200);
    assert_eq!(investigation_call["result"], "succeeded");
    assert_eq!(investigation_call["source_system"], "tpa-demo");
    assert_eq!(investigation_call["actor_role"], "tpa_system");
    assert_eq!(investigation_call["customer_scope_id"], "demo-customer");
    assert_eq!(investigation_call["claim_id"], "CLM-API-CALLS");
    assert_eq!(investigation_call["run_id"], investigation["run_id"]);
    assert_eq!(investigation_call["audit_id"], investigation["audit_id"]);
    assert_eq!(
        investigation_call["idempotency_key"],
        investigation["idempotency_key"]
    );
    assert!(investigation_call["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference == "investigation_results:INV-API-CALLS"));

    let qa_call = calls
        .iter()
        .find(|call| call["event_type"] == "qa.result.received")
        .expect("QA writeback API call should be visible");
    assert_eq!(qa_call["endpoint"], "/api/v1/qa/results");
    assert_eq!(qa_call["method"], "POST");
    assert_eq!(qa_call["status_code"], 200);
    assert_eq!(qa_call["result"], "succeeded");
    assert_eq!(qa_call["source_system"], "tpa-demo");
    assert_eq!(qa_call["actor_role"], "tpa_system");
    assert_eq!(qa_call["customer_scope_id"], "demo-customer");
    assert_eq!(qa_call["claim_id"], "CLM-API-CALLS");
    assert_eq!(qa_call["run_id"], qa["run_id"]);
    assert_eq!(qa_call["audit_id"], qa["audit_id"]);
    assert_eq!(qa_call["idempotency_key"], qa["idempotency_key"]);
    assert!(qa_call["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference == "qa_reviews:QA-API-CALLS"));
}

#[tokio::test]
async fn records_audit_sample_creation_for_governance_review() {
    let app = build_app(test_config());

    let (status, sample) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/audit-samples",
        r#"{
          "sample_mode": "stratified",
          "population_definition": "Governance-visible stratified sample",
          "inclusion_criteria": {
            "min_risk_score": 70,
            "provider_type": "clinic",
            "provider_region": "BJ",
            "policy_type": "DENTAL",
            "risk_band": "critical"
          },
          "deterministic_seed": "audit-sample-governance-week-1",
          "sample_size": 5,
          "reviewer": "qa-governance-reviewer",
          "assignment_queue": "QA Review"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let sample_id = sample["sample_id"].as_str().unwrap();
    let (status, audit_events) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?event_type=audit_sample.created&actor_id=tpa-demo&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let event = audit_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["payload"]["sample_id"] == sample_id)
        .expect("audit sample creation should be written to global audit events");
    assert_eq!(event["event_status"], "succeeded");
    assert_eq!(event["payload"]["sample_mode"], "stratified");
    assert_eq!(
        event["payload"]["selection_method"],
        "stratified_round_robin"
    );
    assert_eq!(
        event["payload"]["inclusion_criteria"]["provider_type"],
        "clinic"
    );
    assert_eq!(
        event["evidence_refs"][0],
        format!("audit_samples:{sample_id}")
    );

    let (status, governance_events) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?event_group=governance&limit=20",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(governance_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event_type"] == "audit_sample.created"
            && event["payload"]["sample_id"] == sample_id));

    let (status, sample_events) = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/ops/audit-events?sample_id={sample_id}&limit=10"),
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(sample_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event_type"] == "audit_sample.created"
            && event["payload"]["sample_id"] == sample_id));

    let (status, sample_events) = json_request(
        app,
        "GET",
        "/api/v1/ops/audit-events?sample_id=missing-sample&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(sample_events["events"].as_array().unwrap().is_empty());
}
