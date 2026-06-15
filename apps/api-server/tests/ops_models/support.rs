use api_server::config::AppConfig;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

pub(crate) fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        api_key_principals: vec![],
        source_system: "tpa-demo".into(),
        database_url: "postgres://unused".into(),
        model_service_url: "heuristic://local".into(),
        object_storage_uri: "local://demo-artifacts".into(),
        customer_scope_id: "demo-customer".into(),
        retention_policy_id: "demo-retention-policy".into(),
        backup_restore_plan_id: "demo-backup-restore-plan".into(),
        pii_masking_policy_id: "demo-pii-masking-policy".into(),
        key_rotation_policy_id: "demo-key-rotation-policy".into(),
        network_allowlist_id: "demo-network-allowlist".into(),
        alert_routing_policy_id: "demo-alert-routing-policy".into(),
        observability_exporter_endpoint: "local://demo-observability".into(),
        agent_policy_id: "demo-agent-policy".into(),
    }
}

pub(crate) fn restricted_test_config(permissions: &[&str]) -> (AppConfig, String) {
    let restricted_key = "restricted-test-key";
    let permission_spec = permissions.join(",");
    let principal_spec = format!(
        "{}|restricted-actor|fwa_operator|tpa-demo|demo-customer|{}",
        restricted_key, permission_spec
    );
    let config = AppConfig {
        api_key: "dev-secret".into(),
        api_key_principals: vec![principal_spec],
        source_system: "tpa-demo".into(),
        database_url: "postgres://unused".into(),
        model_service_url: "heuristic://local".into(),
        object_storage_uri: "local://demo-artifacts".into(),
        customer_scope_id: "demo-customer".into(),
        retention_policy_id: "demo-retention-policy".into(),
        backup_restore_plan_id: "demo-backup-restore-plan".into(),
        pii_masking_policy_id: "demo-pii-masking-policy".into(),
        key_rotation_policy_id: "demo-key-rotation-policy".into(),
        network_allowlist_id: "demo-network-allowlist".into(),
        alert_routing_policy_id: "demo-alert-routing-policy".into(),
        observability_exporter_endpoint: "local://demo-observability".into(),
        agent_policy_id: "demo-agent-policy".into(),
    };
    (config, restricted_key.into())
}

pub(crate) async fn get_json(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}));
    (status, body)
}

pub(crate) async fn json_request(
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

pub(crate) fn model_lifecycle_payload(model_key: &str, version: &str) -> String {
    format!(r#"{{"evidence_refs":["model_versions:{model_key}:{version}"]}}"#)
}

pub(crate) async fn register_model_dataset_for_test(app: axum::Router, suffix: &str) -> String {
    register_active_model_dataset_for_test(app, suffix).await
}

pub(crate) async fn register_draft_model_dataset_for_test(
    app: axum::Router,
    suffix: &str,
) -> String {
    register_model_dataset_for_test_with_profiles(
        app,
        suffix,
        "draft",
        "data/eval",
        r#"{"missing_rate": 0.0, "psi": 0.01, "owner": "model-ops"}"#,
        r#"{"allowed_values": [0, 1], "missing_rate": 0.0, "psi": 0.01, "owner": "model-ops"}"#,
        r#"{"missing_rate": 0.0, "psi": 0.01, "owner": "model-ops"}"#,
    )
    .await
}

pub(crate) async fn register_active_model_dataset_for_test(
    app: axum::Router,
    suffix: &str,
) -> String {
    register_model_dataset_for_test_with_profiles(
        app,
        suffix,
        "active",
        "s3://fwa-model-data",
        r#"{"missing_rate": 0.0, "psi": 0.01, "owner": "model-ops"}"#,
        r#"{"allowed_values": [0, 1], "missing_rate": 0.0, "psi": 0.01, "owner": "model-ops"}"#,
        r#"{"missing_rate": 0.0, "psi": 0.01, "owner": "model-ops"}"#,
    )
    .await
}

pub(crate) async fn register_unhealthy_model_dataset_for_test(
    app: axum::Router,
    suffix: &str,
) -> String {
    register_model_dataset_for_test_with_profiles(
        app,
        suffix,
        "active",
        "s3://fwa-model-data",
        "{}",
        r#"{"allowed_values": [0, 1]}"#,
        "{}",
    )
    .await
}

async fn register_model_dataset_for_test_with_profiles(
    app: axum::Router,
    suffix: &str,
    status: &str,
    uri_prefix: &str,
    key_profile: &str,
    label_profile: &str,
    feature_profile: &str,
) -> String {
    let (_, dataset) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        &format!(
            r#"{{
              "source_key": "claims_model_eval_{suffix}",
              "display_name": "Claims Model Eval {suffix}",
              "business_domain": "fwa_claims",
              "owner": "model-ops",
              "description": "Evaluation dataset for model governance.",
              "dataset_key": "claims_model_eval_{suffix}",
              "dataset_version": "v1",
              "sample_grain": "claim",
              "label_column": "confirmed_fwa",
              "entity_keys": ["claim_id"],
              "manifest_uri": "{uri_prefix}/claims_model_eval_{suffix}/v1/manifest.json",
              "schema_uri": "{uri_prefix}/claims_model_eval_{suffix}/v1/schema.json",
              "profile_uri": "{uri_prefix}/claims_model_eval_{suffix}/v1/profile.json",
              "storage_format": "parquet",
              "schema_hash": "sha256:model-{suffix}",
              "row_count": 100,
              "status": "{status}",
              "splits": [
                {{
                  "split_name": "validation",
                  "data_uri": "{uri_prefix}/claims_model_eval_{suffix}/v1/split=validation/",
                  "row_count": 100,
                  "positive_count": 25,
                  "negative_count": 75,
                  "label_distribution_json": {{"1": 25, "0": 75}}
                }}
              ],
              "fields": [
                {{
                  "field_name": "claim_id",
                  "logical_type": "string",
                  "nullable": false,
                  "semantic_role": "key",
                  "description": "Claim id.",
                  "profile_json": {key_profile}
                }},
                {{
                  "field_name": "confirmed_fwa",
                  "logical_type": "int8",
                  "nullable": false,
                  "semantic_role": "label",
                  "description": "Confirmed FWA label.",
                  "profile_json": {label_profile}
                }},
                {{
                  "field_name": "claim_amount_to_limit_ratio",
                  "logical_type": "float64",
                  "nullable": false,
                  "semantic_role": "feature",
                  "description": "Claim amount to policy limit ratio.",
                  "profile_json": {feature_profile}
                }}
              ]
            }}"#
        ),
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
              "feature_set_key": "claims_features_{suffix}",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "{uri_prefix}/claims_model_eval_{suffix}/v1/features/",
              "feature_list_json": ["claim_amount_to_limit_ratio"],
              "row_count": 100,
              "label_column": "confirmed_fwa",
              "status": "{status}"
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
              "train_uri": "{uri_prefix}/claims_model_eval_{suffix}/v1/split=train/",
              "validation_uri": "{uri_prefix}/claims_model_eval_{suffix}/v1/split=validation/",
              "test_uri": null,
              "row_counts_json": {{"train": 80, "validation": 20}},
              "label_distribution_json": {{"train": {{"1": 20, "0": 60}}, "validation": {{"1": 5, "0": 15}}}},
              "status": "{status}"
            }}"#
        ),
    )
    .await;
    model_dataset["model_dataset_id"]
        .as_str()
        .unwrap()
        .to_string()
}

pub(crate) async fn register_activation_candidate(app: axum::Router) -> String {
    let model_dataset_id = register_model_dataset_for_test(app.clone(), "activation").await;

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_baseline_activation_base",
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
              "feature_importance_uri": "s3://fwa-models/baseline_fwa/0.1.0/feature_importance.parquet",
              "metrics_json": {{"score_psi": 0.31}}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/investigations/results",
        r#"{
          "claim_id": "CLM-ACTIVATION-LABEL-1",
          "investigation_id": "INV-ACTIVATION-LABEL-1",
          "outcome": "confirmed_fwa",
          "confirmed_fwa": true,
          "saving_amount": "1200.00",
          "currency": "CNY",
          "notes": "Confirmed FWA label ready for model activation.",
          "evidence_refs": ["investigation_results:INV-ACTIVATION-LABEL-1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/retraining-jobs",
        r#"{
          "requested_by": "model-ops",
          "notes": "Queue retraining for activation candidate."
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let job_id = created["job_id"].as_str().unwrap();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-retraining-jobs/claim-next",
        r#"{
          "actor": "trainer-worker",
          "model_key": "baseline_fwa",
          "notes": "Training worker picked up the activation job."
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
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

    let candidate_version = "0.2.0-activation";
    let (status, completed) = json_request(
        app,
        "POST",
        &format!("/api/v1/ops/model-retraining-jobs/{job_id}/output"),
        &format!(
            r#"{{
              "actor": "trainer-worker",
              "notes": "Candidate model and validation report registered.",
              "candidate_model_version": "{candidate_version}",
              "artifact_uri": "s3://fwa-models/baseline_fwa/{candidate_version}/model.onnx",
              "endpoint_url": "http://127.0.0.1:8001/score/baseline_fwa/{candidate_version}",
              "validation_report_uri": "s3://fwa-models/baseline_fwa/{candidate_version}/validation.json",
              "evaluation_run_id": "eval_baseline_activation_candidate",
              "evidence_refs": [
                "model_retraining_jobs:{job_id}",
                "model_artifacts:s3://fwa-models/baseline_fwa/{candidate_version}/model.onnx",
                "model_validation_reports:s3://fwa-models/baseline_fwa/{candidate_version}/validation.json",
                "model_artifact_evaluations:s3://fwa-models/baseline_fwa/{candidate_version}/artifact-evaluation/model_artifact_evaluation_report.json",
                "model_feature_importance:s3://fwa-models/baseline_fwa/{candidate_version}/feature_importance.parquet",
                "model_permutation_importance:s3://fwa-models/baseline_fwa/{candidate_version}/permutation_importance.parquet",
                "automl_factor_rankings:s3://fwa-models/baseline_fwa/{candidate_version}/automl_factor_ranking_report.json",
                "model_overfitting_diagnostics:s3://fwa-models/baseline_fwa/{candidate_version}/overfitting_diagnostics_report.json",
                "rule_candidate_backtests:s3://fwa-models/baseline_fwa/{candidate_version}/rule-candidates/backtest/rule_candidate_backtest_report.json",
                "rule_candidate_review_tasks:s3://fwa-models/baseline_fwa/{candidate_version}/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json",
                "model_evaluations:eval_baseline_activation_candidate"
              ],
              "auc": "0.86",
              "ks": "0.48",
              "precision": "0.78",
              "recall": "0.71",
              "f1": "0.74",
              "accuracy": "0.79",
              "threshold": "0.52",
              "confusion_matrix_json": {{"tp": 12, "fp": 2, "tn": 14, "fn": 2}},
              "feature_importance_uri": "s3://fwa-models/baseline_fwa/{candidate_version}/feature_importance.parquet",
              "permutation_importance_uri": "s3://fwa-models/baseline_fwa/{candidate_version}/permutation_importance.parquet",
              "metrics_json": {{
                "score_psi": 0.04,
                "max_feature_psi": 0.03,
                "score_stability_status": "passed",
                "feature_stability_status": "passed",
                "out_of_time_auc": 0.84,
                "out_of_time_precision": 0.77,
                "out_of_time_recall": 0.70,
                "out_of_time_validation_status": "passed",
                "review_capacity_threshold_status": "passed",
                "overfitting_diagnostics_status": "passed",
                "overfitting_diagnostics_report_uri": "s3://fwa-models/baseline_fwa/{candidate_version}/overfitting_diagnostics_report.json",
                "automl_factor_ranking_status": "passed",
                "automl_factor_ranking_report_uri": "s3://fwa-models/baseline_fwa/{candidate_version}/automl_factor_ranking_report.json",
                "leakage_check_status": "passed",
                "time_group_split_status": "passed",
                "time_split_field": "service_date",
                "group_split_fields": ["member_id", "policy_id", "provider_id"],
                "shadow_comparison_status": "passed",
                "serving_version_lock_status": "passed",
                "artifact_integrity_status": "passed",
                "feature_store_materialization_status": "passed",
                "rust_feature_set_status": "passed",
                "rust_feature_set_manifest_uri": "s3://fwa-models/baseline_fwa/{candidate_version}/rust_feature_set/feature_set_manifest.json",
                "segment_fairness_status": "passed",
                "model_artifact_evaluation_status": "passed",
                "serving_manifest_uri": "s3://fwa-models/baseline_fwa/{candidate_version}/serving_manifest.json",
                "model_artifact_evaluation_report_uri": "s3://fwa-models/baseline_fwa/{candidate_version}/artifact-evaluation/model_artifact_evaluation_report.json",
                "rust_serving_status": "passed",
                "rust_serving_latency_status": "passed",
                "rust_serving_p95_latency_ms": 17,
                "rust_serving_latency_measurement_kind": "simulated_fixture",
                "rust_serving_latency_sample_count": 0,
                "feature_reproducibility_hash": "sha256:activation-features",
                "label_provenance_status": "passed",
                "label_reviewer_source": "qa_review",
                "pilot_validation_status": "passed",
                "rule_candidate_backtest_status": "passed",
                "rule_library_writeback_status": "blocked_pending_human_review_and_policy_governance_approval",
                "rule_candidate_backtest_report_uri": "s3://fwa-models/baseline_fwa/{candidate_version}/rule-candidates/backtest/rule_candidate_backtest_report.json",
                "rule_candidate_review_tasks_uri": "s3://fwa-models/baseline_fwa/{candidate_version}/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json"
              }}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{completed}");
    assert_eq!(completed["candidate_model"]["version"], candidate_version);

    candidate_version.to_string()
}

pub(crate) async fn approve_activation_candidate(app: axum::Router, candidate_version: &str) {
    let (status, review) = json_request(
        app,
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
}

pub(crate) async fn submit_probability_calibration_report_for_test(
    app: axum::Router,
    model_version: &str,
) {
    let report_uri =
        format!("s3://customer-prod-artifacts/model-artifacts/baseline_fwa/{model_version}/calibration/probability_calibration_report.json");
    let (status, response) = json_request(
        app,
        "POST",
        "/api/v1/ops/models/baseline_fwa/probability-calibration-reports",
        &format!(
            r#"{{
              "actor": "worker:build-probability-calibration-report",
              "notes": "Labeled holdout calibration evidence for activation gate.",
              "report_uri": "{report_uri}",
              "report_kind": "probability_calibration_report",
              "model_version": "{model_version}",
              "as_of_date": "2026-06-15",
              "row_count": 120,
              "minimum_calibration_rows": 100,
              "bin_count": 2,
              "expected_calibration_error": 0.02,
              "max_expected_calibration_error": 0.05,
              "brier_score": 0.12,
              "max_brier_score": 0.20,
              "calibration_status": "passed",
              "bins": [
                {{
                  "bin_index": 0,
                  "lower_bound": 0.0,
                  "upper_bound": 0.5,
                  "row_count": 60,
                  "average_predicted_probability": 0.1,
                  "observed_positive_rate": 0.1,
                  "calibration_error": 0.0
                }},
                {{
                  "bin_index": 1,
                  "lower_bound": 0.5,
                  "upper_bound": 1.0,
                  "row_count": 60,
                  "average_predicted_probability": 0.8,
                  "observed_positive_rate": 0.76,
                  "calibration_error": 0.04
                }}
              ],
              "review_tasks": [],
              "evidence_refs": [
                "model_versions:baseline_fwa:{model_version}",
                "probability_calibration_reports:{report_uri}",
                "probability_calibration_input:s3://customer-prod-artifacts/baseline_fwa/{model_version}/calibration/holdout-predictions.json",
                "calibration_labels:s3://customer-prod-artifacts/baseline_fwa/{model_version}/calibration/holdout-labels.json"
              ],
              "governance_boundary": "calibration report is evidence only; it must not relabel outcomes, rewrite model probabilities, change routing thresholds, or activate calibrated serving"
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{response}");
    assert_eq!(response["model_version"], model_version);
    assert_eq!(response["calibration_status"], "passed");
}

pub(crate) async fn activate_candidate_for_test(app: axum::Router, candidate_version: &str) {
    approve_activation_candidate(app.clone(), candidate_version).await;
    submit_probability_calibration_report_for_test(app.clone(), candidate_version).await;
    let (status, activated) = json_request(
        app,
        "POST",
        "/api/v1/ops/models/baseline_fwa/activate",
        &model_lifecycle_payload("baseline_fwa", candidate_version),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(activated["version"], candidate_version);
    assert_eq!(activated["status"], "active");
}
