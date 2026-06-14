use api_server::app::build_app;
use axum::http::StatusCode;

use super::{json_request, register_model_dataset_for_dashboard, test_config};

#[tokio::test]
async fn returns_dashboard_summary_from_scoring_and_pilot_events() {
    let app = build_app(test_config()).unwrap();

    let (status, score) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-0287",
            "claim_amount": "8000",
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
              "unit_amount": "8000",
              "total_amount": "8000"
            }
          ],
          "member": {
            "external_member_id": "MBR-0287"
          },
          "policy": {
            "external_policy_id": "POL-0287",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-0287",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "High"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(score["rag"], "Red");

    let (status, _) = json_request(
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
          "evidence_refs": ["agent_run:agent_CLM-0287", "rule_runs:EARLY_CLAIM", "campaigns:prepay-fwa-sprint-q1", "knowledge_cases:KC-1001"]
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
          "population_definition": "High risk claims for QA dashboard",
          "inclusion_criteria": { "min_risk_score": 70 },
          "deterministic_seed": "dashboard-qa",
          "sample_size": 1,
          "reviewer": "qa-reviewer-1",
          "assignment_queue": "QA Review"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let selected_lead = &sample["selected_leads"].as_array().unwrap()[0];
    let selected_scheme = selected_lead["scheme_family"].as_str().unwrap().to_string();
    let qa_case_id = format!(
        "qa_{}_{}",
        sample["sample_id"].as_str().unwrap(),
        selected_lead["lead_id"].as_str().unwrap()
    );

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        &format!(
            r#"{{
          "qa_case_id": "{qa_case_id}",
          "claim_id": "CLM-0287",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Reviewer should attach provider history evidence.",
          "evidence_refs": ["audit:investigation.result.received", "rule_runs:EARLY_CLAIM"]
        }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let feedback_id = format!("qa_feedback_{qa_case_id}");
    let (status, _) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/qa/feedback-items/{feedback_id}/status"),
        &format!(
            r#"{{
          "status": "in_progress",
          "actor_id": "dashboard-qa-lead",
          "notes": "Dashboard should show in-progress feedback as unresolved.",
          "evidence_refs": ["qa_feedback:{feedback_id}"]
        }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    for (qa_case_id, issue_type, feedback_target, evidence_ref) in [
        (
            "QA-DASHBOARD-FEATURES",
            "model_under_scored_confirmed_issue",
            "features",
            "features:claim_amount_to_limit_ratio",
        ),
        (
            "QA-DASHBOARD-PROVIDER",
            "provider_pattern",
            "provider_profile",
            "provider_profile:PRV-0287:90d",
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
          "notes": "Dashboard label pool should expose {feedback_target} feedback labels.",
          "evidence_refs": ["{evidence_ref}"]
        }}"#
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    let (status, agent_investigation) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-0287",
          "risk_score": 87,
          "rag": "RED",
          "top_reasons": ["金额高于同病种同地区 P99", "Provider 风险画像偏高"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["provider_outlier"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let agent_run_id = agent_investigation["agent_run_id"].as_str().unwrap();
    let (status, _) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/agent-runs/{agent_run_id}/approvals"),
        &format!(
            r#"{{
          "decision": "approved",
          "approver": "qa-lead",
          "reason": "Evidence package is sufficient for manual review routing.",
          "evidence_refs": ["agent_run:{agent_run_id}", "agent_approval:manual_review_required"]
        }}"#,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/medical-review/results",
        r#"{
          "claim_id": "CLM-0287",
          "scoring_audit_id": "audit_dashboard_scoring_0287",
          "reviewer": "medical-reviewer-1",
          "decision": "request_more_evidence",
          "notes": "Dashboard label pool should include medical evidence gap feedback.",
          "evidence_refs": ["audit:audit_dashboard_scoring_0287", "documents:medical_record"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let model_dataset_id = register_model_dataset_for_dashboard(app.clone()).await;
    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_dashboard_baseline_001",
              "model_key": "baseline_fwa",
              "model_version": "0.1.0",
              "model_dataset_id": "{model_dataset_id}",
              "scheme_family": "diagnosis_procedure_mismatch",
              "auc": "0.81",
              "ks": "0.42",
              "precision": "0.70",
              "recall": "0.60",
              "f1": "0.65",
              "accuracy": "0.74",
              "threshold": "0.50",
              "confusion_matrix_json": {{"tp": 10, "fp": 2, "tn": 12, "fn": 3}},
              "feature_importance_uri": "data/eval/dashboard_model_eval/v1/feature_importance.parquet",
              "metrics_json": {{"score_psi": 0.31}}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, dashboard) = json_request(app, "GET", "/api/v1/ops/dashboard/summary", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(dashboard["suspected_claims"], 1);
    assert_eq!(dashboard["confirmed_fwa"], 1);
    assert_eq!(dashboard["qa_reviews"], 3);
    assert_eq!(dashboard["investigation_results"], 1);
    assert_eq!(dashboard["qa_queue"]["sampled_cases"], 1);
    assert_eq!(dashboard["qa_queue"]["open_cases"], 0);
    assert_eq!(dashboard["qa_queue"]["reviewed_cases"], 1);
    assert_eq!(dashboard["qa_queue"]["disagreement_cases"], 1);
    assert_eq!(dashboard["qa_queue"]["disagreement_rate"], 1.0);
    assert_eq!(dashboard["qa_queue"]["feedback_open_count"], 2);
    assert_eq!(dashboard["qa_queue"]["feedback_in_progress_count"], 1);
    assert_eq!(dashboard["qa_queue"]["feedback_resolved_count"], 0);
    assert_eq!(dashboard["qa_queue"]["feedback_dismissed_count"], 0);
    assert_eq!(dashboard["qa_queue"]["unresolved_feedback_count"], 3);
    assert_eq!(dashboard["qa_queue"]["rules_unresolved_feedback_count"], 1);
    assert_eq!(dashboard["qa_queue"]["models_unresolved_feedback_count"], 0);
    assert_eq!(
        dashboard["qa_queue"]["features_unresolved_feedback_count"],
        1
    );
    assert_eq!(
        dashboard["qa_queue"]["provider_profile_unresolved_feedback_count"],
        1
    );
    assert_eq!(
        dashboard["qa_queue"]["workflow_unresolved_feedback_count"],
        0
    );
    assert_eq!(dashboard["qa_queue"]["tpa_unresolved_feedback_count"], 0);
    assert_eq!(dashboard["agent_governance"]["total_runs"], 1);
    assert_eq!(dashboard["agent_governance"]["successful_runs"], 1);
    assert_eq!(dashboard["agent_governance"]["evidence_backed_runs"], 1);
    assert_eq!(dashboard["agent_governance"]["tool_call_count"], 1);
    assert_eq!(dashboard["agent_governance"]["policy_check_count"], 1);
    assert_eq!(
        dashboard["agent_governance"]["denied_policy_check_count"],
        0
    );
    assert_eq!(dashboard["agent_governance"]["failed_tool_call_count"], 0);
    assert_eq!(dashboard["agent_governance"]["pending_approvals"], 0);
    assert_eq!(dashboard["agent_governance"]["approved_approvals"], 1);
    assert_eq!(dashboard["agent_governance"]["rejected_approvals"], 0);
    assert_eq!(dashboard["model_governance"]["total_models"], 1);
    assert_eq!(dashboard["model_governance"]["evaluated_models"], 1);
    assert_eq!(dashboard["model_governance"]["drift_watch_count"], 0);
    assert_eq!(dashboard["model_governance"]["drift_detected_count"], 1);
    assert_eq!(dashboard["model_governance"]["average_precision"], 0.7);
    assert_eq!(dashboard["model_governance"]["average_recall"], 0.6);
    assert_eq!(dashboard["risk_amount"], "8000");
    assert_eq!(dashboard["saving_amount"], "8200.00");
    assert_eq!(
        dashboard["value_measurement"]["prevented_payment"],
        "8200.00"
    );
    assert_eq!(dashboard["value_measurement"]["recovered_amount"], "0.00");
    assert_eq!(dashboard["value_measurement"]["estimated_impact"], "0.00");
    assert_eq!(
        dashboard["scheme_distribution"][selected_scheme.as_str()],
        1
    );
    let attributions = dashboard["saving_attributions"].as_array().unwrap();
    assert_eq!(attributions.len(), 2);
    assert!(attributions.iter().any(|attribution| {
        attribution["source_type"] == "agent"
            && attribution["source_id"] == "agent_CLM-0287"
            && attribution["saving_amount"] == "4100.00"
            && attribution["evidence_refs"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("agent_run:agent_CLM-0287"))
    }));
    assert!(attributions.iter().any(|attribution| {
        attribution["source_type"] == "rule"
            && attribution["source_id"] == "EARLY_CLAIM"
            && attribution["saving_amount"] == "4100.00"
            && attribution["evidence_refs"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("rule_runs:EARLY_CLAIM"))
    }));
    let saving_segments = dashboard["saving_segments"].as_array().unwrap();
    assert!(saving_segments.iter().any(|segment| {
        segment["segment_type"] == "provider"
            && segment["segment_id"] == "PRV-0287"
            && segment["saving_amount"] == "8200.00"
            && segment["claim_count"] == 1
            && segment["attribution_count"] == 2
            && segment["roi"].as_f64().unwrap() > 0.0
    }));
    assert!(saving_segments.iter().any(|segment| {
        segment["segment_type"] == "scheme"
            && segment["segment_id"] == selected_scheme
            && segment["saving_amount"] == "8200.00"
            && segment["claim_count"] == 1
            && segment["attribution_count"] == 2
    }));
    assert!(saving_segments.iter().any(|segment| {
        segment["segment_type"] == "campaign"
            && segment["segment_id"] == "prepay-fwa-sprint-q1"
            && segment["saving_amount"] == "8200.00"
            && segment["claim_count"] == 1
            && segment["attribution_count"] == 2
            && segment["roi"].as_f64().unwrap() > 0.0
    }));
    assert_eq!(dashboard["rag_distribution"]["Red"], 1);
    assert!(dashboard["rule_hits"].as_u64().unwrap() >= 1);
    assert_eq!(dashboard["model_scores"]["baseline_fwa"]["scored_runs"], 1);
    assert!(
        dashboard["model_scores"]["baseline_fwa"]["average_score"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    assert_eq!(
        dashboard["layer_scores"]["L1_PEER_BENCHMARK"]["scored_runs"],
        1
    );
    assert_eq!(
        dashboard["layer_scores"]["L7_RISK_FUSION_ROUTING"]["scored_runs"],
        1
    );
    assert!(
        dashboard["layer_scores"]["L7_RISK_FUSION_ROUTING"]["average_score"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    assert!(
        dashboard["layer_scores"]["L7_RISK_FUSION_ROUTING"]["high_risk_count"]
            .as_u64()
            .unwrap()
            >= 1
    );
    assert_eq!(dashboard["label_pool"]["total_labels"], 6);
    assert_eq!(dashboard["label_pool"]["approved_for_training"], 2);
    assert_eq!(dashboard["label_pool"]["needs_review"], 4);
    assert_eq!(dashboard["label_pool"]["rule_feedback"], 1);
    assert_eq!(dashboard["label_pool"]["model_feedback"], 1);
    assert_eq!(dashboard["label_pool"]["features_feedback"], 1);
    assert_eq!(dashboard["label_pool"]["provider_profile_feedback"], 1);
    assert_eq!(dashboard["label_pool"]["workflow_feedback"], 2);
    assert_eq!(dashboard["label_pool"]["case_status_labels"], 0);
    assert_eq!(dashboard["label_pool"]["medical_review_labels"], 1);
    assert_eq!(dashboard["label_pool"]["false_positive_labels"], 0);
    assert_eq!(dashboard["label_pool"]["evidence_backed_labels"], 6);
}
