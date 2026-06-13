use api_server::app::build_app;
use axum::http::StatusCode;

use super::{json_request, score_high_risk_claim, test_config};

#[tokio::test]
async fn links_investigation_result_outcome_back_to_case() {
    let app = build_app(test_config()).unwrap();
    score_high_risk_claim(app.clone(), "CLM-CASE-FINAL").await;

    let (status, leads) = json_request(app.clone(), "GET", "/api/v1/ops/leads", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let lead_id = leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lead| lead["claim_id"] == "CLM-CASE-FINAL")
        .unwrap()["lead_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, triage) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "open_case",
          "assignee": "siu-reviewer-3",
          "reviewer": "medical-reviewer-3",
          "priority": "high",
          "notes": "Open investigation for final outcome writeback.",
          "evidence_refs": ["triage_decisions:open_case_final"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let case_id = triage["case"]["case_id"].as_str().unwrap();
    let notes = "Reviewer confirmed over-treatment after case investigation.";

    let (status, result) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        &format!(
            r#"{{
              "case_id": "{case_id}",
              "claim_id": "CLM-CASE-FINAL",
              "investigation_id": "INV-CASE-FINAL-1",
              "outcome": "confirmed_fwa",
              "confirmed_fwa": true,
              "financial_impact_type": "prevented_payment",
              "saving_amount": "1200.00",
              "currency": "CNY",
              "notes": "{notes}",
              "evidence_refs": [
                "investigation_cases:{case_id}",
                "investigation_results:INV-CASE-FINAL-1"
              ]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["event_type"], "investigation.result.received");

    let (status, cases) = json_request(app.clone(), "GET", "/api/v1/ops/cases", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["case_id"] == case_id)
        .unwrap();
    assert_eq!(case["final_outcome"], "confirmed_fwa");
    assert_eq!(case["reviewer_notes"], notes);
    assert_eq!(case["investigation_result_id"], "INV-CASE-FINAL-1");

    let (status, audit) =
        json_request(app, "GET", "/api/v1/audit/claims/CLM-CASE-FINAL", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let result_event = audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "investigation.result.received")
        .expect("investigation result should be audited");
    assert_eq!(result_event["payload"]["case_id"], case_id);
}

#[tokio::test]
async fn rejects_investigation_result_for_unknown_case() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "case_id": "case_missing",
          "claim_id": "CLM-CASE-MISSING",
          "investigation_id": "INV-CASE-MISSING-1",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "financial_impact_type": "prevented_payment",
          "saving_amount": "1200.00",
          "currency": "CNY",
          "notes": "Reviewer confirmed over-treatment after case investigation.",
          "evidence_refs": [
            "investigation_cases:case_missing",
            "investigation_results:INV-CASE-MISSING-1"
          ]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "CASE_NOT_FOUND");
}

#[tokio::test]
async fn replayed_investigation_result_clears_case_projection_when_unlinked() {
    let app = build_app(test_config()).unwrap();
    score_high_risk_claim(app.clone(), "CLM-CASE-REPLAY").await;

    let (status, leads) = json_request(app.clone(), "GET", "/api/v1/ops/leads", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let lead_id = leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lead| lead["claim_id"] == "CLM-CASE-REPLAY")
        .unwrap()["lead_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, triage) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "open_case",
          "assignee": "siu-reviewer-4",
          "reviewer": "medical-reviewer-4",
          "priority": "high",
          "notes": "Open investigation for replay handling.",
          "evidence_refs": ["triage_decisions:open_case_replay"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let case_id = triage["case"]["case_id"].as_str().unwrap();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        &format!(
            r#"{{
              "case_id": "{case_id}",
              "claim_id": "CLM-CASE-REPLAY",
              "investigation_id": "INV-CASE-REPLAY-1",
              "outcome": "confirmed_fwa",
              "confirmed_fwa": true,
              "financial_impact_type": "prevented_payment",
              "saving_amount": "1200.00",
              "currency": "CNY",
              "notes": "Initial case-linked outcome.",
              "evidence_refs": [
                "investigation_cases:{case_id}",
                "investigation_results:INV-CASE-REPLAY-1"
              ]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-CASE-REPLAY",
          "investigation_id": "INV-CASE-REPLAY-1",
          "outcome": "not_fwa",
          "confirmed_fwa": false,
          "financial_impact_type": "estimated_impact",
          "saving_amount": "0.00",
          "currency": "CNY",
          "notes": "Replay removed the case linkage after final reconciliation.",
          "evidence_refs": ["investigation_results:INV-CASE-REPLAY-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, cases) = json_request(app, "GET", "/api/v1/ops/cases", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["case_id"] == case_id)
        .unwrap();
    assert!(case["final_outcome"].is_null());
    assert!(case["reviewer_notes"].is_null());
    assert!(case["investigation_result_id"].is_null());
}
