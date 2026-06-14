use super::*;

#[test]
fn mines_rule_candidates_from_feature_importance_without_rule_library_writeback() {
    let root = temp_root("rule-candidate-mining");
    let validation_report = root.join("validation.json");
    let feature_importance = root.join("feature_importance.parquet");
    write_validation_report(
        &validation_report,
        "0.1.0-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.84,
        0.78,
        0.74,
        "passed",
    );
    write_feature_importance_parquet(
        &feature_importance,
        &[
            ("claim_amount_to_limit_ratio", 0.91),
            ("provider_profile_score", 0.72),
            ("high_cost_item_ratio", 0.53),
            ("service_date_ord", 0.12),
        ],
    );

    let output_dir = root.join("out");
    let plan = mine_rule_candidates(
        &validation_report.to_string_lossy(),
        &feature_importance.to_string_lossy(),
        &output_dir,
    )
    .expect("rule candidate mining");

    assert_eq!(plan.plan_kind, "explainable_model_rule_candidate_mining");
    assert_eq!(plan.source_algorithm, "xgboost");
    assert_eq!(plan.candidate_rules.len(), 3);
    assert_eq!(
        plan.candidate_rules
            .iter()
            .map(|candidate| candidate.source_feature.as_str())
            .collect::<Vec<_>>(),
        vec![
            "claim_amount_to_limit_ratio",
            "provider_profile_score",
            "high_cost_item_ratio"
        ]
    );
    assert!(plan
        .promotion_boundary
        .contains("backtest and human review"));
    assert_eq!(
        plan.candidate_rules[0].gate_status,
        "blocked_until_backtest_and_human_review"
    );
    assert!(plan.candidate_rules[0]
        .required_before_rule_library_writeback
        .contains(&"deterministic_backtest".to_string()));
    assert_eq!(
        plan.candidate_rules[0].draft_rule_template["conditions"][0]["operator"],
        "threshold_selected_by_backtest"
    );
    assert_eq!(
        plan.candidate_rules[0].draft_rule_template["scheme_family"],
        "high_risk_claim"
    );
    assert_eq!(plan.backtest_requests.len(), 3);
    assert_eq!(
        plan.backtest_requests[0].backtest_kind,
        "deterministic_rule_candidate_backtest"
    );
    assert_eq!(plan.review_tasks.len(), 3);
    assert_eq!(
        plan.review_tasks[0].required_review,
        "human_approval_required_before_rule_library_writeback"
    );
    assert!(output_dir.join("rule_candidate_mining_plan.json").is_file());
    assert!(output_dir
        .join("rule_candidate_backtest_requests.json")
        .is_file());
    assert!(output_dir
        .join("rule_candidate_review_tasks.json")
        .is_file());
}

#[test]
fn backtests_rule_candidates_before_rule_library_writeback() {
    let root = temp_root("rule-candidate-backtest");
    let dataset_pack = build_demo_ml_datasets(root.join("datasets"), "2026-06-backtest")
        .expect("demo ML datasets");
    let validation_report = root.join("validation.json");
    let feature_importance = root.join("feature_importance.parquet");
    write_validation_report(
        &validation_report,
        "0.2.0-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.86,
        0.8,
        0.75,
        "passed",
    );
    write_feature_importance_parquet(
        &feature_importance,
        &[
            ("amount_to_limit_ratio", 0.91),
            ("high_cost_item_ratio", 0.72),
            ("provider_risk_tier", 0.53),
        ],
    );
    let mining_dir = root.join("mining");
    mine_rule_candidates(
        &validation_report.to_string_lossy(),
        &feature_importance.to_string_lossy(),
        &mining_dir,
    )
    .expect("rule candidate mining");

    let output_dir = root.join("backtest");
    let report = run_rule_candidate_backtest(
        &mining_dir
            .join("rule_candidate_mining_plan.json")
            .to_string_lossy(),
        &dataset_pack.labeled_manifest_uri,
        &output_dir,
    )
    .expect("rule candidate backtest");

    assert_eq!(report.report_kind, "deterministic_rule_candidate_backtest");
    assert_eq!(report.dataset_key, "rust_demo_claim_risk_labeled");
    assert_eq!(
        report.rule_library_writeback_status,
        "blocked_pending_human_review_and_policy_governance_approval"
    );
    assert_eq!(report.candidate_results.len(), 3);
    assert_eq!(
        report.candidate_results[0].gate_status,
        "backtested_but_blocked_until_human_review"
    );
    assert!(report.candidate_results[0].selected_threshold.is_finite());
    assert_eq!(report.candidate_results[0].selected_operator, ">=");
    assert_eq!(
        report.candidate_results[0].rule_library_writeback_template["conditions"][0]["operator"],
        ">="
    );
    assert!(report.candidate_results[0].condition_refs[0].starts_with("rule_conditions:"));
    assert!(report.candidate_results[0]
        .evidence_refs
        .contains(&report.candidate_results[0].condition_refs[0]));
    assert!(report.candidate_results[0]
        .metrics_by_split
        .contains_key("train"));
    assert!(report.candidate_results[0]
        .metrics_by_split
        .contains_key("validation"));
    assert!(report.candidate_results[0]
        .metrics_by_split
        .contains_key("out_of_time"));
    assert_eq!(report.review_tasks.len(), 3);
    assert_eq!(
        report.review_tasks[0].required_review,
        "human_approval_required_after_backtest_before_rule_library_writeback"
    );
    assert!(output_dir
        .join("rule_candidate_backtest_report.json")
        .is_file());
    assert!(output_dir
        .join("rule_candidate_backtest_review_tasks.json")
        .is_file());
}

#[test]
fn enriches_training_output_with_rule_backtest_handoff_before_fwa_registration() {
    let root = temp_root("training-rule-backtest-handoff");
    let dataset_pack =
        build_demo_ml_datasets(root.join("datasets"), "2026-06-handoff").expect("demo ML datasets");
    let artifact_dir = root.join("artifact");
    fs::create_dir_all(&artifact_dir).unwrap();
    let validation_report = artifact_dir.join("validation.json");
    let feature_importance = artifact_dir.join("feature_importance.parquet");
    write_validation_report(
        &validation_report,
        "0.2.0-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.86,
        0.8,
        0.75,
        "passed",
    );
    write_feature_importance_parquet(
        &feature_importance,
        &[
            ("amount_to_limit_ratio", 0.91),
            ("high_cost_item_ratio", 0.72),
            ("provider_risk_tier", 0.53),
        ],
    );
    let artifact_path = artifact_dir.join("model.onnx");
    fs::write(&artifact_path, b"onnx-placeholder").unwrap();
    let output = CompleteRetrainingJobPayload {
        actor: "trainer-worker".into(),
        notes: "training output".into(),
        candidate_model_version: "0.2.0-xgboost-candidate".into(),
        artifact_uri: artifact_path.to_string_lossy().into_owned(),
        artifact_sha256: Some(test_sha256(&artifact_path)),
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
        validation_report_uri: validation_report.to_string_lossy().into_owned(),
        evaluation_run_id: "eval_baseline_fwa_candidate".into(),
        auc: Some("0.8600".into()),
        ks: None,
        precision: Some("0.8000".into()),
        recall: Some("0.7500".into()),
        f1: None,
        accuracy: None,
        threshold: Some("0.5000".into()),
        confusion_matrix_json: serde_json::json!({}),
        feature_importance_uri: Some(feature_importance.to_string_lossy().into_owned()),
        permutation_importance_uri: None,
        metrics_json: serde_json::json!({}),
        evidence_refs: vec![
            format!("model_artifacts:{}", artifact_path.display()),
            format!("model_validation_reports:{}", validation_report.display()),
            "model_evaluations:eval_baseline_fwa_candidate".into(),
        ],
        mined_rule_owner: Some("external-training-platform".into()),
        mined_rule_candidates: vec![serde_json::json!({
            "rule_id": "candidate_training_amount",
            "version": 1,
            "name": "Training mined amount candidate",
            "scheme_family": "high_risk_claim",
            "conditions": [
                {"field": "amount_to_limit_ratio", "operator": ">=", "value": 0.82}
            ],
            "action": {
                "score": 22,
                "alert_code": "TRAINING_MINED_AMOUNT",
                "recommended_action": "ManualReview",
                "reason": "training mined candidate"
            }
        })],
    };

    let output = enrich_retraining_output_with_rule_candidate_workflow(
        output,
        &dataset_pack.labeled_manifest_uri,
    )
    .expect("rule backtest handoff");

    let report_uri = output.metrics_json["rule_candidate_backtest_report_uri"]
        .as_str()
        .expect("rule candidate backtest report uri");
    let review_tasks_uri = output.metrics_json["rule_candidate_review_tasks_uri"]
        .as_str()
        .expect("rule candidate review tasks uri");
    assert_eq!(
        output.metrics_json["rule_candidate_backtest_status"],
        "passed"
    );
    assert_eq!(output.metrics_json["rule_candidate_review_task_count"], 3);
    assert_eq!(
        output.metrics_json["rule_library_writeback_status"],
        "blocked_pending_human_review_and_policy_governance_approval"
    );
    assert_eq!(
        output.metrics_json["mined_rule_candidates_source"],
        "training_platform_and_deterministic_rule_candidate_backtest"
    );
    assert_eq!(
        output.metrics_json["training_platform_mined_rule_candidate_count"],
        1
    );
    assert_eq!(
        output.metrics_json["mined_rule_candidates_backtested_count"],
        3
    );
    assert!(Path::new(report_uri).is_file());
    assert!(Path::new(review_tasks_uri).is_file());
    assert!(output
        .evidence_refs
        .contains(&format!("rule_candidate_backtests:{report_uri}")));
    assert_eq!(output.mined_rule_candidates.len(), 4);
    assert!(output
        .mined_rule_candidates
        .iter()
        .any(|candidate| candidate["rule_id"] == "candidate_training_amount"));
    assert_eq!(
        output.mined_rule_candidates[1]["conditions"][0]["operator"],
        ">="
    );
    assert!(output.mined_rule_candidates[1]["conditions"][0]["value"]
        .as_f64()
        .expect("backtested rule candidate threshold")
        .is_finite());
    assert_eq!(
        output.mined_rule_candidates[1]["action"]["action_class"],
        "manual_review"
    );
}
