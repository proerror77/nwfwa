use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{get_json, json_request, register_model_dataset_for_test, test_config};

#[tokio::test]
async fn blocks_model_retraining_job_when_readiness_is_blocked() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/models/baseline_fwa/retraining-jobs",
        r#"{
          "requested_by": "model-ops",
          "notes": "Queue retraining from model drift."
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "MODEL_RETRAINING_NOT_READY");
}

#[tokio::test]
async fn queues_updates_and_completes_model_retraining_job_from_readiness() {
    let app = build_app(test_config()).unwrap();
    let model_dataset_id = register_model_dataset_for_test(app.clone(), "retraining_job").await;

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_baseline_retraining_job",
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
              "feature_importance_uri": "data/eval/claims_model_eval_retraining_job/v1/feature_importance.parquet",
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
          "claim_id": "CLM-RETRAINING-JOB-1",
          "investigation_id": "INV-RETRAINING-JOB-1",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "1200.00",
          "currency": "CNY",
          "notes": "Confirmed FWA label ready for retraining job.",
          "evidence_refs": ["investigation_results:INV-RETRAINING-JOB-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/retraining-jobs",
        r#"{
          "requested_by": "model-ops",
          "notes": " "
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_RETRAINING_JOB_NOTES");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/retraining-jobs",
        r#"{
          "requested_by": "model-ops",
          "notes": "Queue retraining after contacting alice@example.com."
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_MODEL_RETRAINING_JOB");

    let (status, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/retraining-jobs",
        r#"{
          "requested_by": "model-ops",
          "notes": "Queue retraining from score drift."
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(created["model_key"], "baseline_fwa");
    assert_eq!(created["status"], "queued");
    assert_eq!(created["requested_by"], "model-ops");
    assert_eq!(created["readiness_recommendation"], "prepare_retraining");
    assert!(created["trigger_summary"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("score drift status: drift")));
    let job_id = created["job_id"].as_str().unwrap();

    let (status, jobs) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/retraining-jobs",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(jobs["jobs"][0]["job_id"], job_id);

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/claim-next",
        r#"{
          "actor": "trainer-worker",
          "model_key": "baseline_fwa",
          "notes": " "
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_RETRAINING_JOB_NOTES");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/claim-next",
        r#"{
          "actor": "trainer-worker",
          "model_key": "baseline_fwa",
          "notes": "Worker called 13800138000 before claiming the job."
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_MODEL_RETRAINING_JOB");

    let (status, updated) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/claim-next",
        r#"{
          "actor": "trainer-worker",
          "model_key": "baseline_fwa",
          "notes": "Training worker picked up the job."
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["job_id"], job_id);
    assert_eq!(updated["status"], "running");
    assert_eq!(updated["updated_by"], "trainer-worker");
    assert_eq!(updated["status_note"], "Training worker picked up the job.");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/claim-next",
        r#"{
          "actor": "trainer-worker",
          "model_key": "baseline_fwa",
          "notes": "No work expected."
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "MODEL_RETRAINING_JOB_NOT_FOUND");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/model-retraining-jobs/{job_id}/status"),
        r#"{
          "status": "validation",
          "actor": "trainer-worker",
          "notes": " "
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_RETRAINING_JOB_NOTES");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/model-retraining-jobs/{job_id}/status"),
        r#"{
          "status": "validation",
          "actor": "trainer-worker",
          "notes": "Validation notes include ID 11010519491231002X."
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_MODEL_RETRAINING_JOB");

    let (status, updated) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/model-retraining-jobs/{job_id}/status"),
        r#"{
          "status": "validation",
          "actor": "trainer-worker",
          "notes": "Validation metrics are ready."
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["status"], "validation");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/model-retraining-jobs/{job_id}/status"),
        r#"{
          "status": "completed",
          "actor": "trainer-worker",
          "notes": "Attempt to close the job without registering external training output."
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_RETRAINING_JOB_STATUS");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/model-retraining-jobs/{job_id}/output"),
        r#"{
          "actor": "trainer-worker",
          "notes": " ",
          "candidate_model_version": "0.2.0-candidate",
          "artifact_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/model.onnx",
          "validation_report_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
          "evaluation_run_id": "eval_baseline_retraining_job_candidate",
          "evidence_refs": [
            "model_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/model.onnx",
            "model_validation_reports:s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
            "model_evaluations:eval_baseline_retraining_job_candidate"
          ],
          "confusion_matrix_json": {"tp": 12, "fp": 2, "tn": 14, "fn": 2},
          "metrics_json": {"score_psi": 0.04}
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_RETRAINING_OUTPUT_NOTES");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/model-retraining-jobs/{job_id}/output"),
        r#"{
          "actor": "trainer-worker",
          "notes": "Candidate report sent to alice@example.com.",
          "candidate_model_version": "0.2.0-candidate",
          "artifact_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/model.onnx",
          "validation_report_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
          "evaluation_run_id": "eval_baseline_retraining_job_candidate",
          "evidence_refs": [
            "model_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/model.onnx",
            "model_validation_reports:s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
            "model_evaluations:eval_baseline_retraining_job_candidate"
          ],
          "confusion_matrix_json": {"tp": 12, "fp": 2, "tn": 14, "fn": 2},
          "metrics_json": {"score_psi": 0.04}
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_MODEL_RETRAINING_JOB");

    let (status, completed) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/model-retraining-jobs/{job_id}/output"),
        &format!(
            r#"{{
          "actor": "trainer-worker",
          "notes": "Candidate model and validation report registered.",
          "candidate_model_version": "0.2.0-candidate",
          "artifact_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/rust_serving_artifact.json",
          "artifact_sha256": "sha256:rust-serving-artifact",
          "training_artifact_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/model.joblib",
          "training_artifact_sha256": "sha256:training-artifact",
          "serving_manifest_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/serving_manifest.json",
          "endpoint_url": "http://127.0.0.1:8001/score/baseline_fwa/0.2.0-candidate",
          "validation_report_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
          "evaluation_run_id": "eval_baseline_retraining_job_candidate",
          "evidence_refs": [
            "model_retraining_jobs:{job_id}",
            "model_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/rust_serving_artifact.json",
            "model_training_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/model.joblib",
          "model_serving_manifests:s3://fwa-models/baseline_fwa/0.2.0-candidate/serving_manifest.json",
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
          "confusion_matrix_json": {{"tp": 12, "fp": 2, "tn": 14, "fn": 2}},
          "feature_importance_uri": "data/eval/claims_model_eval_retraining_job_candidate/v1/feature_importance.parquet",
          "permutation_importance_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/permutation_importance.parquet",
          "metrics_json": {{
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
            "shadow_comparison_status": "passed",
            "review_capacity_threshold_status": "passed",
            "model_artifact_evaluation_status": "passed",
            "model_artifact_evaluation_report_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/artifact-evaluation/model_artifact_evaluation_report.json",
            "rule_candidate_backtest_status": "passed",
            "rule_candidate_backtest_report_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_report.json",
            "rule_candidate_review_tasks_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json",
            "rule_library_writeback_status": "blocked_pending_human_review_and_policy_governance_approval"
          }},
          "mined_rule_candidates": [
            {{
              "rule_id": "candidate_retraining_amount_ratio",
              "version": 1,
              "name": "Retraining mined amount ratio candidate",
              "review_mode": "both",
              "scheme_family": "high_risk_claim",
              "conditions": [
                {{
                  "field": "claim_amount_to_limit_ratio",
                  "operator": ">=",
                  "value": 0.82
                }}
              ],
              "action": {{
                "score": 22,
                "alert_code": "RETRAINING_AMOUNT_RATIO_CANDIDATE",
                "recommended_action": "ManualReview",
                "reason": "External training platform mined this explainable candidate from feature importance and backtest evidence."
              }}
            }},
            {{
              "rule_id": "candidate_tree_provider_profile_and_amount",
              "version": 1,
              "name": "Decision tree mined provider profile and amount candidate",
              "review_mode": "both",
              "scheme_family": "high_risk_claim",
              "conditions": [
                {{
                  "field": "provider_profile_score",
                  "operator": ">",
                  "value": 47.5
                }},
                {{
                  "field": "claim_amount_to_limit_ratio",
                  "operator": "<",
                  "value": 0.56
                }},
                {{
                  "field": "provider_region",
                  "operator": "in",
                  "value": ["SH", "BJ"]
                }}
              ],
              "action": {{
                "score": 34,
                "alert_code": "TREE_MINED_PROVIDER_AMOUNT",
                "recommended_action": "ManualReview",
                "reason": "External training platform mined this shallow decision-tree path from training data. Human review is required."
              }}
            }}
          ]
        }}"#
        )
        .as_str(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(completed["job"]["status"], "completed");
    assert_eq!(
        completed["job"]["candidate_model_version"],
        "0.2.0-candidate"
    );
    assert_eq!(
        completed["job"]["output_evaluation_id"],
        "eval_baseline_retraining_job_candidate"
    );
    assert_eq!(completed["candidate_model"]["status"], "candidate");
    assert_eq!(
        completed["candidate_model"]["artifact_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0-candidate/rust_serving_artifact.json"
    );
    assert_eq!(completed["evaluation"]["model_version"], "0.2.0-candidate");
    assert_eq!(
        completed["evaluation"]["permutation_importance_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0-candidate/permutation_importance.parquet"
    );
    assert_eq!(
        completed["evaluation"]["metrics_json"]["training_artifact_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0-candidate/model.joblib"
    );
    assert_eq!(
        completed["evaluation"]["metrics_json"]["training_artifact_sha256"],
        "sha256:training-artifact"
    );
    assert_eq!(
        completed["evaluation"]["metrics_json"]["serving_manifest_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0-candidate/serving_manifest.json"
    );
    assert_eq!(
        completed["evaluation"]["metrics_json"]["permutation_importance_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0-candidate/permutation_importance.parquet"
    );
    assert_eq!(
        completed["mined_rule_candidates"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        completed["mined_rule_candidates"][0]["summary"]["rule_id"],
        "candidate_retraining_amount_ratio"
    );
    assert_eq!(
        completed["mined_rule_candidates"][0]["summary"]["status"],
        "draft"
    );
    assert_eq!(
        completed["mined_rule_candidates"][0]["summary"]["owner"],
        "external-training-platform"
    );
    assert_eq!(
        completed["mined_rule_candidates"][1]["summary"]["rule_id"],
        "candidate_tree_provider_profile_and_amount"
    );
    assert_eq!(
        completed["mined_rule_candidates"][1]["summary"]["status"],
        "draft"
    );

    let (status, audit) = get_json(
        app.clone(),
        "/api/v1/ops/audit-events?event_type=model.retraining.output_registered&limit=5",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let output_event = &audit["events"][0];
    assert!(output_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(format!(
            "model_retraining_jobs:{job_id}"
        ))));
    assert!(output_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
        "model_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/rust_serving_artifact.json"
    )));
    assert!(output_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "model_training_artifacts:s3://fwa-models/baseline_fwa/0.2.0-candidate/model.joblib"
        )));
    assert!(output_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "model_serving_manifests:s3://fwa-models/baseline_fwa/0.2.0-candidate/serving_manifest.json"
        )));
    assert!(output_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "model_permutation_importance:s3://fwa-models/baseline_fwa/0.2.0-candidate/permutation_importance.parquet"
        )));
    assert!(output_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "model_overfitting_diagnostics:s3://fwa-models/baseline_fwa/0.2.0-candidate/overfitting_diagnostics_report.json"
        )));
    assert!(output_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "model_validation_reports:s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json"
        )));
    assert!(output_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "model_evaluations:eval_baseline_retraining_job_candidate"
        )));
    assert_eq!(
        output_event["payload"]["mined_rule_candidate_count"],
        serde_json::json!(2)
    );
    assert_eq!(
        output_event["payload"]["training_boundary"],
        "external training platform completed model training and rule mining; FWA recorded candidate artifacts and rule drafts only"
    );

    let (status, rules) = get_json(app.clone(), "/api/v1/ops/rules").await;
    assert_eq!(status, StatusCode::OK);
    let saved_rule = rules["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["rule_id"] == "candidate_retraining_amount_ratio")
        .expect("training-platform mined rule candidate should be saved");
    assert_eq!(saved_rule["status"], "draft");
    let saved_tree_rule = rules["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["rule_id"] == "candidate_tree_provider_profile_and_amount")
        .expect("training-platform tree-mined rule candidate should be saved");
    assert_eq!(saved_tree_rule["status"], "draft");

    let (status, gates) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/versions/0.2.0-candidate/promotion-gates",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        gates["artifact_evidence"]["permutation_importance_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0-candidate/permutation_importance.parquet"
    );

    let (status, models) = get_json(app, "/api/v1/ops/models").await;
    assert_eq!(status, StatusCode::OK);
    assert!(models["models"]
        .as_array()
        .unwrap()
        .iter()
        .any(|model| model["version"] == "0.2.0-candidate" && model["status"] == "candidate"));
}
