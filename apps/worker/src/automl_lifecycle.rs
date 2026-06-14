use anyhow::{bail, Context};
use std::{collections::BTreeSet, fs, path::Path};

use super::{
    json_array_len, json_string, lifecycle_stage, nested_json_array_contains, read_json_report,
    required_non_empty, unsupervised_factor_ranking_passed, write_json,
};

pub fn build_model_promotion_orchestration_report(
    candidate_ranking_uri: &str,
    artifact_evaluation_report_uris: &[String],
    mlops_monitoring_report_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let candidate_ranking_uri = required_non_empty("candidate_ranking_uri", candidate_ranking_uri)?;
    if artifact_evaluation_report_uris.is_empty() {
        bail!("at least one artifact_evaluation_report_uri is required");
    }
    let mlops_monitoring_report_uri =
        required_non_empty("mlops_monitoring_report_uri", mlops_monitoring_report_uri)?;

    let candidate_ranking = read_json_report(candidate_ranking_uri)?;
    let artifact_reports = artifact_evaluation_report_uris
        .iter()
        .map(|uri| read_json_report(uri))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let mlops_monitoring = read_json_report(mlops_monitoring_report_uri)?;

    let recommended_candidate_model_version =
        json_string(&candidate_ranking, "recommended_candidate_model_version");
    let recommended_candidate =
        recommended_candidate_model_version
            .as_deref()
            .and_then(|version| {
                candidate_ranking
                    .get("candidates")
                    .and_then(|value| value.as_array())
                    .into_iter()
                    .flatten()
                    .find(|candidate| {
                        json_string(candidate, "candidate_model_version").as_deref()
                            == Some(version)
                    })
            });
    let model_key = recommended_candidate
        .and_then(|candidate| json_string(candidate, "model_key"))
        .unwrap_or_else(|| "missing".into());
    let model_version = recommended_candidate_model_version.unwrap_or_else(|| "missing".into());
    let recommended_candidate_gate_passed = recommended_candidate.is_some_and(|candidate| {
        json_string(candidate, "gate_status").as_deref() == Some("passed")
            && json_string(candidate, "recommended_action").as_deref() == Some("open_human_review")
    });
    let artifact_evaluations_passed = artifact_reports.iter().any(|report| {
        json_string(report, "model_key").as_deref() == Some(model_key.as_str())
            && json_string(report, "model_version").as_deref() == Some(model_version.as_str())
            && json_string(report, "gate_status").as_deref() == Some("passed")
            && json_string(report, "rust_serving_status").as_deref() == Some("passed")
            && json_string(report, "latency_status").as_deref() == Some("passed")
    });
    let monitoring_passed = mlops_monitoring["report_kind"] == "mlops_monitoring_report"
        && json_string(&mlops_monitoring, "overall_status").as_deref() == Some("passed")
        && json_string(&mlops_monitoring, "promotion_boundary")
            .is_some_and(|boundary| boundary.contains("must not activate models"));

    let mut blocking_reasons = Vec::new();
    if !recommended_candidate_gate_passed {
        blocking_reasons.push("recommended_candidate_gate_not_passed".to_string());
    }
    if !artifact_evaluations_passed {
        blocking_reasons.push("serving_artifact_gate_not_passed".to_string());
    }
    if !monitoring_passed {
        blocking_reasons.push("mlops_monitoring_not_clear_for_review".to_string());
    }
    let orchestration_status = if blocking_reasons.is_empty() {
        "ready_after_reviewer_approval"
    } else {
        "blocked_pending_evidence"
    };
    let promotion_gates_path =
        format!("/api/v1/ops/models/{model_key}/versions/{model_version}/promotion-gates");
    let activation_path =
        format!("/api/v1/ops/models/{model_key}/versions/{model_version}/activate");

    let report = serde_json::json!({
        "report_kind": "reviewer_approved_model_promotion_orchestration",
        "report_version": 1,
        "model_key": model_key,
        "candidate_model_version": model_version,
        "orchestration_status": orchestration_status,
        "activation_policy": "automatic_after_reviewer_approval_and_fresh_promotion_gates_pass",
        "required_pre_activation_gates": [
            "recommended_candidate_gate_passed",
            "rust_serving_artifact_gate_passed",
            "mlops_monitoring_clear_for_review",
            "human_model_governance_review_approved",
            "fresh_promotion_gates_pass_before_activation"
        ],
        "automation_steps": [
            {
                "step": "submit_or_verify_model_governance_review",
                "required_decision": "approved",
                "endpoint": format!("/api/v1/ops/models/{model_key}/versions/{model_version}/promotion-review")
            },
            {
                "step": "recheck_promotion_gates",
                "required_result": "all_non_active_version_gates_passed",
                "endpoint": promotion_gates_path
            },
            {
                "step": "activate_approved_model_version",
                "required_result": "model_status_active",
                "endpoint": activation_path,
                "worker_command": format!("cargo run --locked -p worker -- promote-approved-model-version --api-url <api-url> --api-key <api-key> --model-key {model_key} --model-version {model_version}")
            }
        ],
        "blocking_reasons": blocking_reasons,
        "governance_boundary": "orchestration may activate a model only after recorded reviewer approval and a fresh promotion-gate pass; it must not bypass human approval, publish rules, assign fraud labels, or activate from stale evidence",
        "evidence_refs": [
            format!("automl_candidate_ranking:{candidate_ranking_uri}"),
            format!("mlops_monitoring_reports:{mlops_monitoring_report_uri}"),
            format!("model_versions:{model_key}:{model_version}")
        ],
        "artifact_evaluation_refs": artifact_evaluation_report_uris
            .iter()
            .map(|uri| format!("model_artifact_evaluations:{uri}"))
            .collect::<Vec<_>>()
    });

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create model promotion orchestration output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("model_promotion_orchestration_report.json"),
        &report,
    )?;
    Ok(report)
}

pub fn build_automl_lifecycle_closure_report(
    demo_index_uri: &str,
    candidate_ranking_uri: &str,
    artifact_evaluation_report_uris: &[String],
    rule_backtest_report_uri: &str,
    provider_clustering_report_uri: &str,
    provider_graph_clustering_report_uri: &str,
    claim_entity_clustering_report_uri: &str,
    mlops_monitoring_report_uri: &str,
    mlops_scheduler_execution_report_uri: &str,
    mlops_monitoring_cycle_report_uri: &str,
    model_promotion_orchestration_report_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let demo_index_uri = required_non_empty("demo_index_uri", demo_index_uri)?;
    let candidate_ranking_uri = required_non_empty("candidate_ranking_uri", candidate_ranking_uri)?;
    if artifact_evaluation_report_uris.is_empty() {
        bail!("at least one artifact_evaluation_report_uri is required");
    }
    let rule_backtest_report_uri =
        required_non_empty("rule_backtest_report_uri", rule_backtest_report_uri)?;
    let provider_clustering_report_uri = required_non_empty(
        "provider_clustering_report_uri",
        provider_clustering_report_uri,
    )?;
    let provider_graph_clustering_report_uri = required_non_empty(
        "provider_graph_clustering_report_uri",
        provider_graph_clustering_report_uri,
    )?;
    let claim_entity_clustering_report_uri = required_non_empty(
        "claim_entity_clustering_report_uri",
        claim_entity_clustering_report_uri,
    )?;
    let mlops_monitoring_report_uri =
        required_non_empty("mlops_monitoring_report_uri", mlops_monitoring_report_uri)?;
    let mlops_scheduler_execution_report_uri = required_non_empty(
        "mlops_scheduler_execution_report_uri",
        mlops_scheduler_execution_report_uri,
    )?;
    let mlops_monitoring_cycle_report_uri = required_non_empty(
        "mlops_monitoring_cycle_report_uri",
        mlops_monitoring_cycle_report_uri,
    )?;
    let model_promotion_orchestration_report_uri = required_non_empty(
        "model_promotion_orchestration_report_uri",
        model_promotion_orchestration_report_uri,
    )?;

    let demo_index = read_json_report(demo_index_uri)?;
    let candidate_ranking = read_json_report(candidate_ranking_uri)?;
    let artifact_reports = artifact_evaluation_report_uris
        .iter()
        .map(|uri| read_json_report(uri))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let rule_backtest = read_json_report(rule_backtest_report_uri)?;
    let provider_clustering = read_json_report(provider_clustering_report_uri)?;
    let provider_graph_clustering = read_json_report(provider_graph_clustering_report_uri)?;
    let claim_entity_clustering = read_json_report(claim_entity_clustering_report_uri)?;
    let mlops_monitoring = read_json_report(mlops_monitoring_report_uri)?;
    let mlops_scheduler_execution = read_json_report(mlops_scheduler_execution_report_uri)?;
    let mlops_monitoring_cycle = read_json_report(mlops_monitoring_cycle_report_uri)?;
    let model_promotion_orchestration = read_json_report(model_promotion_orchestration_report_uri)?;

    let dataset_manifests = demo_index
        .get("dataset_manifests")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let labeled_dataset_count = dataset_manifests
        .iter()
        .filter(|dataset| json_string(dataset, "label_column").is_some())
        .count();
    let unlabeled_dataset_count = dataset_manifests
        .iter()
        .filter(|dataset| json_string(dataset, "label_column").is_none())
        .count();
    let dataset_portfolio_passed = labeled_dataset_count >= 1 && unlabeled_dataset_count >= 2;

    let candidate_algorithms = candidate_ranking
        .get("candidates")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|candidate| json_string(candidate, "algorithm"))
        .collect::<BTreeSet<_>>();
    let supervised_candidates_passed = candidate_ranking["plan_kind"] == "automl_candidate_ranking"
        && candidate_algorithms.contains("xgboost")
        && candidate_algorithms.contains("lightgbm")
        && candidate_algorithms.contains("deep_learning")
        && candidate_ranking
            .get("recommended_candidate_model_version")
            .is_some_and(|value| !value.is_null());

    let runtime_kinds = artifact_reports
        .iter()
        .filter_map(|report| json_string(report, "runtime_kind"))
        .collect::<BTreeSet<_>>();
    let rust_serving_passed = artifact_reports.iter().all(|report| {
        json_string(report, "gate_status").as_deref() == Some("passed")
            && json_string(report, "rust_serving_status").as_deref() == Some("passed")
    });
    let onnx_serving_passed = rust_serving_passed
        && runtime_kinds.contains("xgboost_onnx")
        && runtime_kinds.contains("lightgbm_onnx");

    let rule_backtest_passed = rule_backtest["report_kind"]
        == "deterministic_rule_candidate_backtest"
        && json_string(&rule_backtest, "rule_library_writeback_status")
            .is_some_and(|status| status.contains("blocked_pending_human_review"))
        && json_array_len(&rule_backtest, "candidate_results") > 0
        && json_array_len(&rule_backtest, "review_tasks") > 0;

    let provider_clustering_passed = provider_clustering["report_kind"]
        == "provider_peer_clustering"
        && json_string(&provider_clustering, "governance_boundary")
            .is_some_and(|boundary| boundary.contains("must not create confirmed FWA labels"))
        && json_array_len(&provider_clustering, "anomaly_candidates") > 0
        && unsupervised_factor_ranking_passed(
            &provider_clustering,
            "provider_peer_unsupervised_factor_ranking",
        );
    let provider_graph_clustering_passed = provider_graph_clustering["report_kind"]
        == "provider_graph_community_clustering"
        && json_string(&provider_graph_clustering, "governance_boundary")
            .is_some_and(|boundary| boundary.contains("must not create confirmed FWA labels"))
        && json_array_len(&provider_graph_clustering, "anomaly_candidates") > 0
        && json_array_len(&provider_graph_clustering, "review_tasks") > 0
        && unsupervised_factor_ranking_passed(
            &provider_graph_clustering,
            "provider_graph_unsupervised_factor_ranking",
        );
    let claim_entity_clustering_passed = claim_entity_clustering["report_kind"]
        == "claim_entity_clustering"
        && json_string(&claim_entity_clustering, "governance_boundary")
            .is_some_and(|boundary| boundary.contains("rule-library writeback"))
        && json_array_len(&claim_entity_clustering, "review_tasks") > 0
        && unsupervised_factor_ranking_passed(
            &claim_entity_clustering,
            "claim_entity_unsupervised_factor_ranking",
        );

    let monitoring_status =
        json_string(&mlops_monitoring, "overall_status").unwrap_or_else(|| "missing".into());
    let monitoring_loop_passed = mlops_monitoring["report_kind"] == "mlops_monitoring_report"
        && monitoring_status != "blocked"
        && json_string(&mlops_monitoring, "promotion_boundary")
            .is_some_and(|boundary| boundary.contains("must not activate models"));
    let scheduler_status = json_string(&mlops_scheduler_execution, "scheduler_status")
        .unwrap_or_else(|| "missing".into());
    let alert_delivery_status = json_string(&mlops_scheduler_execution, "alert_delivery_status")
        .unwrap_or_else(|| "missing".into());
    let scheduler_loop_passed = mlops_scheduler_execution["report_kind"]
        == "mlops_scheduler_execution_report"
        && scheduler_status.starts_with("completed")
        && json_string(&mlops_scheduler_execution, "governance_boundary")
            .is_some_and(|boundary| boundary.contains("must not create retraining jobs"));
    let cycle_status =
        json_string(&mlops_monitoring_cycle, "cycle_status").unwrap_or_else(|| "missing".into());
    let cycle_loop_passed = mlops_monitoring_cycle["report_kind"]
        == "mlops_monitoring_cycle_execution"
        && cycle_status.starts_with("completed")
        && json_string(&mlops_monitoring_cycle, "governance_boundary")
            .is_some_and(|boundary| boundary.contains("must not create retraining jobs"));
    let promotion_orchestration_passed = model_promotion_orchestration["report_kind"]
        == "reviewer_approved_model_promotion_orchestration"
        && json_string(&model_promotion_orchestration, "orchestration_status").as_deref()
            == Some("ready_after_reviewer_approval")
        && json_string(&model_promotion_orchestration, "activation_policy")
            .is_some_and(|policy| policy.contains("fresh_promotion_gates_pass"))
        && json_string(&model_promotion_orchestration, "governance_boundary")
            .is_some_and(|boundary| boundary.contains("after recorded reviewer approval"))
        && nested_json_array_contains(
            &model_promotion_orchestration,
            &["required_pre_activation_gates"],
            "human_model_governance_review_approved",
        )
        && nested_json_array_contains(
            &model_promotion_orchestration,
            &["required_pre_activation_gates"],
            "fresh_promotion_gates_pass_before_activation",
        )
        && json_array_len(&model_promotion_orchestration, "automation_steps") >= 3;

    let stages = vec![
        lifecycle_stage(
            "demo_dataset_portfolio",
            dataset_portfolio_passed,
            format!(
                "{labeled_dataset_count} labeled dataset(s), {unlabeled_dataset_count} unlabeled dataset(s)"
            ),
            vec![format!("demo_dataset_index:{demo_index_uri}")],
        ),
        lifecycle_stage(
            "supervised_candidate_ranking",
            supervised_candidates_passed,
            format!(
                "candidate algorithms: {}",
                candidate_algorithms
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            vec![format!("automl_candidate_ranking:{candidate_ranking_uri}")],
        ),
        lifecycle_stage(
            "onnx_rust_serving_gate",
            onnx_serving_passed,
            format!(
                "artifact runtime kinds: {}",
                runtime_kinds.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
            artifact_evaluation_report_uris
                .iter()
                .map(|uri| format!("model_artifact_evaluations:{uri}"))
                .collect(),
        ),
        lifecycle_stage(
            "explainable_rule_backtest_gate",
            rule_backtest_passed,
            "rule candidates are backtested and blocked before rule-library writeback".into(),
            vec![format!("rule_candidate_backtests:{rule_backtest_report_uri}")],
        ),
        lifecycle_stage(
            "unlabeled_clustering_reviews",
            provider_clustering_passed
                && provider_graph_clustering_passed
                && claim_entity_clustering_passed,
            "provider-peer, provider graph-community, and claim/member/provider clustering create review candidates only"
                .into(),
            vec![
                format!("provider_peer_clustering:{provider_clustering_report_uri}"),
                format!("provider_graph_clustering:{provider_graph_clustering_report_uri}"),
                format!("claim_entity_clustering:{claim_entity_clustering_report_uri}"),
            ],
        ),
        lifecycle_stage(
            "mlops_monitoring_loop",
            monitoring_loop_passed && scheduler_loop_passed && cycle_loop_passed,
            format!(
                "monitoring status: {monitoring_status}; scheduler: {scheduler_status}; alert delivery: {alert_delivery_status}; cycle: {cycle_status}"
            ),
            vec![
                format!("mlops_monitoring_reports:{mlops_monitoring_report_uri}"),
                format!(
                    "mlops_scheduler_execution_reports:{mlops_scheduler_execution_report_uri}"
                ),
                format!("mlops_monitoring_cycles:{mlops_monitoring_cycle_report_uri}"),
            ],
        ),
        lifecycle_stage(
            "reviewer_approved_promotion_orchestration",
            promotion_orchestration_passed,
            "model promotion is automated only after reviewer approval and a fresh promotion-gate pass"
                .into(),
            vec![format!(
                "model_promotion_orchestrations:{model_promotion_orchestration_report_uri}"
            )],
        ),
    ];
    let closure_status = if stages
        .iter()
        .all(|stage| stage["status"].as_str() == Some("passed"))
    {
        "closed_with_human_governance_gates"
    } else {
        "incomplete"
    };

    let report = serde_json::json!({
        "report_kind": "rust_automl_lifecycle_closure",
        "report_version": 1,
        "closure_status": closure_status,
        "lifecycle_stages": stages,
        "governance_boundary": "Rust lifecycle closure may open monitoring, review, retraining preparation, and rule-candidate backtest work only; it must not auto-activate models, assign fraud labels, or write back to the rule library",
        "required_human_gates": [
            "model_governance_review_before_shadow_or_activation",
            "reviewer_approved_promotion_before_model_activation",
            "human_rule_review_after_backtest_before_rule_library_writeback",
            "anomaly_review_before_case_creation_or_label_assignment",
            "mlops_monitoring_review_before_retraining_or_rollback_action"
        ],
        "evidence_refs": [
            format!("demo_dataset_index:{demo_index_uri}"),
            format!("automl_candidate_ranking:{candidate_ranking_uri}"),
            format!("rule_candidate_backtests:{rule_backtest_report_uri}"),
            format!("provider_peer_clustering:{provider_clustering_report_uri}"),
            format!("provider_graph_clustering:{provider_graph_clustering_report_uri}"),
            format!("claim_entity_clustering:{claim_entity_clustering_report_uri}"),
            format!("mlops_monitoring_reports:{mlops_monitoring_report_uri}"),
            format!("mlops_scheduler_execution_reports:{mlops_scheduler_execution_report_uri}"),
            format!("mlops_monitoring_cycles:{mlops_monitoring_cycle_report_uri}"),
            format!("model_promotion_orchestrations:{model_promotion_orchestration_report_uri}")
        ],
        "artifact_evaluation_refs": artifact_evaluation_report_uris
            .iter()
            .map(|uri| format!("model_artifact_evaluations:{uri}"))
            .collect::<Vec<_>>()
    });

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create Auto MLOps lifecycle closure output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("rust_automl_lifecycle_closure_report.json"),
        &report,
    )?;
    Ok(report)
}
