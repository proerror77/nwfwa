use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{
    json_request, public_mvp_parquet_fixture_uri, seed_rule_promotion_evidence, test_config,
};

#[tokio::test]
async fn records_rule_promotion_review_and_uses_it_for_approval_gate() {
    let app = build_app(test_config()).unwrap();

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
          "notes": "Rejected until backtest evidence is attached.",
          "evidence_refs": [
            "rules:rule_early_claim:v1",
            "rule_promotion_reviews:local://template/rule-review.json"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_PROMOTION_REVIEW_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/promotion-reviews",
        r#"{
          "decision": "rejected",
          "reviewer": "rule-governance",
          "notes": "Rejected until production evidence is attached.",
          "evidence_refs": [
            "rules:rule_early_claim:v1",
            "rule_promotion_reviews:file://tmp/rule-review.json"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_PROMOTION_REVIEW_EVIDENCE");

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
    let app = build_app(test_config()).unwrap();

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
    let app = build_app(test_config()).unwrap();
    let dataset_uri = public_mvp_parquet_fixture_uri("rule-backtest");
    let payload = r#"{
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
          "dataset_uri": "__DATASET_URI__",
          "label_column": "confirmed_fwa",
          "claim_id_column": "claim_id",
          "samples": [],
          "expected_review_capacity": 20
        }"#
    .replace("__DATASET_URI__", &dataset_uri);

    let (status, body) = json_request(app, "POST", "/api/v1/ops/rules/backtest", &payload).await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["sample_count"], 18);
    assert!(body["matched_count"].as_u64().unwrap() > 0);
    assert!(body["precision"].as_f64().unwrap() > 0.0);
    assert!(!body["matched_claim_ids"].as_array().unwrap().is_empty());
    assert!(body["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(format!("dataset:{dataset_uri}"))));
}

#[tokio::test]
async fn backtest_recommends_review_when_labeled_metrics_pass() {
    let app = build_app(test_config()).unwrap();

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
    let app = build_app(test_config()).unwrap();

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
async fn rule_shadow_run_evidence_feeds_rule_promotion_gates() {
    let app = build_app(test_config()).unwrap();

    seed_rule_promotion_evidence(app.clone()).await;

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/rule_early_claim/shadow-runs",
        r#"{
          "rule_version": 1,
          "reviewed_count": 3,
          "matched_count": 2,
          "false_positive_count": 0,
          "false_positive_rate": 0.0,
          "report_uri": "artifacts/rules/rule_early_claim/shadow_report.json",
          "decision": "shadow_passed",
          "reviewer": "rule-shadow-review",
          "notes": "Shadow run reviewed against labeled runtime evidence.",
          "evidence_refs": [
            "rules:rule_early_claim:v1",
            "rule_shadow_runs:artifacts/rules/rule_early_claim/shadow_report.json"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["rule_id"], "rule_early_claim");
    assert_eq!(body["decision"], "shadow_passed");
    assert_eq!(
        body["report_uri"],
        "artifacts/rules/rule_early_claim/shadow_report.json"
    );

    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/ops/rules/rule_early_claim/promotion-gates",
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(!body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("shadow rollout missing")));
    let shadow_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Shadow or limited rollout")
        .unwrap();
    assert_eq!(shadow_gate["passed"], true);
    assert_eq!(shadow_gate["evidence_source"], "shadow");
}

#[tokio::test]
async fn rule_promotion_gates_block_unresolved_backtest_blockers() {
    let app = build_app(test_config()).unwrap();

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
    let app = build_app(test_config()).unwrap();

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
    let app = build_app(test_config()).unwrap();

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
