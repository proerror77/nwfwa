use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{json_request, rule_lifecycle_payload, test_config};

#[tokio::test]
async fn saves_discovered_candidate_rule_for_lifecycle() {
    let app = build_app(test_config()).unwrap();

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
        r#"{"evidence_refs":["rules:candidate_early_high_amount:v1","rule_reviews:local://template/candidate-review.json"]}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_RULE_LIFECYCLE_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidate_early_high_amount/submit",
        r#"{"evidence_refs":["rules:candidate_early_high_amount:v1","rule_reviews:http://localhost:8080/candidate-review.json"]}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_RULE_LIFECYCLE_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidate_early_high_amount/submit",
        r#"{"evidence_refs":["rules:candidate_early_high_amount:v1","rule_reviews:file://tmp/candidate-review.json"]}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_RULE_LIFECYCLE_EVIDENCE");

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
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidate-reviews",
        r#"{
          "decision": "rejected",
          "reviewer": "rule-review",
          "notes": "Rejected because this candidate is not explainable enough for the governed rule library.",
          "evidence_refs": ["dataset:inline", "rule_candidate_reviews:local://template/review.json"],
          "rule": {
            "rule_id": "candidate_tree_local_review",
            "version": 1,
            "name": "Local evidence tree candidate",
            "scheme_family": "high_risk_claim",
            "conditions": [
              {
                "field": "claim_amount_to_limit_ratio",
                "operator": ">=",
                "value": 0.8
              }
            ],
            "action": {
              "score": 30,
              "alert_code": "TREE_LOCAL_REVIEW",
              "recommended_action": "ManualReview",
              "reason": "测试本地证据候选规则"
            }
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_CANDIDATE_REVIEW_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidate-reviews",
        r#"{
          "decision": "rejected",
          "reviewer": "rule-review",
          "notes": "Rejected because this candidate is not explainable enough for the governed rule library.",
          "evidence_refs": ["dataset:inline", "rule_candidate_reviews:http://127.0.0.1:8080/review.json"],
          "rule": {
            "rule_id": "candidate_tree_loopback_review",
            "version": 1,
            "name": "Loopback evidence tree candidate",
            "scheme_family": "high_risk_claim",
            "conditions": [
              {
                "field": "claim_amount_to_limit_ratio",
                "operator": ">=",
                "value": 0.8
              }
            ],
            "action": {
              "score": 30,
              "alert_code": "TREE_LOOPBACK_REVIEW",
              "recommended_action": "ManualReview",
              "reason": "Loopback evidence candidate rule"
            }
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_CANDIDATE_REVIEW_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidate-reviews",
        r#"{
          "decision": "rejected",
          "reviewer": "rule-review",
          "notes": "Rejected because this candidate is not explainable enough for the governed rule library.",
          "evidence_refs": ["dataset:inline", "rule_candidate_reviews:file://tmp/review.json"],
          "rule": {
            "rule_id": "candidate_tree_file_review",
            "version": 1,
            "name": "File evidence tree candidate",
            "scheme_family": "high_risk_claim",
            "conditions": [
              {
                "field": "claim_amount_to_limit_ratio",
                "operator": ">=",
                "value": 0.8
              }
            ],
            "action": {
              "score": 30,
              "alert_code": "TREE_FILE_REVIEW",
              "recommended_action": "ManualReview",
              "reason": "File evidence candidate rule"
            }
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_CANDIDATE_REVIEW_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidate-reviews",
        r#"{
          "decision": "rejected",
          "reviewer": "rule-review",
          "notes": "Rejected because this candidate is not explainable enough for the governed rule library.",
          "evidence_refs": ["dataset:inline", "rule_candidate_reviews:{review_id}"],
          "rule": {
            "rule_id": "candidate_tree_template_review",
            "version": 1,
            "name": "Template evidence tree candidate",
            "scheme_family": "high_risk_claim",
            "conditions": [
              {
                "field": "claim_amount_to_limit_ratio",
                "operator": ">=",
                "value": 0.8
              }
            ],
            "action": {
              "score": 30,
              "alert_code": "TREE_TEMPLATE_REVIEW",
              "recommended_action": "ManualReview",
              "reason": "测试模板证据候选规则"
            }
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_CANDIDATE_REVIEW_EVIDENCE");

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
async fn accepted_discovered_candidate_review_requires_backtest_and_shadow() {
    let app = build_app(test_config()).unwrap();

    let candidate_rule = r#"{
      "rule_id": "candidate_tree_accepted_review",
      "version": 1,
      "name": "Accepted tree candidate",
      "review_mode": "both",
      "scheme_family": "high_risk_claim",
      "conditions": [
        {
          "field": "claim_amount_to_limit_ratio",
          "operator": ">=",
          "value": 0.8
        }
      ],
      "action": {
        "score": 30,
        "alert_code": "TREE_ACCEPTED_REVIEW",
        "recommended_action": "ManualReview",
        "reason": "测试接受候选规则"
      }
    }"#;

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidate-reviews",
        &format!(
            r#"{{
              "decision": "accepted",
              "reviewer": "rule-review",
              "notes": "Accepted candidate after backtest evidence.",
              "evidence_refs": ["dataset:inline", "backtest:precheck"],
              "rule": {candidate_rule}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "RULE_CANDIDATE_BACKTEST_REQUIRED");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/backtest",
        &format!(
            r#"{{
              "rule": {candidate_rule},
              "samples": [
                {{
                  "external_claim_id": "CLM-CAND-TP-1",
                  "claim_amount": "9000",
                  "currency": "CNY",
                  "service_date": "2026-01-05",
                  "confirmed_fwa": true,
                  "policy": {{
                    "external_policy_id": "POL-CAND-TP-1",
                    "coverage_start_date": "2026-01-01",
                    "coverage_end_date": "2026-12-31",
                    "coverage_limit": "10000"
                  }}
                }},
                {{
                  "external_claim_id": "CLM-CAND-TP-2",
                  "claim_amount": "8500",
                  "currency": "CNY",
                  "service_date": "2026-01-07",
                  "confirmed_fwa": true,
                  "policy": {{
                    "external_policy_id": "POL-CAND-TP-2",
                    "coverage_start_date": "2026-01-01",
                    "coverage_end_date": "2026-12-31",
                    "coverage_limit": "10000"
                  }}
                }},
                {{
                  "external_claim_id": "CLM-CAND-TN",
                  "claim_amount": "500",
                  "currency": "CNY",
                  "service_date": "2026-03-01",
                  "confirmed_fwa": false,
                  "policy": {{
                    "external_policy_id": "POL-CAND-TN",
                    "coverage_start_date": "2026-01-01",
                    "coverage_end_date": "2026-12-31",
                    "coverage_limit": "10000"
                  }}
                }}
              ],
              "expected_review_capacity": 5
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["promotion_recommendation"], "eligible_for_review");
    assert!(body["blockers"].as_array().unwrap().is_empty());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidate-reviews",
        &format!(
            r#"{{
              "decision": "accepted",
              "reviewer": "rule-review",
              "notes": "Accepted candidate after backtest evidence.",
              "evidence_refs": ["dataset:inline", "backtest:api"],
              "rule": {candidate_rule}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "RULE_CANDIDATE_SHADOW_REQUIRED");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidates",
        &format!(
            r#"{{
              "owner": "rule-discovery-shadow",
              "rule": {candidate_rule}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidate_tree_accepted_review/shadow-runs",
        r#"{
          "rule_version": 1,
          "reviewed_count": 3,
          "matched_count": 2,
          "false_positive_count": 0,
          "false_positive_rate": 0.0,
          "report_uri": "artifacts/rules/candidate_tree_accepted_review/shadow_report.json",
          "decision": "shadow_passed",
          "reviewer": "rule-shadow-review",
          "notes": "Shadow run passed before final candidate acceptance.",
          "evidence_refs": [
            "rules:candidate_tree_accepted_review:v1",
            "rule_shadow_runs:artifacts/rules/candidate_tree_accepted_review/shadow_report.json"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidate-reviews",
        &format!(
            r#"{{
              "decision": "accepted",
              "reviewer": "rule-review",
              "notes": "Accepted candidate after backtest and shadow evidence.",
              "evidence_refs": [
                "dataset:inline",
                "backtest:api",
                "rule_shadow_runs:artifacts/rules/candidate_tree_accepted_review/shadow_report.json"
              ],
              "rule": {candidate_rule}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["rule_id"], "candidate_tree_accepted_review");
    assert_eq!(body["decision"], "accepted");
    assert_eq!(body["entered_rule_library"], false);
    assert_eq!(body["accepted_for_governance_review"], true);
    assert_eq!(
        body["saved_draft_rule_id"],
        "candidate_tree_accepted_review"
    );
    assert_eq!(body["active_rule_writeback"], false);

    let (status, body) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/rules/candidate_tree_accepted_review",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["summary"]["status"], "draft");
    assert_eq!(body["summary"]["active_version"], serde_json::Value::Null);

    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/ops/audit-events?event_type=rule.candidate.reviewed&rule_id=candidate_tree_accepted_review",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let event = &body["events"].as_array().unwrap()[0];
    assert_eq!(event["payload"]["decision"], "accepted");
    assert_eq!(event["payload"]["entered_rule_library"], false);
    assert_eq!(event["payload"]["accepted_for_governance_review"], true);
    assert_eq!(event["payload"]["active_rule_writeback"], false);
}

#[tokio::test]
async fn deterministic_adjudication_rule_requires_customer_policy_gates() {
    let app = build_app(test_config()).unwrap();

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
    let app = build_app(test_config()).unwrap();

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
    let app = build_app(test_config()).unwrap();

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
