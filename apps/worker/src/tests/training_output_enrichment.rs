use super::*;

#[test]
fn enriches_training_output_with_rust_feature_set_evidence() {
    let root = temp_root("training-output-feature-set");
    let pack =
        build_demo_ml_datasets(&root, "2026-06-training-feature-set").expect("demo ML datasets");
    let artifact_dir = root.join("artifacts/baseline_fwa/0.1.0-candidate-job");
    fs::create_dir_all(&artifact_dir).unwrap();
    let output = CompleteRetrainingJobPayload {
        actor: "trainer-worker".into(),
        notes: "training output".into(),
        candidate_model_version: "0.1.0-candidate-job".into(),
        artifact_uri: artifact_dir
            .join("model.onnx")
            .to_string_lossy()
            .into_owned(),
        artifact_sha256: Some("sha256:serving".into()),
        training_artifact_uri: Some(
            artifact_dir
                .join("model.joblib")
                .to_string_lossy()
                .into_owned(),
        ),
        training_artifact_sha256: Some("sha256:training".into()),
        serving_manifest_uri: None,
        onnx_parity_report_uri: None,
        endpoint_url: None,
        validation_report_uri: artifact_dir
            .join("validation.json")
            .to_string_lossy()
            .into_owned(),
        evaluation_run_id: "eval_baseline_fwa_candidate".into(),
        auc: Some("0.82".into()),
        ks: None,
        precision: Some("0.70".into()),
        recall: Some("0.68".into()),
        f1: None,
        accuracy: None,
        threshold: Some("0.50".into()),
        confusion_matrix_json: serde_json::json!({}),
        feature_importance_uri: Some(
            artifact_dir
                .join("feature_importance.parquet")
                .to_string_lossy()
                .into_owned(),
        ),
        permutation_importance_uri: None,
        metrics_json: serde_json::json!({
            "feature_reproducibility_hash": "sha256:trainer-hash",
            "feature_store_materialization_status": "passed"
        }),
        evidence_refs: vec![
            format!(
                "model_artifacts:{}",
                artifact_dir.join("model.onnx").display()
            ),
            format!(
                "model_validation_reports:{}",
                artifact_dir.join("validation.json").display()
            ),
            "model_evaluations:eval_baseline_fwa_candidate".into(),
        ],
        mined_rule_owner: None,
        mined_rule_candidates: Vec::new(),
    };

    let output = enrich_retraining_output_with_rust_feature_set(output, &pack.labeled_manifest_uri)
        .expect("enriched training output");

    assert_eq!(output.artifact_sha256.as_deref(), Some("sha256:serving"));
    assert_eq!(
        output.training_artifact_sha256.as_deref(),
        Some("sha256:training")
    );
    assert_eq!(
        output.metrics_json["trainer_feature_reproducibility_hash"],
        "sha256:trainer-hash"
    );
    assert!(output.metrics_json["feature_reproducibility_hash"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert_ne!(
        output.metrics_json["feature_reproducibility_hash"],
        "sha256:trainer-hash"
    );
    assert_eq!(output.metrics_json["rust_feature_set_status"], "passed");
    let feature_set_manifest_uri = output.metrics_json["rust_feature_set_manifest_uri"]
        .as_str()
        .expect("rust feature set manifest uri");
    assert!(Path::new(feature_set_manifest_uri).is_file());
    assert!(
        output
            .evidence_refs
            .iter()
            .any(|reference| reference
                == &format!("feature_set_manifests:{feature_set_manifest_uri}"))
    );
}

#[tokio::test]
async fn enriches_training_output_with_rust_serving_evaluation_evidence() {
    let root = temp_root("training-output-artifact-evaluation");
    let pack =
        build_demo_ml_datasets(&root, "2026-06-training-artifact-eval").expect("demo ML datasets");
    let artifact_dir = root.join("artifacts/baseline_fwa/0.2.0-candidate-job");
    fs::create_dir_all(&artifact_dir).unwrap();
    let artifact_path = artifact_dir.join("rust_serving_artifact.json");
    write_json(
        artifact_path.clone(),
        &serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-candidate-job",
            "runtime_kind": "rust_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": ["amount_to_limit_ratio", "peer_percentile"],
            "intercept": -2.0,
            "coefficients": {
                "amount_to_limit_ratio": 2.4,
                "peer_percentile": 1.1
            }
        }),
    )
    .unwrap();
    let artifact_sha256 = test_sha256(&artifact_path);
    let serving_manifest_path = artifact_dir.join("serving_manifest.json");
    write_json(
        serving_manifest_path.clone(),
        &serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-candidate-job",
            "runtime_kind": "rust_logistic_regression",
            "artifact_uri": artifact_path.to_string_lossy(),
            "artifact_sha256": artifact_sha256,
            "version_lock": "0.2.0-candidate-job",
            "feature_columns": ["amount_to_limit_ratio", "peer_percentile"],
            "threshold": 0.5
        }),
    )
    .unwrap();
    let output = CompleteRetrainingJobPayload {
        actor: "trainer-worker".into(),
        notes: "training output".into(),
        candidate_model_version: "0.2.0-candidate-job".into(),
        artifact_uri: artifact_path.to_string_lossy().into_owned(),
        artifact_sha256: Some(test_sha256(&artifact_path)),
        training_artifact_uri: Some(
            artifact_dir
                .join("model.joblib")
                .to_string_lossy()
                .into_owned(),
        ),
        training_artifact_sha256: Some("sha256:training".into()),
        serving_manifest_uri: Some(serving_manifest_path.to_string_lossy().into_owned()),
        onnx_parity_report_uri: None,
        endpoint_url: None,
        validation_report_uri: artifact_dir
            .join("validation.json")
            .to_string_lossy()
            .into_owned(),
        evaluation_run_id: "eval_baseline_fwa_candidate".into(),
        auc: Some("0.84".into()),
        ks: None,
        precision: Some("0.72".into()),
        recall: Some("0.69".into()),
        f1: None,
        accuracy: None,
        threshold: Some("0.50".into()),
        confusion_matrix_json: serde_json::json!({}),
        feature_importance_uri: None,
        permutation_importance_uri: None,
        metrics_json: serde_json::json!({
            "feature_reproducibility_hash": "sha256:rust-feature-hash",
            "rust_feature_set_status": "passed",
            "rust_feature_set_manifest_uri": artifact_dir
                .join("rust_feature_set/feature_set_manifest.json")
                .to_string_lossy(),
            "feature_store_materialization_status": "passed"
        }),
        evidence_refs: vec![format!("model_artifacts:{}", artifact_path.display())],
        mined_rule_owner: None,
        mined_rule_candidates: Vec::new(),
    };

    let output =
        enrich_retraining_output_with_model_artifact_evaluation(output, &pack.labeled_manifest_uri)
            .await
            .expect("enriched training output");

    assert_eq!(
        output.metrics_json["model_artifact_evaluation_status"],
        "passed"
    );
    assert_eq!(output.metrics_json["rust_serving_status"], "passed");
    assert_eq!(output.metrics_json["rust_serving_latency_status"], "passed");
    let report_uri = output.metrics_json["model_artifact_evaluation_report_uri"]
        .as_str()
        .expect("artifact evaluation report uri");
    assert!(Path::new(report_uri).is_file());
    assert!(output
        .evidence_refs
        .iter()
        .any(|reference| reference == &format!("model_artifact_evaluations:{report_uri}")));
}
