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
        model_service_url: "http://unused".into(),
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

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_baseline_activation_base",
              "model_key": "baseline_fwa",
              "model_version": "0.1.0",
              "model_dataset_id": "{model_dataset_id}",
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
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
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
    assert_eq!(status, StatusCode::OK);

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
              "auc": "0.86",
              "ks": "0.48",
              "precision": "0.78",
              "recall": "0.71",
              "f1": "0.74",
              "accuracy": "0.79",
              "threshold": "0.52",
              "confusion_matrix_json": {{"tp": 12, "fp": 2, "tn": 14, "fn": 2}},
              "feature_importance_uri": "data/eval/claims_model_eval_activation_candidate/v1/feature_importance.parquet",
              "metrics_json": {{
                "score_psi": 0.04,
                "out_of_time_auc": 0.84,
                "review_capacity_threshold_status": "passed",
                "leakage_check_status": "passed",
                "shadow_comparison_status": "passed",
                "feature_reproducibility_hash": "sha256:activation-features",
                "label_provenance_status": "passed",
                "label_reviewer_source": "qa_review"
              }}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(completed["candidate_model"]["version"], candidate_version);

    candidate_version.to_string()
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
    assert_eq!(body["total_count"], 15);
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
                "label_reviewer_source": "qa_review"
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
    for label in ["Feature reproducibility", "Label provenance"] {
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
          "feedback_target": "models",
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

    let (status, body) = get_json(app, "/api/v1/ops/models/baseline_fwa/promotion-gates").await;

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
        &format!("/api/v1/ops/model-retraining-jobs/{job_id}/output"),
        r#"{
          "actor": "trainer-worker",
          "notes": " ",
          "candidate_model_version": "0.2.0-candidate",
          "artifact_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/model.onnx",
          "validation_report_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
          "evaluation_run_id": "eval_baseline_retraining_job_candidate",
          "confusion_matrix_json": {"tp": 12, "fp": 2, "tn": 14, "fn": 2},
          "metrics_json": {"score_psi": 0.04}
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_RETRAINING_OUTPUT_NOTES");

    let (status, completed) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/model-retraining-jobs/{job_id}/output"),
        r#"{
          "actor": "trainer-worker",
          "notes": "Candidate model and validation report registered.",
          "candidate_model_version": "0.2.0-candidate",
          "artifact_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/model.onnx",
          "endpoint_url": "http://127.0.0.1:8001/score/baseline_fwa/0.2.0-candidate",
          "validation_report_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
          "evaluation_run_id": "eval_baseline_retraining_job_candidate",
          "auc": "0.86",
          "ks": "0.48",
          "precision": "0.78",
          "recall": "0.71",
          "f1": "0.74",
          "accuracy": "0.79",
          "threshold": "0.52",
          "confusion_matrix_json": {"tp": 12, "fp": 2, "tn": 14, "fn": 2},
          "feature_importance_uri": "data/eval/claims_model_eval_retraining_job_candidate/v1/feature_importance.parquet",
          "metrics_json": {
            "score_psi": 0.04,
            "shadow_comparison_status": "passed",
            "review_capacity_threshold_status": "passed"
          }
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
        "s3://fwa-models/baseline_fwa/0.2.0-candidate/model.onnx"
    );
    assert_eq!(completed["evaluation"]["model_version"], "0.2.0-candidate");

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
        "endpoint_url": "http://127.0.0.1:8001/score/baseline_fwa/0.2.0-candidate",
        "validation_report_uri": "s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
        "evaluation_run_id": "eval_baseline_retraining_job_candidate",
        "auc": "0.86",
        "ks": "0.48",
        "precision": "0.78",
        "recall": "0.71",
        "f1": "0.74",
        "accuracy": "0.79",
        "threshold": "0.52",
        "confusion_matrix_json": {"tp": 12, "fp": 2, "tn": 14, "fn": 2},
        "feature_importance_uri": "data/eval/claims_model_eval_retraining_job_candidate/v1/feature_importance.parquet",
        "metrics_json": {"score_psi": 0.04}
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

    let mut csv_feature_importance = valid_request.clone();
    csv_feature_importance["feature_importance_uri"] =
        serde_json::json!("data/eval/feature_importance.csv");
    let payload = csv_feature_importance.to_string();
    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/model-retraining-jobs/job_1/output",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_RETRAINING_OUTPUT_FEATURE_IMPORTANCE");
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
    assert_eq!(status, StatusCode::OK);
    assert_eq!(review["model_version"], candidate_version);

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

    let (status, gates) = get_json(app, "/api/v1/ops/models/baseline_fwa/promotion-gates").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(gates["decision"], "routing_allowed");
    assert!(gates["blockers"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn rolls_back_active_model_version() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/rollback",
        &model_lifecycle_payload("baseline_fwa", "0.1.0"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["model_key"], "baseline_fwa");
    assert_eq!(body["version"], "0.1.0");
    assert_eq!(body["status"], "approved");

    let (status, body) = get_json(app.clone(), "/api/v1/ops/models").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["models"][0]["status"], "approved");

    let (status, audit) = get_json(
        app.clone(),
        "/api/v1/ops/audit-events?event_type=model.rollback.completed&limit=5",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        audit["events"][0]["evidence_refs"][0],
        "model_versions:baseline_fwa:0.1.0"
    );

    let (status, body) = get_json(app, "/api/v1/ops/models/baseline_fwa/promotion-gates").await;
    assert_eq!(status, StatusCode::OK);
    let active_gate = body["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["label"] == "Active version")
        .expect("model promotion gates should include active-version gate");
    assert_eq!(active_gate["passed"], false);
    assert_eq!(active_gate["evidence_source"], "missing");
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("model is not active")));
}

#[tokio::test]
async fn rolls_back_active_model_when_newer_candidate_exists() {
    let app = build_app(test_config());
    let candidate_version = register_activation_candidate(app.clone()).await;

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/rollback",
        &model_lifecycle_payload("baseline_fwa", "0.1.0"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["model_key"], "baseline_fwa");
    assert_eq!(body["version"], "0.1.0");
    assert_eq!(body["status"], "approved");

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
        .any(|model| model["version"] == "0.1.0" && model["status"] == "approved"));
}

#[tokio::test]
async fn blocks_model_rollback_when_model_is_not_active() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/rollback",
        &model_lifecycle_payload("baseline_fwa", "0.1.0"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/models/baseline_fwa/rollback",
        &model_lifecycle_payload("baseline_fwa", "0.1.0"),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "MODEL_ROLLBACK_REQUIRES_ACTIVE");
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
