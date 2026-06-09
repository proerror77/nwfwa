use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{get_json, json_request, register_model_dataset_for_test, test_config};

#[tokio::test]
async fn model_retraining_readiness_blocks_without_training_inputs() {
    let app = build_app(test_config());

    let (status, body) =
        get_json(app, "/api/v1/ops/models/baseline_fwa/retraining-readiness").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["model_key"], "baseline_fwa");
    assert_eq!(body["model_version"], "0.1.0");
    assert_eq!(body["recommendation"], "blocked");
    assert_eq!(body["latest_evaluation_id"], "none");
    assert_eq!(body["drift_status"], "not_available");
    assert_eq!(body["source_dataset_id"], "none");
    assert_eq!(body["source_data_quality_score"], serde_json::Value::Null);
    assert_eq!(body["source_data_quality_status"], "missing");
    assert_eq!(body["open_model_feedback_count"], 0);
    assert_eq!(body["approved_label_count"], 0);
    assert_eq!(body["needs_review_label_count"], 0);
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("latest model evaluation missing")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("source data quality score missing")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("approved model outcome labels missing")));
}

#[tokio::test]
async fn model_retraining_readiness_ignores_feedback_and_labels_for_other_model_versions() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-RETRAINING-OTHER-VERSION-1",
          "claim_id": "CLM-RETRAINING-OTHER-VERSION-1",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "model_under_scored_confirmed_issue",
          "feedback_target": "model",
          "notes": "Feedback belongs to an older model version only.",
          "evidence_refs": [
            "qa_reviews:QA-RETRAINING-OTHER-VERSION-1",
            "model_versions:baseline_fwa:0.0.1"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-RETRAINING-OTHER-VERSION-LABEL-1",
          "investigation_id": "INV-RETRAINING-OTHER-VERSION-LABEL-1",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "1200.00",
          "currency": "CNY",
          "notes": "Confirmed FWA label belongs to an older model version only.",
          "evidence_refs": [
            "investigation_results:INV-RETRAINING-OTHER-VERSION-LABEL-1",
            "model_versions:baseline_fwa:0.0.1"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) =
        get_json(app, "/api/v1/ops/models/baseline_fwa/retraining-readiness").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["model_version"], "0.1.0");
    assert_eq!(body["open_model_feedback_count"], 0);
    assert_eq!(body["approved_label_count"], 0);
    assert_eq!(body["needs_review_label_count"], 0);
    assert!(!body["retraining_triggers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("open model QA feedback")));
    assert!(!body["retraining_triggers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("approved model labels available")));
}

#[tokio::test]
async fn model_retraining_readiness_prepares_when_drift_and_labels_are_ready() {
    let app = build_app(test_config());
    let model_dataset_id = register_model_dataset_for_test(app.clone(), "retraining_ready").await;

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_baseline_retraining_ready",
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
              "feature_importance_uri": "data/eval/claims_model_eval_retraining_ready/v1/feature_importance.parquet",
              "metrics_json": {{"score_psi": 0.31}}
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
          "claim_id": "CLM-RETRAINING-LABEL-1",
          "investigation_id": "INV-RETRAINING-LABEL-1",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "1200.00",
          "currency": "CNY",
          "notes": "Confirmed FWA label ready for retraining.",
          "evidence_refs": ["investigation_results:INV-RETRAINING-LABEL-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) =
        get_json(app, "/api/v1/ops/models/baseline_fwa/retraining-readiness").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["recommendation"], "prepare_retraining");
    assert_eq!(
        body["latest_evaluation_id"],
        "eval_baseline_retraining_ready"
    );
    assert_eq!(body["drift_status"], "drift");
    assert_eq!(body["source_data_quality_score"], 1.0);
    assert_eq!(body["source_data_quality_status"], "ready");
    assert_eq!(body["approved_label_count"], 1);
    assert_eq!(body["needs_review_label_count"], 0);
    assert_eq!(body["blockers"].as_array().unwrap().len(), 0);
    assert!(body["retraining_triggers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("score drift status: drift")));
    assert!(body["retraining_triggers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("approved model labels available")));
}
