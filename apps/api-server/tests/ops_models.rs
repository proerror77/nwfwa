#![recursion_limit = "256"]

use api_server::{app::build_app, config::AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
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
        backup_restore_plan_id: "demo-backup-restore-plan".into(),
        pii_masking_policy_id: "demo-pii-masking-policy".into(),
        key_rotation_policy_id: "demo-key-rotation-policy".into(),
        network_allowlist_id: "demo-network-allowlist".into(),
        alert_routing_policy_id: "demo-alert-routing-policy".into(),
        observability_exporter_endpoint: "local://demo-observability".into(),
        agent_policy_id: "demo-agent-policy".into(),
    }
}

async fn get_json(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
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

fn model_lifecycle_payload(model_key: &str, version: &str) -> String {
    format!(r#"{{"evidence_refs":["model_versions:{model_key}:{version}"]}}"#)
}

async fn register_model_dataset_for_test(app: axum::Router, suffix: &str) -> String {
    register_model_dataset_for_test_with_profiles(
        app,
        suffix,
        r#"{"missing_rate": 0.0, "psi": 0.01, "owner": "model-ops"}"#,
        r#"{"allowed_values": [0, 1], "missing_rate": 0.0, "psi": 0.01, "owner": "model-ops"}"#,
        r#"{"missing_rate": 0.0, "psi": 0.01, "owner": "model-ops"}"#,
    )
    .await
}

async fn register_unhealthy_model_dataset_for_test(app: axum::Router, suffix: &str) -> String {
    register_model_dataset_for_test_with_profiles(
        app,
        suffix,
        "{}",
        r#"{"allowed_values": [0, 1]}"#,
        "{}",
    )
    .await
}

async fn register_model_dataset_for_test_with_profiles(
    app: axum::Router,
    suffix: &str,
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
              "manifest_uri": "data/eval/claims_model_eval_{suffix}/v1/manifest.json",
              "schema_uri": "data/eval/claims_model_eval_{suffix}/v1/schema.json",
              "profile_uri": "data/eval/claims_model_eval_{suffix}/v1/profile.json",
              "storage_format": "parquet",
              "schema_hash": "sha256:model-{suffix}",
              "row_count": 100,
              "status": "draft",
              "splits": [
                {{
                  "split_name": "validation",
                  "data_uri": "data/eval/claims_model_eval_{suffix}/v1/split=validation/",
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
              "features_uri": "data/eval/claims_model_eval_{suffix}/v1/features/",
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
              "train_uri": "data/eval/claims_model_eval_{suffix}/v1/split=train/",
              "validation_uri": "data/eval/claims_model_eval_{suffix}/v1/split=validation/",
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

async fn register_activation_candidate(app: axum::Router) -> String {
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
              "feature_importance_uri": "data/eval/claims_model_eval_activation/v1/feature_importance.parquet",
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
                "model_feature_importance:data/eval/claims_model_eval_activation_candidate/v1/feature_importance.parquet",
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
              "feature_importance_uri": "data/eval/claims_model_eval_activation_candidate/v1/feature_importance.parquet",
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

async fn approve_activation_candidate(app: axum::Router, candidate_version: &str) {
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

async fn activate_candidate_for_test(app: axum::Router, candidate_version: &str) {
    approve_activation_candidate(app.clone(), candidate_version).await;
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

#[tokio::test]
async fn lists_baseline_model_versions() {
    let app = build_app(test_config());

    let (status, body) = get_json(app, "/api/v1/ops/models").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["models"][0]["model_key"], "baseline_fwa");
    assert_eq!(body["models"][0]["version"], "0.1.0");
    assert_eq!(body["models"][0]["runtime_kind"], "python_http");
    assert_eq!(body["models"][0]["status"], "active");
    assert_eq!(body["models"][0]["review_mode"], "both");
}

#[tokio::test]
async fn returns_empty_model_performance_without_scores() {
    let app = build_app(test_config());

    let (status, body) = get_json(app, "/api/v1/ops/models/baseline_fwa/performance").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["model_key"], "baseline_fwa");
    assert_eq!(body["data_status"], "empty");
    assert_eq!(body["scored_runs"], 0);
    assert_eq!(body["average_score"], 0.0);
    assert_eq!(body["high_risk_count"], 0);
    assert_eq!(body["drift_status"], "not_available");
    assert_eq!(body["score_psi"], serde_json::Value::Null);
}

#[tokio::test]
async fn returns_model_drift_from_latest_evaluation_metrics() {
    let app = build_app(test_config());

    let (_, dataset) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        r#"{
          "source_key": "claims_model_eval",
          "display_name": "Claims Model Eval",
          "business_domain": "fwa_claims",
          "owner": "model-ops",
          "description": "Evaluation dataset for model drift.",
          "dataset_key": "claims_model_eval",
          "dataset_version": "v1",
          "sample_grain": "claim",
          "label_column": "confirmed_fwa",
          "entity_keys": ["claim_id"],
          "manifest_uri": "data/eval/claims_model_eval/v1/manifest.json",
          "schema_uri": "data/eval/claims_model_eval/v1/schema.json",
          "profile_uri": "data/eval/claims_model_eval/v1/profile.json",
          "storage_format": "parquet",
          "schema_hash": "sha256:model-drift",
          "row_count": 100,
          "status": "draft",
          "splits": [
            {
              "split_name": "validation",
              "data_uri": "data/eval/claims_model_eval/v1/split=validation/",
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
              "feature_set_key": "claims_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/eval/claims_model_eval/v1/features/",
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
        app.clone(),
        "POST",
        "/api/v1/ops/model-datasets",
        &format!(
            r#"{{
              "business_domain": "fwa_claims",
              "task_type": "binary_classification",
              "label_name": "confirmed_fwa",
              "feature_set_id": "{feature_set_id}",
              "train_uri": "data/eval/claims_model_eval/v1/split=train/",
              "validation_uri": "data/eval/claims_model_eval/v1/split=validation/",
              "test_uri": null,
              "row_counts_json": {{"train": 80, "validation": 20}},
              "label_distribution_json": {{"train": {{"1": 20, "0": 60}}, "validation": {{"1": 5, "0": 15}}}},
              "status": "draft"
            }}"#
        ),
    )
    .await;
    let model_dataset_id = model_dataset["model_dataset_id"].as_str().unwrap();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_baseline_drift_001",
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
              "feature_importance_uri": "data/eval/claims_model_eval/v1/feature_importance.parquet",
              "metrics_json": {{"score_psi": 0.04}}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_baseline_drift_002",
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
              "feature_importance_uri": "data/eval/claims_model_eval/v1/feature_importance.parquet",
              "metrics_json": {{"score_psi": 0.18}}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = get_json(app, "/api/v1/ops/models/baseline_fwa/performance").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["score_psi"], 0.18);
    assert_eq!(body["drift_status"], "watch");
}

#[tokio::test]
async fn returns_model_promotion_gates_without_evaluation_evidence() {
    let app = build_app(test_config());

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
    let app = build_app(test_config());
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
    let app = build_app(test_config());
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
    let app = build_app(test_config());
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
    let app = build_app(test_config());
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
    let app = build_app(test_config());
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
    let app = build_app(test_config());
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
    let app = build_app(test_config());

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
    let app = build_app(test_config());
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

#[tokio::test]
async fn submits_mlops_monitoring_report_as_review_only_governance_event() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/mlops-monitoring-reports",
        r#"{
          "actor": "mlops-worker",
          "notes": "Rust monitoring loop found drift and shadow review signals.",
          "report_uri": "data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json",
          "report_kind": "mlops_monitoring_report",
          "model_version": "0.1.0",
          "overall_status": "watch",
          "retraining_recommendation": "prepare_retraining",
          "triggers": ["model_drift_detected", "shadow_comparison_review_required"],
          "review_tasks": [
            {"task_kind": "mlops_monitoring_review", "trigger": "model_drift_detected"}
          ],
          "evidence_refs": [
            "model_versions:baseline_fwa:0.1.0"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_MLOPS_MONITORING_EVIDENCE");

    let (status, response) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/mlops-monitoring-reports",
        r#"{
          "actor": "mlops-worker",
          "notes": "Rust monitoring loop found drift and shadow review signals.",
          "report_uri": "data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json",
          "report_kind": "mlops_monitoring_report",
          "model_version": "0.1.0",
          "overall_status": "watch",
          "retraining_recommendation": "prepare_retraining",
          "triggers": ["model_drift_detected", "shadow_comparison_review_required"],
          "review_tasks": [
            {"task_kind": "mlops_monitoring_review", "trigger": "model_drift_detected"}
          ],
          "evidence_refs": [
            "model_versions:baseline_fwa:0.1.0",
            "model_monitoring_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["model_key"], "baseline_fwa");
    assert_eq!(response["model_version"], "0.1.0");
    assert_eq!(response["monitoring_status"], "watch");
    assert_eq!(response["retraining_recommendation"], "prepare_retraining");
    assert_eq!(response["trigger_count"], 2);
    assert_eq!(response["review_task_count"], 1);
    assert!(response["next_actions"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "prepare_retraining_job_after_human_approval"
        )));
    assert!(response["governance_boundary"]
        .as_str()
        .unwrap()
        .contains("must not auto-create retraining jobs"));

    let (status, review_queue) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/mlops-monitoring-review-queue",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(review_queue["tasks"].as_array().unwrap().len(), 1);
    let task = &review_queue["tasks"][0];
    assert_eq!(task["model_key"], "baseline_fwa");
    assert_eq!(task["model_version"], "0.1.0");
    assert_eq!(task["task_kind"], "mlops_monitoring_review");
    assert_eq!(task["trigger"], "model_drift_detected");
    assert_eq!(task["review_status"], "open");
    assert_eq!(
        task["report_uri"],
        "data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json"
    );
    let task_id = task["task_id"].as_str().unwrap();

    let (status, invalid_decision) = json_request(
        app.clone(),
        "POST",
        &format!(
            "/api/v1/ops/models/baseline_fwa/mlops-monitoring-review-tasks/{}/reviews",
            task_id.replace(':', "%3A")
        ),
        &format!(
            r#"{{
              "decision": "auto_retrain",
              "reviewer": "model-governance",
              "notes": "Reject invalid decision contract.",
              "evidence_refs": [
                "model_versions:baseline_fwa:0.1.0",
                "model_monitoring_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json",
                "model_monitoring_review_tasks:{task_id}"
              ]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        invalid_decision["code"],
        "INVALID_MLOPS_MONITORING_REVIEW_TASK_DECISION"
    );

    let (status, missing_task_evidence) = json_request(
        app.clone(),
        "POST",
        &format!(
            "/api/v1/ops/models/baseline_fwa/mlops-monitoring-review-tasks/{}/reviews",
            task_id.replace(':', "%3A")
        ),
        r#"{
          "decision": "acknowledged",
          "reviewer": "model-governance",
          "notes": "Missing task evidence should fail.",
          "evidence_refs": [
            "model_versions:baseline_fwa:0.1.0",
            "model_monitoring_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        missing_task_evidence["code"],
        "MISSING_MLOPS_MONITORING_REVIEW_TASK_EVIDENCE"
    );

    let (status, missing_task) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/mlops-monitoring-review-tasks/missing-task/reviews",
        r#"{
          "decision": "acknowledged",
          "reviewer": "model-governance",
          "notes": "Missing task should fail before evidence validation.",
          "evidence_refs": [
            "model_versions:baseline_fwa:0.1.0",
            "model_monitoring_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json",
            "model_monitoring_review_tasks:missing-task"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        missing_task["code"],
        "MODEL_MONITORING_REVIEW_TASK_NOT_FOUND"
    );

    let (status, reviewed) = json_request(
        app.clone(),
        "POST",
        &format!(
            "/api/v1/ops/models/baseline_fwa/mlops-monitoring-review-tasks/{}/reviews",
            task_id.replace(':', "%3A")
        ),
        &format!(
            r#"{{
              "decision": "prepare_retraining",
              "reviewer": "model-governance",
              "notes": "Approved monitoring signal for retraining preparation.",
              "evidence_refs": [
                "model_versions:baseline_fwa:0.1.0",
                "model_monitoring_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json",
                "model_monitoring_review_tasks:{task_id}"
              ]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reviewed["task_id"], task_id);
    assert_eq!(reviewed["decision"], "prepare_retraining");
    assert_eq!(reviewed["reviewer"], "model-governance");

    let (status, reviewed_queue) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/mlops-monitoring-review-queue",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let reviewed_task = &reviewed_queue["tasks"][0];
    assert_eq!(reviewed_task["task_id"], task_id);
    assert_eq!(reviewed_task["review_status"], "prepare_retraining");
    assert_eq!(reviewed_task["reviewer"], "model-governance");
    assert!(reviewed_task["review_audit_id"]
        .as_str()
        .unwrap()
        .starts_with("aud_"));

    let (status, jobs) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/retraining-jobs",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(jobs["jobs"].as_array().unwrap().is_empty());

    let (status, audit) = get_json(
        app.clone(),
        "/api/v1/ops/audit-events?event_type=model.mlops_monitoring.report_submitted&model_key=baseline_fwa&model_version=0.1.0&limit=5",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let event = &audit["events"][0];
    assert_eq!(
        event["event_type"],
        "model.mlops_monitoring.report_submitted"
    );
    assert_eq!(event["payload"]["monitoring_status"], "watch");
    assert_eq!(
        event["payload"]["retraining_recommendation"],
        "prepare_retraining"
    );
    assert!(event["evidence_refs"].as_array().unwrap().contains(
        &serde_json::json!(
            "model_monitoring_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json"
        )
    ));

    let (status, review_audit) = get_json(
        app,
        "/api/v1/ops/audit-events?event_type=model.mlops_monitoring.review_task_reviewed&model_key=baseline_fwa&model_version=0.1.0&limit=5",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let review_event = &review_audit["events"][0];
    assert_eq!(
        review_event["event_type"],
        "model.mlops_monitoring.review_task_reviewed"
    );
    assert_eq!(review_event["payload"]["task_id"], task_id);
    assert_eq!(review_event["payload"]["decision"], "prepare_retraining");
    assert_eq!(
        review_event["payload"]["notes"],
        "Approved monitoring signal for retraining preparation."
    );
}

#[tokio::test]
async fn submits_mlops_alert_delivery_without_creating_retraining_job() {
    let app = build_app(test_config());

    let (status, response) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/mlops-alert-deliveries",
        r#"{
          "actor": "mlops-worker",
          "notes": "Queue alert-router delivery for drift and shadow review.",
          "scheduler_execution_report_uri": "data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/scheduler/mlops_scheduler_execution_report.json",
          "report_kind": "mlops_scheduler_execution_report",
          "model_version": "0.1.0",
          "alert_delivery_status": "queued_for_external_alert_router",
          "alert_delivery_tasks": [
            {
              "task_kind": "mlops_alert_delivery",
              "trigger": "model_drift_detected",
              "route_key": "mlops_retraining_readiness",
              "delivery_status": "queued_for_external_alert_router"
            }
          ],
          "evidence_refs": [
            "model_versions:baseline_fwa:0.1.0",
            "mlops_scheduler_execution_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/scheduler/mlops_scheduler_execution_report.json"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{response}");
    assert_eq!(response["model_key"], "baseline_fwa");
    assert_eq!(response["model_version"], "0.1.0");
    assert_eq!(
        response["alert_delivery_status"],
        "queued_for_external_alert_router"
    );
    assert_eq!(response["alert_delivery_task_count"], 1);
    assert_eq!(response["alert_routing_policy_configured"], true);
    assert!(response["next_actions"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("confirm_customer_alert_router_receipt")));
    assert!(response["governance_boundary"]
        .as_str()
        .unwrap()
        .contains("must not create retraining jobs"));

    let (status, alert_queue) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/mlops-alert-delivery-queue",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(alert_queue["tasks"].as_array().unwrap().len(), 1);
    let alert_task = &alert_queue["tasks"][0];
    assert_eq!(alert_task["model_key"], "baseline_fwa");
    assert_eq!(alert_task["model_version"], "0.1.0");
    assert_eq!(alert_task["task_kind"], "mlops_alert_delivery");
    assert_eq!(alert_task["trigger"], "model_drift_detected");
    assert_eq!(alert_task["route_key"], "mlops_retraining_readiness");
    assert_eq!(
        alert_task["delivery_status"],
        "queued_for_external_alert_router"
    );
    assert_eq!(alert_task["review_status"], "open");
    let alert_task_id = alert_task["task_id"].as_str().unwrap();

    let (status, invalid_alert_decision) = json_request(
        app.clone(),
        "POST",
        &format!(
            "/api/v1/ops/models/baseline_fwa/mlops-alert-delivery-tasks/{}/reviews",
            alert_task_id.replace(':', "%3A")
        ),
        &format!(
            r#"{{
              "decision": "auto_escalate",
              "reviewer": "alert-router-owner",
              "notes": "Reject invalid alert decision.",
              "evidence_refs": [
                "model_versions:baseline_fwa:0.1.0",
                "mlops_scheduler_execution_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/scheduler/mlops_scheduler_execution_report.json",
                "mlops_alert_delivery_tasks:{alert_task_id}"
              ]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        invalid_alert_decision["code"],
        "INVALID_MLOPS_ALERT_DELIVERY_TASK_DECISION"
    );

    let (status, missing_alert_task_evidence) = json_request(
        app.clone(),
        "POST",
        &format!(
            "/api/v1/ops/models/baseline_fwa/mlops-alert-delivery-tasks/{}/reviews",
            alert_task_id.replace(':', "%3A")
        ),
        r#"{
          "decision": "receipt_confirmed",
          "reviewer": "alert-router-owner",
          "notes": "Missing alert task evidence should fail.",
          "evidence_refs": [
            "model_versions:baseline_fwa:0.1.0",
            "mlops_scheduler_execution_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/scheduler/mlops_scheduler_execution_report.json"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        missing_alert_task_evidence["code"],
        "MISSING_MLOPS_ALERT_DELIVERY_TASK_EVIDENCE"
    );

    let (status, alert_review) = json_request(
        app.clone(),
        "POST",
        &format!(
            "/api/v1/ops/models/baseline_fwa/mlops-alert-delivery-tasks/{}/reviews",
            alert_task_id.replace(':', "%3A")
        ),
        &format!(
            r#"{{
              "decision": "receipt_confirmed",
              "reviewer": "alert-router-owner",
              "notes": "Confirmed customer alert router receipt.",
              "evidence_refs": [
                "model_versions:baseline_fwa:0.1.0",
                "mlops_scheduler_execution_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/scheduler/mlops_scheduler_execution_report.json",
                "mlops_alert_delivery_tasks:{alert_task_id}"
              ]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(alert_review["task_id"], alert_task_id);
    assert_eq!(alert_review["decision"], "receipt_confirmed");

    let (status, reviewed_alert_queue) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/mlops-alert-delivery-queue",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let reviewed_alert_task = &reviewed_alert_queue["tasks"][0];
    assert_eq!(reviewed_alert_task["task_id"], alert_task_id);
    assert_eq!(reviewed_alert_task["review_status"], "receipt_confirmed");
    assert_eq!(reviewed_alert_task["reviewer"], "alert-router-owner");

    let (status, jobs) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/retraining-jobs",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(jobs["jobs"].as_array().unwrap().is_empty());

    let (status, audit) = get_json(
        app.clone(),
        "/api/v1/ops/audit-events?event_type=model.mlops_alert_delivery.submitted&model_key=baseline_fwa&model_version=0.1.0&limit=5",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let event = &audit["events"][0];
    assert_eq!(event["event_type"], "model.mlops_alert_delivery.submitted");
    assert_eq!(event["payload"]["alert_delivery_task_count"], 1);
    assert_eq!(
        event["payload"]["alert_routing_policy_ref"],
        "configured_alert_routing_policy"
    );
    assert!(event["evidence_refs"].as_array().unwrap().contains(
        &serde_json::json!(
            "mlops_scheduler_execution_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/scheduler/mlops_scheduler_execution_report.json"
        )
    ));

    let (status, review_audit) = get_json(
        app,
        "/api/v1/ops/audit-events?event_type=model.mlops_alert_delivery.task_reviewed&model_key=baseline_fwa&model_version=0.1.0&limit=5",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let review_event = &review_audit["events"][0];
    assert_eq!(
        review_event["event_type"],
        "model.mlops_alert_delivery.task_reviewed"
    );
    assert_eq!(review_event["payload"]["task_id"], alert_task_id);
    assert_eq!(review_event["payload"]["decision"], "receipt_confirmed");
    assert_eq!(
        review_event["payload"]["notes"],
        "Confirmed customer alert router receipt."
    );
}

#[tokio::test]
async fn blocks_model_retraining_job_when_readiness_is_blocked() {
    let app = build_app(test_config());

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
    let app = build_app(test_config());
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
        r#"{
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
            "shadow_comparison_status": "passed",
            "review_capacity_threshold_status": "passed",
            "model_artifact_evaluation_status": "passed",
            "model_artifact_evaluation_report_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/artifact-evaluation/model_artifact_evaluation_report.json",
            "rule_candidate_backtest_status": "passed",
            "rule_candidate_backtest_report_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_report.json",
            "rule_candidate_review_tasks_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json",
            "rule_library_writeback_status": "blocked_pending_human_review_and_policy_governance_approval"
          },
          "mined_rule_candidates": [
            {
              "rule_id": "candidate_retraining_amount_ratio",
              "version": 1,
              "name": "Retraining mined amount ratio candidate",
              "review_mode": "both",
              "scheme_family": "high_risk_claim",
              "conditions": [
                {
                  "field": "claim_amount_to_limit_ratio",
                  "operator": ">=",
                  "value": 0.82
                }
              ],
              "action": {
                "score": 22,
                "alert_code": "RETRAINING_AMOUNT_RATIO_CANDIDATE",
                "recommended_action": "ManualReview",
                "reason": "External training platform mined this explainable candidate from feature importance and backtest evidence."
              }
            },
            {
              "rule_id": "candidate_tree_provider_profile_and_amount",
              "version": 1,
              "name": "Decision tree mined provider profile and amount candidate",
              "review_mode": "both",
              "scheme_family": "high_risk_claim",
              "conditions": [
                {
                  "field": "provider_profile_score",
                  "operator": ">=",
                  "value": 47.5
                },
                {
                  "field": "claim_amount_to_limit_ratio",
                  "operator": ">=",
                  "value": 0.56
                }
              ],
              "action": {
                "score": 34,
                "alert_code": "TREE_MINED_PROVIDER_AMOUNT",
                "recommended_action": "ManualReview",
                "reason": "External training platform mined this shallow decision-tree path from training data. Human review is required."
              }
            }
          ]
        }"#,
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

#[tokio::test]
async fn blocks_model_promotion_when_score_drift_is_detected() {
    let app = build_app(test_config());
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
              "feature_importance_uri": "data/eval/claims_model_eval_drift_gate/v1/feature_importance.parquet",
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
    let app = build_app(test_config());
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
              "feature_importance_uri": "data/eval/claims_model_eval_candidate_drift/v1/feature_importance.parquet",
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
    let app = build_app(test_config());

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
    let app = build_app(test_config());
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
    let app = build_app(test_config());
    let candidate_version = register_activation_candidate(app.clone()).await;

    let (status, review) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/promotion-reviews",
        r#"{
          "decision": "approved",
          "reviewer": "model-governance",
          "notes": "Approved candidate for production activation.",
          "evidence_refs": ["model_versions:baseline_fwa:{candidate_version}"]
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
        &model_lifecycle_payload("baseline_fwa", "0.1.0"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_TARGET_MODEL_VERSION_EVIDENCE");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains(&candidate_version));

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
async fn model_promotion_and_activation_are_version_scoped() {
    let app = build_app(test_config());
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
    let app = build_app(test_config());
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
    let app = build_app(test_config());
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
    let app = build_app(test_config());
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
    let app = build_app(test_config());
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
                  "feature_importance_uri": "data/eval/rollback_window/{index}/feature_importance.parquet",
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
    let app = build_app(test_config());
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
    let app = build_app(test_config());

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

#[tokio::test]
async fn rejects_missing_api_key_for_model_ops() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/ops/models")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rejects_missing_api_key_for_model_promotion_gates() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/ops/models/baseline_fwa/promotion-gates")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
