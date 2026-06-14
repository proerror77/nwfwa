use anyhow::{bail, Context};
use arrow_array::{Float64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use std::{collections::BTreeSet, fs, path::Path, sync::Arc};

use super::{
    build_automl_lifecycle_closure_report, build_mlops_monitoring_cycle_evidence,
    build_mlops_monitoring_plan, build_mlops_monitoring_report,
    build_mlops_scheduler_execution_report, build_model_promotion_orchestration_report,
    cluster_claim_entities, cluster_provider_graph_communities, cluster_provider_peers,
    json_array_len, json_string, mine_rule_candidates, nested_json_array_contains,
    nested_json_string, rank_automl_candidates, read_json_report, run_mlops_monitoring_plan,
    run_rule_candidate_backtest, write_json, write_parquet,
};

pub fn build_demo_automl_lifecycle_evidence(
    demo_root: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let demo_root = demo_root.as_ref();
    let output_dir = output_dir.as_ref();
    let index_uri = demo_root.join("index.json");
    let labeled_manifest = demo_root.join("labeled_claim_risk/manifest.json");
    let provider_manifest = demo_root.join("unlabeled_provider_peer_clustering/manifest.json");
    let claim_manifest = demo_root.join("unlabeled_shadow_scoring/manifest.json");
    for required_file in [
        &index_uri,
        &labeled_manifest,
        &provider_manifest,
        &claim_manifest,
    ] {
        if !required_file.is_file() {
            bail!(
                "demo lifecycle evidence requires existing demo file {}",
                required_file.display()
            );
        }
    }
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "create demo lifecycle evidence dir {}",
            output_dir.display()
        )
    })?;

    let validation_dir = output_dir.join("validation");
    fs::create_dir_all(&validation_dir)
        .with_context(|| format!("create validation dir {}", validation_dir.display()))?;
    let xgboost_validation = validation_dir.join("xgboost_validation.json");
    let lightgbm_validation = validation_dir.join("lightgbm_validation.json");
    let deep_learning_validation = validation_dir.join("deep_learning_validation.json");
    write_demo_validation_report(
        &xgboost_validation,
        "0.2.0-xgboost-candidate",
        "xgboost",
        "xgboost_onnx",
        0.86,
        0.80,
        0.76,
    )?;
    write_demo_validation_report(
        &lightgbm_validation,
        "0.2.0-lightgbm-candidate",
        "lightgbm",
        "lightgbm_onnx",
        0.85,
        0.79,
        0.75,
    )?;
    write_demo_validation_report(
        &deep_learning_validation,
        "0.2.0-deep_learning-candidate",
        "deep_learning",
        "deep_learning_sklearn_mlp",
        0.84,
        0.78,
        0.74,
    )?;

    let feature_importance_dir = output_dir.join("feature-importance");
    fs::create_dir_all(&feature_importance_dir).with_context(|| {
        format!(
            "create feature importance dir {}",
            feature_importance_dir.display()
        )
    })?;
    let feature_importance_uri = feature_importance_dir.join("feature_importance.parquet");
    write_demo_feature_importance_parquet(&feature_importance_uri)?;

    let ranking = rank_automl_candidates(
        &[
            xgboost_validation.to_string_lossy().into_owned(),
            lightgbm_validation.to_string_lossy().into_owned(),
            deep_learning_validation.to_string_lossy().into_owned(),
        ],
        output_dir.join("ranking"),
    )?;

    let artifact_eval_dir = output_dir.join("artifact-evaluation");
    fs::create_dir_all(&artifact_eval_dir).with_context(|| {
        format!(
            "create artifact evaluation dir {}",
            artifact_eval_dir.display()
        )
    })?;
    let xgboost_artifact_eval = artifact_eval_dir.join("xgboost_model_artifact_evaluation.json");
    let lightgbm_artifact_eval = artifact_eval_dir.join("lightgbm_model_artifact_evaluation.json");
    let deep_learning_artifact_eval =
        artifact_eval_dir.join("deep_learning_model_artifact_evaluation.json");
    write_demo_artifact_evaluation_report(
        &xgboost_artifact_eval,
        "0.2.0-xgboost-candidate",
        "xgboost_onnx",
        24,
    )?;
    write_demo_artifact_evaluation_report(
        &lightgbm_artifact_eval,
        "0.2.0-lightgbm-candidate",
        "lightgbm_onnx",
        21,
    )?;
    write_demo_artifact_evaluation_report(
        &deep_learning_artifact_eval,
        "0.2.0-deep_learning-candidate",
        "deep_learning_sklearn_mlp",
        24,
    )?;

    let rule_candidate_dir = output_dir.join("rule-candidates");
    mine_rule_candidates(
        &xgboost_validation.to_string_lossy(),
        &feature_importance_uri.to_string_lossy(),
        &rule_candidate_dir,
    )?;
    let rule_backtest_dir = rule_candidate_dir.join("backtest");
    run_rule_candidate_backtest(
        &rule_candidate_dir
            .join("rule_candidate_mining_plan.json")
            .to_string_lossy(),
        &labeled_manifest.to_string_lossy(),
        &rule_backtest_dir,
    )?;

    let provider_cluster_dir = output_dir.join("clustering/provider-peer");
    let provider_clustering =
        cluster_provider_peers(&provider_manifest.to_string_lossy(), &provider_cluster_dir)?;
    let provider_graph_dir = output_dir.join("clustering/provider-graph");
    let provider_graph = cluster_provider_graph_communities(
        &provider_manifest.to_string_lossy(),
        &provider_graph_dir,
    )?;
    let claim_cluster_dir = output_dir.join("clustering/claim-entity");
    let claim_clustering =
        cluster_claim_entities(&claim_manifest.to_string_lossy(), &claim_cluster_dir)?;

    let monitoring_inputs_dir = output_dir.join("monitoring-inputs");
    fs::create_dir_all(&monitoring_inputs_dir).with_context(|| {
        format!(
            "create monitoring input dir {}",
            monitoring_inputs_dir.display()
        )
    })?;
    let shadow_report = monitoring_inputs_dir.join("shadow_report.json");
    let drift_report = monitoring_inputs_dir.join("drift_report.json");
    let fairness_report = monitoring_inputs_dir.join("fairness_report.json");
    let scheduler_dir = output_dir.join("monitoring/scheduler");
    fs::create_dir_all(&scheduler_dir)
        .with_context(|| format!("create MLOps scheduler dir {}", scheduler_dir.display()))?;
    let monitoring_plan = build_mlops_monitoring_plan(
        &labeled_manifest.to_string_lossy(),
        &monitoring_inputs_dir
            .join("rust_serving_artifact.json")
            .to_string_lossy(),
        "baseline_fwa",
        "0.2.0-xgboost-candidate",
        "0 2 * * *",
    )?;
    let monitoring_plan_uri = scheduler_dir.join("mlops_monitoring_plan.json");
    write_json(monitoring_plan_uri.clone(), &monitoring_plan)?;
    run_mlops_monitoring_plan(
        &monitoring_plan_uri.to_string_lossy(),
        &monitoring_inputs_dir,
    )?;
    build_mlops_monitoring_report(
        "baseline_fwa",
        "0.2.0-xgboost-candidate",
        &xgboost_artifact_eval.to_string_lossy(),
        &shadow_report.to_string_lossy(),
        &drift_report.to_string_lossy(),
        &fairness_report.to_string_lossy(),
        output_dir.join("monitoring"),
    )?;
    build_mlops_scheduler_execution_report(
        &monitoring_plan_uri.to_string_lossy(),
        &output_dir
            .join("monitoring/mlops_monitoring_report.json")
            .to_string_lossy(),
        &scheduler_dir,
    )?;
    build_mlops_monitoring_cycle_evidence(
        &monitoring_plan_uri.to_string_lossy(),
        &xgboost_artifact_eval.to_string_lossy(),
        &shadow_report.to_string_lossy(),
        &drift_report.to_string_lossy(),
        &fairness_report.to_string_lossy(),
        output_dir.join("monitoring/cycle"),
    )?;
    let promotion_orchestration_dir = output_dir.join("promotion-orchestration");
    build_model_promotion_orchestration_report(
        &output_dir
            .join("ranking/automl_candidate_ranking.json")
            .to_string_lossy(),
        &[
            xgboost_artifact_eval.to_string_lossy().into_owned(),
            lightgbm_artifact_eval.to_string_lossy().into_owned(),
            deep_learning_artifact_eval.to_string_lossy().into_owned(),
        ],
        &output_dir
            .join("monitoring/mlops_monitoring_report.json")
            .to_string_lossy(),
        &promotion_orchestration_dir,
    )?;

    let closure = build_automl_lifecycle_closure_report(
        &index_uri.to_string_lossy(),
        &output_dir
            .join("ranking/automl_candidate_ranking.json")
            .to_string_lossy(),
        &[
            xgboost_artifact_eval.to_string_lossy().into_owned(),
            lightgbm_artifact_eval.to_string_lossy().into_owned(),
            deep_learning_artifact_eval.to_string_lossy().into_owned(),
        ],
        &rule_backtest_dir
            .join("rule_candidate_backtest_report.json")
            .to_string_lossy(),
        &provider_cluster_dir
            .join("provider_peer_clustering_report.json")
            .to_string_lossy(),
        &provider_graph_dir
            .join("provider_graph_community_report.json")
            .to_string_lossy(),
        &claim_cluster_dir
            .join("claim_entity_clustering_report.json")
            .to_string_lossy(),
        &output_dir
            .join("monitoring/mlops_monitoring_report.json")
            .to_string_lossy(),
        &scheduler_dir
            .join("mlops_scheduler_execution_report.json")
            .to_string_lossy(),
        &output_dir
            .join("monitoring/cycle/mlops_monitoring_cycle_report.json")
            .to_string_lossy(),
        &promotion_orchestration_dir
            .join("model_promotion_orchestration_report.json")
            .to_string_lossy(),
        output_dir.join("closure"),
    )?;

    let index = serde_json::json!({
        "evidence_pack_kind": "rust_automl_demo_lifecycle_evidence",
        "evidence_pack_version": 1,
        "demo_root": demo_root.to_string_lossy(),
        "output_dir": output_dir.to_string_lossy(),
        "recommended_candidate_model_version": ranking.recommended_candidate_model_version,
        "provider_anomaly_candidate_count": provider_clustering.anomaly_candidates.len(),
        "provider_graph_anomaly_candidate_count": provider_graph.anomaly_candidates.len(),
        "claim_entity_anomaly_candidate_count": claim_clustering.anomaly_candidates.len(),
        "closure_status": closure["closure_status"],
        "artifacts": {
            "xgboost_validation_report": xgboost_validation,
            "lightgbm_validation_report": lightgbm_validation,
            "deep_learning_validation_report": deep_learning_validation,
            "candidate_ranking": output_dir.join("ranking/automl_candidate_ranking.json"),
            "xgboost_artifact_evaluation": xgboost_artifact_eval,
            "lightgbm_artifact_evaluation": lightgbm_artifact_eval,
            "deep_learning_artifact_evaluation": deep_learning_artifact_eval,
            "feature_importance": feature_importance_uri,
            "rule_candidate_plan": rule_candidate_dir.join("rule_candidate_mining_plan.json"),
            "rule_backtest_report": rule_backtest_dir.join("rule_candidate_backtest_report.json"),
            "provider_clustering_report": provider_cluster_dir.join("provider_peer_clustering_report.json"),
            "provider_graph_report": provider_graph_dir.join("provider_graph_community_report.json"),
            "claim_entity_clustering_report": claim_cluster_dir.join("claim_entity_clustering_report.json"),
            "mlops_monitoring_report": output_dir.join("monitoring/mlops_monitoring_report.json"),
            "mlops_monitoring_plan": monitoring_plan_uri,
            "mlops_scheduler_execution_report": scheduler_dir.join("mlops_scheduler_execution_report.json"),
            "mlops_monitoring_cycle_report": output_dir.join("monitoring/cycle/mlops_monitoring_cycle_report.json"),
            "model_promotion_orchestration_report": promotion_orchestration_dir.join("model_promotion_orchestration_report.json"),
            "lifecycle_closure_report": output_dir.join("closure/rust_automl_lifecycle_closure_report.json")
        },
        "governance_boundary": "demo evidence only; the pack proves the Rust AutoML lifecycle contract but must not activate models, assign fraud labels, or write rules without the recorded human gates"
    });
    write_json(
        output_dir.join("demo_lifecycle_evidence_index.json"),
        &index,
    )?;
    Ok(index)
}

pub fn verify_demo_automl_lifecycle(
    demo_root: impl AsRef<Path>,
    evidence_dir: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let demo_root = demo_root.as_ref();
    let evidence_dir = evidence_dir.as_ref();
    let demo_index_uri = demo_root.join("index.json");
    let evidence_index_uri = evidence_dir.join("demo_lifecycle_evidence_index.json");

    let demo_index = read_json_report(&demo_index_uri.to_string_lossy())?;
    let evidence_index = read_json_report(&evidence_index_uri.to_string_lossy())?;
    let artifacts = evidence_index
        .get("artifacts")
        .and_then(|value| value.as_object())
        .context("demo lifecycle evidence index requires artifacts")?;

    let mut checks = Vec::new();
    let mut blocking_reasons = Vec::new();

    let dataset_manifests = demo_index
        .get("dataset_manifests")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let labeled_datasets = dataset_manifests
        .iter()
        .filter(|dataset| json_string(dataset, "label_column").is_some())
        .collect::<Vec<_>>();
    let unlabeled_datasets = dataset_manifests
        .iter()
        .filter(|dataset| json_string(dataset, "label_column").is_none())
        .collect::<Vec<_>>();
    push_verification_check(
        &mut checks,
        &mut blocking_reasons,
        "demo_dataset_portfolio",
        demo_index["pack_kind"] == "rust_automl_demo_datasets"
            && labeled_datasets.len() == 1
            && unlabeled_datasets.len() >= 2,
        format!(
            "{} labeled dataset(s), {} unlabeled dataset(s)",
            labeled_datasets.len(),
            unlabeled_datasets.len()
        ),
        vec![format!("demo_dataset_index:{}", demo_index_uri.display())],
    );

    let labeled_manifest_uri =
        json_string(&demo_index, "labeled_manifest_uri").context("missing labeled_manifest_uri")?;
    let labeled_manifest = read_json_report(&labeled_manifest_uri)?;
    let labeled_split_names = labeled_manifest
        .get("splits")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|split| json_string(split, "split_name"))
        .collect::<BTreeSet<_>>();
    push_verification_check(
        &mut checks,
        &mut blocking_reasons,
        "labeled_supervised_dataset",
        json_string(&labeled_manifest, "label_column").as_deref() == Some("confirmed_fwa")
            && json_string(&labeled_manifest, "label_policy")
                .is_some_and(|policy| policy.contains("weak_rust_demo_label"))
            && labeled_split_names.contains("train")
            && labeled_split_names.contains("validation")
            && labeled_split_names.contains("out_of_time"),
        "labeled claim-risk manifest carries confirmed_fwa and train/validation/out_of_time splits"
            .into(),
        vec![format!("dataset_manifest:{labeled_manifest_uri}")],
    );

    let unlabeled_manifest_uris = demo_index
        .get("unlabeled_manifest_uris")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>();
    let unlabeled_contract_passed = unlabeled_manifest_uris.len() >= 2
        && unlabeled_manifest_uris.iter().all(|uri| {
            read_json_report(uri).is_ok_and(|manifest| {
                json_string(&manifest, "label_column").is_none()
                    && json_string(&manifest, "label_policy")
                        .is_some_and(|policy| policy.starts_with("unlabeled_"))
                    && nested_json_array_contains(
                        &manifest,
                        &["governance", "blocked_uses"],
                        "supervised_training",
                    )
            })
        });
    push_verification_check(
        &mut checks,
        &mut blocking_reasons,
        "unlabeled_dataset_boundaries",
        unlabeled_contract_passed,
        "unlabeled manifests carry no label column and block supervised training".into(),
        unlabeled_manifest_uris
            .iter()
            .map(|uri| format!("dataset_manifest:{uri}"))
            .collect(),
    );

    let xgboost_validation_uri = required_artifact_uri(artifacts, "xgboost_validation_report")?;
    let lightgbm_validation_uri = required_artifact_uri(artifacts, "lightgbm_validation_report")?;
    let deep_learning_validation_uri =
        required_artifact_uri(artifacts, "deep_learning_validation_report")?;
    let xgboost_validation = read_json_report(&xgboost_validation_uri)?;
    let lightgbm_validation = read_json_report(&lightgbm_validation_uri)?;
    let deep_learning_validation = read_json_report(&deep_learning_validation_uri)?;
    push_verification_check(
        &mut checks,
        &mut blocking_reasons,
        "supervised_algorithm_portfolio",
        validation_report_matches(&xgboost_validation, "xgboost", "xgboost_onnx")
            && validation_report_matches(&lightgbm_validation, "lightgbm", "lightgbm_onnx")
            && deep_learning_validation_report_matches(&deep_learning_validation),
        "XGBoost, LightGBM, and deep learning validation reports carry governed runtime evidence"
            .into(),
        vec![
            format!("model_validation_reports:{xgboost_validation_uri}"),
            format!("model_validation_reports:{lightgbm_validation_uri}"),
            format!("model_validation_reports:{deep_learning_validation_uri}"),
        ],
    );

    let xgboost_artifact_uri = required_artifact_uri(artifacts, "xgboost_artifact_evaluation")?;
    let lightgbm_artifact_uri = required_artifact_uri(artifacts, "lightgbm_artifact_evaluation")?;
    let deep_learning_artifact_uri =
        required_artifact_uri(artifacts, "deep_learning_artifact_evaluation")?;
    let xgboost_artifact = read_json_report(&xgboost_artifact_uri)?;
    let lightgbm_artifact = read_json_report(&lightgbm_artifact_uri)?;
    let deep_learning_artifact = read_json_report(&deep_learning_artifact_uri)?;
    push_verification_check(
        &mut checks,
        &mut blocking_reasons,
        "rust_onnx_serving_gate",
        artifact_evaluation_matches(&xgboost_artifact, "xgboost_onnx")
            && artifact_evaluation_matches(&lightgbm_artifact, "lightgbm_onnx")
            && artifact_evaluation_matches(&deep_learning_artifact, "deep_learning_sklearn_mlp"),
        "XGBoost, LightGBM, and deep learning artifact evaluations pass serving gates".into(),
        vec![
            format!("model_artifact_evaluations:{xgboost_artifact_uri}"),
            format!("model_artifact_evaluations:{lightgbm_artifact_uri}"),
            format!("model_artifact_evaluations:{deep_learning_artifact_uri}"),
        ],
    );

    let rule_backtest_uri = required_artifact_uri(artifacts, "rule_backtest_report")?;
    let rule_backtest = read_json_report(&rule_backtest_uri)?;
    push_verification_check(
        &mut checks,
        &mut blocking_reasons,
        "rule_candidate_backtest_before_writeback",
        rule_backtest["report_kind"] == "deterministic_rule_candidate_backtest"
            && json_string(&rule_backtest, "rule_library_writeback_status")
                .is_some_and(|status| status.contains("blocked_pending_human_review"))
            && json_array_len(&rule_backtest, "review_tasks") > 0,
        "explainable rule candidates are backtested and blocked before rule-library writeback"
            .into(),
        vec![format!("rule_candidate_backtests:{rule_backtest_uri}")],
    );

    let provider_clustering_uri = required_artifact_uri(artifacts, "provider_clustering_report")?;
    let provider_graph_uri = required_artifact_uri(artifacts, "provider_graph_report")?;
    let claim_entity_uri = required_artifact_uri(artifacts, "claim_entity_clustering_report")?;
    let provider_clustering = read_json_report(&provider_clustering_uri)?;
    let provider_graph = read_json_report(&provider_graph_uri)?;
    let claim_entity = read_json_report(&claim_entity_uri)?;
    push_verification_check(
        &mut checks,
        &mut blocking_reasons,
        "unlabeled_clustering_review_only",
        clustering_report_matches(
            &provider_clustering,
            "provider_peer_clustering",
            "must not create confirmed FWA labels",
        ) && clustering_report_matches(
            &provider_graph,
            "provider_graph_community_clustering",
            "must not create confirmed FWA labels",
        ) && clustering_report_matches(
            &claim_entity,
            "claim_entity_clustering",
            "rule-library writeback",
        ),
        "provider-peer, graph-community, and claim-entity clustering create review candidates only"
            .into(),
        vec![
            format!("provider_peer_clustering:{provider_clustering_uri}"),
            format!("provider_graph_clustering:{provider_graph_uri}"),
            format!("claim_entity_clustering:{claim_entity_uri}"),
        ],
    );

    let monitoring_report_uri = required_artifact_uri(artifacts, "mlops_monitoring_report")?;
    let scheduler_report_uri =
        required_artifact_uri(artifacts, "mlops_scheduler_execution_report")?;
    let monitoring_cycle_uri = required_artifact_uri(artifacts, "mlops_monitoring_cycle_report")?;
    let monitoring_report = read_json_report(&monitoring_report_uri)?;
    let scheduler_report = read_json_report(&scheduler_report_uri)?;
    let monitoring_cycle = read_json_report(&monitoring_cycle_uri)?;
    push_verification_check(
        &mut checks,
        &mut blocking_reasons,
        "auto_mlops_monitoring_loop",
        monitoring_report["report_kind"] == "mlops_monitoring_report"
            && json_string(&monitoring_report, "promotion_boundary")
                .is_some_and(|boundary| boundary.contains("must not activate models"))
            && scheduler_report["report_kind"] == "mlops_scheduler_execution_report"
            && json_string(&scheduler_report, "governance_boundary")
                .is_some_and(|boundary| boundary.contains("must not create retraining jobs"))
            && monitoring_cycle["report_kind"] == "mlops_monitoring_cycle_execution"
            && json_string(&monitoring_cycle, "governance_boundary")
                .is_some_and(|boundary| boundary.contains("must not create retraining jobs")),
        "monitoring, scheduler, and cycle reports are evidence-only and do not perform lifecycle actions"
            .into(),
        vec![
            format!("mlops_monitoring_reports:{monitoring_report_uri}"),
            format!("mlops_scheduler_execution_reports:{scheduler_report_uri}"),
            format!("mlops_monitoring_cycles:{monitoring_cycle_uri}"),
        ],
    );

    let promotion_orchestration_uri =
        required_artifact_uri(artifacts, "model_promotion_orchestration_report")?;
    let promotion_orchestration = read_json_report(&promotion_orchestration_uri)?;
    push_verification_check(
        &mut checks,
        &mut blocking_reasons,
        "reviewer_approved_promotion_orchestration",
        promotion_orchestration["report_kind"] == "reviewer_approved_model_promotion_orchestration"
            && json_string(&promotion_orchestration, "orchestration_status").as_deref()
                == Some("ready_after_reviewer_approval")
            && json_string(&promotion_orchestration, "activation_policy")
                .is_some_and(|policy| policy.contains("fresh_promotion_gates_pass"))
            && nested_json_array_contains(
                &promotion_orchestration,
                &["required_pre_activation_gates"],
                "human_model_governance_review_approved",
            ),
        "promotion orchestration is ready only after reviewer approval and fresh gate recheck"
            .into(),
        vec![format!(
            "model_promotion_orchestrations:{promotion_orchestration_uri}"
        )],
    );

    let closure_report_uri = required_artifact_uri(artifacts, "lifecycle_closure_report")?;
    let closure_report = read_json_report(&closure_report_uri)?;
    let closure_stage_count = closure_report
        .get("lifecycle_stages")
        .and_then(|value| value.as_array())
        .map(|stages| stages.len())
        .unwrap_or(0);
    let closure_stages_passed = closure_report
        .get("lifecycle_stages")
        .and_then(|value| value.as_array())
        .is_some_and(|stages| stages.iter().all(|stage| stage["status"] == "passed"));
    push_verification_check(
        &mut checks,
        &mut blocking_reasons,
        "lifecycle_closure_report",
        closure_report["report_kind"] == "rust_automl_lifecycle_closure"
            && closure_report["closure_status"] == "closed_with_human_governance_gates"
            && closure_stages_passed
            && closure_stage_count >= 7
            && json_array_len(&closure_report, "required_human_gates") >= 4,
        format!("{closure_stage_count} lifecycle stage(s) closed with human governance gates"),
        vec![format!("automl_lifecycle_closure:{closure_report_uri}")],
    );

    let verification_status = if blocking_reasons.is_empty() {
        "passed"
    } else {
        "blocked"
    };
    let report = serde_json::json!({
        "report_kind": "rust_automl_demo_lifecycle_verification",
        "report_version": 1,
        "verification_status": verification_status,
        "demo_root": demo_root.to_string_lossy(),
        "evidence_dir": evidence_dir.to_string_lossy(),
        "checks": checks,
        "blocking_reasons": blocking_reasons,
        "evidence_refs": [
            format!("demo_dataset_index:{}", demo_index_uri.display()),
            format!("demo_lifecycle_evidence_index:{}", evidence_index_uri.display()),
            format!("automl_lifecycle_closure:{closure_report_uri}")
        ],
        "governance_boundary": "verification proves the Rust Auto MLOps demo lifecycle evidence pack only; it must not activate models, assign labels, or publish rules"
    });
    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create Auto MLOps lifecycle verification output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("rust_automl_lifecycle_verification_report.json"),
        &report,
    )?;
    Ok(report)
}

fn write_demo_validation_report(
    path: &Path,
    candidate_model_version: &str,
    algorithm: &str,
    runtime_kind: &str,
    auc: f64,
    precision: f64,
    recall: f64,
) -> anyhow::Result<()> {
    let onnx_runtime = matches!(algorithm, "xgboost" | "lightgbm");
    let algorithm_family = if algorithm == "deep_learning" {
        "deep_learning"
    } else {
        "gradient_boosted_tree"
    };
    let validation_metrics = serde_json::json!({
        "auc": auc,
        "precision": precision,
        "recall": recall
    });
    let metrics_json = serde_json::json!({
        "algorithm": algorithm,
        "algorithm_family": algorithm_family,
        "runtime_kind": runtime_kind,
        "out_of_time_auc": auc - 0.02,
        "out_of_time_average_precision": auc - 0.05,
        "out_of_time_precision": precision,
        "out_of_time_recall": recall,
        "time_group_split_status": "passed",
        "time_split_field": "service_month",
        "group_split_fields": ["provider_id", "member_id"],
        "leakage_check_status": "passed",
        "out_of_time_validation_status": "passed",
        "score_stability_status": "passed",
        "feature_stability_status": "passed",
        "score_psi": 0.03,
        "max_feature_psi": 0.08,
        "overfitting_diagnostics_status": "passed",
        "overfitting_diagnostics_report_uri": format!("data/rust-automl-demo/lifecycle-evidence/diagnostics/{candidate_model_version}_overfitting_diagnostics_report.json"),
        "automl_feature_search_status": "passed",
        "automl_feature_search_report_uri": format!("data/rust-automl-demo/lifecycle-evidence/feature-search/{candidate_model_version}_automl_feature_search_report.json"),
        "automl_selected_feature_count": 8,
        "automl_factor_ranking_status": "passed",
        "automl_factor_ranking_report_uri": format!("data/rust-automl-demo/lifecycle-evidence/factor-ranking/{candidate_model_version}_automl_factor_ranking_report.json"),
        "automl_ranked_factor_count": 5,
        "permutation_importance_status": "passed",
        "permutation_importance_uri": format!("data/rust-automl-demo/lifecycle-evidence/permutation/{candidate_model_version}_permutation_importance.json"),
        "feature_reproducibility_hash": format!("sha256:{candidate_model_version}-demo-feature-set"),
        "shadow_comparison_status": "passed",
        "serving_version_lock_status": "passed",
        "artifact_integrity_status": "passed",
        "feature_store_materialization_status": "passed",
        "rust_feature_set_status": "passed",
        "rust_feature_set_manifest_uri": format!("data/rust-automl-demo/labeled_claim_risk/feature-set/feature_set_manifest.json"),
        "segment_fairness_status": "passed",
        "model_artifact_evaluation_status": "passed",
        "onnx_parity_status": if onnx_runtime { "passed" } else { "not_required" },
        "onnx_parity_gate_status": if onnx_runtime { "passed" } else { "not_required" },
        "onnx_parity_report_uri": if onnx_runtime {
            serde_json::Value::String(format!("data/rust-automl-demo/lifecycle-evidence/onnx-parity/{candidate_model_version}_onnx_parity_report.json"))
        } else {
            serde_json::Value::Null
        },
        "label_provenance_status": "passed"
    });
    write_json(
        path.to_path_buf(),
        &serde_json::json!({
            "model_key": "baseline_fwa",
            "candidate_model_version": candidate_model_version,
            "dataset_key": "rust_demo_claim_risk_labeled",
            "dataset_version": "2026-06-rust-automl-demo",
            "algorithm": algorithm,
            "validation_metrics": validation_metrics,
            "metrics_json": metrics_json
        }),
    )
}

fn write_demo_artifact_evaluation_report(
    path: &Path,
    model_version: &str,
    runtime_kind: &str,
    p95_latency_ms: u64,
) -> anyhow::Result<()> {
    let evidence_refs = if runtime_kind.ends_with("_onnx") {
        vec![format!("model_onnx_parity_reports:data/rust-automl-demo/lifecycle-evidence/onnx-parity/{model_version}_onnx_parity_report.json")]
    } else {
        vec![format!("model_artifacts:data/rust-automl-demo/lifecycle-evidence/serving/{model_version}/model.joblib")]
    };
    write_json(
        path.to_path_buf(),
        &serde_json::json!({
            "report_kind": "model_artifact_evaluation",
            "report_version": 1,
            "model_key": "baseline_fwa",
            "model_version": model_version,
            "runtime_kind": runtime_kind,
            "serving_manifest_uri": format!("data/rust-automl-demo/lifecycle-evidence/serving/{model_version}/serving_manifest.json"),
            "dataset_key": "rust_demo_claim_risk_labeled",
            "dataset_version": "2026-06-rust-automl-demo",
            "evaluated_split": "validation",
            "row_count": 4,
            "contract_status": "passed",
            "rust_serving_status": "passed",
            "parity_status": "passed",
            "latency_status": "passed",
            "gate_status": "passed",
            "max_abs_probability_delta": 0.00001,
            "average_abs_probability_delta": 0.000004,
            "p95_latency_ms": p95_latency_ms,
            "latency_budget_ms": 100,
            "blocking_reasons": [],
            "sample_results": [],
            "evidence_refs": evidence_refs
        }),
    )
}

fn write_demo_feature_importance_parquet(path: &Path) -> anyhow::Result<()> {
    let rows = [
        ("amount_to_limit_ratio", 0.39),
        ("peer_percentile", 0.28),
        ("high_cost_item_ratio", 0.20),
        ("provider_risk_tier", 0.08),
        ("diagnosis_procedure_mismatch", 0.05),
    ];
    let schema = Arc::new(Schema::new(vec![
        Field::new("feature", DataType::Utf8, false),
        Field::new("coefficient", DataType::Float64, true),
        Field::new("importance", DataType::Float64, false),
        Field::new("importance_kind", DataType::Utf8, false),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(
                rows.iter().map(|(feature, _)| *feature).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(vec![None; rows.len()])),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|(_, importance)| *importance)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(vec!["feature_importance"; rows.len()])),
        ],
    )?;
    write_parquet(path.to_path_buf(), schema, &batch)
}

fn push_verification_check(
    checks: &mut Vec<serde_json::Value>,
    blocking_reasons: &mut Vec<String>,
    check: &str,
    passed: bool,
    summary: String,
    evidence_refs: Vec<String>,
) {
    if !passed {
        blocking_reasons.push(format!("{check}: {summary}"));
    }
    checks.push(serde_json::json!({
        "check": check,
        "status": if passed { "passed" } else { "blocked" },
        "summary": summary,
        "evidence_refs": evidence_refs
    }));
}

fn required_artifact_uri(
    artifacts: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> anyhow::Result<String> {
    artifacts
        .get(key)
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .with_context(|| format!("demo lifecycle evidence index requires artifacts.{key}"))
}

fn validation_report_matches(
    report: &serde_json::Value,
    algorithm: &str,
    runtime_kind: &str,
) -> bool {
    json_string(report, "algorithm").as_deref() == Some(algorithm)
        && nested_json_string(report, &["metrics_json", "runtime_kind"]).as_deref()
            == Some(runtime_kind)
        && nested_json_string(report, &["metrics_json", "onnx_parity_gate_status"]).as_deref()
            == Some("passed")
        && nested_json_string(report, &["metrics_json", "onnx_parity_report_uri"]).is_some()
}

fn deep_learning_validation_report_matches(report: &serde_json::Value) -> bool {
    json_string(report, "algorithm").as_deref() == Some("deep_learning")
        && nested_json_string(report, &["metrics_json", "algorithm_family"]).as_deref()
            == Some("deep_learning")
        && nested_json_string(report, &["metrics_json", "runtime_kind"]).as_deref()
            == Some("deep_learning_sklearn_mlp")
        && nested_json_string(report, &["metrics_json", "onnx_parity_status"]).as_deref()
            == Some("not_required")
        && nested_json_string(report, &["metrics_json", "automl_factor_ranking_status"]).as_deref()
            == Some("passed")
        && nested_json_string(report, &["metrics_json", "overfitting_diagnostics_status"])
            .as_deref()
            == Some("passed")
}

fn artifact_evaluation_matches(report: &serde_json::Value, runtime_kind: &str) -> bool {
    report["report_kind"] == "model_artifact_evaluation"
        && json_string(report, "runtime_kind").as_deref() == Some(runtime_kind)
        && json_string(report, "gate_status").as_deref() == Some("passed")
        && json_string(report, "rust_serving_status").as_deref() == Some("passed")
        && json_string(report, "latency_status").as_deref() == Some("passed")
}

fn clustering_report_matches(
    report: &serde_json::Value,
    report_kind: &str,
    required_boundary_text: &str,
) -> bool {
    report["report_kind"] == report_kind
        && json_string(report, "governance_boundary")
            .is_some_and(|boundary| boundary.contains(required_boundary_text))
        && json_array_len(report, "anomaly_candidates") > 0
        && json_array_len(report, "review_tasks") > 0
}
