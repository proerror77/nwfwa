use super::*;

#[tokio::test]
async fn evaluates_model_artifact_with_rust_serving_parity_gate() {
    let root = temp_root("model-artifact-evaluation");
    let artifact_path = root.join("rust_serving_artifact.json");
    fs::write(
        &artifact_path,
        serde_json::to_vec(&serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "runtime_kind": "rust_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
            "intercept": -2.0,
            "coefficients": {
                "claim_amount_to_limit_ratio": 4.0,
                "provider_profile_score": 0.01
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let artifact_sha256 = test_sha256(&artifact_path);
    let serving_manifest_path = root.join("serving_manifest.json");
    write_json(
        serving_manifest_path.clone(),
        &serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "runtime_kind": "rust_logistic_regression",
            "artifact_uri": artifact_path.to_string_lossy(),
            "artifact_sha256": artifact_sha256,
            "version_lock": "0.2.0-rust",
            "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
            "threshold": 0.5
        }),
    )
    .unwrap();

    let dataset_dir = root.join("dataset");
    let validation_dir = dataset_dir.join("split=validation");
    fs::create_dir_all(&validation_dir).unwrap();
    let schema = Arc::new(Schema::new(vec![
        Field::new("claim_id", DataType::Utf8, false),
        Field::new("claim_amount_to_limit_ratio", DataType::Float64, false),
        Field::new("provider_profile_score", DataType::Float64, false),
        Field::new("expected_probability", DataType::Float64, false),
        Field::new("confirmed_fwa", DataType::Int8, false),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(vec!["CLM-EVAL-1", "CLM-EVAL-2"])),
            Arc::new(Float64Array::from(vec![0.8, 0.2])),
            Arc::new(Float64Array::from(vec![20.0, 10.0])),
            Arc::new(Float64Array::from(vec![0.8022, 0.2497])),
            Arc::new(Int8Array::from(vec![1, 0])),
        ],
    )
    .unwrap();
    write_parquet(validation_dir.join("part-00000.parquet"), schema, &batch).unwrap();
    let dataset_manifest_path = dataset_dir.join("manifest.json");
    write_json(
        dataset_manifest_path.clone(),
        &serde_json::json!({
            "dataset_key": "claims_model",
            "dataset_version": "2026-06-eval",
            "business_domain": "health_fwa",
            "sample_grain": "claim",
            "label_column": "confirmed_fwa",
            "entity_keys": ["claim_id"],
            "splits": [
                {"split_name": "validation", "data_uri": "split=validation/"}
            ]
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report = evaluate_model_artifact(
        &serving_manifest_path.to_string_lossy(),
        &dataset_manifest_path.to_string_lossy(),
        "validation",
        &output_dir,
        Some("expected_probability"),
        0.0001,
        100,
        10,
        None,
    )
    .await
    .expect("model artifact evaluation");

    assert_eq!(report.report_kind, "model_artifact_evaluation");
    assert_eq!(report.runtime_kind, "rust_logistic_regression");
    assert_eq!(report.row_count, 2);
    assert_eq!(report.contract_status, "passed");
    assert_eq!(report.rust_serving_status, "passed");
    assert_eq!(report.parity_status, "passed");
    assert_eq!(report.latency_status, "passed");
    assert_eq!(report.gate_status, "passed");
    assert_eq!(report.max_abs_probability_delta, Some(0.0));
    assert_eq!(report.sample_results[0].score, 80);
    assert_eq!(report.sample_results[1].label, "LOW_RISK");
    assert!(output_dir
        .join("model_artifact_evaluation_report.json")
        .is_file());
}
