use super::*;

#[test]
fn builds_external_training_handoff_from_manifest() {
    let root = temp_root("training-handoff");
    let manifest_path = root.join("manifest.json");
    fs::write(
        &manifest_path,
        serde_json::json!({
            "dataset_key": "claims_model",
            "dataset_version": "2026-06-02",
            "business_domain": "health_fwa",
            "sample_grain": "claim",
            "label_column": "confirmed_fwa",
            "entity_keys": ["claim_id", "member_id", "policy_id", "provider_id"],
            "time_split_field": "service_date",
            "group_split_fields": ["member_id", "policy_id", "provider_id"],
            "splits": [
                {"split_name": "train", "data_uri": "train.parquet"},
                {"split_name": "validation", "data_uri": "validation.parquet"},
                {"split_name": "out_of_time", "data_uri": "out_of_time.parquet"}
            ]
        })
        .to_string(),
    )
    .unwrap();

    let handoff = build_training_handoff(
        &manifest_path,
        "s3://fwa-models",
        "baseline_fwa",
        "0.1.0",
        "model_retraining_job_1",
        "trainer-worker",
    )
    .expect("training handoff");

    assert_eq!(handoff["handoff_kind"], "external_training_platform");
    assert_eq!(handoff["handoff_version"], 2);
    assert_eq!(handoff["dataset"]["dataset_key"], "claims_model");
    assert_eq!(handoff["dataset"]["dataset_version"], "2026-06-02");
    assert_eq!(
        handoff["dataset"]["manifest_uri"],
        serde_json::json!(manifest_path.to_string_lossy())
    );
    assert_eq!(handoff["training_job"]["model_key"], "baseline_fwa");
    assert_eq!(handoff["training_job"]["algorithm"], "logistic_regression");
    assert_eq!(
        handoff["training_job"]["runtime_kind"],
        "rust_logistic_regression"
    );
    assert_eq!(
        handoff["training_job"]["candidate_model_version"],
        "0.1.0-candidate-model_retraining_job_1"
    );
    assert_eq!(
            handoff["artifact_contract"]["serving_artifact_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/rust_serving_artifact.json"
        );
    assert_eq!(
        handoff["artifact_contract"]["serving_artifact_format"],
        "rust_json"
    );
    assert_eq!(
            handoff["artifact_contract"]["rust_serving_artifact_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/rust_serving_artifact.json"
        );
    assert_eq!(
            handoff["artifact_contract"]["rust_feature_set_manifest_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/rust_feature_set/feature_set_manifest.json"
        );
    assert_eq!(
            handoff["artifact_contract"]["feature_importance_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/feature_importance.parquet"
        );
    assert_eq!(
            handoff["artifact_contract"]["permutation_importance_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/permutation_importance.parquet"
        );
    assert_eq!(
        handoff["feature_set_contract"]["builder"],
        "worker build-feature-set"
    );
    assert_eq!(
        handoff["rule_candidate_workflow_contract"]["candidate_builder"],
        "worker mine-rule-candidates"
    );
    assert_eq!(
        handoff["rule_candidate_workflow_contract"]["backtest_builder"],
        "worker run-rule-candidate-backtest"
    );
    assert_eq!(
        handoff["rule_candidate_workflow_contract"]["writeback_boundary"],
        "human_review_required_before_rule_library_writeback"
    );
    assert_eq!(
        handoff["output_contract"]["submit_path"],
        "/api/v1/ops/model-retraining-jobs/model_retraining_job_1/output"
    );
    assert_eq!(
        handoff["output_contract"]["artifact_uri"],
        "artifact_contract.serving_artifact_uri"
    );
    assert_eq!(
        handoff["output_contract"]["permutation_importance_uri"],
        "artifact_contract.permutation_importance_uri"
    );
    assert_eq!(
        handoff["data_contract"]["source"],
        "same_parquet_dataset_manifest"
    );
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("feature_set_manifests")));
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("model_feature_importance")));
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("model_permutation_importance")));
    assert!(handoff["output_contract"]["required_metrics_fields"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field.as_str().unwrap().contains("max_feature_psi")));
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("rule_candidate_backtests")));
}

#[test]
fn builds_xgboost_training_handoff_with_onnx_contract() {
    let root = temp_root("xgboost-training-handoff");
    let pack = build_demo_ml_datasets(&root, "2026-06-xgboost-handoff").expect("demo ML datasets");

    let handoff = build_training_handoff_with_algorithm(
        &pack.labeled_manifest_uri,
        "s3://fwa-models",
        "baseline_fwa",
        "0.1.0",
        "model_retraining_job_1",
        "trainer-worker",
        "xgboost",
    )
    .expect("xgboost handoff");

    assert_eq!(handoff["handoff_version"], 2);
    assert_eq!(handoff["training_job"]["algorithm"], "xgboost");
    assert_eq!(handoff["training_job"]["runtime_kind"], "xgboost_onnx");
    assert_eq!(
        handoff["training_job"]["candidate_model_version"],
        "0.1.0-xgboost-candidate-model_retraining_job_1"
    );
    assert_eq!(
        handoff["artifact_contract"]["serving_artifact_uri"],
        "s3://fwa-models/baseline_fwa/0.1.0-xgboost-candidate-model_retraining_job_1/model.onnx"
    );
    assert_eq!(
        handoff["artifact_contract"]["serving_artifact_format"],
        "onnx"
    );
    assert_eq!(
        handoff["artifact_contract"]["onnx_artifact_uri"],
        "s3://fwa-models/baseline_fwa/0.1.0-xgboost-candidate-model_retraining_job_1/model.onnx"
    );
    assert_eq!(
            handoff["artifact_contract"]["onnx_parity_report_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-xgboost-candidate-model_retraining_job_1/onnx_parity_report.json"
        );
    assert_eq!(
        handoff["output_contract"]["onnx_parity_report_uri"],
        "artifact_contract.onnx_parity_report_uri"
    );
    assert_eq!(
            handoff["artifact_contract"]["feature_importance_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-xgboost-candidate-model_retraining_job_1/feature_importance.parquet"
        );
    assert_eq!(
            handoff["artifact_contract"]["permutation_importance_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-xgboost-candidate-model_retraining_job_1/permutation_importance.parquet"
        );
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("model_onnx_parity_reports")));
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("model_permutation_importance")));
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("rule_candidate_review_tasks")));
}

#[test]
fn builds_deep_learning_training_handoff_with_joblib_contract() {
    let root = temp_root("deep-learning-training-handoff");
    let pack =
        build_demo_ml_datasets(&root, "2026-06-deep-learning-handoff").expect("demo ML datasets");

    let handoff = build_training_handoff_with_algorithm(
        &pack.labeled_manifest_uri,
        "s3://fwa-models",
        "baseline_fwa",
        "0.1.0",
        "model_retraining_job_1",
        "trainer-worker",
        "deep_learning",
    )
    .expect("deep learning handoff");

    assert_eq!(handoff["handoff_version"], 2);
    assert_eq!(handoff["training_job"]["algorithm"], "deep_learning");
    assert_eq!(
        handoff["training_job"]["runtime_kind"],
        "deep_learning_sklearn_mlp"
    );
    assert_eq!(
        handoff["training_job"]["candidate_model_version"],
        "0.1.0-deep_learning-candidate-model_retraining_job_1"
    );
    assert_eq!(
            handoff["artifact_contract"]["serving_artifact_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-deep_learning-candidate-model_retraining_job_1/model.joblib"
        );
    assert_eq!(
        handoff["artifact_contract"]["serving_artifact_format"],
        "joblib"
    );
    assert!(handoff["artifact_contract"]["rust_serving_artifact_uri"].is_null());
    assert!(handoff["artifact_contract"]["onnx_artifact_uri"].is_null());
    assert!(handoff["artifact_contract"]["onnx_parity_report_uri"].is_null());
    assert!(handoff["output_contract"]["onnx_parity_report_uri"].is_null());
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .all(|reference| !reference
            .as_str()
            .unwrap()
            .contains("model_onnx_parity_reports")));
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("model_permutation_importance")));
}
