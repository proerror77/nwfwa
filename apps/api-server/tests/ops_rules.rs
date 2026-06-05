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

fn rule_lifecycle_payload(rule_id: &str, version: u32) -> String {
    format!(r#"{{"evidence_refs":["rules:{rule_id}:v{version}"]}}"#)
}

async fn seed_rule_promotion_evidence(app: axum::Router) {
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

#[tokio::test]
async fn lists_rule_library() {
    let app = build_app(test_config());

    let (status, body) = json_request(app, "GET", "/api/v1/ops/rules", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let early_claim = body["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["rule_id"] == "rule_early_claim")
        .expect("rule_early_claim should be listed");
    assert_eq!(early_claim["status"], "active");
    assert_eq!(early_claim["active_version"], 1);
    assert_eq!(early_claim["review_mode"], "both");
    assert_eq!(early_claim["scheme_family"], "early_high_value_claim");
    assert_eq!(early_claim["applicability_scope"]["review_mode"], "both");
    assert_eq!(
        early_claim["applicability_scope"]["scheme_family"],
        "early_high_value_claim"
    );
    assert_eq!(early_claim["applicability_scope"]["source"], "rule_dsl");
    assert_eq!(early_claim["backtest_result"]["status"], "not_run");
    assert_eq!(early_claim["estimated_saving"], "0.00");
    assert_eq!(
        early_claim["false_positive_history"]["status"],
        "not_observed"
    );
    assert_eq!(
        early_claim["false_positive_history"]["false_positive_count"],
        0
    );
    assert!(early_claim["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("rules:rule_early_claim:v1")));
}

#[tokio::test]
async fn ships_minimum_mvp_default_rule_set() {
    let app = build_app(test_config());

    let (status, body) = json_request(app, "GET", "/api/v1/ops/rules", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let rules = body["rules"].as_array().unwrap();
    assert!(
        rules.len() >= 16,
        "default rule pack should cover PRD-required FWA rule families"
    );
    let alert_codes = rules
        .iter()
        .map(|rule| rule["alert_code"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    let scheme_families = rules
        .iter()
        .map(|rule| rule["scheme_family"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    for expected in [
        "EARLY_CLAIM",
        "LARGE_LIMIT_USAGE",
        "PEER_P95_AMOUNT",
        "PEER_P99_AMOUNT",
        "EARLY_HIGH_AMOUNT",
        "LOW_MEDICAL_MATCH",
        "MANY_CLAIM_ITEMS",
        "HIGH_COST_SINGLE_ITEM",
        "PROVIDER_HIGH_RISK_TIER",
        "PROVIDER_PROFILE_HIGH",
        "DUPLICATE_CLAIM",
        "UPCODING_COMPLEXITY",
        "UNBUNDLING_COMPONENT_PATTERN",
        "MEDICALLY_UNNECESSARY_SERVICE",
        "SAME_MEMBER_REPEATED_SERVICE",
        "RELATIONSHIP_CONCENTRATION",
    ] {
        assert!(alert_codes.contains(expected), "missing {expected}");
    }
    for expected in [
        "early_high_value_claim",
        "duplicate_billing",
        "upcoding",
        "unbundling",
        "medically_unnecessary_service",
        "excessive_utilization",
        "diagnosis_procedure_mismatch",
        "provider_peer_outlier",
        "relationship_concentration",
    ] {
        assert!(scheme_families.contains(expected), "missing {expected}");
    }
}

#[tokio::test]
async fn returns_rule_detail_with_versions() {
    let app = build_app(test_config());

    let (status, body) = json_request(app, "GET", "/api/v1/ops/rules/rule_early_claim", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["summary"]["rule_id"], "rule_early_claim");
    assert_eq!(body["summary"]["review_mode"], "both");
    assert_eq!(body["summary"]["scheme_family"], "early_high_value_claim");
    assert_eq!(body["versions"][0]["version"], 1);
    assert_eq!(body["versions"][0]["review_mode"], "both");
    assert_eq!(
        body["versions"][0]["scheme_family"],
        "early_high_value_claim"
    );
    assert!(body["versions"][0]["dsl"]["conditions"].is_array());
}

#[tokio::test]
async fn returns_rule_promotion_gates_for_unreviewed_rule() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/ops/rules/rule_early_claim/promotion-gates",
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["rule_id"], "rule_early_claim");
    assert_eq!(body["rule_version"], 1);
    assert_eq!(body["review_mode"], "both");
    assert_eq!(body["decision"], "routing_blocked");
    assert_eq!(body["total_count"], 9);
    assert_eq!(body["passed_count"], 5);
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("backtest evidence missing")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("shadow rollout missing")));
}

#[tokio::test]
async fn records_rule_promotion_review_and_uses_it_for_approval_gate() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/promotion-reviews",
        r#"{
          "decision": "rejected",
          "reviewer": "rule-governance",
          "notes": " ",
          "evidence_refs": ["rules:rule_early_claim:v1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_PROMOTION_REVIEW_NOTES");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/promotion-reviews",
        r#"{
          "decision": "rejected",
          "reviewer": "rule-governance",
          "notes": "Rejected until backtest evidence is attached.",
          "evidence_refs": []
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "MISSING_PROMOTION_REVIEW_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/promotion-reviews",
        r#"{
          "decision": "rejected",
          "reviewer": "rule-governance",
          "notes": "Reviewer contacted alice@example.com about approval evidence.",
          "evidence_refs": ["rules:rule_early_claim:v1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_PROMOTION_REVIEW");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/promotion-reviews",
        r#"{
          "decision": "rejected",
          "reviewer": "rule-governance",
          "notes": "Rejected until backtest evidence is attached.",
          "evidence_refs": ["rules:rule_early_claim:v1", " "]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "MISSING_PROMOTION_REVIEW_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/promotion-reviews",
        r#"{
          "decision": "rejected",
          "reviewer": "rule-governance",
          "notes": "Rejected until backtest evidence is attached.",
          "evidence_refs": ["rules:rule_early_claim:v1"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["rule_id"], "rule_early_claim");
    assert_eq!(body["rule_version"], 1);
    assert_eq!(body["decision"], "rejected");
    assert_eq!(body["reviewer"], "rule-governance");
    assert_eq!(body["evidence_refs"][0], "rules:rule_early_claim:v1");

    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/ops/rules/rule_early_claim/promotion-gates",
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("approval missing")));
    let approval_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Approval before routing")
        .unwrap();
    assert_eq!(approval_gate["passed"], false);
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
              "confirmed_fwa": true,
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
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-NO-MATCH",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-NORMAL",
              "claim_amount": "400",
              "currency": "CNY",
              "service_date": "2026-03-01",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-NORMAL",
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
    assert_eq!(body["sample_count"], 3);
    assert_eq!(body["matched_count"], 1);
    assert_eq!(body["reviewed_count"], 3);
    assert_eq!(body["confirmed_fwa_count"], 2);
    assert_eq!(body["false_positive_count"], 0);
    assert!((body["match_rate"].as_f64().unwrap() - (1.0 / 3.0)).abs() < f64::EPSILON);
    assert_eq!(body["precision"], 1.0);
    assert_eq!(body["recall"], 0.5);
    assert!(body["lift"].as_f64().unwrap() > 1.0);
    assert_eq!(body["false_positive_rate"], 0.0);
    assert_eq!(body["estimated_saving"], "800.00");
    assert_eq!(body["promotion_recommendation"], "needs_more_evidence");
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("recall below 0.60")));
    assert_eq!(body["matched_claim_ids"][0], "CLM-MATCH");
    assert!(body["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("rules:candidate_early_claim:v1")));
}

#[tokio::test]
async fn backtests_dataset_mined_rule_against_parquet_dataset() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/rules/backtest",
        r#"{
          "rule": {
            "rule_id": "candidate_mined_claim_amount_to_limit_ratio_gte_0_8",
            "version": 1,
            "name": "Mined amount ratio rule",
            "conditions": [
              {
                "field": "claim_amount_to_limit_ratio",
                "operator": ">=",
                "value": 0.8
              }
            ],
            "action": {
              "score": 30,
              "alert_code": "MINED_CLAIM_AMOUNT_TO_LIMIT_RATIO",
              "recommended_action": "ManualReview",
              "reason": "数据集挖掘金额比例阈值"
            }
          },
          "dataset_uri": "data/public-mvp/split=train/part-00000.parquet",
          "label_column": "confirmed_fwa",
          "claim_id_column": "claim_id",
          "samples": [],
          "expected_review_capacity": 20
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["sample_count"], 18);
    assert!(body["matched_count"].as_u64().unwrap() > 0);
    assert!(body["precision"].as_f64().unwrap() > 0.0);
    assert!(body["matched_claim_ids"].as_array().unwrap().len() > 0);
    assert!(body["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "dataset:data/public-mvp/split=train/part-00000.parquet"
        )));
}

#[tokio::test]
async fn backtest_recommends_review_when_labeled_metrics_pass() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/rules/backtest",
        r#"{
          "rule": {
            "rule_id": "candidate_early_high_amount",
            "version": 1,
            "name": "Candidate early high amount",
            "conditions": [
              {
                "field": "days_since_policy_start",
                "operator": "<=",
                "value": 10
              },
              {
                "field": "claim_amount_to_limit_ratio",
                "operator": ">=",
                "value": 0.7
              }
            ],
            "action": {
              "score": 30,
              "alert_code": "EARLY_HIGH_AMOUNT",
              "recommended_action": "ManualReview",
              "reason": "保单生效早期发生高额理赔"
            }
          },
          "samples": [
            {
              "external_claim_id": "CLM-TP-1",
              "claim_amount": "9000",
              "currency": "CNY",
              "service_date": "2026-01-05",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-TP-1",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-TP-2",
              "claim_amount": "8000",
              "currency": "CNY",
              "service_date": "2026-01-08",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-TP-2",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-TN",
              "claim_amount": "500",
              "currency": "CNY",
              "service_date": "2026-03-01",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-TN",
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
    assert_eq!(body["matched_count"], 2);
    assert_eq!(body["precision"], 1.0);
    assert_eq!(body["recall"], 1.0);
    assert_eq!(body["promotion_recommendation"], "eligible_for_review");
    assert!(body["blockers"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn persisted_backtest_evidence_feeds_rule_promotion_gates() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
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
              "external_claim_id": "CLM-BT-TP-1",
              "claim_amount": "8000",
              "currency": "CNY",
              "service_date": "2026-01-06",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-BT-TP-1",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-BT-TP-2",
              "claim_amount": "7000",
              "currency": "CNY",
              "service_date": "2026-01-07",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-BT-TP-2",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-BT-TN",
              "claim_amount": "500",
              "currency": "CNY",
              "service_date": "2026-03-01",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-BT-TN",
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

    let (status, body) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/rules/rule_early_claim/promotion-gates",
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["reviewed_count"], 3);
    assert_eq!(body["saving_amount"], "1500.00");
    let blockers = body["blockers"].as_array().unwrap();
    assert!(!blockers.contains(&serde_json::json!("backtest evidence missing")));
    assert!(!blockers.contains(&serde_json::json!("estimated saving missing")));
    assert!(!blockers.contains(&serde_json::json!("false-positive burden missing")));
    assert!(blockers.contains(&serde_json::json!("shadow rollout missing")));
    let backtest_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Deterministic backtest evidence")
        .unwrap();
    assert_eq!(backtest_gate["evidence_source"], "backtest");
    let shadow_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Shadow or limited rollout")
        .unwrap();
    assert_eq!(shadow_gate["evidence_source"], "missing");

    let (status, body) = json_request(app, "GET", "/api/v1/ops/rules/rule_early_claim", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(body["audit_events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event_type"] == "rule.backtest.completed"));
    assert_eq!(body["summary"]["backtest_result"]["status"], "completed");
    assert_eq!(body["summary"]["backtest_result"]["sample_count"], 3);
    assert_eq!(
        body["summary"]["backtest_result"]["estimated_saving"],
        "1500.00"
    );
    assert_eq!(body["summary"]["estimated_saving"], "1500.00");
    assert_eq!(
        body["summary"]["false_positive_history"]["status"],
        "observed"
    );
    assert_eq!(
        body["summary"]["false_positive_history"]["false_positive_count"],
        0
    );
    assert!(body["summary"]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("rules:rule_early_claim:v1")));
}

#[tokio::test]
async fn rule_promotion_gates_block_unresolved_backtest_blockers() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
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
              "external_claim_id": "CLM-BT-UNDERPOWERED",
              "claim_amount": "8000",
              "currency": "CNY",
              "service_date": "2026-01-06",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-BT-UNDERPOWERED",
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
    assert_eq!(body["promotion_recommendation"], "needs_more_evidence");
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("reviewed sample count below 2")));

    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/ops/rules/rule_early_claim/promotion-gates",
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["decision"], "routing_blocked");
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("backtest blockers unresolved")));
    let backtest_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Deterministic backtest evidence")
        .unwrap();
    assert_eq!(backtest_gate["passed"], false);
    assert_eq!(backtest_gate["blocker"], "backtest blockers unresolved");
    assert_eq!(backtest_gate["evidence_source"], "backtest");
}

#[tokio::test]
async fn rule_promotion_gates_include_rule_feedback_labels() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-RULE-GATE-1",
          "claim_id": "CLM-RULE-GATE-1",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "QA found a rule handling issue that must be reviewed before routing impact.",
          "evidence_refs": ["qa_reviews:QA-RULE-GATE-1", "rule_runs:EARLY_CLAIM"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-RULE-GATE-1/status",
        r#"{
          "status": "in_progress",
          "actor_id": "rule-ops",
          "notes": "Rule operator accepted the feedback for review.",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-RULE-GATE-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/rules/rule_early_claim/promotion-gates",
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let feedback_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Rule feedback governance")
        .expect("rule promotion gates should include rule feedback governance");
    assert_eq!(feedback_gate["passed"], false);
    assert_eq!(feedback_gate["evidence_source"], "labels");
    assert_eq!(feedback_gate["blocker"], "rule feedback labels need review");
    let closure_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Rule QA feedback closure")
        .expect("rule promotion gates should include QA feedback closure");
    assert_eq!(closure_gate["passed"], false);
    assert_eq!(closure_gate["evidence_source"], "qa_feedback");
    assert_eq!(closure_gate["blocker"], "unresolved rule QA feedback");
    assert_eq!(body["open_rule_feedback_count"], 0);
    assert_eq!(body["unresolved_rule_feedback_count"], 1);
    assert_eq!(body["approved_label_count"], 0);
    assert_eq!(body["needs_review_label_count"], 1);
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("rule feedback labels need review")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("unresolved rule QA feedback")));

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-RULE-GATE-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "rule-ops",
          "notes": "Rule operator resolved the feedback after threshold review.",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-RULE-GATE-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/ops/rules/rule_early_claim/promotion-gates",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let feedback_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Rule feedback governance")
        .unwrap();
    assert_eq!(feedback_gate["passed"], true);
    let closure_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Rule QA feedback closure")
        .unwrap();
    assert_eq!(closure_gate["passed"], true);
    assert_eq!(body["unresolved_rule_feedback_count"], 0);
    assert_eq!(body["approved_label_count"], 1);
    assert_eq!(body["needs_review_label_count"], 0);
    assert!(!body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("rule feedback labels need review")));
    assert!(!body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("unresolved rule QA feedback")));
}

#[tokio::test]
async fn rule_promotion_gates_ignore_feedback_for_other_rules() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-OTHER-RULE-GATE-1",
          "claim_id": "CLM-OTHER-RULE-GATE-1",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "provider_pattern",
          "feedback_target": "rules",
          "notes": "QA found a different rule issue.",
          "evidence_refs": ["qa_reviews:QA-OTHER-RULE-GATE-1", "rule_runs:HIGH_AMOUNT_TO_LIMIT"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/ops/rules/rule_early_claim/promotion-gates",
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["open_rule_feedback_count"], 0);
    assert_eq!(body["unresolved_rule_feedback_count"], 0);
    assert_eq!(body["approved_label_count"], 0);
    assert_eq!(body["needs_review_label_count"], 0);
    assert!(!body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("rule feedback labels need review")));
    assert!(!body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("unresolved rule QA feedback")));
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
              "external_claim_id": "CLM-FWA-HIGH-1",
              "claim_amount": "9000",
              "currency": "CNY",
              "service_date": "2026-01-05",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-FWA-HIGH-1",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-FWA-HIGH-2",
              "claim_amount": "8500",
              "currency": "CNY",
              "service_date": "2026-01-06",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-FWA-HIGH-2",
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
              "external_claim_id": "CLM-NORMAL-LATE-MEDIUM",
              "claim_amount": "2000",
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
    assert_eq!(body["sample_count"], 4);
    assert_eq!(body["positive_count"], 2);
    let candidate = body["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| {
            candidate["rule"]["conditions"][0]["field"] == "claim_amount_to_limit_ratio"
        })
        .expect("missing amount ratio candidate");
    assert!(candidate["rule"]["rule_id"]
        .as_str()
        .unwrap()
        .starts_with("candidate_mined_claim_amount_to_limit_ratio_gte_"));
    assert_eq!(
        candidate["rule"]["conditions"][0]["field"],
        "claim_amount_to_limit_ratio"
    );
    assert_eq!(candidate["rule"]["conditions"][0]["operator"], ">=");
    assert_eq!(candidate["support"], 2);
    assert_eq!(candidate["precision"], 1.0);
    assert!(candidate["lift"].as_f64().unwrap() > 1.0);
    assert_eq!(candidate["false_positive_rate"], 0.0);
    assert_eq!(candidate["estimated_saving"], "1750.00");
    assert!(candidate["condition_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .starts_with("rule_conditions:candidate_mined_claim_amount_to_limit_ratio_gte_")));
    assert!(candidate["explanation"]
        .as_str()
        .unwrap()
        .contains("单层决策树阈值规则"));
}

#[tokio::test]
async fn discovers_candidate_rules_from_parquet_dataset() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/rules/discover",
        r#"{
          "min_support": 2,
          "dataset_uri": "data/public-mvp/split=train/part-00000.parquet",
          "label_column": "confirmed_fwa",
          "claim_id_column": "claim_id",
          "candidate_feature_fields": ["claim_amount_to_limit_ratio"],
          "samples": []
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["sample_count"], 18);
    assert_eq!(body["positive_count"], 13);
    let candidate = &body["candidates"][0];
    assert_eq!(
        candidate["rule"]["conditions"][0]["field"],
        "claim_amount_to_limit_ratio"
    );
    assert!(candidate["rule"]["rule_id"]
        .as_str()
        .unwrap()
        .starts_with("candidate_mined_claim_amount_to_limit_ratio_"));
    assert!(candidate["explanation"]
        .as_str()
        .unwrap()
        .contains("负样本标准差"));
    assert!(candidate["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "dataset:data/public-mvp/split=train/part-00000.parquet"
        )));
}

#[tokio::test]
async fn discovers_shallow_tree_rule_candidates_from_labeled_samples() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/rules/discover",
        r#"{
          "min_support": 2,
          "max_tree_depth": 2,
          "candidate_feature_fields": [
            "days_since_policy_start",
            "claim_amount_to_limit_ratio"
          ],
          "samples": [
            {
              "external_claim_id": "CLM-TREE-TP-1",
              "claim_amount": "9000",
              "currency": "CNY",
              "service_date": "2026-01-03",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-TREE-TP-1",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-TREE-TP-2",
              "claim_amount": "8500",
              "currency": "CNY",
              "service_date": "2026-01-04",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-TREE-TP-2",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-TREE-FP-EARLY-LOW",
              "claim_amount": "500",
              "currency": "CNY",
              "service_date": "2026-01-04",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-TREE-FP-EARLY-LOW",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-TREE-FP-LATE-HIGH",
              "claim_amount": "9000",
              "currency": "CNY",
              "service_date": "2026-02-15",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-TREE-FP-LATE-HIGH",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-TREE-TN",
              "claim_amount": "500",
              "currency": "CNY",
              "service_date": "2026-02-15",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-TREE-TN",
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
    let candidate = body["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| {
            candidate["rule"]["rule_id"]
                .as_str()
                .unwrap()
                .starts_with("candidate_tree_")
        })
        .expect("missing shallow tree candidate");
    let conditions = candidate["rule"]["conditions"].as_array().unwrap();
    assert_eq!(conditions.len(), 2);
    assert!(conditions.iter().any(|condition| {
        condition["field"] == "claim_amount_to_limit_ratio" && condition["operator"] == ">="
    }));
    assert!(conditions.iter().any(|condition| {
        condition["field"] == "days_since_policy_start" && condition["operator"] == "<="
    }));
    assert_eq!(candidate["support"], 2);
    assert_eq!(candidate["precision"], 1.0);
    assert_eq!(candidate["false_positive_rate"], 0.0);
    assert!(candidate["explanation"]
        .as_str()
        .unwrap()
        .contains("浅层决策树"));
}

#[tokio::test]
async fn discovers_rule_candidates_from_model_explanations() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/rules/discover",
        r#"{
          "min_support": 1,
          "source_model_key": "baseline_fwa",
          "source_model_version": "0.3.0-candidate",
          "feature_importance_uri": "data/eval/baseline_fwa/v3/feature_importance.parquet",
          "model_explanations": [
            {
              "feature": "claim_amount_to_limit_ratio",
              "direction": "increases_risk",
              "contribution": 1.4,
              "reason": "large positive logistic contribution"
            }
          ],
          "samples": [
            {
              "external_claim_id": "CLM-ML-TP",
              "claim_amount": "9000",
              "currency": "CNY",
              "service_date": "2026-01-05",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-ML-TP",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-ML-TN",
              "claim_amount": "500",
              "currency": "CNY",
              "service_date": "2026-03-01",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-ML-TN",
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
    let candidates = body["candidates"].as_array().unwrap();
    let candidate = candidates
        .iter()
        .find(|candidate| {
            candidate["rule"]["rule_id"]
                .as_str()
                .unwrap()
                .starts_with("candidate_mined_claim_amount_to_limit_ratio_gte_")
        })
        .expect("missing mined model-supported candidate rule");
    assert_eq!(
        candidate["rule"]["conditions"][0]["field"],
        "claim_amount_to_limit_ratio"
    );
    assert_eq!(candidate["rule"]["conditions"][0]["operator"], ">=");
    assert!(candidate["condition_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .starts_with("rule_conditions:candidate_mined_claim_amount_to_limit_ratio_gte_")));
    assert_eq!(candidate["precision"], 1.0);
    assert!(candidate["explanation"]
        .as_str()
        .unwrap()
        .contains("模型解释备注"));
    assert!(candidate["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "model_versions:baseline_fwa:0.3.0-candidate"
        )));
}

#[tokio::test]
async fn saves_discovered_candidate_rule_for_lifecycle() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidates",
        r#"{
          "owner": "rule-discovery",
          "rule": {
            "rule_id": "candidate_early_high_amount",
            "version": 1,
            "name": "Early high amount candidate",
            "scheme_family": "early_high_value_claim",
            "conditions": [
              {
                "field": "days_since_policy_start",
                "operator": "<=",
                "value": 10
              },
              {
                "field": "claim_amount_to_limit_ratio",
                "operator": ">=",
                "value": 0.7
              }
            ],
            "action": {
              "score": 30,
              "alert_code": "EARLY_HIGH_AMOUNT_CANDIDATE",
              "recommended_action": "ManualReview",
              "reason": "保单生效早期发生高额理赔"
            }
          }
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["summary"]["rule_id"], "candidate_early_high_amount");
    assert_eq!(body["summary"]["status"], "draft");
    assert_eq!(body["summary"]["owner"], "rule-discovery");
    assert_eq!(
        body["versions"][0]["alert_code"],
        "EARLY_HIGH_AMOUNT_CANDIDATE"
    );

    let (status, body) =
        json_request(app.clone(), "GET", "/api/v1/ops/rules/conditions", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let candidate_conditions = body["conditions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|condition| condition["source_rule_key"] == "candidate_early_high_amount")
        .collect::<Vec<_>>();
    assert_eq!(candidate_conditions.len(), 2);
    assert!(candidate_conditions.iter().any(|condition| {
        condition["condition_key"] == "candidate_early_high_amount_v1_c1"
            && condition["field"] == "days_since_policy_start"
            && condition["operator"] == "<="
            && condition["value"] == 10
            && condition["status"] == "candidate"
            && condition["owner"] == "rule-discovery"
    }));
    assert!(candidate_conditions
        .iter()
        .all(
            |condition| condition["evidence_refs"].as_array().unwrap().contains(
                &serde_json::json!(format!(
                    "rule_conditions:{}",
                    condition["condition_key"].as_str().unwrap()
                ))
            )
        ));

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidate_early_high_amount/submit",
        &rule_lifecycle_payload("candidate_early_high_amount", 1),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["status"], "submitted");

    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/ops/rules/candidate_early_high_amount",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["summary"]["status"], "submitted");
}

#[tokio::test]
async fn records_rejected_discovered_candidate_review_without_saving_rule() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidate-reviews",
        r#"{
          "decision": "rejected",
          "reviewer": "rule-review",
          "notes": "Rejected because this candidate is not explainable enough for the governed rule library.",
          "evidence_refs": ["dataset:inline", "backtest:manual-review"],
          "rule": {
            "rule_id": "candidate_tree_rejected_review",
            "version": 1,
            "name": "Rejected tree candidate",
            "scheme_family": "high_risk_claim",
            "conditions": [
              {
                "field": "claim_amount_to_limit_ratio",
                "operator": ">=",
                "value": 0.8
              },
              {
                "field": "provider_peer_payment_zscore",
                "operator": ">=",
                "value": 1.5
              }
            ],
            "action": {
              "score": 30,
              "alert_code": "TREE_REJECTED_REVIEW",
              "recommended_action": "ManualReview",
              "reason": "测试拒绝候选规则"
            }
          }
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["rule_id"], "candidate_tree_rejected_review");
    assert_eq!(body["decision"], "rejected");
    assert_eq!(body["entered_rule_library"], false);

    let (status, body) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/rules/candidate_tree_rejected_review",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "RULE_NOT_FOUND");

    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/ops/audit-events?event_type=rule.candidate.reviewed&rule_id=candidate_tree_rejected_review",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let event = &body["events"].as_array().unwrap()[0];
    assert_eq!(event["event_type"], "rule.candidate.reviewed");
    assert_eq!(event["payload"]["decision"], "rejected");
    assert_eq!(event["payload"]["entered_rule_library"], false);
}

#[tokio::test]
async fn deterministic_adjudication_rule_requires_customer_policy_gates() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidates",
        r#"{
          "owner": "customer-policy",
          "rule": {
            "rule_id": "candidate_customer_hard_deny",
            "version": 1,
            "name": "Customer approved hard deny candidate",
            "review_mode": "pre_payment",
            "scheme_family": "diagnosis_procedure_mismatch",
            "conditions": [
              {
                "field": "diagnosis_procedure_match_score",
                "operator": "<=",
                "value": 0.1
              }
            ],
            "action": {
              "score": 0,
              "alert_code": "CUSTOMER_HARD_DENY",
              "recommended_action": "ManualReview",
              "action_class": "hard_deny",
              "required_evidence": [
                {
                  "evidence_type": "policy_eligibility",
                  "blocking": true,
                  "policy_authority_ref": "policy:eligibility:v1",
                  "exception_check": "no_approved_exception"
                }
              ],
              "adjudication_policy": {
                "customer_approval_ref": "customer-rule-list:demo:v1",
                "appeal_or_override_route": "appeals:manual-review:v1",
                "effective_date": "2026-01-01",
                "rollback_plan_ref": "rollback:rules:v1",
                "production_threshold_ref": "thresholds:prepay:v1",
                "routing_impact_ref": "routing-impact:shadow:v1"
              },
              "reason": "Customer-approved deterministic ineligibility rule"
            }
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/ops/rules/candidate_customer_hard_deny/promotion-gates",
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["decision"], "routing_blocked");
    assert_eq!(body["total_count"], 15);
    let gates = body["gates"].as_array().unwrap();
    let customer_gate = gates
        .iter()
        .find(|gate| gate["label"] == "Customer-approved adjudication rule list")
        .expect("adjudication rule list gate");
    assert_eq!(customer_gate["passed"], true);
    let authority_gate = gates
        .iter()
        .find(|gate| gate["label"] == "Policy authority and exception check")
        .expect("policy authority gate");
    assert_eq!(authority_gate["passed"], true);
    let routing_impact_gate = gates
        .iter()
        .find(|gate| gate["label"] == "Routing impact promotion")
        .expect("routing impact gate");
    assert_eq!(routing_impact_gate["passed"], false);
    assert_eq!(
        routing_impact_gate["blocker"],
        "routing impact evidence missing"
    );
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("routing impact evidence missing")));
}

#[tokio::test]
async fn saves_candidate_rule_with_explicit_scheme_family() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/rules/candidates",
        r#"{
          "owner": "rule-discovery",
          "rule": {
            "rule_id": "candidate_explicit_scheme",
            "version": 1,
            "name": "Explicit scheme candidate",
            "review_mode": "pre_payment",
            "scheme_family": "diagnosis_procedure_mismatch",
            "conditions": [
              {
                "field": "diagnosis_procedure_match_score",
                "operator": "<=",
                "value": 0.35
              }
            ],
            "action": {
              "score": 25,
              "alert_code": "BESPOKE_PATTERN",
              "recommended_action": "ManualReview",
              "reason": "候选规则必须显式映射 FWA scheme"
            }
          }
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        body["summary"]["scheme_family"],
        "diagnosis_procedure_mismatch"
    );
    assert_eq!(
        body["summary"]["applicability_scope"]["scheme_family"],
        "diagnosis_procedure_mismatch"
    );
    assert_eq!(
        body["versions"][0]["scheme_family"],
        "diagnosis_procedure_mismatch"
    );
    assert_eq!(
        body["versions"][0]["dsl"]["scheme_family"],
        "diagnosis_procedure_mismatch"
    );
}

#[tokio::test]
async fn rejects_candidate_rule_without_valid_scheme_family() {
    let app = build_app(test_config());

    let missing_scheme = r#"{
      "owner": "rule-discovery",
      "rule": {
        "rule_id": "candidate_missing_scheme",
        "version": 1,
        "name": "Missing scheme candidate",
        "review_mode": "pre_payment",
        "conditions": [
          {
            "field": "diagnosis_procedure_match_score",
            "operator": "<=",
            "value": 0.35
          }
        ],
        "action": {
          "score": 25,
          "alert_code": "BESPOKE_PATTERN",
          "recommended_action": "ManualReview",
          "reason": "候选规则缺少显式 FWA scheme"
        }
      }
    }"#;
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidates",
        missing_scheme,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_RULE_CANDIDATE");

    let invalid_scheme = r#"{
      "owner": "rule-discovery",
      "rule": {
        "rule_id": "candidate_invalid_scheme",
        "version": 1,
        "name": "Invalid scheme candidate",
        "review_mode": "pre_payment",
        "scheme_family": "not_a_scheme",
        "conditions": [
          {
            "field": "diagnosis_procedure_match_score",
            "operator": "<=",
            "value": 0.35
          }
        ],
        "action": {
          "score": 25,
          "alert_code": "BESPOKE_PATTERN",
          "recommended_action": "ManualReview",
          "reason": "候选规则 scheme 必须属于治理 taxonomy"
        }
      }
    }"#;
    let (status, body) =
        json_request(app, "POST", "/api/v1/ops/rules/candidates", invalid_scheme).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_RULE_CANDIDATE");
}

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
