use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{json_request, test_config};

#[tokio::test]
async fn writes_investigation_and_qa_results_then_returns_claim_audit_history() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-0287",
          "investigation_id": "INV-MISSING-EVIDENCE",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": " ",
          "evidence_refs": ["agent_run:agent_CLM-0287"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_INVESTIGATION_RESULT_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": " ",
          "investigation_id": "INV-MISSING-CLAIM",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "TPA investigation confirmed over-treatment signals.",
          "evidence_refs": ["agent_run:agent_CLM-0287"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_INVESTIGATION_RESULT_IDENTITY");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-0287",
          "investigation_id": "INV-BLANK-EVIDENCE",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "TPA investigation confirmed over-treatment signals.",
          "evidence_refs": ["agent_run:agent_CLM-0287", " "]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_INVESTIGATION_RESULT_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-0287",
          "investigation_id": "INV-BAD-IMPACT",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "financial_impact_type": "unsupported_impact",
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "TPA investigation confirmed over-treatment signals.",
          "evidence_refs": ["agent_run:agent_CLM-0287"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "UNSUPPORTED_FINANCIAL_IMPACT_TYPE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-0287",
          "investigation_id": "INV-NEGATIVE-SAVING",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "financial_impact_type": "prevented_payment",
          "saving_amount": "-1.00",
          "currency": "CNY",
          "notes": "TPA investigation confirmed over-treatment signals.",
          "evidence_refs": ["agent_run:agent_CLM-0287"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_INVESTIGATION_SAVING_AMOUNT");

    let (status, investigation) = json_request(
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
          "customer_scope_id": "spoofed-customer",
          "evidence_refs": ["agent_run:agent_CLM-0287", "knowledge_cases:KC-1001"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(investigation["claim_id"], "CLM-0287");
    assert_eq!(investigation["event_type"], "investigation.result.received");
    assert!(investigation["idempotency_key"]
        .as_str()
        .unwrap()
        .starts_with("tpa-writeback:investigation.result.received:"));

    let (status, repeated_investigation) = json_request(
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
          "evidence_refs": ["agent_run:agent_CLM-0287", "knowledge_cases:KC-1001"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        repeated_investigation["idempotency_key"],
        investigation["idempotency_key"]
    );

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-MISSING-EVIDENCE",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Reviewer should attach provider history evidence.",
          "evidence_refs": []
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_QA_RESULT_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": " ",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Reviewer should attach provider history evidence.",
          "evidence_refs": ["audit:investigation.result.received"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_QA_RESULT_IDENTITY");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-BLANK-EVIDENCE",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Reviewer should attach provider history evidence.",
          "evidence_refs": ["audit:investigation.result.received", " "]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_QA_RESULT_EVIDENCE");

    let (status, qa) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-9001",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Reviewer should attach provider history evidence.",
          "customer_scope_id": "spoofed-customer",
          "evidence_refs": ["audit:investigation.result.received", "rule_runs:EARLY_CLAIM"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(qa["claim_id"], "CLM-0287");
    assert_eq!(qa["event_type"], "qa.result.received");
    assert!(qa["idempotency_key"]
        .as_str()
        .unwrap()
        .starts_with("tpa-writeback:qa.result.received:"));

    let (status, repeated_qa) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-9001",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Reviewer should attach provider history evidence.",
          "evidence_refs": ["audit:investigation.result.received", "rule_runs:EARLY_CLAIM"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(repeated_qa["idempotency_key"], qa["idempotency_key"]);

    let (status, audit) = json_request(app, "GET", "/api/v1/audit/claims/CLM-0287", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(audit["claim_id"], "CLM-0287");
    let events = audit["events"].as_array().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["event_type"], "investigation.result.received");
    assert_eq!(events[1]["event_type"], "qa.result.received");
    assert_eq!(events[0]["actor_role"], "tpa_system");
    assert_eq!(events[1]["actor_role"], "tpa_system");
    assert_eq!(events[0]["payload"]["customer_scope_id"], "demo-customer");
    assert_eq!(events[1]["payload"]["customer_scope_id"], "demo-customer");
    assert_eq!(events[0]["payload"]["actor_id"], "tpa-demo");
    assert_eq!(events[1]["payload"]["actor_id"], "tpa-demo");
    assert_eq!(events[0]["payload"]["actor_role"], "tpa_system");
    assert_eq!(events[1]["payload"]["actor_role"], "tpa_system");
    assert!(events
        .iter()
        .all(|event| !event["evidence_refs"].as_array().unwrap().is_empty()));
}

#[tokio::test]
async fn investigation_result_writeback_preserves_canonical_evidence_refs_from_scoring_audit() {
    let app = build_app(test_config()).unwrap();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "canonical_claim_context": {
            "claim_header": {
              "external_claim_id": "CLM-INVESTIGATION-CANONICAL",
              "total_amount": 12000,
              "currency": "CNY",
              "service_date": "2026-01-06"
            },
            "member_policy_snapshot": {
              "masked_member_id": "masked-member-investigation",
              "masked_certificate_id": "masked-cert-investigation",
              "policy_id": "POL-INVESTIGATION-CANONICAL",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": 15000
            },
            "provider_snapshot": {
              "provider_id": "PRV-INVESTIGATION-CANONICAL",
              "name": "Investigation Trace Hospital",
              "provider_type": "hospital",
              "region": "SH",
              "risk_tier": "High"
            },
            "itemized_bill_lines": [
              {
                "item_code": "IMG-INVESTIGATION",
                "item_name": "High cost imaging",
                "fee_category": "procedure",
                "amount": 12000,
                "diagnosis_list": [{ "code": "J10", "name": "Influenza" }],
                "source_path": "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]",
                "evidence_refs": ["invoice:INV-INVESTIGATION:fee_detail:LINE-1"]
              }
            ]
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
          "claim_id": "CLM-INVESTIGATION-CANONICAL",
          "investigation_id": "INV-CANONICAL-WRITEBACK",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "financial_impact_type": "prevented_payment",
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "TPA investigation confirmed over-treatment signals.",
          "evidence_refs": ["investigation_results:INV-CANONICAL-WRITEBACK"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        investigation["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "invoice:INV-INVESTIGATION:fee_detail:LINE-1"
            )),
        "investigation response should preserve canonical evidence refs"
    );

    let (status, audit) = json_request(
        app.clone(),
        "GET",
        "/api/v1/audit/claims/CLM-INVESTIGATION-CANONICAL",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let investigation_event = audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "investigation.result.received")
        .expect("investigation event should be in audit history");
    assert!(
        investigation_event["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "invoice:INV-INVESTIGATION:fee_detail:LINE-1"
            )),
        "investigation audit event should preserve canonical evidence refs"
    );
    assert!(
        investigation_event["payload"]["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "invoice:INV-INVESTIGATION:fee_detail:LINE-1"
            )),
        "investigation audit payload should preserve canonical evidence refs"
    );

    let (status, labels) = json_request(app, "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let label = labels["labels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|label| label["source_id"] == "INV-CANONICAL-WRITEBACK")
        .expect("investigation label should be listed");
    assert!(
        label["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "invoice:INV-INVESTIGATION:fee_detail:LINE-1"
            )),
        "investigation outcome label should preserve canonical evidence refs"
    );
}

#[tokio::test]
async fn rejects_pii_in_investigation_and_qa_writebacks() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-PII-WRITEBACK",
          "investigation_id": "INV-PII-NOTES",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "Reviewer copied member email alice@example.com into notes.",
          "evidence_refs": ["agent_run:agent_CLM-PII-WRITEBACK"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_WRITEBACK");

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-PII-EVIDENCE",
          "claim_id": "CLM-PII-WRITEBACK",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Reviewer should attach provider history evidence.",
          "evidence_refs": ["audit:scoring.completed", "phone:13800138000"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_WRITEBACK");
}

#[tokio::test]
async fn rejects_unsupported_qa_feedback_target() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-UNSUPPORTED-TARGET",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "unknown_target",
          "notes": "Unsupported feedback target should not enter QA governance.",
          "evidence_refs": ["audit:scoring.completed"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "UNSUPPORTED_FEEDBACK_TARGET");
}

#[tokio::test]
async fn rejects_unsupported_qa_conclusion() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-UNSUPPORTED-CONCLUSION",
          "claim_id": "CLM-0287",
          "qa_conclusion": "unknown_conclusion",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Unsupported QA conclusion should not enter QA governance.",
          "evidence_refs": ["audit:scoring.completed"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "UNSUPPORTED_QA_CONCLUSION");
}

#[tokio::test]
async fn rejects_unsupported_qa_issue_type() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-UNSUPPORTED-ISSUE",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "unknown_issue",
          "feedback_target": "rules",
          "notes": "Unsupported QA issue type should not enter QA governance.",
          "evidence_refs": ["audit:scoring.completed"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "UNSUPPORTED_QA_ISSUE_TYPE");
}

#[tokio::test]
async fn accepts_prd_issue_types_for_qa_writeback() {
    let app = build_app(test_config()).unwrap();

    for issue_type in [
        "confirmed_fwa",
        "false_positive",
        "improper_payment",
        "insufficient_evidence",
        "abuse_not_fraud",
        "documentation_issue",
        "medical_necessity_issue",
        "policy_exclusion",
    ] {
        let (status, body) = json_request(
            app.clone(),
            "POST",
            "/api/v1/qa/results",
            &format!(
                r#"{{
                  "qa_case_id": "QA-PRD-{issue_type}",
                  "claim_id": "CLM-0287",
                  "qa_conclusion": "issue_found_escalate",
                  "issue_type": "{issue_type}",
                  "feedback_target": "rules",
                  "notes": "PRD governed QA label should enter feedback and label governance.",
                  "evidence_refs": ["audit:scoring.completed", "qa_reviews:QA-PRD-{issue_type}"]
                }}"#
            ),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "expected PRD issue_type {issue_type} to be accepted: {body:?}"
        );
        assert_eq!(body["event_type"], "qa.result.received");
    }
}
