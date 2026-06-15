use api_server::app::build_app;
use axum::http::StatusCode;

use super::{json_request, renewal_dataset_payload, test_config};

#[tokio::test]
async fn registers_feature_set_model_dataset_and_evaluation_trace() {
    let app = build_app(test_config()).unwrap();
    let (_, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("parquet"),
    )
    .await;
    let dataset_id = created["dataset_id"].as_str().unwrap();

    let (status, feature_set) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/feature-sets",
        &format!(
            r#"{{
              "business_domain": "renewal_retention",
              "feature_set_key": "renewal_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/features/renewal_automl_20211105/v1/",
              "feature_list_json": ["member_age", "sum_premium", "issue_rate"],
              "row_count": 88622,
              "label_column": "m_2_keep_status",
              "status": "draft"
            }}"#
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let feature_set_id = feature_set["feature_set_id"].as_str().unwrap();
    assert_eq!(feature_set["dataset_id"], dataset_id);

    let (status, model_dataset) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-datasets",
        &format!(
            r#"{{
              "business_domain": "renewal_retention",
              "task_type": "binary_classification",
              "label_name": "renewal_m2_keep_status",
              "feature_set_id": "{feature_set_id}",
              "train_uri": "data/features/renewal_automl_20211105/v1/split=train/",
              "validation_uri": "data/features/renewal_automl_20211105/v1/split=validation/",
              "test_uri": null,
              "row_counts_json": {{"train": 68664, "validation": 19958}},
              "label_distribution_json": {{"train": {{"1": 35837, "0": 32827}}, "validation": {{"1": 9342, "0": 10616}}}},
              "status": "draft"
            }}"#
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let model_dataset_id = model_dataset["model_dataset_id"].as_str().unwrap();
    assert_eq!(model_dataset["feature_set_id"], feature_set_id);

    let (status, evaluation) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_renewal_v1",
              "model_key": "renewal_baseline",
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
              "feature_importance_uri": "s3://fwa-models/renewal_baseline/0.1.0/feature_importance.parquet",
              "metrics_json": {{"data_status": "validation"}}
            }}"#
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        evaluation["evaluation"]["evaluation_run_id"],
        "eval_renewal_v1"
    );
    assert_eq!(
        evaluation["evaluation"]["model_dataset_id"],
        model_dataset_id
    );
    assert_eq!(
        evaluation["evaluation"]["scheme_family"],
        "diagnosis_procedure_mismatch"
    );

    let (status, loaded) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/model-evaluations/eval_renewal_v1",
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(loaded["evaluation"]["model_key"], "renewal_baseline");
    assert_eq!(loaded["evaluation"]["model_dataset_id"], model_dataset_id);
    assert_eq!(
        loaded["evaluation"]["scheme_family"],
        "diagnosis_procedure_mismatch"
    );

    let (status, listed) =
        json_request(app.clone(), "GET", "/api/v1/ops/model-evaluations", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        listed["evaluations"][0]["evaluation_run_id"],
        "eval_renewal_v1"
    );
    assert_eq!(
        listed["evaluations"][0]["model_dataset_id"],
        model_dataset_id
    );
    assert_eq!(
        listed["evaluations"][0]["scheme_family"],
        "diagnosis_procedure_mismatch"
    );
    assert_eq!(listed["lineage"][0]["evaluation_run_id"], "eval_renewal_v1");
    assert_eq!(listed["lineage"][0]["model_key"], "renewal_baseline");
    assert_eq!(listed["lineage"][0]["source_dataset_id"], dataset_id);
    assert_eq!(
        listed["lineage"][0]["source_dataset_key"],
        "renewal_automl_20211105"
    );
    assert_eq!(listed["lineage"][0]["source_dataset_version"], "v1");
    assert_eq!(listed["lineage"][0]["source_data_quality_status"], "watch");

    let (status, audit_events) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?event_group=governance&limit=20",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let audit_event_types = audit_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["event_type"].as_str().unwrap())
        .collect::<Vec<_>>();
    for event_type in [
        "dataset.registered",
        "feature_set.registered",
        "model_dataset.registered",
        "model_evaluation.registered",
    ] {
        assert!(
            audit_event_types.contains(&event_type),
            "missing governance audit event {event_type}"
        );
    }
    let model_evaluation_event = audit_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "model_evaluation.registered")
        .unwrap();
    assert_eq!(
        model_evaluation_event["payload"]["evaluation_run_id"],
        "eval_renewal_v1"
    );
    assert_eq!(
        model_evaluation_event["payload"]["scheme_family"],
        "diagnosis_procedure_mismatch"
    );
    assert_eq!(
        model_evaluation_event["payload"]["customer_scope_id"],
        "demo-customer"
    );
    assert_eq!(
        model_evaluation_event["evidence_refs"][0],
        "model_evaluations:eval_renewal_v1"
    );

    let (status, dataset_events) = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/ops/audit-events?dataset_id={dataset_id}&limit=10"),
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(dataset_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event_type"] == "dataset.registered"
            && event["payload"]["dataset_id"] == dataset_id));

    let (status, feature_set_events) = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/ops/audit-events?feature_set_id={feature_set_id}&limit=10"),
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(feature_set_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event_type"] == "feature_set.registered"
            && event["payload"]["feature_set_id"] == feature_set_id));

    let (status, model_dataset_events) = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/ops/audit-events?model_dataset_id={model_dataset_id}&limit=10"),
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(model_dataset_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event_type"] == "model_dataset.registered"
            && event["payload"]["model_dataset_id"] == model_dataset_id));

    let (status, evaluation_events) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?evaluation_run_id=eval_renewal_v1&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(evaluation_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event_type"] == "model_evaluation.registered"
            && event["payload"]["evaluation_run_id"] == "eval_renewal_v1"));

    let (status, missing_dataset_events) = json_request(
        app,
        "GET",
        "/api/v1/ops/audit-events?dataset_id=missing-dataset&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(missing_dataset_events["events"]
        .as_array()
        .unwrap()
        .is_empty());
}
