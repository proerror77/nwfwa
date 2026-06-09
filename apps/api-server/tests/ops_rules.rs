use api_server::app::build_app;
use axum::http::StatusCode;

#[path = "ops_rules/candidate_reviews.rs"]
mod candidate_reviews;
#[path = "ops_rules/discovery.rs"]
mod discovery;
#[path = "ops_rules/library.rs"]
mod library;
#[path = "ops_rules/promotion_gates.rs"]
mod promotion_gates;
#[path = "ops_rules/support.rs"]
mod support;

use support::{json_request, rule_lifecycle_payload, seed_rule_promotion_evidence, test_config};

#[tokio::test]
async fn records_rule_candidate_and_lifecycle_audit_events() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidates",
        r#"{
          "owner": "rule-discovery",
          "rule": {
            "rule_id": "candidate_audit_rule",
            "version": 1,
            "name": "Audited candidate rule",
            "scheme_family": "high_risk_claim",
            "conditions": [
              {
                "field": "days_since_policy_start",
                "operator": "<=",
                "value": 10
              }
            ],
            "action": {
              "score": 25,
              "alert_code": "AUDITED_CANDIDATE",
              "recommended_action": "ManualReview",
              "reason": "候选规则需要治理审计"
            }
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidate_audit_rule/submit",
        &rule_lifecycle_payload("candidate_audit_rule", 1),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) =
        json_request(app, "GET", "/api/v1/ops/rules/candidate_audit_rule", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let audit_events = body["audit_events"].as_array().unwrap();
    assert_eq!(audit_events.len(), 2);
    assert_eq!(audit_events[0]["event_type"], "rule.candidate.saved");
    assert_eq!(
        audit_events[0]["payload"]["rule_id"],
        "candidate_audit_rule"
    );
    assert_eq!(
        audit_events[0]["payload"]["customer_scope_id"],
        "demo-customer"
    );
    assert_eq!(audit_events[0]["payload"]["to_status"], "draft");
    assert_eq!(audit_events[1]["event_type"], "rule.status.changed");
    assert_eq!(
        audit_events[1]["payload"]["customer_scope_id"],
        "demo-customer"
    );
    assert_eq!(audit_events[1]["payload"]["from_status"], "draft");
    assert_eq!(audit_events[1]["payload"]["to_status"], "submitted");
    assert_eq!(
        audit_events[1]["evidence_refs"][0],
        "rules:candidate_audit_rule:v1"
    );
}

#[tokio::test]
async fn returns_rule_performance_metrics_from_scoring_and_outcomes() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-RULE-TRUE",
            "claim_amount": "8000",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "policy": {
            "external_policy_id": "POL-RULE-TRUE",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000"
          },
          "member": {
            "external_member_id": "MBR-RULE-TRUE"
          },
          "provider": {
            "external_provider_id": "PRV-RULE-TRUE",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-RULE-FALSE",
            "claim_amount": "100",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "policy": {
            "external_policy_id": "POL-RULE-FALSE",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000"
          },
          "member": {
            "external_member_id": "MBR-RULE-FALSE"
          },
          "provider": {
            "external_provider_id": "PRV-RULE-FALSE",
            "name": "Northwind Clinic",
            "provider_type": "clinic",
            "region": "SH"
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
          "claim_id": "CLM-RULE-TRUE",
          "investigation_id": "INV-RULE-TRUE",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "Confirmed FWA.",
          "evidence_refs": ["rule_runs:EARLY_CLAIM"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-RULE-FALSE",
          "investigation_id": "INV-RULE-FALSE",
          "outcome": "cleared",
          "confirmed_fwa": false,
          "saving_amount": "0.00",
          "currency": "CNY",
          "notes": "Cleared after investigation.",
          "evidence_refs": ["rule_runs:EARLY_CLAIM"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(app, "GET", "/api/v1/ops/rules/performance", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let rules = body["rules"].as_array().unwrap();
    let early_claim = rules
        .iter()
        .find(|rule| rule["rule_id"] == "rule_early_claim")
        .expect("early claim rule performance");
    assert_eq!(early_claim["trigger_count"], 2);
    assert_eq!(early_claim["reviewed_count"], 2);
    assert_eq!(early_claim["confirmed_fwa_count"], 1);
    assert_eq!(early_claim["false_positive_count"], 1);
    assert_eq!(early_claim["saving_amount"], "8200.00");
    assert_eq!(early_claim["precision"], 0.5);
    assert_eq!(early_claim["false_positive_rate"], 0.5);
    assert_eq!(early_claim["mark_rate"], 1.0);
    assert!(early_claim["roi"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn advances_rule_lifecycle() {
    let app = build_app(test_config());
    seed_rule_promotion_evidence(app.clone()).await;

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/submit",
        r#"{"evidence_refs": []}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "MISSING_RULE_LIFECYCLE_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/submit",
        r#"{"evidence_refs": [" "]}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "MISSING_RULE_LIFECYCLE_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/submit",
        r#"{"evidence_refs": ["email:alice@example.com"]}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_RULE_LIFECYCLE");

    for (uri, expected_status) in [
        ("/api/v1/ops/rules/rule_early_claim/submit", "submitted"),
        ("/api/v1/ops/rules/rule_early_claim/approve", "approved"),
        ("/api/v1/ops/rules/rule_early_claim/publish", "active"),
    ] {
        let (status, body) = json_request(
            app.clone(),
            "POST",
            uri,
            &rule_lifecycle_payload("rule_early_claim", 1),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body["rule_id"], "rule_early_claim");
        assert_eq!(body["status"], expected_status);
    }
}

#[tokio::test]
async fn blocks_rule_publish_before_approval() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/publish",
        &rule_lifecycle_payload("rule_early_claim", 1),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "RULE_APPROVAL_REQUIRED");

    let (status, body) = json_request(app, "GET", "/api/v1/ops/rules/rule_early_claim", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["summary"]["status"], "active");
}

#[tokio::test]
async fn blocks_rule_approval_before_submit() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/approve",
        &rule_lifecycle_payload("rule_early_claim", 1),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "RULE_STATUS_REQUIRED");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("rule must be submitted before approved"));

    let (status, body) = json_request(app, "GET", "/api/v1/ops/rules/rule_early_claim", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["summary"]["status"], "active");
}

#[tokio::test]
async fn blocks_rule_publish_when_promotion_gates_are_blocked() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/submit",
        &rule_lifecycle_payload("rule_early_claim", 1),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/approve",
        &rule_lifecycle_payload("rule_early_claim", 1),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/publish",
        &rule_lifecycle_payload("rule_early_claim", 1),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "RULE_PROMOTION_GATES_BLOCKED");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("backtest evidence missing"));

    let (status, body) = json_request(app, "GET", "/api/v1/ops/rules/rule_early_claim", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["summary"]["status"], "approved");
    assert!(body["summary"]["active_version"].is_null());
}

#[tokio::test]
async fn rolls_back_active_rule_with_audit_event() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/rollback",
        &rule_lifecycle_payload("rule_early_claim", 1),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["rule_id"], "rule_early_claim");
    assert_eq!(body["status"], "approved");
    assert!(body["active_version"].is_null());
    assert_eq!(body["latest_version"], 1);

    let (status, body) = json_request(app, "GET", "/api/v1/ops/rules/rule_early_claim", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["summary"]["status"], "approved");
    assert!(body["summary"]["active_version"].is_null());
    let rollback_event = body["audit_events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "rule.rollback.completed")
        .expect("rollback should be audited");
    assert_eq!(rollback_event["payload"]["from_status"], "active");
    assert_eq!(rollback_event["payload"]["to_status"], "approved");
    assert_eq!(rollback_event["payload"]["rule_version"], 1);
    assert_eq!(
        rollback_event["payload"]["customer_scope_id"],
        "demo-customer"
    );
    assert_eq!(
        rollback_event["evidence_refs"][0],
        "rules:rule_early_claim:v1"
    );
}

#[tokio::test]
async fn blocks_rule_rollback_when_rule_is_not_active() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/rollback",
        &rule_lifecycle_payload("rule_early_claim", 1),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/rules/rule_early_claim/rollback",
        &rule_lifecycle_payload("rule_early_claim", 1),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "RULE_ROLLBACK_REQUIRES_ACTIVE");
}
