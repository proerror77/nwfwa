use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{json_request, test_config};

#[tokio::test]
async fn rejects_invalid_model_retraining_output_contract() {
    let app = build_app(test_config());
    let valid_request = serde_json::json!({
        "actor": "trainer-worker",
        "notes": "Candidate model and validation report registered.",
        "candidate_model_version": "0.2.0-candidate",
        "artifact_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/model.onnx",
        "artifact_sha256": "sha256:serving-artifact",
        "training_artifact_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/model.joblib",
        "training_artifact_sha256": "sha256:training-artifact",
        "endpoint_url": "http://127.0.0.1:8001/score/baseline_fwa/0.2.0-candidate",
        "validation_report_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
        "evaluation_run_id": "eval_baseline_retraining_job_candidate",
        "evidence_refs": [
          "model_retraining_jobs:job_1",
          "model_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/model.onnx",
          "model_training_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/model.joblib",
          "model_artifact_evaluations:s3://fwa-models/baseline_fwa/0.2.0-candidate/artifact-evaluation/model_artifact_evaluation_report.json",
          "model_feature_importance:data/eval/claims_model_eval_retraining_job_candidate/v1/feature_importance.parquet",
          "model_permutation_importance:s3://fwa-models/baseline_fwa/0.2.0-candidate/permutation_importance.parquet",
          "automl_factor_rankings:s3://fwa-models/baseline_fwa/0.2.0-candidate/automl_factor_ranking_report.json",
          "model_overfitting_diagnostics:s3://fwa-models/baseline_fwa/0.2.0-candidate/overfitting_diagnostics_report.json",
          "model_validation_reports:s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
          "model_evaluations:eval_baseline_retraining_job_candidate",
          "rule_candidate_backtests:s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_report.json",
          "rule_candidate_review_tasks:s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json"
        ],
        "auc": "0.86",
        "ks": "0.48",
        "precision": "0.78",
        "recall": "0.71",
        "f1": "0.74",
        "accuracy": "0.79",
        "threshold": "0.52",
        "confusion_matrix_json": {"tp": 12, "fp": 2, "tn": 14, "fn": 2},
        "feature_importance_uri": "data/eval/claims_model_eval_retraining_job_candidate/v1/feature_importance.parquet",
        "permutation_importance_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/permutation_importance.parquet",
        "metrics_json": {
          "out_of_time_auc": 0.82,
          "out_of_time_precision": 0.76,
          "out_of_time_recall": 0.71,
          "score_psi": 0.04,
          "max_feature_psi": 0.08,
          "time_group_split_status": "passed",
          "time_split_field": "service_date",
          "group_split_fields": ["member_id", "policy_id", "provider_id"],
          "leakage_check_status": "passed",
          "out_of_time_validation_status": "passed",
          "score_stability_status": "passed",
          "feature_stability_status": "passed",
          "automl_factor_ranking_status": "passed",
          "automl_factor_ranking_report_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/automl_factor_ranking_report.json",
          "overfitting_diagnostics_status": "passed",
          "overfitting_diagnostics_report_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/overfitting_diagnostics_report.json",
          "feature_reproducibility_hash": "sha256:retraining-feature-set",
          "model_artifact_evaluation_status": "passed",
          "model_artifact_evaluation_report_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/artifact-evaluation/model_artifact_evaluation_report.json",
          "rule_candidate_backtest_status": "passed",
          "rule_candidate_backtest_report_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_report.json",
          "rule_candidate_review_tasks_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json",
          "rule_library_writeback_status": "blocked_pending_human_review_and_policy_governance_approval"
        }
    });

    let mut invalid_metric = valid_request.clone();
    invalid_metric["threshold"] = serde_json::json!("1.01");
    let payload = invalid_metric.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_RETRAINING_OUTPUT_METRIC");

    let mut blank_endpoint = valid_request.clone();
    blank_endpoint["endpoint_url"] = serde_json::json!(" ");
    let payload = blank_endpoint.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_RETRAINING_OUTPUT_ENDPOINT");

    let mut unsupported_endpoint = valid_request.clone();
    unsupported_endpoint["endpoint_url"] =
        serde_json::json!("s3://fwa-models/baseline_fwa/0.2.0-candidate/score");
    let payload = unsupported_endpoint.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_RETRAINING_OUTPUT_ENDPOINT");

    let mut empty_confusion_matrix = valid_request.clone();
    empty_confusion_matrix["confusion_matrix_json"] = serde_json::json!({});
    let payload = empty_confusion_matrix.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_RETRAINING_OUTPUT_CONFUSION_MATRIX");

    let mut empty_metrics = valid_request.clone();
    empty_metrics["metrics_json"] = serde_json::json!({});
    let payload = empty_metrics.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_RETRAINING_OUTPUT_METRICS");

    let mut missing_artifact_evaluation = valid_request.clone();
    missing_artifact_evaluation["metrics_json"]["model_artifact_evaluation_status"] =
        serde_json::json!("missing");
    let payload = missing_artifact_evaluation.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_RETRAINING_OUTPUT_ARTIFACT_EVALUATION"
    );

    let mut missing_rule_backtest = valid_request.clone();
    missing_rule_backtest["metrics_json"]["rule_candidate_backtest_status"] =
        serde_json::json!("missing");
    let payload = missing_rule_backtest.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_RETRAINING_OUTPUT_RULE_CANDIDATE_WORKFLOW"
    );

    let mut unsafe_rule_writeback = valid_request.clone();
    unsafe_rule_writeback["metrics_json"]["rule_library_writeback_status"] =
        serde_json::json!("ready_for_writeback");
    let payload = unsafe_rule_writeback.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_RETRAINING_OUTPUT_RULE_CANDIDATE_WORKFLOW"
    );

    let mut csv_feature_importance = valid_request.clone();
    csv_feature_importance["feature_importance_uri"] =
        serde_json::json!("data/eval/feature_importance.csv");
    let payload = csv_feature_importance.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_RETRAINING_OUTPUT_FEATURE_IMPORTANCE");

    let mut txt_feature_importance = valid_request.clone();
    txt_feature_importance["feature_importance_uri"] =
        serde_json::json!("data/eval/feature_importance.txt");
    let payload = txt_feature_importance.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_RETRAINING_OUTPUT_FEATURE_IMPORTANCE");

    let mut invalid_training_artifact = valid_request.clone();
    invalid_training_artifact["training_artifact_uri"] =
        serde_json::json!("s3://fwa-models/baseline_fwa/0.2.0-candidate/model.csv");
    invalid_training_artifact["evidence_refs"] = serde_json::json!([
        "model_retraining_jobs:job_1",
        "model_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/model.onnx",
        "model_training_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/model.csv",
        "model_validation_reports:s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
        "model_evaluations:eval_baseline_retraining_job_candidate"
    ]);
    let payload = invalid_training_artifact.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_TRAINING_ARTIFACT_URI");

    let mut invalid_training_artifact_sha = valid_request.clone();
    invalid_training_artifact_sha["training_artifact_sha256"] = serde_json::json!("not-sha");
    let payload = invalid_training_artifact_sha.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_TRAINING_ARTIFACT_SHA256");

    let mut missing_training_artifact_evidence = valid_request.clone();
    missing_training_artifact_evidence["evidence_refs"] = serde_json::json!([
        "model_retraining_jobs:job_1",
        "model_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/model.onnx",
        "model_validation_reports:s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
        "model_evaluations:eval_baseline_retraining_job_candidate"
    ]);
    let payload = missing_training_artifact_evidence.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_RETRAINING_OUTPUT_EVIDENCE");

    let mut invalid_serving_manifest = valid_request.clone();
    invalid_serving_manifest["serving_manifest_uri"] =
        serde_json::json!("s3://fwa-models/baseline_fwa/0.2.0-candidate/serving_manifest.csv");
    invalid_serving_manifest["evidence_refs"] = serde_json::json!([
        "model_retraining_jobs:job_1",
        "model_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/model.onnx",
        "model_training_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/model.joblib",
        "model_serving_manifests:s3://fwa-models/baseline_fwa/0.2.0-candidate/serving_manifest.csv",
        "model_validation_reports:s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
        "model_evaluations:eval_baseline_retraining_job_candidate"
    ]);
    let payload = invalid_serving_manifest.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_SERVING_MANIFEST_URI");

    let mut missing_serving_manifest_evidence = valid_request.clone();
    missing_serving_manifest_evidence["serving_manifest_uri"] =
        serde_json::json!("s3://fwa-models/baseline_fwa/0.2.0-candidate/serving_manifest.json");
    let payload = missing_serving_manifest_evidence.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_RETRAINING_OUTPUT_EVIDENCE");

    let mut serving_manifest_evidence = valid_request.clone();
    serving_manifest_evidence["serving_manifest_uri"] =
        serde_json::json!("s3://fwa-models/baseline_fwa/0.2.0-candidate/serving_manifest.json");
    serving_manifest_evidence["evidence_refs"] = serde_json::json!([
        "model_retraining_jobs:job_1",
        "model_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/model.onnx",
        "model_training_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/model.joblib",
        "serving_manifests:s3://fwa-models/baseline_fwa/0.2.0-candidate/serving_manifest.json",
        "model_artifact_evaluations:s3://fwa-models/baseline_fwa/0.2.0-candidate/artifact-evaluation/model_artifact_evaluation_report.json",
        "model_feature_importance:data/eval/claims_model_eval_retraining_job_candidate/v1/feature_importance.parquet",
        "model_permutation_importance:s3://fwa-models/baseline_fwa/0.2.0-candidate/permutation_importance.parquet",
        "automl_factor_rankings:s3://fwa-models/baseline_fwa/0.2.0-candidate/automl_factor_ranking_report.json",
        "model_overfitting_diagnostics:s3://fwa-models/baseline_fwa/0.2.0-candidate/overfitting_diagnostics_report.json",
        "model_validation_reports:s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
        "model_evaluations:eval_baseline_retraining_job_candidate",
        "rule_candidate_backtests:s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_report.json",
        "rule_candidate_review_tasks:s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json"
    ]);
    let payload = serving_manifest_evidence.to_string();
    let (status, _body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_ne!(status, StatusCode::BAD_REQUEST);

    let mut missing_evidence_refs = valid_request.clone();
    missing_evidence_refs["evidence_refs"] = serde_json::json!([]);
    let payload = missing_evidence_refs.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_RETRAINING_OUTPUT_EVIDENCE");

    let mut missing_permutation_importance_evidence = valid_request.clone();
    missing_permutation_importance_evidence["evidence_refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| {
            reference.as_str()
                != Some("model_permutation_importance:s3://fwa-models/baseline_fwa/0.2.0-candidate/permutation_importance.parquet")
        });
    let payload = missing_permutation_importance_evidence.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_RETRAINING_OUTPUT_EVIDENCE");

    let mut missing_feature_importance_evidence = valid_request.clone();
    missing_feature_importance_evidence["evidence_refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| {
            reference.as_str()
                != Some("model_feature_importance:data/eval/claims_model_eval_retraining_job_candidate/v1/feature_importance.parquet")
        });
    let payload = missing_feature_importance_evidence.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_RETRAINING_OUTPUT_EVIDENCE");

    let mut missing_factor_ranking = valid_request.clone();
    missing_factor_ranking
        .as_object_mut()
        .unwrap()
        .remove("feature_importance_uri");
    let payload = missing_factor_ranking.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_RETRAINING_OUTPUT_OVERFITTING_EVIDENCE"
    );

    let mut missing_out_of_time_metric = valid_request.clone();
    missing_out_of_time_metric["metrics_json"]
        .as_object_mut()
        .unwrap()
        .remove("out_of_time_auc");
    let payload = missing_out_of_time_metric.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_RETRAINING_OUTPUT_OVERFITTING_EVIDENCE"
    );

    let mut failed_leakage_gate = valid_request.clone();
    failed_leakage_gate["metrics_json"]["leakage_check_status"] = serde_json::json!("failed");
    let payload = failed_leakage_gate.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_RETRAINING_OUTPUT_OVERFITTING_EVIDENCE"
    );

    let mut missing_factor_ranking_report = valid_request.clone();
    missing_factor_ranking_report["metrics_json"]
        .as_object_mut()
        .unwrap()
        .remove("automl_factor_ranking_report_uri");
    let payload = missing_factor_ranking_report.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_RETRAINING_OUTPUT_OVERFITTING_EVIDENCE"
    );

    let mut missing_overfitting_diagnostics = valid_request.clone();
    missing_overfitting_diagnostics["metrics_json"]
        .as_object_mut()
        .unwrap()
        .remove("overfitting_diagnostics_report_uri");
    let payload = missing_overfitting_diagnostics.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_RETRAINING_OUTPUT_OVERFITTING_EVIDENCE"
    );

    let mut missing_overfitting_diagnostics_evidence = valid_request.clone();
    missing_overfitting_diagnostics_evidence["evidence_refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| {
            reference.as_str()
                != Some("model_overfitting_diagnostics:s3://fwa-models/baseline_fwa/0.2.0-candidate/overfitting_diagnostics_report.json")
        });
    let payload = missing_overfitting_diagnostics_evidence.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_RETRAINING_OUTPUT_EVIDENCE");

    let mut failed_overfitting_diagnostics = valid_request.clone();
    failed_overfitting_diagnostics["metrics_json"]["overfitting_diagnostics_status"] =
        serde_json::json!("failed");
    let payload = failed_overfitting_diagnostics.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_RETRAINING_OUTPUT_OVERFITTING_EVIDENCE"
    );

    let mut missing_feature_stability = valid_request.clone();
    missing_feature_stability["metrics_json"]
        .as_object_mut()
        .unwrap()
        .remove("max_feature_psi");
    let payload = missing_feature_stability.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_RETRAINING_OUTPUT_OVERFITTING_EVIDENCE"
    );

    let mut pii_evidence_refs = valid_request.clone();
    pii_evidence_refs["evidence_refs"] =
        serde_json::json!(["model_artifacts:s3://fwa-models/alice@example.com/model.onnx"]);
    let payload = pii_evidence_refs.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_MODEL_RETRAINING_JOB");

    let mut csv_model_artifact = valid_request.clone();
    csv_model_artifact["artifact_uri"] =
        serde_json::json!("s3://fwa-models/baseline_fwa/report.csv");
    let payload = csv_model_artifact.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_ARTIFACT_URI");

    let mut rust_json_model_artifact = valid_request.clone();
    rust_json_model_artifact["artifact_uri"] = serde_json::json!(
        "s3://fwa-models/baseline_fwa/0.2.0-candidate/rust_serving_artifact.json"
    );
    rust_json_model_artifact["evidence_refs"] = serde_json::json!([
        "model_retraining_jobs:job_1",
        "model_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/rust_serving_artifact.json",
        "model_training_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/model.joblib",
        "model_artifact_evaluations:s3://fwa-models/baseline_fwa/0.2.0-candidate/artifact-evaluation/model_artifact_evaluation_report.json",
        "model_feature_importance:data/eval/claims_model_eval_retraining_job_candidate/v1/feature_importance.parquet",
        "model_permutation_importance:s3://fwa-models/baseline_fwa/0.2.0-candidate/permutation_importance.parquet",
        "automl_factor_rankings:s3://fwa-models/baseline_fwa/0.2.0-candidate/automl_factor_ranking_report.json",
        "model_overfitting_diagnostics:s3://fwa-models/baseline_fwa/0.2.0-candidate/overfitting_diagnostics_report.json",
        "model_validation_reports:s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
        "model_evaluations:eval_baseline_retraining_job_candidate",
        "rule_candidate_backtests:s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_report.json",
        "rule_candidate_review_tasks:s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json"
    ]);
    let payload = rust_json_model_artifact.to_string();
    let (status, _body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_ne!(status, StatusCode::BAD_REQUEST);

    let mut unsupported_model_artifact = valid_request.clone();
    unsupported_model_artifact["artifact_uri"] =
        serde_json::json!("s3://fwa-models/baseline_fwa/model.txt");
    unsupported_model_artifact["evidence_refs"] = serde_json::json!([
        "model_retraining_jobs:job_1",
        "model_artifacts:s3://fwa-models/baseline_fwa/model.txt",
        "model_validation_reports:s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
        "model_evaluations:eval_baseline_retraining_job_candidate"
    ]);
    let payload = unsupported_model_artifact.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_ARTIFACT_URI");

    let mut csv_validation_report = valid_request.clone();
    csv_validation_report["validation_report_uri"] =
        serde_json::json!("s3://fwa-models/baseline_fwa/validation.csv");
    let payload = csv_validation_report.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_VALIDATION_REPORT_URI");

    let mut unsupported_validation_report = valid_request.clone();
    unsupported_validation_report["validation_report_uri"] =
        serde_json::json!("s3://fwa-models/baseline_fwa/validation.txt");
    unsupported_validation_report["evidence_refs"] = serde_json::json!([
        "model_retraining_jobs:job_1",
        "model_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/model.onnx",
        "model_validation_reports:s3://fwa-models/baseline_fwa/validation.txt",
        "model_evaluations:eval_baseline_retraining_job_candidate"
    ]);
    let payload = unsupported_validation_report.to_string();
    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_VALIDATION_REPORT_URI");
}
