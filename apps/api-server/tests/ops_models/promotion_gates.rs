use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{
    get_json, json_request, register_activation_candidate, register_model_dataset_for_test,
    register_unhealthy_model_dataset_for_test, test_config,
};

#[tokio::test]
async fn returns_model_promotion_gates_without_evaluation_evidence() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = get_json(app, "/api/v1/ops/models/baseline_fwa/promotion-gates").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["model_key"], "baseline_fwa");
    assert_eq!(body["model_version"], "0.1.0");
    assert_eq!(body["review_mode"], "both");
    assert_eq!(body["decision"], "routing_blocked");
    assert_eq!(body["latest_evaluation_id"], "none");
    assert_eq!(body["source_dataset_id"], "none");
    assert_eq!(body["source_data_quality_score"], serde_json::Value::Null);
    assert_eq!(body["source_data_quality_status"], "missing");
    assert_eq!(body["passed_count"], 2);
    assert_eq!(body["total_count"], 22);
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("dataset version missing")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("source data quality score missing")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("time/group split strategy missing")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("feature reproducibility hash missing")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("label provenance missing")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("model drift status unavailable")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("shadow comparison missing")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("serving version lock missing")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("artifact integrity missing")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "rust feature-set materialization missing"
        )));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("segment fairness review missing")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "rust serving artifact evaluation missing"
        )));
    let dataset_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Immutable dataset")
        .unwrap();
    assert_eq!(dataset_gate["evidence_source"], "missing");
}

#[tokio::test]
async fn model_promotion_gates_require_data_quality_and_label_provenance() {
    let app = build_app(test_config()).unwrap();
    let model_dataset_id = register_model_dataset_for_test(app.clone(), "quality_gate").await;

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_baseline_quality_gate",
              "model_key": "baseline_fwa",
              "model_version": "0.1.0",
              "model_dataset_id": "{model_dataset_id}",
              "scheme_family": "diagnosis_procedure_mismatch",
              "auc": "0.81",
              "ks": "0.42",
              "precision": "0.73",
              "recall": "0.68",
              "f1": "0.70",
              "accuracy": "0.74",
              "threshold": "0.50",
              "confusion_matrix_json": {{"tp": 10, "fp": 2, "tn": 12, "fn": 3}},
              "feature_importance_uri": "data/eval/claims_model_eval_quality_gate/v1/feature_importance.parquet",
              "metrics_json": {{
                "data_quality_score": 0.91,
                "feature_reproducibility_hash": "sha256:quality-gate-features",
                "label_provenance_status": "passed",
                "label_reviewer_source": "qa_review",
                "pilot_validation_status": "passed",
                "serving_version_lock_status": "passed",
                "artifact_integrity_status": "passed",
                "feature_store_materialization_status": "passed",
                "rust_feature_set_status": "passed",
                "rust_feature_set_manifest_uri": "s3://fwa-models/baseline_fwa/0.1.0/rust_feature_set/feature_set_manifest.json",
                "segment_fairness_status": "passed"
              }}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = get_json(app, "/api/v1/ops/models/baseline_fwa/promotion-gates").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["source_data_quality_score"], 1.0);
    assert_eq!(body["source_data_quality_status"], "ready");
    let source_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Source data quality")
        .unwrap();
    assert_eq!(source_gate["passed"], true);
    assert_eq!(source_gate["evidence_source"], "dataset");
    for label in [
        "Serving version lock",
        "Artifact integrity",
        "Feature materialization",
        "Segment fairness",
        "Feature reproducibility",
        "Label provenance",
    ] {
        let gate = body["gates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|gate| gate["label"] == label)
            .unwrap();
        assert_eq!(gate["passed"], true);
        assert_eq!(gate["evidence_source"], "evaluation");
    }
}

#[tokio::test]
async fn model_promotion_gates_require_rust_feature_set_evidence() {
    let app = build_app(test_config()).unwrap();
    let model_dataset_id = register_model_dataset_for_test(app.clone(), "rust_feature_set").await;

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_baseline_rust_feature_set",
              "model_key": "baseline_fwa",
              "model_version": "0.1.0",
              "model_dataset_id": "{model_dataset_id}",
              "scheme_family": "diagnosis_procedure_mismatch",
              "auc": "0.81",
              "ks": "0.42",
              "precision": "0.73",
              "recall": "0.68",
              "f1": "0.70",
              "accuracy": "0.74",
              "threshold": "0.50",
              "confusion_matrix_json": {{"tp": 10, "fp": 2, "tn": 12, "fn": 3}},
              "feature_importance_uri": "data/eval/claims_model_eval_rust_feature_set/v1/feature_importance.parquet",
              "metrics_json": {{
                "data_quality_score": 0.91,
                "feature_reproducibility_hash": "sha256:rust-feature-set-features",
                "label_provenance_status": "passed",
                "label_reviewer_source": "qa_review",
                "pilot_validation_status": "passed",
                "serving_version_lock_status": "passed",
                "artifact_integrity_status": "passed",
                "feature_store_materialization_status": "passed",
                "segment_fairness_status": "passed"
              }}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = get_json(app, "/api/v1/ops/models/baseline_fwa/promotion-gates").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "rust feature-set materialization missing"
        )));
    let gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Feature materialization")
        .unwrap();
    assert_eq!(gate["passed"], false);
    assert_eq!(gate["evidence_source"], "missing");
    assert_eq!(
        body["artifact_evidence"]["serving_manifest_uri"],
        serde_json::Value::Null
    );
    assert_eq!(
        body["artifact_evidence"]["rust_serving_status"],
        serde_json::Value::Null
    );
}

#[tokio::test]
async fn model_promotion_gates_require_rust_serving_artifact_evaluation() {
    let app = build_app(test_config()).unwrap();
    let model_dataset_id = register_model_dataset_for_test(app.clone(), "rust_serving_gate").await;

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_baseline_rust_serving_gate",
              "model_key": "baseline_fwa",
              "model_version": "0.1.0",
              "model_dataset_id": "{model_dataset_id}",
              "scheme_family": "diagnosis_procedure_mismatch",
              "auc": "0.81",
              "ks": "0.42",
              "precision": "0.73",
              "recall": "0.68",
              "f1": "0.70",
              "accuracy": "0.74",
              "threshold": "0.50",
              "confusion_matrix_json": {{"tp": 10, "fp": 2, "tn": 12, "fn": 3}},
              "feature_importance_uri": "data/eval/claims_model_eval_rust_serving_gate/v1/feature_importance.parquet",
              "metrics_json": {{
                "data_quality_score": 0.91,
                "out_of_time_auc": 0.79,
                "review_capacity_threshold_status": "passed",
                "leakage_check_status": "passed",
                "time_group_split_status": "passed",
                "time_split_field": "service_date",
                "group_split_fields": ["member_id", "policy_id", "provider_id"],
                "shadow_comparison_status": "passed",
                "serving_version_lock_status": "passed",
                "artifact_integrity_status": "passed",
                "feature_store_materialization_status": "passed",
                "rust_feature_set_status": "passed",
                "rust_feature_set_manifest_uri": "s3://fwa-models/baseline_fwa/0.1.0/rust_feature_set/feature_set_manifest.json",
                "segment_fairness_status": "passed",
                "feature_reproducibility_hash": "sha256:rust-serving-gate-features",
                "label_provenance_status": "passed",
                "label_reviewer_source": "qa_review",
                "pilot_validation_status": "passed"
              }}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = get_json(app, "/api/v1/ops/models/baseline_fwa/promotion-gates").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "rust serving artifact evaluation missing"
        )));
    let gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Rust serving evaluation")
        .unwrap();
    assert_eq!(gate["passed"], false);
    assert_eq!(gate["evidence_source"], "missing");
}

#[tokio::test]
async fn model_promotion_gates_require_time_group_split_strategy() {
    let app = build_app(test_config()).unwrap();
    let model_dataset_id = register_model_dataset_for_test(app.clone(), "split_strategy").await;

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_baseline_split_strategy",
              "model_key": "baseline_fwa",
              "model_version": "0.1.0",
              "model_dataset_id": "{model_dataset_id}",
              "scheme_family": "diagnosis_procedure_mismatch",
              "auc": "0.81",
              "ks": "0.42",
              "precision": "0.73",
              "recall": "0.68",
              "f1": "0.70",
              "accuracy": "0.74",
              "threshold": "0.50",
              "confusion_matrix_json": {{"tp": 10, "fp": 2, "tn": 12, "fn": 3}},
              "feature_importance_uri": "data/eval/claims_model_eval_split_strategy/v1/feature_importance.parquet",
              "metrics_json": {{
                "time_group_split_status": "passed",
                "time_split_field": "service_date",
                "group_split_fields": ["member_id", "policy_id", "provider_id"]
              }}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = get_json(app, "/api/v1/ops/models/baseline_fwa/promotion-gates").await;

    assert_eq!(status, StatusCode::OK);
    let split_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Time/group split strategy")
        .expect("model promotion gates should include time/group split strategy");
    assert_eq!(split_gate["passed"], true);
    assert_eq!(split_gate["evidence_source"], "evaluation");
    assert!(!body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("time/group split strategy missing")));
}

#[tokio::test]
async fn model_promotion_gates_block_public_research_dataset_evidence() {
    let app = build_app(test_config()).unwrap();
    let model_dataset_id = register_model_dataset_for_test(app.clone(), "public_research").await;

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_baseline_public_research",
              "model_key": "baseline_fwa",
              "model_version": "0.1.0",
              "model_dataset_id": "{model_dataset_id}",
              "scheme_family": "diagnosis_procedure_mismatch",
              "auc": "0.81",
              "ks": "0.42",
              "precision": "0.73",
              "recall": "0.68",
              "f1": "0.70",
              "accuracy": "0.74",
              "threshold": "0.50",
              "confusion_matrix_json": {{"tp": 10, "fp": 2, "tn": 12, "fn": 3}},
              "feature_importance_uri": "data/eval/claims_model_eval_public_research/v1/feature_importance.parquet",
              "metrics_json": {{
                "dataset_usage_scope": "public_kaggle_research",
                "time_group_split_status": "passed",
                "time_split_field": "service_date",
                "group_split_fields": ["member_id", "policy_id", "provider_id"],
                "review_capacity_threshold_status": "passed",
                "leakage_check_status": "passed",
                "shadow_comparison_status": "passed",
                "feature_reproducibility_hash": "sha256:public-research-features",
                "label_provenance_status": "passed",
                "label_reviewer_source": "kaggle_public_dataset",
                "approval_status": "approved",
                "out_of_time_auc": 0.79
              }}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = get_json(app, "/api/v1/ops/models/baseline_fwa/promotion-gates").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["decision"], "routing_blocked");
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("pilot/customer validation missing")));
    let validation_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Pilot/customer validation")
        .expect("model promotion gates should include pilot/customer validation");
    assert_eq!(validation_gate["passed"], false);
    assert_eq!(validation_gate["evidence_source"], "evaluation");
}

#[tokio::test]
async fn model_promotion_gates_block_unhealthy_source_dataset() {
    let app = build_app(test_config()).unwrap();
    let model_dataset_id =
        register_unhealthy_model_dataset_for_test(app.clone(), "unhealthy_source").await;

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_baseline_unhealthy_source",
              "model_key": "baseline_fwa",
              "model_version": "0.1.0",
              "model_dataset_id": "{model_dataset_id}",
              "scheme_family": "diagnosis_procedure_mismatch",
              "auc": "0.81",
              "ks": "0.42",
              "precision": "0.73",
              "recall": "0.68",
              "f1": "0.70",
              "accuracy": "0.74",
              "threshold": "0.50",
              "confusion_matrix_json": {{"tp": 10, "fp": 2, "tn": 12, "fn": 3}},
              "feature_importance_uri": "data/eval/claims_model_eval_unhealthy_source/v1/feature_importance.parquet",
              "metrics_json": {{
                "data_quality_score": 0.99,
                "feature_reproducibility_hash": "sha256:unhealthy-source-features",
                "label_provenance_status": "passed",
                "label_reviewer_source": "qa_review"
              }}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = get_json(app, "/api/v1/ops/models/baseline_fwa/promotion-gates").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["source_data_quality_score"], 0.6666666666666667);
    assert_eq!(body["source_data_quality_status"], "watch");
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "source dataset data quality below threshold"
        )));
    let source_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Source data quality")
        .unwrap();
    assert_eq!(source_gate["passed"], false);
    assert_eq!(source_gate["evidence_source"], "dataset");
    assert_eq!(
        source_gate["blocker"],
        "source dataset data quality below threshold"
    );
}

#[tokio::test]
async fn model_promotion_gates_include_label_governance_evidence() {
    let app = build_app(test_config()).unwrap();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-MODEL-LABEL-1",
          "investigation_id": "INV-MODEL-LABEL-1",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "1200.00",
          "currency": "CNY",
          "notes": "Confirmed FWA label ready for model evaluation.",
          "evidence_refs": ["investigation_results:INV-MODEL-LABEL-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-MODEL-LABEL-1",
          "claim_id": "CLM-MODEL-LABEL-2",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "model_under_scored_confirmed_issue",
          "feedback_target": "model",
          "notes": "Needs model-governance review before training use.",
          "evidence_refs": ["qa_reviews:QA-MODEL-LABEL-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-MODEL-LABEL-1/status",
        r#"{
          "status": "in_progress",
          "actor_id": "model-ops",
          "notes": "Model operator accepted the feedback for review.",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-MODEL-LABEL-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/promotion-gates",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let label_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Label governance")
        .expect("model promotion gates should include label governance");
    assert_eq!(label_gate["passed"], false);
    assert_eq!(label_gate["evidence_source"], "labels");
    assert_eq!(label_gate["blocker"], "model outcome labels need review");
    let closure_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Model QA feedback closure")
        .expect("model promotion gates should include QA feedback closure");
    assert_eq!(closure_gate["passed"], false);
    assert_eq!(closure_gate["evidence_source"], "qa_feedback");
    assert_eq!(closure_gate["blocker"], "unresolved model QA feedback");
    assert_eq!(body["open_model_feedback_count"], 0);
    assert_eq!(body["unresolved_model_feedback_count"], 1);
    assert_eq!(body["approved_label_count"], 1);
    assert_eq!(body["needs_review_label_count"], 1);
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("model outcome labels need review")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("unresolved model QA feedback")));

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-MODEL-LABEL-1/status",
        r#"{
          "status": "resolved",
          "actor_id": "model-ops",
          "notes": "Model operator approved the label after review.",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-MODEL-LABEL-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = get_json(app, "/api/v1/ops/models/baseline_fwa/promotion-gates").await;
    assert_eq!(status, StatusCode::OK);
    let label_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Label governance")
        .unwrap();
    assert_eq!(label_gate["passed"], true);
    let closure_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Model QA feedback closure")
        .unwrap();
    assert_eq!(closure_gate["passed"], true);
    assert_eq!(body["unresolved_model_feedback_count"], 0);
    assert_eq!(body["approved_label_count"], 2);
    assert_eq!(body["needs_review_label_count"], 0);
    assert!(!body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("model outcome labels need review")));
    assert!(!body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("unresolved model QA feedback")));
}

#[tokio::test]
async fn model_promotion_gates_ignore_feedback_and_labels_for_other_model_versions() {
    let app = build_app(test_config()).unwrap();
    let candidate_version = register_activation_candidate(app.clone()).await;

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-MODEL-OTHER-VERSION-1",
          "claim_id": "CLM-MODEL-OTHER-VERSION-1",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "model_under_scored_confirmed_issue",
          "feedback_target": "model",
          "notes": "Feedback applies to the currently active baseline, not the candidate.",
          "evidence_refs": [
            "qa_reviews:QA-MODEL-OTHER-VERSION-1",
            "model_versions:baseline_fwa:0.1.0"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = get_json(app, "/api/v1/ops/models/baseline_fwa/promotion-gates").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["model_version"], candidate_version);
    assert_eq!(body["unresolved_model_feedback_count"], 0);
    assert_eq!(body["needs_review_label_count"], 0);
    let closure_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Model QA feedback closure")
        .expect("model promotion gates should include QA feedback closure");
    assert_eq!(closure_gate["passed"], true);
    let label_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Label governance")
        .expect("model promotion gates should include label governance");
    assert_eq!(label_gate["passed"], true);
}
