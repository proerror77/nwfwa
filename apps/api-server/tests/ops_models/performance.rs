use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{get_json, json_request, test_config};

#[tokio::test]
async fn lists_baseline_model_versions() {
    let app = build_app(test_config()).unwrap();

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
    let app = build_app(test_config()).unwrap();

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
    let app = build_app(test_config()).unwrap();

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

    let (status, invalid_feature_importance) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_baseline_drift_local_feature_importance",
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
              "feature_importance_uri": "local://template/model-evaluations/feature_importance.parquet",
              "metrics_json": {{"score_psi": 0.04}}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        invalid_feature_importance["code"],
        "MODEL_EVALUATION_FEATURE_IMPORTANCE_FORMAT_INVALID"
    );

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
              "feature_importance_uri": "s3://fwa-models/baseline_fwa/0.1.0/feature_importance.parquet",
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
              "feature_importance_uri": "s3://fwa-models/baseline_fwa/0.1.0/feature_importance.parquet",
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
