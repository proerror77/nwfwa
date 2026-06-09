use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{json_request, test_config};

#[tokio::test]
async fn lists_qa_feedback_items_for_rule_and_model_operators() {
    let app = build_app(test_config());

    for (qa_case_id, feedback_target, issue_type) in [
        ("QA-RULE-1001", "rules", "alert_handling_incomplete"),
        (
            "QA-MODEL-1001",
            "model",
            "model_under_scored_confirmed_issue",
        ),
    ] {
        let (status, _) = json_request(
            app.clone(),
            "POST",
            "/api/v1/qa/results",
            &format!(
                r#"{{
                  "qa_case_id": "{qa_case_id}",
                  "claim_id": "CLM-0287",
                  "qa_conclusion": "issue_found_escalate",
                  "issue_type": "{issue_type}",
                  "feedback_target": "{feedback_target}",
                  "notes": "Reviewer notes stay in the source QA review, not the feedback queue summary.",
                  "evidence_refs": ["audit:scoring.completed", "rule_runs:EARLY_CLAIM"]
                }}"#,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    let (status, feedback) =
        json_request(app.clone(), "GET", "/api/v1/ops/qa/feedback-items", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let items = feedback["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["feedback_id"], "qa_feedback_QA-RULE-1001");
    assert_eq!(items[0]["feedback_target"], "rules");
    assert_eq!(items[0]["status"], "open");
    assert_eq!(items[0]["source"], "qa_review");
    assert_eq!(items[0]["note_present"], true);
    assert_eq!(items[0]["status_updated_by"], serde_json::Value::Null);
    assert_eq!(items[0]["status_audit_id"], serde_json::Value::Null);
    assert!(items[0]["status_evidence_refs"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(items[0]["summary"]
        .as_str()
        .unwrap()
        .contains("QA-RULE-1001"));
    assert!(items[0].get("notes").is_none());
    assert_eq!(items[1]["feedback_target"], "model");
    assert!(items
        .iter()
        .all(|item| !item["evidence_refs"].as_array().unwrap().is_empty()));

    let (status, feedback) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/qa/feedback-items?feedback_target=rules&status=open",
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let items = feedback["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["feedback_id"], "qa_feedback_QA-RULE-1001");

    let (status, body) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/qa/feedback-items?status=unknown",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "UNSUPPORTED_QA_FEEDBACK_STATUS");

    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/ops/qa/feedback-items?feedback_target=unknown",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "UNSUPPORTED_FEEDBACK_TARGET");
}

#[tokio::test]
async fn accepts_prd_model_feedback_target_and_canonicalizes_legacy_alias() {
    let app = build_app(test_config());

    for (qa_case_id, feedback_target) in [
        ("QA-MODEL-PRD-1001", "model"),
        ("QA-MODEL-LEGACY-1001", "models"),
    ] {
        let (status, body) = json_request(
            app.clone(),
            "POST",
            "/api/v1/qa/results",
            &format!(
                r#"{{
                  "qa_case_id": "{qa_case_id}",
                  "claim_id": "CLM-MODEL-FEEDBACK",
                  "qa_conclusion": "issue_found_escalate",
                  "issue_type": "model_under_scored_confirmed_issue",
                  "feedback_target": "{feedback_target}",
                  "notes": "QA feedback is directed to model operations.",
                  "evidence_refs": ["qa_reviews:{qa_case_id}", "model_versions:baseline_fwa:0.1.0"]
                }}"#
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    let (status, feedback) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/qa/feedback-items?feedback_target=model",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = feedback["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|item| item["feedback_target"] == "model"));

    let (status, feedback) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/qa/feedback-items?feedback_target=models",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(feedback["items"].as_array().unwrap().len(), 2);

    let (status, labels) = json_request(app, "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let labels = labels["labels"].as_array().unwrap();
    assert!(labels.iter().any(|label| {
        label["source_id"] == "QA-MODEL-PRD-1001"
            && label["label_name"] == "model_under_scored_confirmed_issue"
            && label["feedback_target"] == "model"
    }));
    assert!(labels.iter().any(|label| {
        label["source_id"] == "QA-MODEL-LEGACY-1001"
            && label["label_name"] == "model_under_scored_confirmed_issue"
            && label["feedback_target"] == "model"
    }));
}

#[tokio::test]
async fn updates_qa_feedback_item_status_with_audit_trail() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-FEEDBACK-STATUS-1",
          "claim_id": "CLM-FEEDBACK-STATUS-1",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Rule feedback should be worked by rule ops.",
          "evidence_refs": ["audit:scoring.completed", "rule_runs:EARLY_CLAIM"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-FEEDBACK-STATUS-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "rule-ops",
          "notes": "Missing evidence should be rejected.",
          "evidence_refs": []
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_QA_FEEDBACK_STATUS_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-FEEDBACK-STATUS-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "rule-ops",
          "notes": "Blank evidence reference should be rejected.",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-FEEDBACK-STATUS-1", " "]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_QA_FEEDBACK_STATUS_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-FEEDBACK-STATUS-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "rule-ops",
          "notes": " ",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-FEEDBACK-STATUS-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_QA_FEEDBACK_STATUS_NOTES");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-FEEDBACK-STATUS-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "rule-ops",
          "notes": "Reviewer contacted alice@example.com about the feedback.",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-FEEDBACK-STATUS-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_QA_FEEDBACK_STATUS");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-FEEDBACK-STATUS-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "rule-ops",
          "notes": "Rule threshold reviewed and accepted.",
          "evidence_refs": ["phone:13800138000"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_QA_FEEDBACK_STATUS");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-FEEDBACK-STATUS-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "rule-ops",
          "notes": "Rule threshold reviewed and accepted.",
          "evidence_refs": ["rule_runs:EARLY_CLAIM"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_QA_FEEDBACK_TARGET_EVIDENCE");

    let (status, update) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-FEEDBACK-STATUS-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "rule-ops",
          "notes": "Rule threshold reviewed and accepted.",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-FEEDBACK-STATUS-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(update["item"]["status"], "resolved");
    assert_eq!(
        update["item"]["feedback_id"],
        "qa_feedback_QA-FEEDBACK-STATUS-1"
    );
    assert!(!update["audit_id"].as_str().unwrap().is_empty());
    assert_eq!(update["item"]["status_updated_by"], "rule-ops");
    assert_eq!(update["item"]["status_audit_id"], update["audit_id"]);
    assert_eq!(
        update["item"]["status_evidence_refs"],
        serde_json::json!(["qa_feedback:qa_feedback_QA-FEEDBACK-STATUS-1"])
    );

    let (status, feedback) =
        json_request(app.clone(), "GET", "/api/v1/ops/qa/feedback-items", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(feedback["items"][0]["status"], "resolved");
    assert_eq!(feedback["items"][0]["status_updated_by"], "rule-ops");
    assert_eq!(feedback["items"][0]["status_audit_id"], update["audit_id"]);
    assert_eq!(
        feedback["items"][0]["status_evidence_refs"],
        serde_json::json!(["qa_feedback:qa_feedback_QA-FEEDBACK-STATUS-1"])
    );

    let (status, audit) = json_request(
        app,
        "GET",
        "/api/v1/audit/claims/CLM-FEEDBACK-STATUS-1",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let status_event = audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| {
            event["event_type"] == "qa.feedback.status.updated"
                && event["payload"]["to_status"] == "resolved"
        })
        .expect("feedback status update should be audited");
    assert_eq!(
        status_event["payload"]["customer_scope_id"],
        "demo-customer"
    );
}

#[tokio::test]
async fn summarizes_qa_feedback_queue_for_review_operations() {
    let app = build_app(test_config());

    for (qa_case_id, feedback_target, issue_type, qa_conclusion) in [
        (
            "QA-QUEUE-RULE-1001",
            "rules",
            "alert_handling_incomplete",
            "issue_found_escalate",
        ),
        (
            "QA-QUEUE-MODEL-1001",
            "model",
            "model_under_scored_confirmed_issue",
            "issue_found_return",
        ),
        (
            "QA-QUEUE-TPA-1001",
            "tpa",
            "workflow_missing_evidence",
            "issue_found_escalate",
        ),
        (
            "QA-QUEUE-FEATURES-1001",
            "features",
            "medical_reasonableness",
            "issue_found_return",
        ),
        (
            "QA-QUEUE-PROVIDER-1001",
            "provider_profile",
            "provider_pattern",
            "issue_found_escalate",
        ),
        (
            "QA-QUEUE-WORKFLOW-1001",
            "workflow",
            "qa_review_completed",
            "issue_found_return",
        ),
    ] {
        let (status, _) = json_request(
            app.clone(),
            "POST",
            "/api/v1/qa/results",
            &format!(
                r#"{{
                  "qa_case_id": "{qa_case_id}",
                  "claim_id": "CLM-QA-QUEUE",
                  "qa_conclusion": "{qa_conclusion}",
                  "issue_type": "{issue_type}",
                  "feedback_target": "{feedback_target}",
                  "notes": "QA feedback needs operational follow-up.",
                  "evidence_refs": ["audit:scoring.completed", "qa_reviews:{qa_case_id}"]
                }}"#,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    for (feedback_id, status) in [
        ("qa_feedback_QA-QUEUE-MODEL-1001", "in_progress"),
        ("qa_feedback_QA-QUEUE-TPA-1001", "resolved"),
        ("qa_feedback_QA-QUEUE-WORKFLOW-1001", "dismissed"),
    ] {
        let (status_code, _) = json_request(
            app.clone(),
            "POST",
            &format!("/api/v1/ops/qa/feedback-items/{feedback_id}/status"),
            &format!(
                r#"{{
                  "status": "{status}",
                  "actor_id": "qa-lead",
                  "notes": "Update QA feedback status for queue distribution.",
                  "evidence_refs": ["qa_feedback:{feedback_id}"]
                }}"#,
            ),
        )
        .await;
        assert_eq!(status_code, StatusCode::OK);
    }

    let (status, summary) = json_request(app, "GET", "/api/v1/ops/qa/queue-summary", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary["open_count"], 3);
    assert_eq!(summary["in_progress_count"], 1);
    assert_eq!(summary["resolved_count"], 1);
    assert_eq!(summary["dismissed_count"], 1);
    assert_eq!(summary["unresolved_count"], 4);
    assert_eq!(summary["rules_feedback_count"], 1);
    assert_eq!(summary["models_feedback_count"], 0);
    assert_eq!(summary["features_feedback_count"], 1);
    assert_eq!(summary["provider_profile_feedback_count"], 1);
    assert_eq!(summary["workflow_feedback_count"], 0);
    assert_eq!(summary["tpa_feedback_count"], 0);
    assert_eq!(summary["high_priority_count"], 2);
    assert_eq!(summary["evidence_backed_count"], 3);
    assert_eq!(summary["highest_priority"], "high");
}

#[tokio::test]
async fn lists_qa_queue_items_from_audit_samples() {
    let app = build_app(test_config());

    let (status, score) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-QA-QUEUE-ITEM",
            "claim_amount": "9300.00",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "items": [
            {
              "item_code": "IMG-QA-1",
              "item_type": "procedure",
              "description": "Imaging",
              "quantity": 1,
              "unit_amount": "9300.00",
              "total_amount": "9300.00"
            }
          ],
          "member": { "external_member_id": "MBR-QA-QUEUE-ITEM" },
          "policy": {
            "external_policy_id": "POL-QA-QUEUE-ITEM",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000.00",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-QA-QUEUE-ITEM",
            "name": "QA Queue Hospital",
            "provider_type": "hospital",
            "region": "Shanghai",
            "risk_tier": "High"
          }
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
          "population_definition": "High risk claims for QA queue",
          "inclusion_criteria": { "min_risk_score": 70 },
          "deterministic_seed": "qa-week-1",
          "sample_size": 1,
          "reviewer": "qa-reviewer-1",
          "assignment_queue": "QA Review"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, queue) = json_request(app.clone(), "GET", "/api/v1/ops/qa/queue", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let items = queue["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["claim_id"], "CLM-QA-QUEUE-ITEM");
    assert_eq!(items[0]["sample_id"], sample["sample_id"]);
    assert_eq!(items[0]["risk_score"], score["risk_score"]);
    assert_eq!(items[0]["assignment_queue"], "QA Review");
    assert_eq!(items[0]["reviewer"], "qa-reviewer-1");
    assert_eq!(items[0]["status"], "open");
    assert!(items[0]["qa_case_id"]
        .as_str()
        .unwrap()
        .starts_with("qa_sample_"));
    assert!(!items[0]["evidence_refs"].as_array().unwrap().is_empty());

    let qa_case_id = items[0]["qa_case_id"].as_str().unwrap();
    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        &format!(
            r#"{{
              "qa_case_id": "{qa_case_id}",
              "claim_id": "CLM-QA-QUEUE-ITEM",
              "qa_conclusion": "pass",
              "issue_type": "qa_review_completed",
              "feedback_target": "workflow",
              "notes": "Reviewer completed sampled QA case.",
              "evidence_refs": ["qa_queue:{qa_case_id}", "audit:scoring.completed"]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, queue) = json_request(app, "GET", "/api/v1/ops/qa/queue", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let items = queue["items"].as_array().unwrap();
    assert_eq!(items[0]["qa_case_id"], qa_case_id);
    assert_eq!(items[0]["status"], "reviewed");
    assert_eq!(items[0]["qa_conclusion"], "pass");
    assert_eq!(items[0]["issue_type"], "qa_review_completed");
}

#[tokio::test]
async fn qa_queue_items_include_canonical_trace_from_prior_scoring_audit() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "canonical_claim_context": {
            "claim_header": {
              "external_claim_id": "CLM-QA-CANONICAL",
              "total_amount": 9300,
              "currency": "CNY",
              "service_date": "2026-01-06"
            },
            "member_policy_snapshot": {
              "masked_member_id": "masked-member-qa",
              "masked_certificate_id": "masked-cert-qa",
              "member_birth_date": "1988-03-12",
              "member_gender": "F",
              "policy_id": "POL-QA-CANONICAL",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": 10000
            },
            "provider_snapshot": {
              "provider_id": "PRV-QA-CANONICAL",
              "name": "QA Trace Hospital",
              "provider_type": "hospital",
              "region": "SH",
              "risk_tier": "High"
            },
            "itemized_bill_lines": [
              {
                "item_name": "High cost imaging",
                "fee_category": "procedure",
                "amount": 9300,
                "diagnosis_list": [{ "code": "J10", "name": "Influenza" }],
                "source_path": "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]",
                "evidence_refs": ["invoice:INV-QA:fee_detail:LINE-1"]
              }
            ],
            "document_evidence": [
              {
                "document_id": "MR-QA-1",
                "medical_record_type": "outpatient_record",
                "source_refs": ["medical_record:MR-QA-1"]
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/audit-samples",
        r#"{
          "sample_mode": "qa_calibration",
          "population_definition": "Canonical high risk claims for QA queue",
          "inclusion_criteria": { "min_risk_score": 70 },
          "deterministic_seed": "qa-canonical-trace",
          "sample_size": 1,
          "reviewer": "qa-reviewer-1",
          "assignment_queue": "QA Review"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, queue) = json_request(app, "GET", "/api/v1/ops/qa/queue", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let item = queue["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["claim_id"] == "CLM-QA-CANONICAL")
        .expect("canonical scored claim should enter QA queue");
    assert!(
        item["canonical_source_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]"
            )),
        "QA queue should expose normalized bill-line source path"
    );
    assert!(
        item["canonical_source_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("medical_record:MR-QA-1")),
        "QA queue should expose normalized document source ref"
    );
    assert!(
        item["canonical_evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("invoice:INV-QA:fee_detail:LINE-1")),
        "QA queue should expose canonical evidence refs for QA writeback"
    );
}

#[tokio::test]
async fn qa_result_writeback_preserves_canonical_evidence_refs_from_scoring_audit() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "canonical_claim_context": {
            "claim_header": {
              "external_claim_id": "CLM-QA-WRITEBACK-CANONICAL",
              "total_amount": 9300,
              "currency": "CNY",
              "service_date": "2026-01-06"
            },
            "member_policy_snapshot": {
              "masked_member_id": "masked-member-qa-writeback",
              "masked_certificate_id": "masked-cert-qa-writeback",
              "policy_id": "POL-QA-WRITEBACK-CANONICAL",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": 10000
            },
            "provider_snapshot": {
              "provider_id": "PRV-QA-WRITEBACK-CANONICAL",
              "name": "QA Trace Hospital",
              "provider_type": "hospital",
              "region": "SH",
              "risk_tier": "High"
            },
            "itemized_bill_lines": [
              {
                "item_name": "High cost imaging",
                "fee_category": "procedure",
                "amount": 9300,
                "diagnosis_list": [{ "code": "J10", "name": "Influenza" }],
                "source_path": "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]",
                "evidence_refs": ["invoice:INV-QA-WRITEBACK:fee_detail:LINE-1"]
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, qa) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-WRITEBACK-CANONICAL",
          "claim_id": "CLM-QA-WRITEBACK-CANONICAL",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "workflow_missing_evidence",
          "feedback_target": "workflow",
          "notes": "Reviewer found incomplete evidence handling.",
          "evidence_refs": ["qa_reviews:QA-WRITEBACK-CANONICAL"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        qa["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "invoice:INV-QA-WRITEBACK:fee_detail:LINE-1"
            )),
        "QA writeback response should preserve canonical evidence refs"
    );

    let (status, audit) = json_request(
        app,
        "GET",
        "/api/v1/audit/claims/CLM-QA-WRITEBACK-CANONICAL",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let qa_event = audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "qa.result.received")
        .expect("QA result should be in audit history");
    assert!(
        qa_event["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "invoice:INV-QA-WRITEBACK:fee_detail:LINE-1"
            )),
        "QA audit event should preserve canonical evidence refs"
    );
    assert!(
        qa_event["payload"]["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "invoice:INV-QA-WRITEBACK:fee_detail:LINE-1"
            )),
        "QA audit payload should preserve canonical evidence refs"
    );
}
