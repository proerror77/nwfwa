use super::*;

#[test]
fn onnx_runtime_requires_passed_parity_report() {
    let root = temp_root("onnx-parity-gate");
    let parity_report = root.join("onnx_parity_report.json");
    write_json(
        parity_report.clone(),
        &serde_json::json!({
            "report_kind": "onnx_probability_parity",
            "status": "passed",
            "serving_runtime_kind": "xgboost_onnx",
            "max_abs_probability_delta": 0.00001,
            "tolerance": 0.0001
        }),
    )
    .unwrap();

    let gate =
        validate_onnx_parity_for_runtime("xgboost_onnx", Some(&parity_report.to_string_lossy()))
            .expect("onnx parity")
            .expect("onnx gate");
    assert_eq!(gate.gate_status, "passed");
    assert_eq!(gate.status, "passed");
    assert_eq!(gate.serving_runtime_kind, "xgboost_onnx");

    let missing = validate_onnx_parity_for_runtime("lightgbm_onnx", None);
    assert!(missing.is_err());
    let deep_learning_missing = validate_onnx_parity_for_runtime("deep_learning_onnx", None);
    assert!(deep_learning_missing.is_err());

    write_json(
        parity_report.clone(),
        &serde_json::json!({
            "report_kind": "onnx_probability_parity",
            "status": "failed",
            "serving_runtime_kind": "xgboost_onnx"
        }),
    )
    .unwrap();
    let blocked =
        validate_onnx_parity_for_runtime("xgboost_onnx", Some(&parity_report.to_string_lossy()))
            .expect("blocked parity")
            .expect("onnx gate");
    assert_eq!(blocked.gate_status, "blocked");
}

#[test]
fn builds_reviewer_approved_model_promotion_orchestration_report() {
    let root = temp_root("model-promotion-orchestration");
    let xgboost_validation = root.join("xgboost-validation.json");
    let lightgbm_validation = root.join("lightgbm-validation.json");
    write_validation_report(
        &xgboost_validation,
        "0.2.0-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.86,
        0.80,
        0.76,
        "passed",
    );
    write_validation_report(
        &lightgbm_validation,
        "0.2.0-lightgbm-candidate",
        "lightgbm",
        "gradient_boosted_tree",
        0.85,
        0.79,
        0.75,
        "passed",
    );
    rank_automl_candidates(
        &[
            xgboost_validation.to_string_lossy().into_owned(),
            lightgbm_validation.to_string_lossy().into_owned(),
        ],
        root.join("ranking"),
    )
    .expect("candidate ranking");
    let artifact_eval = root.join("xgboost-artifact-evaluation.json");
    write_json(
        artifact_eval.clone(),
        &serde_json::json!({
            "report_kind": "model_artifact_evaluation",
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-xgboost-candidate",
            "runtime_kind": "xgboost_onnx",
            "gate_status": "passed",
            "rust_serving_status": "passed",
            "latency_status": "passed"
        }),
    )
    .unwrap();
    let monitoring_report = root.join("mlops-monitoring.json");
    write_json(
        monitoring_report.clone(),
        &serde_json::json!({
            "report_kind": "mlops_monitoring_report",
            "overall_status": "passed",
            "promotion_boundary": "monitoring can open review only; it must not activate models"
        }),
    )
    .unwrap();

    let report = build_model_promotion_orchestration_report(
        &root
            .join("ranking/automl_candidate_ranking.json")
            .to_string_lossy(),
        &[artifact_eval.to_string_lossy().into_owned()],
        &monitoring_report.to_string_lossy(),
        root.join("promotion"),
    )
    .expect("promotion orchestration report");

    assert_eq!(
        report["report_kind"],
        "reviewer_approved_model_promotion_orchestration"
    );
    assert_eq!(
        report["orchestration_status"],
        "ready_after_reviewer_approval"
    );
    assert!(report["activation_policy"]
        .as_str()
        .unwrap()
        .contains("fresh_promotion_gates_pass"));
    assert!(report["required_pre_activation_gates"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("human_model_governance_review_approved")));
    assert!(report["automation_steps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|step| step["step"] == "activate_approved_model_version"));
    assert!(root
        .join("promotion/model_promotion_orchestration_report.json")
        .is_file());
}

#[test]
fn builds_automl_lifecycle_closure_report_from_governed_evidence() {
    let root = temp_root("automl-lifecycle-closure");
    let pack = build_demo_ml_datasets(&root, "2026-06-closure-demo").expect("demo ML datasets");

    let xgboost_validation = root.join("xgboost-validation.json");
    let lightgbm_validation = root.join("lightgbm-validation.json");
    let deep_learning_validation = root.join("deep-learning-validation.json");
    write_validation_report(
        &xgboost_validation,
        "0.2.0-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.86,
        0.80,
        0.76,
        "passed",
    );
    write_validation_report(
        &lightgbm_validation,
        "0.2.0-lightgbm-candidate",
        "lightgbm",
        "gradient_boosted_tree",
        0.85,
        0.79,
        0.75,
        "passed",
    );
    write_validation_report(
        &deep_learning_validation,
        "0.2.0-deep_learning-candidate",
        "deep_learning",
        "deep_learning",
        0.84,
        0.78,
        0.74,
        "passed",
    );
    let ranking = rank_automl_candidates(
        &[
            xgboost_validation.to_string_lossy().into_owned(),
            lightgbm_validation.to_string_lossy().into_owned(),
            deep_learning_validation.to_string_lossy().into_owned(),
        ],
        root.join("ranking"),
    )
    .expect("ranking");
    assert_eq!(
        ranking.recommended_candidate_model_version.as_deref(),
        Some("0.2.0-xgboost-candidate")
    );

    let xgboost_artifact_eval = root.join("xgboost-artifact-evaluation.json");
    let lightgbm_artifact_eval = root.join("lightgbm-artifact-evaluation.json");
    let deep_learning_artifact_eval = root.join("deep-learning-artifact-evaluation.json");
    write_json(
        xgboost_artifact_eval.clone(),
        &serde_json::json!({
            "report_kind": "model_artifact_evaluation",
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-xgboost-candidate",
            "runtime_kind": "xgboost_onnx",
            "gate_status": "passed",
            "rust_serving_status": "passed",
            "latency_status": "passed",
            "p95_latency_ms": 24
        }),
    )
    .unwrap();
    write_json(
        lightgbm_artifact_eval.clone(),
        &serde_json::json!({
            "report_kind": "model_artifact_evaluation",
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-lightgbm-candidate",
            "runtime_kind": "lightgbm_onnx",
            "gate_status": "passed",
            "rust_serving_status": "passed",
            "latency_status": "passed",
            "p95_latency_ms": 21
        }),
    )
    .unwrap();
    write_json(
        deep_learning_artifact_eval.clone(),
        &serde_json::json!({
            "report_kind": "model_artifact_evaluation",
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-deep_learning-candidate",
            "runtime_kind": "deep_learning_sklearn_mlp",
            "gate_status": "passed",
            "rust_serving_status": "passed",
            "latency_status": "passed",
            "p95_latency_ms": 24
        }),
    )
    .unwrap();

    let rule_backtest = root.join("rule-backtest.json");
    write_json(
            rule_backtest.clone(),
            &serde_json::json!({
                "report_kind": "deterministic_rule_candidate_backtest",
                "rule_library_writeback_status": "blocked_pending_human_review_and_policy_governance_approval",
                "candidate_results": [
                    {"candidate_rule_key": "rule_candidate_high_amount", "gate_status": "passed"}
                ],
                "review_tasks": [
                    {"task_kind": "rule_candidate_backtest_review"}
                ]
            }),
        )
        .unwrap();

    let provider_manifest = pack
        .unlabeled_manifest_uris
        .iter()
        .find(|uri| uri.contains("unlabeled_provider_peer_clustering"))
        .expect("provider manifest");
    let provider_cluster_dir = root.join("provider-clusters");
    cluster_provider_peers(provider_manifest, &provider_cluster_dir).expect("provider clustering");
    let provider_graph_dir = root.join("provider-graph");
    cluster_provider_graph_communities(provider_manifest, &provider_graph_dir)
        .expect("provider graph clustering");
    let claim_manifest = pack
        .unlabeled_manifest_uris
        .iter()
        .find(|uri| uri.contains("unlabeled_shadow_scoring"))
        .expect("claim manifest");
    let claim_cluster_dir = root.join("claim-entity-clusters");
    cluster_claim_entities(claim_manifest, &claim_cluster_dir).expect("claim clustering");

    let mlops_monitoring = build_mlops_monitoring_report(
        "baseline_fwa",
        "0.2.0",
        &xgboost_artifact_eval.to_string_lossy(),
        &root.join("shadow.json").to_string_lossy(),
        &root.join("drift.json").to_string_lossy(),
        &root.join("fairness.json").to_string_lossy(),
        root.join("monitoring"),
    );
    assert!(mlops_monitoring.is_err());
    write_json(
        root.join("shadow.json"),
        &serde_json::json!({"status": "passed"}),
    )
    .unwrap();
    write_json(
        root.join("drift.json"),
        &serde_json::json!({"status": "stable"}),
    )
    .unwrap();
    write_json(
        root.join("fairness.json"),
        &serde_json::json!({"status": "passed", "segments": []}),
    )
    .unwrap();
    build_mlops_monitoring_report(
        "baseline_fwa",
        "0.2.0",
        &xgboost_artifact_eval.to_string_lossy(),
        &root.join("shadow.json").to_string_lossy(),
        &root.join("drift.json").to_string_lossy(),
        &root.join("fairness.json").to_string_lossy(),
        root.join("monitoring"),
    )
    .expect("monitoring report");
    let monitoring_plan = build_mlops_monitoring_plan(
        &pack.labeled_manifest_uri,
        &root.join("rust_serving_artifact.json").to_string_lossy(),
        "baseline_fwa",
        "0.2.0",
        "0 2 * * *",
    )
    .expect("monitoring plan");
    let monitoring_plan_uri = root.join("monitoring-plan.json");
    write_json(monitoring_plan_uri.clone(), &monitoring_plan).unwrap();
    build_mlops_scheduler_execution_report(
        &monitoring_plan_uri.to_string_lossy(),
        &root
            .join("monitoring/mlops_monitoring_report.json")
            .to_string_lossy(),
        root.join("scheduler"),
    )
    .expect("scheduler execution report");
    build_mlops_monitoring_cycle_evidence(
        &monitoring_plan_uri.to_string_lossy(),
        &xgboost_artifact_eval.to_string_lossy(),
        &root.join("shadow.json").to_string_lossy(),
        &root.join("drift.json").to_string_lossy(),
        &root.join("fairness.json").to_string_lossy(),
        root.join("cycle"),
    )
    .expect("monitoring cycle report");
    let promotion_orchestration_dir = root.join("promotion-orchestration");
    build_model_promotion_orchestration_report(
        &root
            .join("ranking/automl_candidate_ranking.json")
            .to_string_lossy(),
        &[
            xgboost_artifact_eval.to_string_lossy().into_owned(),
            lightgbm_artifact_eval.to_string_lossy().into_owned(),
            deep_learning_artifact_eval.to_string_lossy().into_owned(),
        ],
        &root
            .join("monitoring/mlops_monitoring_report.json")
            .to_string_lossy(),
        &promotion_orchestration_dir,
    )
    .expect("promotion orchestration report");

    let report = build_automl_lifecycle_closure_report(
        &root.join("index.json").to_string_lossy(),
        &root
            .join("ranking/automl_candidate_ranking.json")
            .to_string_lossy(),
        &[
            xgboost_artifact_eval.to_string_lossy().into_owned(),
            lightgbm_artifact_eval.to_string_lossy().into_owned(),
            deep_learning_artifact_eval.to_string_lossy().into_owned(),
        ],
        &rule_backtest.to_string_lossy(),
        &provider_cluster_dir
            .join("provider_peer_clustering_report.json")
            .to_string_lossy(),
        &provider_graph_dir
            .join("provider_graph_community_report.json")
            .to_string_lossy(),
        &claim_cluster_dir
            .join("claim_entity_clustering_report.json")
            .to_string_lossy(),
        &root
            .join("monitoring/mlops_monitoring_report.json")
            .to_string_lossy(),
        &root
            .join("scheduler/mlops_scheduler_execution_report.json")
            .to_string_lossy(),
        &root
            .join("cycle/mlops_monitoring_cycle_report.json")
            .to_string_lossy(),
        &promotion_orchestration_dir
            .join("model_promotion_orchestration_report.json")
            .to_string_lossy(),
        root.join("closure"),
    )
    .expect("lifecycle closure report");

    assert_eq!(report["report_kind"], "rust_automl_lifecycle_closure");
    assert_eq!(
        report["closure_status"],
        "closed_with_human_governance_gates"
    );
    assert_eq!(report["lifecycle_stages"].as_array().unwrap().len(), 7);
    assert!(report["lifecycle_stages"]
        .as_array()
        .unwrap()
        .iter()
        .all(|stage| stage["status"] == "passed"));
    assert!(report["governance_boundary"]
        .as_str()
        .unwrap()
        .contains("must not auto-activate models"));
    let clustering_stage = report["lifecycle_stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|stage| stage["stage"] == "unlabeled_clustering_reviews")
        .expect("clustering stage");
    assert!(clustering_stage["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|evidence_ref| evidence_ref
            .as_str()
            .unwrap()
            .starts_with("provider_graph_clustering:")));
    let monitoring_stage = report["lifecycle_stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|stage| stage["stage"] == "mlops_monitoring_loop")
        .expect("monitoring stage");
    assert!(monitoring_stage["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|evidence_ref| evidence_ref
            .as_str()
            .unwrap()
            .starts_with("mlops_scheduler_execution_reports:")));
    let promotion_stage = report["lifecycle_stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|stage| stage["stage"] == "reviewer_approved_promotion_orchestration")
        .expect("promotion orchestration stage");
    assert_eq!(promotion_stage["status"], "passed");
    assert!(root
        .join("closure/rust_automl_lifecycle_closure_report.json")
        .is_file());
}

#[test]
fn builds_demo_automl_lifecycle_evidence_pack() {
    let root = temp_root("demo-automl-lifecycle-evidence");
    let demo_root = root.join("demo");
    build_demo_ml_datasets(&demo_root, "2026-06-rust-automl-demo").expect("demo ML datasets");
    let output_dir = root.join("lifecycle-evidence");

    let index = build_demo_automl_lifecycle_evidence(&demo_root, &output_dir)
        .expect("demo lifecycle evidence");

    assert_eq!(
        index["evidence_pack_kind"],
        "rust_automl_demo_lifecycle_evidence"
    );
    assert_eq!(
        index["closure_status"],
        "closed_with_human_governance_gates"
    );
    assert_eq!(
        index["recommended_candidate_model_version"],
        "0.2.0-xgboost-candidate"
    );
    assert!(output_dir
        .join("ranking/automl_candidate_ranking.json")
        .is_file());
    assert!(output_dir
        .join("validation/deep_learning_validation.json")
        .is_file());
    assert!(output_dir
        .join("artifact-evaluation/deep_learning_model_artifact_evaluation.json")
        .is_file());
    assert!(output_dir
        .join("rule-candidates/backtest/rule_candidate_backtest_report.json")
        .is_file());
    assert!(output_dir
        .join("clustering/provider-peer/provider_peer_clustering_report.json")
        .is_file());
    assert!(output_dir
        .join("clustering/provider-graph/provider_graph_community_report.json")
        .is_file());
    assert!(output_dir
        .join("clustering/claim-entity/claim_entity_clustering_report.json")
        .is_file());
    assert!(output_dir
        .join("monitoring/mlops_monitoring_report.json")
        .is_file());
    assert!(output_dir
        .join("monitoring/scheduler/mlops_monitoring_plan.json")
        .is_file());
    assert!(output_dir
        .join("monitoring/scheduler/mlops_scheduler_execution_report.json")
        .is_file());
    assert!(output_dir
        .join("monitoring/scheduler/mlops_alert_delivery_tasks.json")
        .is_file());
    assert!(output_dir
        .join("monitoring/cycle/mlops_monitoring_cycle_report.json")
        .is_file());
    assert!(output_dir
        .join("promotion-orchestration/model_promotion_orchestration_report.json")
        .is_file());
    assert!(output_dir
        .join("closure/rust_automl_lifecycle_closure_report.json")
        .is_file());
    assert!(output_dir
        .join("demo_lifecycle_evidence_index.json")
        .is_file());
}

#[test]
fn verifies_demo_automl_lifecycle_evidence_pack() {
    let root = temp_root("verify-demo-automl-lifecycle");
    let demo_root = root.join("demo");
    let evidence_dir = root.join("lifecycle-evidence");
    let verification_dir = root.join("verification");
    build_demo_ml_datasets(&demo_root, "2026-06-rust-automl-demo").expect("demo ML datasets");
    build_demo_automl_lifecycle_evidence(&demo_root, &evidence_dir)
        .expect("demo lifecycle evidence");

    let report = verify_demo_automl_lifecycle(&demo_root, &evidence_dir, &verification_dir)
        .expect("verification report");

    assert_eq!(
        report["report_kind"],
        "rust_automl_demo_lifecycle_verification"
    );
    assert_eq!(report["verification_status"], "passed");
    assert!(report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .all(|check| check["status"] == "passed"));
    assert!(verification_dir
        .join("rust_automl_lifecycle_verification_report.json")
        .is_file());
}

#[test]
fn demo_automl_lifecycle_verification_blocks_labeled_unlabeled_manifest() {
    let root = temp_root("verify-demo-automl-lifecycle-blocked");
    let demo_root = root.join("demo");
    let evidence_dir = root.join("lifecycle-evidence");
    build_demo_ml_datasets(&demo_root, "2026-06-rust-automl-demo").expect("demo ML datasets");
    build_demo_automl_lifecycle_evidence(&demo_root, &evidence_dir)
        .expect("demo lifecycle evidence");
    let shadow_manifest_uri = demo_root.join("unlabeled_shadow_scoring/manifest.json");
    let mut shadow_manifest =
        read_json_report(&shadow_manifest_uri.to_string_lossy()).expect("shadow manifest");
    shadow_manifest["label_column"] = serde_json::json!("confirmed_fwa");
    write_json(shadow_manifest_uri, &shadow_manifest).expect("write polluted manifest");

    let report = verify_demo_automl_lifecycle(&demo_root, &evidence_dir, root.join("verification"))
        .expect("verification report");

    assert_eq!(report["verification_status"], "blocked");
    assert!(report["blocking_reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason
            .as_str()
            .unwrap()
            .contains("unlabeled_dataset_boundaries")));
}
