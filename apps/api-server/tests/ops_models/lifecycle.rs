use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{
    activate_candidate_for_test, get_json, json_request, model_lifecycle_payload,
    register_activation_candidate, register_model_dataset_for_test,
    submit_probability_calibration_report_for_test, test_config,
};

#[tokio::test]
async fn blocks_model_promotion_when_score_drift_is_detected() {
    let app = build_app(test_config()).unwrap();
    let model_dataset_id = register_model_dataset_for_test(app.clone(), "drift_gate").await;

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_baseline_drift_gate",
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
              "feature_importance_uri": "s3://fwa-models/baseline_fwa/0.1.0/drift_gate/feature_importance.parquet",
              "metrics_json": {{
                "approval_status": "approved",
                "leakage_check_status": "passed",
                "out_of_time_auc": 0.79,
                "review_capacity_threshold_status": "passed",
                "score_psi": 0.31,
                "shadow_comparison_status": "passed"
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
        .contains(&serde_json::json!("model drift detected")));
    let drift_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Drift status")
        .unwrap();
    assert_eq!(drift_gate["passed"], false);
    assert_eq!(drift_gate["evidence_source"], "evaluation");
}

#[tokio::test]
async fn version_scoped_promotion_gates_use_candidate_drift_metrics() {
    let app = build_app(test_config()).unwrap();
    let candidate_version = register_activation_candidate(app.clone()).await;
    let model_dataset_id = register_model_dataset_for_test(app.clone(), "candidate_drift").await;

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "zzzz_baseline_active_drift_after_candidate",
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
              "feature_importance_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/candidate_drift/feature_importance.parquet",
              "metrics_json": {{"score_psi": 0.31}}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, gates) = get_json(
        app,
        &format!("/api/v1/ops/models/baseline_fwa/versions/{candidate_version}/promotion-gates"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(gates["model_version"], candidate_version);
    let drift_gate = gates["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Drift status")
        .unwrap();
    assert_eq!(drift_gate["passed"], true);
    assert_eq!(drift_gate["evidence_source"], "evaluation");
    assert!(!gates["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("model drift detected")));
}

#[tokio::test]
async fn records_model_promotion_review_and_uses_it_for_approval_gate() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/promotion-reviews",
        r#"{
          "decision": "approved",
          "reviewer": "model-governance",
          "notes": " ",
          "evidence_refs": ["model_versions:baseline_fwa:0.1.0"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_PROMOTION_REVIEW_NOTES");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/promotion-reviews",
        r#"{
          "decision": "approved",
          "reviewer": "model-governance",
          "notes": "Approved for continued shadow evaluation only.",
          "evidence_refs": []
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_PROMOTION_REVIEW_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/promotion-reviews",
        r#"{
          "decision": "approved",
          "reviewer": "model-governance",
          "notes": "Approved for continued shadow evaluation only.",
          "evidence_refs": ["model_versions:baseline_fwa:0.0.1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_TARGET_MODEL_VERSION_EVIDENCE");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("model_versions:baseline_fwa:0.1.0"));

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/promotion-reviews",
        r#"{
          "decision": "approved",
          "reviewer": "model-governance",
          "notes": "Approved for continued shadow evaluation only.",
          "evidence_refs": ["phone:13800138000"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_PROMOTION_REVIEW");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/promotion-reviews",
        r#"{
          "decision": "approved",
          "reviewer": "model-governance",
          "notes": "Approved for continued shadow evaluation only.",
          "evidence_refs": [
            "model_versions:baseline_fwa:0.1.0",
            "model_promotion_reviews:local://template/model-promotion-review.json"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_PROMOTION_REVIEW_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/promotion-reviews",
        r#"{
          "decision": "approved",
          "reviewer": "model-governance",
          "notes": "Approved for continued shadow evaluation only.",
          "evidence_refs": [
            "model_versions:baseline_fwa:0.1.0",
            "model_promotion_reviews:file://tmp/model-promotion-review.json"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_PROMOTION_REVIEW_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/promotion-reviews",
        r#"{
          "decision": "approved",
          "reviewer": "model-governance",
          "notes": "Approved for continued shadow evaluation only.",
          "evidence_refs": ["model_versions:baseline_fwa:0.1.0", " "]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_PROMOTION_REVIEW_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/promotion-reviews",
        r#"{
          "decision": "approved",
          "reviewer": "model-governance",
          "notes": "Approved for continued shadow evaluation only.",
          "evidence_refs": ["model_versions:baseline_fwa:0.1.0"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["model_key"], "baseline_fwa");
    assert_eq!(body["model_version"], "0.1.0");
    assert_eq!(body["decision"], "approved");
    assert_eq!(body["reviewer"], "model-governance");
    assert_eq!(
        body["evidence_refs"][0],
        "model_versions:baseline_fwa:0.1.0"
    );

    let (status, body) = get_json(app, "/api/v1/ops/models/baseline_fwa/promotion-gates").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["decision"], "routing_blocked");
    assert_eq!(body["passed_count"], 3);
    assert!(!body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("approval missing")));
    let approval_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Approval")
        .unwrap();
    assert_eq!(approval_gate["passed"], true);
    assert_eq!(approval_gate["evidence_source"], "approval");
}

#[tokio::test]
async fn blocks_model_activation_when_promotion_gates_are_blocked() {
    let app = build_app(test_config()).unwrap();
    let candidate_version = register_activation_candidate(app.clone()).await;

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/models/baseline_fwa/activate",
        &model_lifecycle_payload("baseline_fwa", &candidate_version),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "MODEL_PROMOTION_GATES_BLOCKED");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("approval missing"));
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains(&candidate_version));
}

#[tokio::test]
async fn activates_candidate_model_after_promotion_gates_pass() {
    let app = build_app(test_config()).unwrap();
    let candidate_version = register_activation_candidate(app.clone()).await;

    let (status, review) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/promotion-reviews",
        r#"{
          "decision": "approved",
          "reviewer": "model-governance",
          "notes": "Approved candidate for production activation.",
          "evidence_refs": ["model_versions:baseline_fwa:0.0.1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(review["code"], "MISSING_TARGET_MODEL_VERSION_EVIDENCE");

    let (status, review) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/promotion-reviews",
        &format!(
            r#"{{
              "decision": "approved",
              "reviewer": "model-governance",
              "notes": "Approved candidate for production activation.",
              "evidence_refs": ["model_versions:baseline_fwa:{candidate_version}"]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(review["model_version"], candidate_version);

    let (status, gates) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/promotion-gates",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        gates["artifact_evidence"]["serving_manifest_uri"],
        format!("s3://fwa-models/baseline_fwa/{candidate_version}/serving_manifest.json")
    );
    assert_eq!(
        gates["artifact_evidence"]["model_artifact_evaluation_report_uri"],
        format!(
            "s3://fwa-models/baseline_fwa/{candidate_version}/artifact-evaluation/model_artifact_evaluation_report.json"
        )
    );
    assert_eq!(gates["artifact_evidence"]["rust_serving_status"], "passed");
    assert_eq!(
        gates["artifact_evidence"]["rust_serving_latency_status"],
        "passed"
    );
    assert_eq!(
        gates["artifact_evidence"]["rust_serving_p95_latency_ms"],
        17
    );
    assert_eq!(
        gates["artifact_evidence"]["rust_serving_latency_measurement_kind"],
        "simulated_fixture"
    );
    assert_eq!(
        gates["artifact_evidence"]["rust_serving_latency_sample_count"],
        0
    );

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/activate",
        r#"{"evidence_refs": []}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_MODEL_LIFECYCLE_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/activate",
        r#"{"evidence_refs": [" "]}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_MODEL_LIFECYCLE_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/activate",
        r#"{"evidence_refs": ["phone:13800138000"]}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_MODEL_LIFECYCLE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/activate",
        r#"{"evidence_refs": ["model_versions:baseline_fwa:0.1.0", "model_activation:{candidate_version}"]}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_LIFECYCLE_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/activate",
        r#"{"evidence_refs": ["model_versions:baseline_fwa:0.1.0", "model_activation:file://tmp/activation.json"]}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_LIFECYCLE_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/activate",
        &model_lifecycle_payload("baseline_fwa", "0.1.0"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_TARGET_MODEL_VERSION_EVIDENCE");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains(&candidate_version));

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/activate",
        &model_lifecycle_payload("baseline_fwa", &candidate_version),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "MODEL_PROMOTION_GATES_BLOCKED");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("probability calibration missing"));

    submit_probability_calibration_report_for_test(app.clone(), &candidate_version).await;

    let (status, activated) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/activate",
        &model_lifecycle_payload("baseline_fwa", &candidate_version),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(activated["model_key"], "baseline_fwa");
    assert_eq!(activated["version"], candidate_version);
    assert_eq!(activated["status"], "active");

    let (status, models) = get_json(app.clone(), "/api/v1/ops/models").await;
    assert_eq!(status, StatusCode::OK);
    assert!(models["models"]
        .as_array()
        .unwrap()
        .iter()
        .any(|model| model["version"] == candidate_version && model["status"] == "active"));
    assert!(models["models"]
        .as_array()
        .unwrap()
        .iter()
        .any(|model| model["version"] == "0.1.0" && model["status"] == "approved"));

    let (status, audit) = get_json(
        app.clone(),
        "/api/v1/ops/audit-events?event_type=model.activation.completed&limit=1",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        audit["events"][0]["payload"]["customer_scope_id"],
        "demo-customer"
    );

    let (status, gates) = get_json(app, "/api/v1/ops/models/baseline_fwa/promotion-gates").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(gates["decision"], "routing_allowed");
    assert!(gates["blockers"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn nonpassing_probability_calibration_blocks_model_activation() {
    let app = build_app(test_config()).unwrap();
    let candidate_version = register_activation_candidate(app.clone()).await;

    let (status, review) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/promotion-reviews",
        &format!(
            r#"{{
              "decision": "approved",
              "reviewer": "model-governance",
              "notes": "Approved exact candidate version pending calibration evidence.",
              "evidence_refs": ["model_versions:baseline_fwa:{candidate_version}"]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(review["model_version"], candidate_version);

    let (status, calibration) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/probability-calibration-reports",
        &format!(
            r#"{{
              "actor": "worker:build-probability-calibration-report",
              "notes": "non-passing holdout calibration evidence",
              "report_uri": "s3://customer-prod-artifacts/model-artifacts/baseline_fwa/{candidate_version}/calibration/probability_calibration_report.json",
              "report_kind": "probability_calibration_report",
              "model_version": "{candidate_version}",
              "as_of_date": "2026-06-14",
              "row_count": 100,
              "minimum_calibration_rows": 100,
              "bin_count": 2,
              "expected_calibration_error": 0.08,
              "max_expected_calibration_error": 0.05,
              "brier_score": 0.18,
              "max_brier_score": 0.20,
              "calibration_status": "needs_calibration_review",
              "bins": [
                {{
                  "bin_index": 0,
                  "lower_bound": 0.0,
                  "upper_bound": 0.5,
                  "row_count": 50,
                  "average_predicted_probability": 0.2,
                  "observed_positive_rate": 0.12,
                  "calibration_error": 0.08
                }},
                {{
                  "bin_index": 1,
                  "lower_bound": 0.5,
                  "upper_bound": 1.0,
                  "row_count": 50,
                  "average_predicted_probability": 0.82,
                  "observed_positive_rate": 0.74,
                  "calibration_error": 0.08
                }}
              ],
              "review_tasks": [
                {{
                  "task_kind": "probability_calibration_review",
                  "severity": "blocker",
                  "summary": "ECE exceeds calibrated probability activation threshold."
                }}
              ],
              "evidence_refs": [
                "model_versions:baseline_fwa:{candidate_version}",
                "probability_calibration_reports:s3://customer-prod-artifacts/model-artifacts/baseline_fwa/{candidate_version}/calibration/probability_calibration_report.json",
                "probability_calibration_input:s3://customer-prod-artifacts/baseline_fwa/{candidate_version}/calibration/holdout-predictions.json",
                "calibration_labels:s3://customer-prod-artifacts/baseline_fwa/{candidate_version}/calibration/holdout-labels.json"
              ],
              "governance_boundary": "calibration report is evidence only; it must not relabel outcomes, rewrite model probabilities, change routing thresholds, or activate calibrated serving"
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        calibration["calibration_status"],
        "needs_calibration_review"
    );
    assert_eq!(calibration["active_calibration_change"], false);
    assert_eq!(
        calibration["calibrated_probability_serving_activation"],
        false
    );

    let gates_uri =
        format!("/api/v1/ops/models/baseline_fwa/versions/{candidate_version}/promotion-gates");
    let (status, gates) = get_json(app.clone(), &gates_uri).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(gates["decision"], "routing_blocked");
    assert!(gates["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("probability calibration failed")));

    let activate_uri =
        format!("/api/v1/ops/models/baseline_fwa/versions/{candidate_version}/activate");
    let (status, body) = json_request(
        app,
        "POST",
        &activate_uri,
        &model_lifecycle_payload("baseline_fwa", &candidate_version),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "MODEL_PROMOTION_GATES_BLOCKED");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("probability calibration failed"));
}

#[tokio::test]
async fn model_promotion_and_activation_are_version_scoped() {
    let app = build_app(test_config()).unwrap();
    let candidate_version = register_activation_candidate(app.clone()).await;

    let review_uri =
        format!("/api/v1/ops/models/baseline_fwa/versions/{candidate_version}/promotion-reviews");
    let (status, review) = json_request(
        app.clone(),
        "POST",
        &review_uri,
        &format!(
            r#"{{
              "decision": "approved",
              "reviewer": "model-governance",
              "notes": "Approved exact candidate version for production activation.",
              "evidence_refs": ["model_versions:baseline_fwa:{candidate_version}"]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(review["model_key"], "baseline_fwa");
    assert_eq!(review["model_version"], candidate_version);

    let gates_uri =
        format!("/api/v1/ops/models/baseline_fwa/versions/{candidate_version}/promotion-gates");
    let (status, gates) = get_json(app.clone(), &gates_uri).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(gates["model_key"], "baseline_fwa");
    assert_eq!(gates["model_version"], candidate_version);
    let approval_gate = gates["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Approval")
        .unwrap();
    assert_eq!(approval_gate["passed"], true);
    assert_eq!(approval_gate["evidence_source"], "approval");

    submit_probability_calibration_report_for_test(app.clone(), &candidate_version).await;

    let activate_uri =
        format!("/api/v1/ops/models/baseline_fwa/versions/{candidate_version}/activate");
    let (status, activated) = json_request(
        app,
        "POST",
        &activate_uri,
        &model_lifecycle_payload("baseline_fwa", &candidate_version),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(activated["model_key"], "baseline_fwa");
    assert_eq!(activated["version"], candidate_version);
    assert_eq!(activated["status"], "active");
}

#[tokio::test]
async fn rolls_back_active_model_version() {
    let app = build_app(test_config()).unwrap();
    let candidate_version = register_activation_candidate(app.clone()).await;
    activate_candidate_for_test(app.clone(), &candidate_version).await;

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/rollback",
        &model_lifecycle_payload("baseline_fwa", &candidate_version),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["model_key"], "baseline_fwa");
    assert_eq!(body["version"], "0.1.0");
    assert_eq!(body["status"], "active");

    let (status, body) = get_json(app.clone(), "/api/v1/ops/models").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["models"]
        .as_array()
        .unwrap()
        .iter()
        .any(|model| model["version"] == "0.1.0" && model["status"] == "active"));
    assert!(body["models"]
        .as_array()
        .unwrap()
        .iter()
        .any(|model| model["version"] == candidate_version && model["status"] == "approved"));

    let (status, audit) = get_json(
        app.clone(),
        "/api/v1/ops/audit-events?event_type=model.rollback.completed&limit=5",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        audit["events"][0]["evidence_refs"][0],
        format!("model_versions:baseline_fwa:{candidate_version}")
    );
    assert_eq!(
        audit["events"][0]["payload"]["replaced_active_version"],
        candidate_version
    );
    assert_eq!(
        audit["events"][0]["payload"]["previous_active_version"],
        "0.1.0"
    );
    assert_eq!(
        audit["events"][0]["payload"]["customer_scope_id"],
        "demo-customer"
    );
}

#[tokio::test]
async fn rollback_requires_active_model_evidence_ref() {
    let app = build_app(test_config()).unwrap();
    let candidate_version = register_activation_candidate(app.clone()).await;
    activate_candidate_for_test(app.clone(), &candidate_version).await;

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/rollback",
        &model_lifecycle_payload("baseline_fwa", "0.1.0"),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_TARGET_MODEL_VERSION_EVIDENCE");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains(&format!("model_versions:baseline_fwa:{candidate_version}")));
}

#[tokio::test]
async fn rollback_can_restore_replaced_active_version_from_rollback_history() {
    let app = build_app(test_config()).unwrap();
    let candidate_version = register_activation_candidate(app.clone()).await;
    activate_candidate_for_test(app.clone(), &candidate_version).await;

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/rollback",
        &model_lifecycle_payload("baseline_fwa", &candidate_version),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["version"], "0.1.0");
    assert_eq!(body["status"], "active");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/rollback",
        &model_lifecycle_payload("baseline_fwa", "0.1.0"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["version"], candidate_version);
    assert_eq!(body["status"], "active");

    let (status, audit) = get_json(
        app,
        "/api/v1/ops/audit-events?event_type=model.rollback.completed&limit=1",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        audit["events"][0]["payload"]["previous_active_version"],
        candidate_version
    );
    assert_eq!(
        audit["events"][0]["payload"]["replaced_active_version"],
        "0.1.0"
    );
}

#[tokio::test]
async fn rollback_uses_lifecycle_history_when_non_lifecycle_governance_events_exceed_window() {
    let app = build_app(test_config()).unwrap();
    let candidate_version = register_activation_candidate(app.clone()).await;
    activate_candidate_for_test(app.clone(), &candidate_version).await;
    let model_dataset_id = register_model_dataset_for_test(app.clone(), "rollback_window").await;

    for index in 0..105 {
        let (status, body) = json_request(
            app.clone(),
            "POST",
            "/api/v1/ops/model-evaluations",
            &format!(
                r#"{{
                  "evaluation_run_id": "eval_rollback_window_{index}",
                  "model_key": "baseline_fwa",
                  "model_version": "{candidate_version}",
                  "model_dataset_id": "{model_dataset_id}",
                  "scheme_family": "diagnosis_procedure_mismatch",
                  "auc": "0.86",
                  "ks": "0.48",
                  "precision": "0.78",
                  "recall": "0.71",
                  "f1": "0.74",
                  "accuracy": "0.79",
                  "threshold": "0.52",
                  "confusion_matrix_json": {{"tp": 12, "fp": 2, "tn": 14, "fn": 2}},
                  "feature_importance_uri": "s3://fwa-models/baseline_fwa/rollback_window/{index}/feature_importance.parquet",
                  "metrics_json": {{"score_psi": 0.04}}
                }}"#
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/models/baseline_fwa/rollback",
        &model_lifecycle_payload("baseline_fwa", &candidate_version),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["version"], "0.1.0");
    assert_eq!(body["status"], "active");
}

#[tokio::test]
async fn rejects_rollback_to_active_model_when_newer_candidate_exists() {
    let app = build_app(test_config()).unwrap();
    let candidate_version = register_activation_candidate(app.clone()).await;

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/rollback",
        &model_lifecycle_payload("baseline_fwa", "0.1.0"),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "MODEL_ROLLBACK_TARGET_NOT_FOUND");

    let (status, models) = get_json(app, "/api/v1/ops/models").await;
    assert_eq!(status, StatusCode::OK);
    assert!(models["models"]
        .as_array()
        .unwrap()
        .iter()
        .any(|model| { model["version"] == candidate_version && model["status"] == "candidate" }));
    assert!(models["models"]
        .as_array()
        .unwrap()
        .iter()
        .any(|model| model["version"] == "0.1.0" && model["status"] == "active"));
}

#[tokio::test]
async fn blocks_model_rollback_without_approved_target() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/models/baseline_fwa/rollback",
        &model_lifecycle_payload("baseline_fwa", "0.1.0"),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "MODEL_ROLLBACK_TARGET_NOT_FOUND");
}
