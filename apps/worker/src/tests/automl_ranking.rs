use super::*;

#[test]
fn ranks_automl_candidates_and_blocks_missing_governance_gates() {
    let root = temp_root("automl-ranking");
    let logistic_report = root.join("logistic-validation.json");
    let xgboost_report = root.join("xgboost-validation.json");
    let lightgbm_report = root.join("lightgbm-validation.json");
    write_validation_report(
        &logistic_report,
        "0.1.0-candidate-logistic",
        "logistic_regression",
        "linear_baseline",
        0.72,
        0.68,
        0.66,
        "passed",
    );
    write_validation_report(
        &xgboost_report,
        "0.1.0-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.84,
        0.78,
        0.74,
        "passed",
    );
    write_validation_report(
        &lightgbm_report,
        "0.1.0-lightgbm-candidate",
        "lightgbm",
        "gradient_boosted_tree",
        0.86,
        0.80,
        0.77,
        "failed",
    );

    let report_uris = vec![
        logistic_report.to_string_lossy().into_owned(),
        xgboost_report.to_string_lossy().into_owned(),
        lightgbm_report.to_string_lossy().into_owned(),
    ];
    let output_dir = root.join("out");
    let ranking = rank_automl_candidates(&report_uris, &output_dir).expect("ranking");

    assert_eq!(ranking.plan_kind, "automl_candidate_ranking");
    assert_eq!(
        ranking.recommended_candidate_model_version.as_deref(),
        Some("0.1.0-xgboost-candidate")
    );
    assert_eq!(
        ranking.candidates[0].candidate_model_version,
        "0.1.0-xgboost-candidate"
    );
    assert_eq!(ranking.candidates[0].gate_status, "passed");
    assert_eq!(
        ranking.candidates[0].recommended_action,
        "open_human_review"
    );
    assert!(ranking.candidates[0]
        .evidence_refs
        .iter()
        .any(|ref_id| { ref_id.starts_with("automl_feature_search_reports:") }));
    assert!(ranking.candidates[0]
        .evidence_refs
        .iter()
        .any(|ref_id| { ref_id.starts_with("automl_factor_rankings:") }));
    assert_eq!(
        ranking.candidates[1].candidate_model_version,
        "0.1.0-candidate-logistic"
    );
    assert_eq!(
        ranking.candidates[2].candidate_model_version,
        "0.1.0-lightgbm-candidate"
    );
    assert_eq!(ranking.candidates[2].gate_status, "blocked");
    assert!(ranking.candidates[2]
        .blocking_reasons
        .contains(&"leakage_check_status:failed".to_string()));
    assert_eq!(ranking.review_tasks.len(), 3);
    assert_eq!(
        ranking.review_tasks[0].required_review,
        "human_approval_required_before_shadow_or_activation"
    );
    assert!(output_dir.join("automl_candidate_ranking.json").is_file());
    assert!(output_dir.join("automl_review_tasks.json").is_file());
}

#[test]
fn automl_candidate_ranking_penalizes_unstable_candidates() {
    let root = temp_root("automl-ranking-stability");
    let stable_report = root.join("stable-validation.json");
    let unstable_report = root.join("unstable-validation.json");
    write_validation_report(
        &stable_report,
        "0.1.0-stable-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.83,
        0.77,
        0.73,
        "passed",
    );
    write_validation_report(
        &unstable_report,
        "0.1.0-unstable-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.90,
        0.82,
        0.80,
        "passed",
    );
    let mut unstable_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&unstable_report).unwrap()).unwrap();
    unstable_json["metrics_json"]["score_psi"] = serde_json::json!(0.249);
    unstable_json["metrics_json"]["max_feature_psi"] = serde_json::json!(0.249);
    fs::write(
        &unstable_report,
        serde_json::to_string(&unstable_json).unwrap(),
    )
    .unwrap();

    let ranking = rank_automl_candidates(
        &[
            stable_report.to_string_lossy().into_owned(),
            unstable_report.to_string_lossy().into_owned(),
        ],
        root.join("out"),
    )
    .expect("ranking");

    assert_eq!(
        ranking.recommended_candidate_model_version.as_deref(),
        Some("0.1.0-stable-xgboost-candidate")
    );
    assert_eq!(
        ranking.candidates[0].candidate_model_version,
        "0.1.0-stable-xgboost-candidate"
    );
    assert!(ranking.candidates[1].overfitting_penalty > ranking.candidates[0].overfitting_penalty);
    assert_eq!(ranking.candidates[1].gate_status, "passed");
}

#[test]
fn automl_candidate_ranking_requires_rust_lifecycle_evidence() {
    let root = temp_root("automl-rust-evidence");
    let validation_report = root.join("validation.json");
    fs::write(
        &validation_report,
        serde_json::json!({
            "model_key": "baseline_fwa",
            "candidate_model_version": "0.1.0-candidate-without-rust-evidence",
            "dataset_key": "claims_model",
            "dataset_version": "2026-06-demo",
            "algorithm": "xgboost",
            "validation_metrics": {
                "auc": 0.84,
                "precision": 0.78,
                "recall": 0.74
            },
            "metrics_json": {
                "algorithm": "xgboost",
                "algorithm_family": "gradient_boosted_tree",
                "out_of_time_auc": 0.84,
                "out_of_time_average_precision": 0.80,
                "out_of_time_precision": 0.78,
                "out_of_time_recall": 0.74,
                "time_group_split_status": "passed",
                "leakage_check_status": "passed",
                "shadow_comparison_status": "passed",
                "serving_version_lock_status": "passed",
                "artifact_integrity_status": "passed",
                "feature_store_materialization_status": "passed",
                "segment_fairness_status": "passed",
                "label_provenance_status": "passed"
            }
        })
        .to_string(),
    )
    .unwrap();

    let ranking = rank_automl_candidates(
        &[validation_report.to_string_lossy().into_owned()],
        root.join("out"),
    )
    .expect("ranking");

    assert_eq!(ranking.recommended_candidate_model_version, None);
    assert_eq!(ranking.candidates[0].gate_status, "blocked");
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"rust_feature_set_status:missing_or_failed".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"rust_feature_set_manifest_uri:missing".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"automl_feature_search_status:missing_or_failed".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"automl_feature_search_report_uri:missing".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"automl_selected_feature_count:missing_or_zero".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"automl_factor_ranking_status:missing_or_failed".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"automl_factor_ranking_report_uri:missing".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"automl_ranked_factor_count:missing_or_zero".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"model_artifact_evaluation_status:missing_or_failed".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"onnx_parity_status:missing_or_failed".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"onnx_parity_report_uri:missing".into()));
}
