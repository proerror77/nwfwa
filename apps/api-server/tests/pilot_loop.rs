use api_server::{
    app::{build_app, build_app_with_parts},
    repository::InMemoryScoringRepository,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use fwa_ml_runtime::HeuristicModelScorer;
use std::sync::Arc;
use tower::ServiceExt;

#[path = "pilot_loop/events_alerts.rs"]
mod events_alerts;
#[path = "pilot_loop/qa_feedback.rs"]
mod qa_feedback;
#[path = "pilot_loop/support.rs"]
mod support;
#[path = "pilot_loop/writebacks.rs"]
mod writebacks;

use support::{json_request, scoped_config, test_config, unauthenticated_request};

#[tokio::test]
async fn lists_governed_outcome_labels_from_investigation_and_qa() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-LABEL-1001",
          "investigation_id": "INV-LABEL-1001",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "8200.00",
          "currency": "CNY",
          "notes": "Confirmed over-treatment after manual investigation.",
          "evidence_refs": ["investigation_results:INV-LABEL-1001", "knowledge_cases:KC-1001"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-LABEL-1001",
          "claim_id": "CLM-LABEL-1001",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "medical_necessity_issue",
          "feedback_target": "model",
          "notes": "QA found missing clinical support and model under-scored the claim.",
          "evidence_refs": ["qa_reviews:QA-LABEL-1001", "model_scores:baseline_fwa"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-LABEL-1002",
          "investigation_id": "INV-LABEL-1002",
          "outcome": "recovery_confirmed",
          "confirmed_fwa": true,
          "financial_impact_type": "recovered_amount",
          "saving_amount": "1200.00",
          "currency": "CNY",
          "notes": "Post-payment recovery confirmed.",
          "evidence_refs": ["investigation_results:INV-LABEL-1002"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/medical-review/results",
        r#"{
          "claim_id": "CLM-LABEL-1001",
          "scoring_audit_id": "audit_scoring_label_1001",
          "reviewer": "medical-reviewer-1",
          "decision": "medical_necessity_issue",
          "notes": "Medical reviewer confirmed the billed service lacks clinical necessity support.",
          "evidence_refs": ["audit:audit_scoring_label_1001", "medical_review:MR-LABEL-1001"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, labels) = json_request(app.clone(), "GET", "/api/v1/ops/labels", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let labels = labels["labels"].as_array().unwrap();
    assert!(labels.iter().any(|label| {
        label["claim_id"] == "CLM-LABEL-1001"
            && label["label_name"] == "confirmed_fwa"
            && label["label_value"] == "true"
            && label["source_type"] == "investigation_result"
            && label["governance_status"] == "approved_for_training"
    }));
    assert!(labels.iter().any(|label| {
        label["claim_id"] == "CLM-LABEL-1001"
            && label["label_name"] == "amount_prevented"
            && label["label_value"] == "8200.00"
            && label["currency"] == "CNY"
    }));
    assert!(labels.iter().any(|label| {
        label["claim_id"] == "CLM-LABEL-1002"
            && label["label_name"] == "amount_recovered"
            && label["label_value"] == "1200.00"
            && label["currency"] == "CNY"
    }));
    assert!(labels.iter().any(|label| {
        label["claim_id"] == "CLM-LABEL-1001"
            && label["label_name"] == "medical_necessity_issue"
            && label["label_value"] == "true"
            && label["source_type"] == "qa_review"
            && label["feedback_target"] == "model"
            && label["governance_status"] == "needs_review"
    }));
    assert!(labels.iter().any(|label| {
        label["claim_id"] == "CLM-LABEL-1001"
            && label["label_name"] == "medical_necessity_issue"
            && label["label_value"] == "true"
            && label["source_type"] == "medical_review"
            && label["feedback_target"] == "model"
            && label["governance_status"] == "approved_for_training"
            && label["source_id"].as_str().unwrap().starts_with("aud_")
    }));
    assert!(labels
        .iter()
        .all(|label| !label["evidence_refs"].as_array().unwrap().is_empty()));

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-LABEL-1001/status",
        r#"{
          "status": "resolved",
          "actor_id": "model-ops",
          "notes": "Model operator approved the QA feedback label for training.",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-LABEL-1001"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, labels) = json_request(app, "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert!(labels["labels"].as_array().unwrap().iter().any(|label| {
        label["claim_id"] == "CLM-LABEL-1001"
            && label["label_name"] == "medical_necessity_issue"
            && label["source_type"] == "qa_review"
            && label["feedback_target"] == "model"
            && label["governance_status"] == "approved_for_training"
    }));
}

#[tokio::test]
async fn lists_governed_outcome_labels_from_terminal_case_status() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-CASE-LABEL-1",
            "claim_amount": "9000",
            "currency": "CNY",
            "service_date": "2026-01-05",
            "diagnosis_code": "J10",
            "member": {
              "external_member_id": "MBR-CASE-LABEL-1"
            },
            "policy": {
              "external_policy_id": "POL-CASE-LABEL-1",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": "10000",
              "currency": "CNY"
            },
            "provider": {
              "external_provider_id": "PRV-CASE-LABEL-1",
              "name": "Northwind Hospital",
              "provider_type": "hospital",
              "region": "SH",
              "risk_tier": "High"
            },
            "items": [
              {
                "item_code": "PROC-001",
                "item_type": "procedure",
                "description": "Imaging",
                "quantity": 1,
                "unit_amount": "9000",
                "total_amount": "9000",
                "currency": "CNY"
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, leads) = json_request(app.clone(), "GET", "/api/v1/ops/leads", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let lead_id = leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lead| lead["claim_id"] == "CLM-CASE-LABEL-1")
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
          "assignee": "siu-case-label-owner",
          "reviewer": "medical-case-label-owner",
          "priority": "high",
          "notes": "Open case for terminal status label generation.",
          "evidence_refs": ["triage_decisions:case_label_generation"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let case_id = triage["case"]["case_id"].as_str().unwrap();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "confirmed",
          "actor_id": "siu-case-label-owner",
          "notes": "Case reviewer confirmed FWA.",
          "evidence_refs": ["case_workflow:confirmed"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, labels) = json_request(app.clone(), "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert!(labels["labels"].as_array().unwrap().iter().any(|label| {
        label["claim_id"] == "CLM-CASE-LABEL-1"
            && label["label_name"] == "confirmed_fwa"
            && label["label_value"] == "true"
            && label["source_type"] == "case_status"
            && label["source_id"] == case_id
            && label["governance_status"] == "approved_for_training"
            && label["feedback_target"] == "model"
            && label["evidence_refs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|reference| {
                    reference == &serde_json::json!(format!("investigation_cases:{case_id}"))
                })
    }));

    let (status, _) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "rejected",
          "actor_id": "siu-case-label-owner",
          "notes": "Case reviewer rejected the lead after investigation.",
          "evidence_refs": ["case_workflow:rejected"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, labels) = json_request(app, "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let labels = labels["labels"].as_array().unwrap();
    assert!(labels.iter().any(|label| {
        label["claim_id"] == "CLM-CASE-LABEL-1"
            && label["label_name"] == "confirmed_fwa"
            && label["label_value"] == "false"
            && label["source_type"] == "case_status"
            && label["source_id"] == case_id
            && label["governance_status"] == "needs_review"
            && label["feedback_target"] == "model"
    }));
    assert!(labels.iter().any(|label| {
        label["claim_id"] == "CLM-CASE-LABEL-1"
            && label["label_name"] == "false_positive"
            && label["label_value"] == "true"
            && label["source_type"] == "case_status"
            && label["source_id"] == case_id
            && label["governance_status"] == "needs_review"
            && label["feedback_target"] == "rules"
    }));
}

#[tokio::test]
async fn returns_member_profile_summary_from_scored_claims() {
    let app = build_app(test_config());

    for (claim_id, policy_id, amount, limit) in [
        ("CLM-MEMBER-1001", "POL-MEMBER-1001", "9200.00", "10000.00"),
        ("CLM-MEMBER-1002", "POL-MEMBER-1002", "1800.00", "12000.00"),
    ] {
        let (status, _) = json_request(
            app.clone(),
            "POST",
            "/api/v1/claims/score",
            &format!(
                r#"{{
                  "source_system": "tpa-demo",
                  "claim": {{
                    "external_claim_id": "{claim_id}",
                    "claim_amount": "{amount}",
                    "currency": "CNY",
                    "service_date": "2026-02-05",
                    "diagnosis_code": "J10",
                    "member": {{
                      "external_member_id": "MBR-PROFILE-1"
                    }},
                    "policy": {{
                      "external_policy_id": "{policy_id}",
                      "product_code": "MED",
                      "coverage_start_date": "2026-01-01",
                      "coverage_end_date": "2026-12-31",
                      "coverage_limit": "{limit}",
                      "currency": "CNY"
                    }},
                    "provider": {{
                      "external_provider_id": "PRV-PROFILE-1",
                      "name": "Profile Hospital",
                      "provider_type": "hospital",
                      "region": "Shanghai",
                      "risk_tier": "Medium"
                    }},
                    "items": [
                      {{
                        "item_code": "PROC-1",
                        "item_type": "procedure",
                        "description": "Procedure",
                        "quantity": 1,
                        "unit_amount": "{amount}",
                        "total_amount": "{amount}",
                        "currency": "CNY"
                      }}
                    ]
                  }}
                }}"#
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    let (status, profile) = json_request(
        app,
        "GET",
        "/api/v1/members/MBR-PROFILE-1/profile-summary",
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(profile["member_id"], "MBR-PROFILE-1");
    assert_eq!(profile["claim_count"], 2);
    assert_eq!(profile["policy_count"], 2);
    assert_eq!(profile["currency"], "CNY");
    assert_eq!(profile["total_claim_amount"], "11000.00");
    assert_eq!(profile["latest_claim_id"], "CLM-MEMBER-1002");
    assert!(profile["high_risk_claim_count"].as_u64().unwrap() >= 1);
    assert!(profile["profile_summary"]
        .as_str()
        .unwrap()
        .contains("2 笔历史理赔"));
    assert!(profile["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("members:MBR-PROFILE-1")));
}

#[tokio::test]
async fn member_profile_summary_is_scoped_to_authenticated_customer() {
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

    let (status, _) = json_request(
        alpha_app,
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-MEMBER-SCOPE-1",
            "claim_amount": "9200.00",
            "currency": "CNY",
            "service_date": "2026-02-05",
            "diagnosis_code": "J10",
            "member": {
              "external_member_id": "MBR-SCOPE-PROFILE-1"
            },
            "policy": {
              "external_policy_id": "POL-MEMBER-SCOPE-1",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": "10000.00",
              "currency": "CNY"
            },
            "provider": {
              "external_provider_id": "PRV-MEMBER-SCOPE-1",
              "name": "Profile Hospital",
              "provider_type": "hospital",
              "region": "Shanghai",
              "risk_tier": "Medium"
            },
            "items": [
              {
                "item_code": "PROC-1",
                "item_type": "procedure",
                "description": "Procedure",
                "quantity": 1,
                "unit_amount": "9200.00",
                "total_amount": "9200.00",
                "currency": "CNY"
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/members/MBR-SCOPE-PROFILE-1/profile-summary")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let response = beta_app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn qa_feedback_and_dashboard_are_scoped_to_authenticated_customer() {
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

    let (status, _) = json_request(
        alpha_app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-SCOPE-ALPHA-1",
          "claim_id": "CLM-QA-SCOPE-ALPHA-1",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "medical_necessity_issue",
          "feedback_target": "rules",
          "notes": "Alpha QA found missing medical necessity evidence.",
          "evidence_refs": ["qa_reviews:QA-SCOPE-ALPHA-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, alpha_feedback) = json_request(
        alpha_app.clone(),
        "GET",
        "/api/v1/ops/qa/feedback-items",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(alpha_feedback["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item["feedback_id"] == "qa_feedback_QA-SCOPE-ALPHA-1"
                && item["claim_id"] == "CLM-QA-SCOPE-ALPHA-1"
        }));

    let (status, beta_feedback) = json_request(
        beta_app.clone(),
        "GET",
        "/api/v1/ops/qa/feedback-items",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!beta_feedback["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item["feedback_id"] == "qa_feedback_QA-SCOPE-ALPHA-1"
                || item["claim_id"] == "CLM-QA-SCOPE-ALPHA-1"
        }));

    let (status, beta_update) = json_request(
        beta_app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-SCOPE-ALPHA-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "beta-reviewer",
          "notes": "Beta reviewer must not update alpha feedback.",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-SCOPE-ALPHA-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(beta_update["code"], "QA_FEEDBACK_NOT_FOUND");

    let (status, beta_summary) = json_request(
        beta_app.clone(),
        "GET",
        "/api/v1/ops/qa/queue-summary",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(beta_summary["open_count"], 0);
    assert_eq!(beta_summary["unresolved_count"], 0);

    let (status, beta_labels) =
        json_request(beta_app.clone(), "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!beta_labels["labels"]
        .as_array()
        .unwrap()
        .iter()
        .any(|label| {
            label["source_id"] == "QA-SCOPE-ALPHA-1" || label["claim_id"] == "CLM-QA-SCOPE-ALPHA-1"
        }));

    let (status, beta_dashboard) =
        json_request(beta_app, "GET", "/api/v1/ops/dashboard/summary", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(beta_dashboard["qa_reviews"], 0);
    assert_eq!(beta_dashboard["qa_queue"]["feedback_open_count"], 0);
    assert_eq!(beta_dashboard["label_pool"]["rule_feedback"], 0);
}

#[tokio::test]
async fn writeback_ids_cannot_be_reused_across_customers() {
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

    let (status, _) = json_request(
        alpha_app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-CROSS-SCOPE-REUSE-1",
          "claim_id": "CLM-QA-CROSS-SCOPE-ALPHA",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "medical_necessity_issue",
          "feedback_target": "rules",
          "notes": "Alpha QA result should own this QA case id.",
          "evidence_refs": ["qa_reviews:QA-CROSS-SCOPE-REUSE-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, beta_qa) = json_request(
        beta_app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-CROSS-SCOPE-REUSE-1",
          "claim_id": "CLM-QA-CROSS-SCOPE-BETA",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "workflow_missing_evidence",
          "feedback_target": "workflow",
          "notes": "Beta must not overwrite alpha QA result.",
          "evidence_refs": ["qa_reviews:QA-CROSS-SCOPE-REUSE-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(beta_qa["code"], "QA_CASE_SCOPE_CONFLICT");

    let (status, alpha_feedback) = json_request(
        alpha_app.clone(),
        "GET",
        "/api/v1/ops/qa/feedback-items",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(alpha_feedback["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item["qa_case_id"] == "QA-CROSS-SCOPE-REUSE-1"
                && item["claim_id"] == "CLM-QA-CROSS-SCOPE-ALPHA"
                && item["feedback_target"] == "rules"
        }));

    let (status, _) = json_request(
        alpha_app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-INV-CROSS-SCOPE-ALPHA",
          "investigation_id": "INV-CROSS-SCOPE-REUSE-1",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "1200.00",
          "currency": "CNY",
          "notes": "Alpha investigation result should own this investigation id.",
          "evidence_refs": ["investigation_results:INV-CROSS-SCOPE-REUSE-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, beta_investigation) = json_request(
        beta_app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-INV-CROSS-SCOPE-BETA",
          "investigation_id": "INV-CROSS-SCOPE-REUSE-1",
          "outcome": "no_issue_found",
          "confirmed_fwa": false,
          "saving_amount": "0.00",
          "currency": "CNY",
          "notes": "Beta must not overwrite alpha investigation result.",
          "evidence_refs": ["investigation_results:INV-CROSS-SCOPE-REUSE-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        beta_investigation["code"],
        "INVESTIGATION_RESULT_SCOPE_CONFLICT"
    );

    let (status, alpha_labels) = json_request(alpha_app, "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert!(alpha_labels["labels"]
        .as_array()
        .unwrap()
        .iter()
        .any(|label| {
            label["source_id"] == "INV-CROSS-SCOPE-REUSE-1"
                && label["claim_id"] == "CLM-INV-CROSS-SCOPE-ALPHA"
        }));

    let (status, beta_labels) = json_request(beta_app, "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!beta_labels["labels"]
        .as_array()
        .unwrap()
        .iter()
        .any(|label| label["source_id"] == "INV-CROSS-SCOPE-REUSE-1"));
}

#[tokio::test]
async fn canonical_evidence_merge_is_scoped_to_authenticated_customer() {
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

    let (status, _) = json_request(
        alpha_app,
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "canonical_claim_context": {
            "claim_header": {
              "external_claim_id": "CLM-CANONICAL-SCOPE-REUSE",
              "total_amount": 9300,
              "currency": "CNY",
              "service_date": "2026-01-06"
            },
            "member_policy_snapshot": {
              "masked_member_id": "masked-member-alpha-canonical",
              "masked_certificate_id": "masked-cert-alpha-canonical",
              "policy_id": "POL-CANONICAL-SCOPE-ALPHA",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": 10000
            },
            "provider_snapshot": {
              "provider_id": "PRV-CANONICAL-SCOPE-ALPHA",
              "name": "Alpha Trace Hospital",
              "provider_type": "hospital",
              "region": "SH",
              "risk_tier": "High"
            },
            "itemized_bill_lines": [
              {
                "item_name": "Alpha-only imaging",
                "fee_category": "procedure",
                "amount": 9300,
                "diagnosis_list": [{ "code": "J10", "name": "Influenza" }],
                "source_path": "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]",
                "evidence_refs": ["invoice:ALPHA-CANONICAL-ONLY:fee_detail:LINE-1"]
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, beta_qa) = json_request(
        beta_app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-CANONICAL-SCOPE-BETA",
          "claim_id": "CLM-CANONICAL-SCOPE-REUSE",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "workflow_missing_evidence",
          "feedback_target": "workflow",
          "notes": "Beta QA should not inherit alpha canonical evidence.",
          "evidence_refs": ["qa_reviews:QA-CANONICAL-SCOPE-BETA"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!beta_qa["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "invoice:ALPHA-CANONICAL-ONLY:fee_detail:LINE-1"
        )));

    let (status, beta_investigation) = json_request(
        beta_app,
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-CANONICAL-SCOPE-REUSE",
          "investigation_id": "INV-CANONICAL-SCOPE-BETA",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "1200.00",
          "currency": "CNY",
          "notes": "Beta investigation should not inherit alpha canonical evidence.",
          "evidence_refs": ["investigation_results:INV-CANONICAL-SCOPE-BETA"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!beta_investigation["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "invoice:ALPHA-CANONICAL-ONLY:fee_detail:LINE-1"
        )));
}

#[tokio::test]
async fn pilot_loop_endpoints_require_api_key() {
    for (method, uri, body) in [
        (
            "POST",
            "/api/v1/investigations/results",
            r#"{
              "claim_id": "CLM-0287",
              "investigation_id": "INV-1001",
              "outcome": "confirmed_fwa",
              "confirmed_fwa": true,
              "saving_amount": "8200.00",
              "currency": "CNY",
              "notes": "missing key",
              "evidence_refs": ["agent_run:agent_CLM-0287"]
            }"#,
        ),
        (
            "POST",
            "/api/v1/qa/results",
            r#"{
              "qa_case_id": "QA-9001",
              "claim_id": "CLM-0287",
              "qa_conclusion": "issue_found_escalate",
              "issue_type": "alert_handling_incomplete",
              "feedback_target": "rules",
              "notes": "missing key",
              "evidence_refs": ["rule_runs:EARLY_CLAIM"]
            }"#,
        ),
        ("GET", "/api/v1/audit/claims/CLM-0287", "{}"),
        ("GET", "/api/v1/members/MBR-PROFILE-1/profile-summary", "{}"),
        ("GET", "/api/v1/ops/webhook-events", "{}"),
        (
            "POST",
            "/api/v1/ops/webhook-events/webhook_audit_1/delivery-attempts",
            r#"{"delivery_status":"failed"}"#,
        ),
    ] {
        let status = unauthenticated_request(method, uri, body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
