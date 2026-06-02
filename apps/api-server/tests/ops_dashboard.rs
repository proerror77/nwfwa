use api_server::{app::build_app, config::AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use rust_decimal::Decimal;
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
    }
}

async fn json_request(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: &str,
) -> (StatusCode, serde_json::Value) {
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
    let body = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}));
    (status, body)
}

async fn register_model_dataset_for_dashboard(app: axum::Router) -> String {
    let (_, dataset) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        r#"{
          "source_key": "dashboard_model_eval",
          "display_name": "Dashboard Model Eval",
          "business_domain": "fwa_claims",
          "owner": "model-ops",
          "description": "Evaluation dataset for dashboard model governance.",
          "dataset_key": "dashboard_model_eval",
          "dataset_version": "v1",
          "sample_grain": "claim",
          "label_column": "confirmed_fwa",
          "entity_keys": ["claim_id"],
          "manifest_uri": "data/eval/dashboard_model_eval/v1/manifest.json",
          "schema_uri": "data/eval/dashboard_model_eval/v1/schema.json",
          "profile_uri": "data/eval/dashboard_model_eval/v1/profile.json",
          "storage_format": "parquet",
          "schema_hash": "sha256:dashboard-model-eval",
          "row_count": 100,
          "status": "draft",
          "splits": [
            {
              "split_name": "validation",
              "data_uri": "data/eval/dashboard_model_eval/v1/split=validation/",
              "row_count": 100,
              "positive_count": 25,
              "negative_count": 75,
              "label_distribution_json": {"1": 25, "0": 75}
            }
          ],
          "fields": [
            {
              "field_name": "claim_id",
              "logical_type": "string",
              "nullable": false,
              "semantic_role": "key",
              "description": "Claim id.",
              "profile_json": {}
            },
            {
              "field_name": "confirmed_fwa",
              "logical_type": "int8",
              "nullable": false,
              "semantic_role": "label",
              "description": "Confirmed FWA label.",
              "profile_json": {"allowed_values": [0, 1]}
            },
            {
              "field_name": "claim_amount_to_limit_ratio",
              "logical_type": "float64",
              "nullable": false,
              "semantic_role": "feature",
              "description": "Claim amount to policy limit ratio.",
              "profile_json": {}
            }
          ]
        }"#,
    )
    .await;
    let dataset_id = dataset["dataset_id"].as_str().unwrap();

    let (_, feature_set) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/feature-sets",
        &format!(
            r#"{{
              "business_domain": "fwa_claims",
              "feature_set_key": "dashboard_claims_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/eval/dashboard_model_eval/v1/features/",
              "feature_list_json": ["claim_amount_to_limit_ratio"],
              "row_count": 100,
              "label_column": "confirmed_fwa",
              "status": "draft"
            }}"#
        ),
    )
    .await;
    let feature_set_id = feature_set["feature_set_id"].as_str().unwrap();

    let (_, model_dataset) = json_request(
        app,
        "POST",
        "/api/v1/ops/model-datasets",
        &format!(
            r#"{{
              "business_domain": "fwa_claims",
              "task_type": "binary_classification",
              "label_name": "confirmed_fwa",
              "feature_set_id": "{feature_set_id}",
              "train_uri": "data/eval/dashboard_model_eval/v1/split=train/",
              "validation_uri": "data/eval/dashboard_model_eval/v1/split=validation/",
              "test_uri": null,
              "row_counts_json": {{"train": 80, "validation": 20}},
              "label_distribution_json": {{"train": {{"1": 20, "0": 60}}, "validation": {{"1": 5, "0": 15}}}},
              "status": "draft"
            }}"#
        ),
    )
    .await;
    model_dataset["model_dataset_id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn returns_dashboard_canonical_trace_coverage() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-DASHBOARD-FULL",
            "claim_amount": "1200",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "items": [
            {
              "item_code": "PROC-LOW",
              "item_type": "procedure",
              "description": "Routine visit",
              "quantity": 1,
              "unit_amount": "1200",
              "total_amount": "1200"
            }
          ],
          "member": { "external_member_id": "MBR-DASHBOARD-FULL" },
          "policy": {
            "external_policy_id": "POL-DASHBOARD-FULL",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-DASHBOARD-FULL",
            "name": "Routine Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "Low"
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
          "canonical_claim_context": {
            "claim_header": {
              "external_claim_id": "CLM-DASHBOARD-CANONICAL",
              "total_amount": 8800,
              "currency": "CNY",
              "service_date": "2026-01-06"
            },
            "member_policy_snapshot": {
              "masked_member_id": "masked-member-dashboard",
              "masked_certificate_id": "masked-cert-dashboard",
              "member_birth_date": "1988-03-12",
              "member_gender": "F",
              "policy_id": "POL-DASHBOARD-CANONICAL",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": 10000
            },
            "provider_snapshot": {
              "provider_id": "PRV-DASHBOARD-CANONICAL",
              "name": "Inbox Hospital",
              "provider_type": "hospital",
              "region": "SH",
              "risk_tier": "High"
            },
            "itemized_bill_lines": [
              {
                "item_name": "High cost imaging",
                "fee_category": "procedure",
                "amount": 8800,
                "diagnosis_list": [{ "code": "J10", "name": "Influenza" }],
                "source_path": "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]",
                "evidence_refs": ["invoice:INV-DASHBOARD:fee_detail:LINE-1"]
              }
            ],
            "document_evidence": [
              {
                "document_id": "MR-DASHBOARD-1",
                "medical_record_type": "outpatient_record",
                "source_refs": ["medical_record:MR-DASHBOARD-1"]
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, dashboard) = json_request(app, "GET", "/api/v1/ops/dashboard/summary", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(dashboard["audit_coverage"]["scoring_runs"], 2);
    assert_eq!(dashboard["audit_coverage"]["canonical_trace_runs"], 1);
    assert_eq!(dashboard["audit_coverage"]["canonical_trace_coverage"], 0.5);
}

#[tokio::test]
async fn returns_dashboard_summary_from_scoring_and_pilot_events() {
    let app = build_app(test_config());

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

#[tokio::test]
async fn dashboard_attributes_savings_to_governed_rule_and_model_evidence() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-GOV-ATTR-1",
          "investigation_id": "INV-GOV-ATTR-1",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "financial_impact_type": "prevented_payment",
          "saving_amount": "6000.00",
          "currency": "CNY",
          "notes": "Confirmed FWA with governed rule and model evidence.",
          "evidence_refs": ["rules:rule_early_claim:v1", "model_versions:baseline_fwa:0.1.0"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, dashboard) = json_request(app, "GET", "/api/v1/ops/dashboard/summary", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let attributions = dashboard["saving_attributions"].as_array().unwrap();
    assert_eq!(attributions.len(), 2);
    assert!(attributions.iter().any(|attribution| {
        attribution["source_type"] == "rule"
            && attribution["source_id"] == "rule_early_claim"
            && attribution["action"] == "investigation_confirmed"
            && attribution["saving_amount"] == "3000.00"
            && attribution["claim_count"] == 1
            && attribution["evidence_refs"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("rules:rule_early_claim:v1"))
    }));
    assert!(attributions.iter().any(|attribution| {
        attribution["source_type"] == "model"
            && attribution["source_id"] == "baseline_fwa"
            && attribution["action"] == "investigation_confirmed"
            && attribution["saving_amount"] == "3000.00"
            && attribution["claim_count"] == 1
            && attribution["evidence_refs"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("model_versions:baseline_fwa:0.1.0"))
    }));
}

#[tokio::test]
async fn dashboard_separates_observed_and_estimated_value() {
    let app = build_app(test_config());

    for body in [
        r#"{
          "claim_id": "CLM-VALUE-1",
          "investigation_id": "INV-VALUE-1",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "financial_impact_type": "prevented_payment",
          "saving_amount": "1000.00",
          "currency": "CNY",
          "notes": "Pre-payment review prevented improper payment.",
          "evidence_refs": ["investigation_results:INV-VALUE-1"]
        }"#,
        r#"{
          "claim_id": "CLM-VALUE-2",
          "investigation_id": "INV-VALUE-2",
          "outcome": "recovery_confirmed",
          "confirmed_fwa": true,
          "financial_impact_type": "recovered_amount",
          "saving_amount": "250.00",
          "currency": "CNY",
          "notes": "Post-payment recovery collected.",
          "evidence_refs": ["investigation_results:INV-VALUE-2"]
        }"#,
        r#"{
          "claim_id": "CLM-VALUE-3",
          "investigation_id": "INV-VALUE-3",
          "outcome": "provider_behavior_change_estimate",
          "confirmed_fwa": true,
          "financial_impact_type": "avoided_future_exposure",
          "saving_amount": "500.00",
          "currency": "CNY",
          "notes": "Estimated avoided future exposure from provider education.",
          "evidence_refs": ["investigation_results:INV-VALUE-3"]
        }"#,
    ] {
        let (status, _) =
            json_request(app.clone(), "POST", "/api/v1/investigations/results", body).await;
        assert_eq!(status, StatusCode::OK);
    }

    let (status, dashboard) = json_request(app, "GET", "/api/v1/ops/dashboard/summary", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        dashboard["value_measurement"]["prevented_payment"],
        "1000.00"
    );
    assert_eq!(dashboard["value_measurement"]["recovered_amount"], "250.00");
    assert_eq!(
        dashboard["value_measurement"]["avoided_future_exposure"],
        "500.00"
    );
    assert_eq!(dashboard["value_measurement"]["estimated_impact"], "500.00");
    assert_eq!(dashboard["value_measurement"]["review_cost"], "0.00");
    assert_eq!(dashboard["value_measurement"]["net_value"], "1750.00");
    assert!(dashboard["value_measurement"]["evidence_caveat"]
        .as_str()
        .unwrap()
        .contains("estimated"));
}

#[tokio::test]
async fn dashboard_summarizes_case_sla_metrics() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-SLA-1",
            "claim_amount": "9000",
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
              "unit_amount": "9000",
              "total_amount": "9000"
            }
          ],
          "member": {
            "external_member_id": "MBR-SLA-1"
          },
          "policy": {
            "external_policy_id": "POL-SLA-1",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-SLA-1",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "High"
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
        .find(|lead| lead["claim_id"] == "CLM-SLA-1")
        .unwrap()["lead_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "open_case",
          "assignee": "siu-reviewer-1",
          "reviewer": "medical-reviewer-1",
          "priority": "high",
          "notes": "Open SLA-tracked investigation.",
          "evidence_refs": ["triage_decisions:sla_dashboard"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, dashboard) = json_request(app, "GET", "/api/v1/ops/dashboard/summary", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(dashboard["case_sla"]["total_cases"], 1);
    assert_eq!(dashboard["case_sla"]["open_cases"], 1);
    assert_eq!(dashboard["case_sla"]["closed_cases"], 0);
    assert_eq!(dashboard["case_sla"]["breached_cases"], 0);
    assert_eq!(dashboard["case_sla"]["sla_breach_rate"], 0.0);
    assert_eq!(dashboard["case_sla"]["average_time_to_triage_hours"], 0.0);
    assert_eq!(dashboard["case_sla"]["average_time_to_closure_hours"], 0.0);
}

#[tokio::test]
async fn dashboard_summarizes_rule_governance_from_rule_performance() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-RULE-GOV-TRUE",
            "claim_amount": "8000",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "policy": {
            "external_policy_id": "POL-RULE-GOV-TRUE",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000"
          },
          "member": {
            "external_member_id": "MBR-RULE-GOV-TRUE"
          },
          "provider": {
            "external_provider_id": "PRV-RULE-GOV-TRUE",
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
            "external_claim_id": "CLM-RULE-GOV-FALSE",
            "claim_amount": "100",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "policy": {
            "external_policy_id": "POL-RULE-GOV-FALSE",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000"
          },
          "member": {
            "external_member_id": "MBR-RULE-GOV-FALSE"
          },
          "provider": {
            "external_provider_id": "PRV-RULE-GOV-FALSE",
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
          "claim_id": "CLM-RULE-GOV-TRUE",
          "investigation_id": "INV-RULE-GOV-TRUE",
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
          "claim_id": "CLM-RULE-GOV-FALSE",
          "investigation_id": "INV-RULE-GOV-FALSE",
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

    let (status, rules) = json_request(app.clone(), "GET", "/api/v1/ops/rules", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let rules = rules["rules"].as_array().unwrap();
    let active_rules = rules
        .iter()
        .filter(|rule| rule["status"] == "active")
        .count() as u64;

    let (status, performance) =
        json_request(app.clone(), "GET", "/api/v1/ops/rules/performance", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let performance = performance["rules"].as_array().unwrap();
    let total_trigger_count = performance
        .iter()
        .map(|rule| rule["trigger_count"].as_u64().unwrap())
        .sum::<u64>();
    let reviewed_count = performance
        .iter()
        .map(|rule| rule["reviewed_count"].as_u64().unwrap())
        .sum::<u64>();
    let confirmed_fwa_count = performance
        .iter()
        .map(|rule| rule["confirmed_fwa_count"].as_u64().unwrap())
        .sum::<u64>();
    let false_positive_count = performance
        .iter()
        .map(|rule| rule["false_positive_count"].as_u64().unwrap())
        .sum::<u64>();
    let saving_amount = performance
        .iter()
        .map(|rule| {
            rule["saving_amount"]
                .as_str()
                .unwrap()
                .parse::<Decimal>()
                .unwrap()
        })
        .sum::<Decimal>();

    let (status, dashboard) = json_request(app, "GET", "/api/v1/ops/dashboard/summary", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        dashboard["rule_governance"]["total_rules"],
        rules.len() as u64
    );
    assert_eq!(dashboard["rule_governance"]["active_rules"], active_rules);
    assert_eq!(
        dashboard["rule_governance"]["triggered_rules"],
        performance
            .iter()
            .filter(|rule| rule["trigger_count"].as_u64().unwrap() > 0)
            .count() as u64
    );
    assert_eq!(
        dashboard["rule_governance"]["total_trigger_count"],
        total_trigger_count
    );
    assert_eq!(
        dashboard["rule_governance"]["reviewed_count"],
        reviewed_count
    );
    assert_eq!(
        dashboard["rule_governance"]["confirmed_fwa_count"],
        confirmed_fwa_count
    );
    assert_eq!(
        dashboard["rule_governance"]["false_positive_count"],
        false_positive_count
    );
    assert_eq!(
        dashboard["rule_governance"]["precision"],
        confirmed_fwa_count as f64 / reviewed_count as f64
    );
    assert_eq!(
        dashboard["rule_governance"]["false_positive_rate"],
        false_positive_count as f64 / reviewed_count as f64
    );
    assert_eq!(
        dashboard["value_measurement"]["false_positive_operational_cost"],
        format!(
            "{:.2}",
            Decimal::from(false_positive_count * 100).round_dp(2)
        )
    );
    assert_eq!(
        dashboard["value_measurement"]["reviewer_capacity_hours"],
        format!(
            "{:.2}",
            Decimal::from(total_trigger_count) * Decimal::new(25, 2)
        )
    );
    assert_eq!(
        dashboard["rule_governance"]["saving_amount"],
        format!("{:.2}", saving_amount.round_dp(2))
    );
    assert!(dashboard["rule_governance"]["roi"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn dashboard_summary_requires_api_key() {
    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/ops/dashboard/summary")
        .body(Body::empty())
        .unwrap();
    let response = build_app(test_config()).oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
