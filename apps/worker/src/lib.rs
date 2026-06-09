use anyhow::{anyhow, bail, Context};
#[cfg(test)]
use arrow_array::Int8Array;
use arrow_array::{Float64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use fwa_core::{ClaimId, ScoringRunId};
use fwa_features::{FeatureMap, FeatureValue};
use fwa_ml_runtime::{ModelScoreRequest, ModelScorer, ServingManifestModelScorer};
use hmac::Hmac;
use parquet::arrow::{arrow_reader::ParquetRecordBatchReaderBuilder, ArrowWriter};
use serde::Serialize;
use sha2::Sha256;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

type HmacSha256 = Hmac<Sha256>;

mod dataset_types;
pub use dataset_types::*;

mod demo_datasets;
pub use demo_datasets::build_demo_ml_datasets;

mod dataset_profile;
pub use dataset_profile::{profile_manifest, profile_manifest_file};

mod feature_set;
pub use feature_set::build_feature_set;

mod health;
pub use health::{
    build_pilot_readiness_report, check_pilot_readiness, worker_health, ApiHealthCheck,
    ApiHealthResponse, ApiPilotReadiness, PilotReadinessReport, WorkerHealthCheck,
    WorkerHealthResponse,
};

mod mlops_delivery;
pub use mlops_delivery::{
    build_mlops_alert_delivery_submission, build_mlops_alert_receiver_payload,
    build_mlops_monitoring_report_submission, deliver_mlops_alert_receiver_webhook,
    submit_mlops_alert_delivery_tasks, submit_mlops_monitoring_report,
    MlopsAlertDeliverySubmission, MlopsMonitoringReportSubmission,
};

mod anomaly_clustering;
pub use anomaly_clustering::{
    build_anomaly_clustering_report_submission, submit_anomaly_clustering_report,
    AnomalyClusteringReportSubmission, AnomalyClusteringReviewTaskSubmission,
};

mod alertmanager;
#[cfg(test)]
pub(crate) use alertmanager::alertmanager_webhook_is_authorized;
pub use alertmanager::{
    build_alertmanager_mlops_alert_delivery_submission, serve_mlops_alert_router,
    submit_alertmanager_webhook_to_fwa, AlertmanagerAlert, AlertmanagerWebhook,
    MlopsAlertRouterConfig,
};

mod mlops_monitoring;
#[cfg(test)]
pub(crate) use mlops_monitoring::sha256_prefixed_hex;
pub use mlops_monitoring::{
    build_mlops_monitoring_plan, build_mlops_monitoring_report,
    build_mlops_scheduler_execution_report, run_mlops_monitoring_plan,
    run_mlops_monitoring_plan_with_inputs, run_scheduled_mlops_monitoring,
    run_scheduled_mlops_monitoring_with_artifact_base_uri,
    run_scheduled_mlops_monitoring_with_options,
};

mod mlops_cycle;
pub use mlops_cycle::{build_mlops_monitoring_cycle_evidence, run_mlops_monitoring_cycle};

mod clustering;
pub use clustering::{
    cluster_claim_entities, cluster_provider_graph_communities, cluster_provider_peers,
    ClaimEntityAnomalyCandidate, ClaimEntityClusterAssignment, ClaimEntityClusterSummary,
    ClaimEntityClusteringReport, ClaimEntityReviewTask, ProviderGraphAnomalyCandidate,
    ProviderGraphCommunityAssignment, ProviderGraphCommunityReport, ProviderGraphCommunitySummary,
    ProviderGraphReviewTask, ProviderPeerAnomalyCandidate, ProviderPeerClusterAssignment,
    ProviderPeerClusterSummary, ProviderPeerClusteringReport, ProviderPeerReviewTask,
    UnsupervisedFactorRank, UnsupervisedFactorRanking,
};

mod automl_lifecycle;
pub use automl_lifecycle::{
    build_automl_lifecycle_closure_report, build_model_promotion_orchestration_report,
};

mod ops_plans;
pub use ops_plans::{
    build_ai_evidence_execution_plan, build_analytics_export_plan, build_governance_ops_plan,
};

mod training_handoff;
pub(crate) use training_handoff::build_training_command;
pub use training_handoff::{build_training_handoff, build_training_handoff_with_algorithm};

mod retraining;
pub use retraining::{
    claim_next_retraining_job, complete_retraining_job_with_mock_output,
    complete_retraining_job_with_training_output, promote_approved_model_version,
    run_one_retraining_job, update_retraining_job_status, AutoMlCandidateRank,
    AutoMlCandidateRanking, AutoMlReviewTask, ClaimedRetrainingJob, ModelArtifactEvaluationReport,
    ModelArtifactEvaluationSample, PromoteApprovedModelVersionResult, RuleCandidateBacktestReport,
    RuleCandidateBacktestRequest, RuleCandidateBacktestResult, RuleCandidateBacktestReviewTask,
    RuleCandidateDraft, RuleCandidateMiningPlan, RuleCandidateReviewTask,
    RuleCandidateSplitMetrics,
};
pub(crate) use retraining::{
    CompleteRetrainingJobPayload, FeatureImportanceRow, ModelArtifactEvaluationRow,
    RuleBacktestRow, TrainingCommand, WorkerServingManifest,
};

fn write_parquet(path: PathBuf, schema: Arc<Schema>, batch: &RecordBatch) -> anyhow::Result<()> {
    let file = File::create(&path).with_context(|| format!("create parquet {}", path.display()))?;
    let mut writer = ArrowWriter::try_new(file, schema, None)
        .with_context(|| format!("open parquet writer {}", path.display()))?;
    writer
        .write(batch)
        .with_context(|| format!("write parquet batch {}", path.display()))?;
    writer
        .close()
        .with_context(|| format!("close parquet writer {}", path.display()))?;
    Ok(())
}

fn write_json(path: PathBuf, value: &impl Serialize) -> anyhow::Result<()> {
    fs::write(&path, serde_json::to_string_pretty(value)?)
        .with_context(|| format!("write json {}", path.display()))
}

fn reject_csv_uri(uri: &str) -> anyhow::Result<()> {
    if uri.to_ascii_lowercase().contains(".csv") {
        bail!("parquet profiler rejects csv data_uri: {uri}");
    }
    Ok(())
}

fn api_url(base_url: &str, path: &str) -> String {
    format!("{}{}", base_url.trim_end_matches('/'), path)
}

pub(crate) fn retraining_job_status_path(job_id: &str) -> String {
    format!("/api/v1/ops/model-retraining-jobs/{job_id}/status")
}

pub(crate) fn retraining_job_output_path(job_id: &str) -> String {
    format!("/api/v1/ops/model-retraining-jobs/{job_id}/output")
}

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

fn lifecycle_stage(
    stage: &str,
    passed: bool,
    summary: String,
    evidence_refs: Vec<String>,
) -> serde_json::Value {
    serde_json::json!({
        "stage": stage,
        "status": if passed { "passed" } else { "missing_or_blocked" },
        "summary": summary,
        "evidence_refs": evidence_refs
    })
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

fn unsupervised_factor_ranking_passed(report: &serde_json::Value, report_kind: &str) -> bool {
    let Some(factor_ranking) = report.get("factor_ranking") else {
        return false;
    };
    factor_ranking["report_kind"] == report_kind
        && factor_ranking
            .get("ranked_factor_count")
            .and_then(|value| value.as_u64())
            .is_some_and(|count| count > 0)
        && json_array_len(factor_ranking, "ranked_factors") > 0
}

fn json_array_len(value: &serde_json::Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(|value| value.as_array())
        .map(|items| items.len())
        .unwrap_or(0)
}

fn read_json_report(uri: &str) -> anyhow::Result<serde_json::Value> {
    let path = Path::new(uri);
    let report_json =
        fs::read_to_string(path).with_context(|| format!("read report {}", path.display()))?;
    serde_json::from_str(&report_json).with_context(|| format!("parse report {}", path.display()))
}

fn nested_json_string(value: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(key)?;
    }
    current
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn nested_json_array_contains(value: &serde_json::Value, path: &[&str], expected: &str) -> bool {
    let mut current = value;
    for key in path {
        let Some(next) = current.get(key) else {
            return false;
        };
        current = next;
    }
    current
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
}

fn json_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn json_u64(value: &serde_json::Value, key: &str) -> Option<u64> {
    value.get(key).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_str().and_then(|value| value.parse::<u64>().ok()))
    })
}

pub fn rank_automl_candidates(
    validation_reports: &[String],
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<AutoMlCandidateRanking> {
    if validation_reports.is_empty() {
        bail!("at least one validation report is required");
    }

    let mut candidates = Vec::new();
    for report_uri in validation_reports {
        let report_path = Path::new(report_uri);
        let report_json = fs::read_to_string(report_path)
            .with_context(|| format!("read validation report {}", report_path.display()))?;
        let report: serde_json::Value =
            serde_json::from_str(&report_json).context("parse validation report")?;
        candidates.push(build_automl_candidate_rank(report_uri, &report)?);
    }

    candidates.sort_by(|left, right| {
        eligible_sort_key(right)
            .partial_cmp(&eligible_sort_key(left))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.candidate_model_version
                    .cmp(&right.candidate_model_version)
            })
    });

    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.rank = index + 1;
    }

    let recommended_candidate_model_version = candidates
        .iter()
        .find(|candidate| candidate.gate_status == "passed")
        .map(|candidate| candidate.candidate_model_version.clone());
    let review_tasks = candidates
        .iter()
        .map(|candidate| AutoMlReviewTask {
            task_kind: "model_candidate_human_review".into(),
            candidate_model_version: candidate.candidate_model_version.clone(),
            review_queue: if candidate.gate_status == "passed" {
                "model_governance_review".into()
            } else {
                "mlops_remediation_review".into()
            },
            required_review: "human_approval_required_before_shadow_or_activation".into(),
            decision_options: vec![
                "reject".into(),
                "request_more_evidence".into(),
                "approve_shadow_only".into(),
            ],
            evidence_refs: candidate.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();
    let ranking = AutoMlCandidateRanking {
        plan_kind: "automl_candidate_ranking".into(),
        plan_version: 1,
        promotion_boundary:
            "ranking opens human review only; no automatic model promotion or rule publication"
                .into(),
        generated_from_reports: validation_reports.to_vec(),
        recommended_candidate_model_version,
        candidates,
        review_tasks,
        evidence_refs: validation_reports
            .iter()
            .map(|report| format!("model_validation_reports:{report}"))
            .collect(),
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create Auto MLOps ranking output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir.as_ref().join("automl_candidate_ranking.json"),
        &ranking,
    )?;
    write_json(
        output_dir.as_ref().join("automl_review_tasks.json"),
        &ranking.review_tasks,
    )?;
    Ok(ranking)
}

pub async fn evaluate_model_artifact(
    serving_manifest_uri: &str,
    dataset_manifest_uri: &str,
    split_name: &str,
    output_dir: impl AsRef<Path>,
    expected_probability_column: Option<&str>,
    probability_tolerance: f64,
    latency_budget_ms: u64,
    max_rows: usize,
    signing_key: Option<&str>,
) -> anyhow::Result<ModelArtifactEvaluationReport> {
    if probability_tolerance < 0.0 {
        bail!("probability_tolerance must be non-negative");
    }
    if max_rows == 0 {
        bail!("max_rows must be greater than zero");
    }

    let serving_manifest_path = Path::new(serving_manifest_uri);
    let serving_manifest_json = fs::read_to_string(serving_manifest_path)
        .with_context(|| format!("read serving manifest {}", serving_manifest_path.display()))?;
    let serving_manifest: WorkerServingManifest =
        serde_json::from_str(&serving_manifest_json).context("parse serving manifest")?;
    validate_worker_serving_manifest(&serving_manifest)?;

    let dataset_manifest_path = Path::new(dataset_manifest_uri);
    let dataset_manifest_json = fs::read_to_string(dataset_manifest_path)
        .with_context(|| format!("read dataset manifest {}", dataset_manifest_path.display()))?;
    let dataset_manifest: ParquetDatasetManifest =
        serde_json::from_str(&dataset_manifest_json).context("parse dataset manifest")?;
    let dataset_base_dir = dataset_manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let rows = read_model_artifact_evaluation_rows(
        &dataset_manifest,
        dataset_base_dir,
        split_name,
        &serving_manifest.feature_columns,
        expected_probability_column,
        max_rows,
    )?;

    let mut scorer = ServingManifestModelScorer::new(serving_manifest_uri);
    if let Some(signing_key) = signing_key {
        scorer = scorer.with_signing_key(signing_key);
    }

    let mut sample_results = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let score = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external(format!(
                    "artifact_eval_{}_{}",
                    safe_id_segment(&serving_manifest.model_key),
                    index
                )),
                claim_id: ClaimId::from_external(row.claim_id.clone()),
                model_key: serving_manifest.model_key.clone(),
                model_version: serving_manifest.model_version.clone(),
                endpoint_url: None,
                features: feature_map_from_values(&row.features),
            })
            .await
            .with_context(|| format!("score evaluation row {}", row.claim_id))?;
        let fraud_probability = score
            .metadata
            .get("fraud_probability")
            .and_then(|value| value.as_f64());
        let abs_probability_delta = match (fraud_probability, row.expected_probability) {
            (Some(actual), Some(expected)) => Some((actual - expected).abs()),
            _ => None,
        };
        sample_results.push(ModelArtifactEvaluationSample {
            claim_id: row.claim_id.clone(),
            score: score.score,
            label: score.label,
            fraud_probability,
            expected_probability: row.expected_probability,
            abs_probability_delta: abs_probability_delta.map(round4),
            latency_ms: score.latency_ms,
        });
    }

    let deltas = sample_results
        .iter()
        .filter_map(|sample| sample.abs_probability_delta)
        .collect::<Vec<_>>();
    let max_abs_probability_delta = deltas.iter().copied().reduce(f64::max).map(round4);
    let average_abs_probability_delta = if deltas.is_empty() {
        None
    } else {
        Some(round4(deltas.iter().sum::<f64>() / deltas.len() as f64))
    };
    let parity_status = if expected_probability_column.is_none() {
        "not_configured"
    } else if deltas.len() != sample_results.len() {
        "failed"
    } else if max_abs_probability_delta.unwrap_or(0.0) <= probability_tolerance {
        "passed"
    } else {
        "failed"
    }
    .to_string();
    let p95_latency_ms = percentile_latency_ms(&sample_results, 0.95);
    let latency_status = if p95_latency_ms <= latency_budget_ms {
        "passed"
    } else {
        "failed"
    }
    .to_string();
    let rust_serving_status = if sample_results.is_empty() {
        "failed"
    } else {
        "passed"
    }
    .to_string();
    let mut blocking_reasons = Vec::new();
    if parity_status == "failed" {
        blocking_reasons.push("serving_probability_parity_failed".into());
    }
    if latency_status == "failed" {
        blocking_reasons.push("serving_latency_budget_failed".into());
    }
    if rust_serving_status == "failed" {
        blocking_reasons.push("rust_serving_execution_failed".into());
    }
    let gate_status = if blocking_reasons.is_empty() {
        "passed"
    } else {
        "blocked"
    }
    .to_string();

    let report = ModelArtifactEvaluationReport {
        report_kind: "model_artifact_evaluation".into(),
        report_version: 1,
        model_key: serving_manifest.model_key.clone(),
        model_version: serving_manifest.model_version.clone(),
        runtime_kind: serving_manifest.runtime_kind.clone(),
        serving_manifest_uri: serving_manifest_uri.into(),
        dataset_key: dataset_manifest.dataset_key,
        dataset_version: dataset_manifest.dataset_version,
        evaluated_split: split_name.into(),
        row_count: sample_results.len(),
        contract_status: "passed".into(),
        rust_serving_status,
        parity_status,
        latency_status,
        gate_status,
        max_abs_probability_delta,
        average_abs_probability_delta,
        p95_latency_ms,
        latency_budget_ms,
        blocking_reasons,
        sample_results,
        evidence_refs: vec![
            format!("serving_manifests:{serving_manifest_uri}"),
            format!("model_artifacts:{}", serving_manifest.artifact_uri),
            format!(
                "model_artifact_checksums:{}",
                serving_manifest.artifact_sha256
            ),
            format!("dataset_manifests:{dataset_manifest_uri}"),
        ],
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create model artifact evaluation output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("model_artifact_evaluation_report.json"),
        &report,
    )?;
    Ok(report)
}

fn validate_worker_serving_manifest(manifest: &WorkerServingManifest) -> anyhow::Result<()> {
    required_non_empty("model_key", &manifest.model_key)?;
    required_non_empty("model_version", &manifest.model_version)?;
    required_non_empty("runtime_kind", &manifest.runtime_kind)?;
    required_non_empty("artifact_uri", &manifest.artifact_uri)?;
    required_non_empty("artifact_sha256", &manifest.artifact_sha256)?;
    if manifest.version_lock != manifest.model_version {
        bail!(
            "serving manifest version_lock mismatch: expected {}, got {}",
            manifest.model_version,
            manifest.version_lock
        );
    }
    if manifest.feature_columns.is_empty() {
        bail!("serving manifest feature_columns must not be empty");
    }
    Ok(())
}

fn read_model_artifact_evaluation_rows(
    manifest: &ParquetDatasetManifest,
    base_dir: &Path,
    split_name: &str,
    feature_columns: &[String],
    expected_probability_column: Option<&str>,
    max_rows: usize,
) -> anyhow::Result<Vec<ModelArtifactEvaluationRow>> {
    if feature_columns.is_empty() {
        bail!("model artifact evaluation requires at least one feature column");
    }
    let Some(split) = manifest
        .splits
        .iter()
        .find(|split| split.split_name == split_name)
    else {
        bail!("dataset manifest missing split {split_name}");
    };
    reject_csv_uri(&split.data_uri)?;
    let parquet_files = resolve_parquet_files(base_dir, &split.data_uri)?;
    if parquet_files.is_empty() {
        bail!("split {} has no parquet files", split.split_name);
    }

    let mut rows = Vec::new();
    for parquet_file in parquet_files {
        if rows.len() >= max_rows {
            break;
        }
        let file = File::open(&parquet_file)
            .with_context(|| format!("open parquet file {}", parquet_file.display()))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .with_context(|| format!("read parquet metadata {}", parquet_file.display()))?;
        let mut reader = builder.with_batch_size(4096).build()?;
        for batch in &mut reader {
            if rows.len() >= max_rows {
                break;
            }
            let batch = batch?;
            let feature_indexes = feature_columns
                .iter()
                .map(|feature| {
                    batch
                        .schema()
                        .index_of(feature)
                        .with_context(|| format!("missing model feature column {feature}"))
                        .map(|index| (feature.clone(), index))
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            let claim_index = batch.schema().index_of("claim_id").ok();
            let expected_index = expected_probability_column
                .map(|column| {
                    batch
                        .schema()
                        .index_of(column)
                        .with_context(|| format!("missing expected probability column {column}"))
                })
                .transpose()?;

            for row_index in 0..batch.num_rows() {
                if rows.len() >= max_rows {
                    break;
                }
                let claim_id = claim_index
                    .and_then(|index| column_value_at(batch.column(index).as_ref(), row_index))
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| format!("{}-row-{row_index}", split.split_name));
                let mut features = BTreeMap::new();
                for (feature, feature_index) in &feature_indexes {
                    let value = column_value_at(batch.column(*feature_index).as_ref(), row_index)
                        .and_then(|value| value.parse::<f64>().ok())
                        .with_context(|| {
                            format!("missing or invalid model feature {feature} at row {row_index}")
                        })?;
                    features.insert(feature.clone(), value);
                }
                let expected_probability = expected_index
                    .and_then(|index| column_value_at(batch.column(index).as_ref(), row_index))
                    .map(|value| {
                        value.parse::<f64>().with_context(|| {
                            format!(
                                "invalid expected probability value at row {row_index}: {value}"
                            )
                        })
                    })
                    .transpose()?;
                rows.push(ModelArtifactEvaluationRow {
                    claim_id,
                    features,
                    expected_probability,
                });
            }
        }
    }
    if rows.is_empty() {
        bail!("model artifact evaluation split {split_name} has no rows");
    }
    Ok(rows)
}

fn feature_map_from_values(values: &BTreeMap<String, f64>) -> FeatureMap {
    values
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                FeatureValue {
                    name: name.clone(),
                    version: 1,
                    value: serde_json::json!(value),
                    evidence_refs: Vec::new(),
                },
            )
        })
        .collect()
}

fn percentile_latency_ms(samples: &[ModelArtifactEvaluationSample], percentile: f64) -> u64 {
    if samples.is_empty() {
        return 0;
    }
    let mut latencies = samples
        .iter()
        .map(|sample| sample.latency_ms)
        .collect::<Vec<_>>();
    latencies.sort_unstable();
    let rank = ((latencies.len() as f64 * percentile).ceil() as usize).saturating_sub(1);
    latencies[rank.min(latencies.len() - 1)]
}

pub fn mine_rule_candidates(
    validation_report: &str,
    feature_importance_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<RuleCandidateMiningPlan> {
    let validation_path = Path::new(validation_report);
    let report_json = fs::read_to_string(validation_path)
        .with_context(|| format!("read validation report {}", validation_path.display()))?;
    let report: serde_json::Value =
        serde_json::from_str(&report_json).context("parse validation report")?;
    let model_key = required_manifest_str(&report, "model_key")?.to_string();
    let candidate_model_version =
        required_manifest_str(&report, "candidate_model_version")?.to_string();
    let algorithm = required_manifest_str(&report, "algorithm")?.to_string();
    let feature_importance = read_feature_importance(Path::new(feature_importance_uri))?;
    if feature_importance.is_empty() {
        bail!("feature importance artifact contains no candidate features");
    }

    let candidate_rules = feature_importance
        .into_iter()
        .take(3)
        .map(|feature| {
            let candidate_rule_key = format!(
                "model_pattern_{}_{}",
                safe_id_segment(&candidate_model_version),
                safe_id_segment(&feature.feature)
            );
            RuleCandidateDraft {
                candidate_rule_key: candidate_rule_key.clone(),
                source_feature: feature.feature.clone(),
                source_importance: feature.importance,
                source_importance_kind: feature.importance_kind.clone(),
                draft_rule_template: serde_json::json!({
                    "rule_id": candidate_rule_key,
                    "version": 0,
                    "name": format!("Model pattern: {}", feature.feature),
                    "review_mode": "both",
                    "scheme_family": "high_risk_claim",
                    "conditions": [
                        {
                            "field": feature.feature,
                            "operator": "threshold_selected_by_backtest",
                            "value": {
                                "threshold_source": "run_rule_candidate_backtest_required"
                            }
                        }
                    ],
                    "action": {
                        "score": "selected_by_backtest",
                        "recommended_action": "manual_review",
                        "action_class": "score_only_or_manual_review_after_approval",
                        "required_evidence": [],
                        "reason": "Explainable model pattern candidate; not publishable before deterministic backtest and human approval."
                    }
                }),
                gate_status: "blocked_until_backtest_and_human_review".into(),
                required_before_rule_library_writeback: vec![
                    "deterministic_backtest".into(),
                    "false_positive_review".into(),
                    "human_rule_promotion_review".into(),
                    "customer_policy_or_model_governance_approval".into(),
                    "shadow_or_limited_rollout_if_high_impact".into(),
                ],
                evidence_refs: vec![
                    format!("model_validation_reports:{validation_report}"),
                    format!("model_feature_importance:{feature_importance_uri}"),
                    format!("model_evaluations:{}", safe_id_segment(&candidate_model_version)),
                ],
            }
        })
        .collect::<Vec<_>>();
    let backtest_requests = candidate_rules
        .iter()
        .map(|candidate| RuleCandidateBacktestRequest {
            candidate_rule_key: candidate.candidate_rule_key.clone(),
            backtest_kind: "deterministic_rule_candidate_backtest".into(),
            required_dataset_splits: vec![
                "train".into(),
                "validation".into(),
                "out_of_time".into(),
            ],
            minimum_evidence: vec![
                "hit_rate_by_split".into(),
                "precision_recall_by_split".into(),
                "false_positive_review".into(),
                "rule_only_baseline_comparison".into(),
                "manual_review_capacity_impact".into(),
            ],
            evidence_refs: candidate.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();
    let review_tasks = candidate_rules
        .iter()
        .map(|candidate| RuleCandidateReviewTask {
            task_kind: "rule_candidate_human_review".into(),
            candidate_rule_key: candidate.candidate_rule_key.clone(),
            review_queue: "rule_studio_candidate_review".into(),
            required_review: "human_approval_required_before_rule_library_writeback".into(),
            decision_options: vec![
                "reject".into(),
                "request_backtest_changes".into(),
                "approve_draft_for_backtest".into(),
            ],
            evidence_refs: candidate.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();
    let plan = RuleCandidateMiningPlan {
        plan_kind: "explainable_model_rule_candidate_mining".into(),
        plan_version: 1,
        source_model_key: model_key,
        source_candidate_model_version: candidate_model_version,
        source_algorithm: algorithm,
        promotion_boundary:
            "candidate rules are drafts only; backtest and human review are required before rule library writeback"
                .into(),
        candidate_rules,
        backtest_requests,
        review_tasks,
        evidence_refs: vec![
            format!("model_validation_reports:{validation_report}"),
            format!("model_feature_importance:{feature_importance_uri}"),
        ],
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create rule candidate mining output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir.as_ref().join("rule_candidate_mining_plan.json"),
        &plan,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("rule_candidate_backtest_requests.json"),
        &plan.backtest_requests,
    )?;
    write_json(
        output_dir.as_ref().join("rule_candidate_review_tasks.json"),
        &plan.review_tasks,
    )?;
    Ok(plan)
}

pub fn run_rule_candidate_backtest(
    candidate_plan: &str,
    dataset_manifest: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<RuleCandidateBacktestReport> {
    let candidate_plan_path = Path::new(candidate_plan);
    let plan_json = fs::read_to_string(candidate_plan_path).with_context(|| {
        format!(
            "read rule candidate mining plan {}",
            candidate_plan_path.display()
        )
    })?;
    let plan: RuleCandidateMiningPlan =
        serde_json::from_str(&plan_json).context("parse rule candidate mining plan")?;
    if plan.candidate_rules.is_empty() {
        bail!("candidate plan contains no rule candidates");
    }

    let manifest_path = Path::new(dataset_manifest);
    let manifest_json = fs::read_to_string(manifest_path)
        .with_context(|| format!("read dataset manifest {}", manifest_path.display()))?;
    let manifest: ParquetDatasetManifest =
        serde_json::from_str(&manifest_json).context("parse parquet dataset manifest")?;
    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let candidate_features = plan
        .candidate_rules
        .iter()
        .map(|candidate| candidate.source_feature.clone())
        .collect::<BTreeSet<_>>();
    let rows = read_rule_backtest_rows(&manifest, base_dir, &candidate_features)?;
    if rows.is_empty() {
        bail!("dataset manifest contains no rows for rule candidate backtest");
    }

    let candidate_results = plan
        .candidate_rules
        .iter()
        .map(|candidate| backtest_rule_candidate(candidate, &rows))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let review_tasks = candidate_results
        .iter()
        .map(|result| RuleCandidateBacktestReviewTask {
            task_kind: "rule_candidate_backtest_review".into(),
            candidate_rule_key: result.candidate_rule_key.clone(),
            review_queue: "rule_studio_candidate_review".into(),
            required_review: "human_approval_required_after_backtest_before_rule_library_writeback"
                .into(),
            decision_options: vec![
                "reject".into(),
                "request_threshold_or_feature_changes".into(),
                "approve_for_policy_governance_review".into(),
            ],
            evidence_refs: result.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();

    let report = RuleCandidateBacktestReport {
        report_kind: "deterministic_rule_candidate_backtest".into(),
        report_version: 1,
        source_plan_kind: plan.plan_kind,
        source_model_key: plan.source_model_key,
        source_candidate_model_version: plan.source_candidate_model_version,
        dataset_key: manifest.dataset_key,
        dataset_version: manifest.dataset_version,
        label_column: manifest.label_column,
        rule_library_writeback_status:
            "blocked_pending_human_review_and_policy_governance_approval".into(),
        candidate_results,
        review_tasks,
        evidence_refs: vec![
            format!("rule_candidate_mining_plan:{candidate_plan}"),
            format!("dataset_manifest:{dataset_manifest}"),
        ],
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create rule candidate backtest output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("rule_candidate_backtest_report.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("rule_candidate_backtest_review_tasks.json"),
        &report.review_tasks,
    )?;
    Ok(report)
}

fn read_rule_backtest_rows(
    manifest: &ParquetDatasetManifest,
    base_dir: &Path,
    candidate_features: &BTreeSet<String>,
) -> anyhow::Result<Vec<RuleBacktestRow>> {
    if candidate_features.is_empty() {
        bail!("rule candidate backtest requires at least one feature");
    }

    let mut rows = Vec::new();
    for split in &manifest.splits {
        reject_csv_uri(&split.data_uri)?;
        let parquet_files = resolve_parquet_files(base_dir, &split.data_uri)?;
        if parquet_files.is_empty() {
            bail!("split {} has no parquet files", split.split_name);
        }

        for parquet_file in parquet_files {
            let file = File::open(&parquet_file)
                .with_context(|| format!("open parquet file {}", parquet_file.display()))?;
            let builder = ParquetRecordBatchReaderBuilder::try_new(file)
                .with_context(|| format!("read parquet metadata {}", parquet_file.display()))?;
            let mut reader = builder.with_batch_size(4096).build()?;
            for batch in &mut reader {
                let batch = batch?;
                let label_index = batch
                    .schema()
                    .index_of(&manifest.label_column)
                    .with_context(|| format!("missing label column {}", manifest.label_column))?;
                let feature_indexes = candidate_features
                    .iter()
                    .map(|feature| {
                        batch
                            .schema()
                            .index_of(feature)
                            .with_context(|| format!("missing candidate feature column {feature}"))
                            .map(|index| (feature.clone(), index))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;

                for row_index in 0..batch.num_rows() {
                    let label = column_value_at(batch.column(label_index).as_ref(), row_index)
                        .and_then(|value| parse_label(&value))
                        .with_context(|| {
                            format!(
                                "missing or invalid label {} at row {}",
                                manifest.label_column, row_index
                            )
                        })?;
                    let mut features = BTreeMap::new();
                    for (feature, feature_index) in &feature_indexes {
                        if let Some(value) =
                            column_value_at(batch.column(*feature_index).as_ref(), row_index)
                                .and_then(|value| value.parse::<f64>().ok())
                        {
                            features.insert(feature.clone(), value);
                        }
                    }
                    rows.push(RuleBacktestRow {
                        split_name: split.split_name.clone(),
                        label,
                        features,
                    });
                }
            }
        }
    }
    Ok(rows)
}

fn backtest_rule_candidate(
    candidate: &RuleCandidateDraft,
    rows: &[RuleBacktestRow],
) -> anyhow::Result<RuleCandidateBacktestResult> {
    let train_rows = rows
        .iter()
        .filter(|row| row.split_name == "train")
        .collect::<Vec<_>>();
    let selection_rows = if train_rows.is_empty() {
        rows.iter().collect::<Vec<_>>()
    } else {
        train_rows
    };
    let threshold = select_threshold(&candidate.source_feature, &selection_rows)?;
    let mut split_names = rows
        .iter()
        .map(|row| row.split_name.clone())
        .collect::<BTreeSet<_>>();
    if split_names.is_empty() {
        split_names.insert("all".into());
    }

    let mut metrics_by_split = BTreeMap::new();
    for split_name in split_names {
        let split_rows = rows
            .iter()
            .filter(|row| row.split_name == split_name)
            .collect::<Vec<_>>();
        metrics_by_split.insert(
            split_name,
            compute_split_metrics(&candidate.source_feature, threshold, &split_rows),
        );
    }

    let condition_ref = format!("rule_conditions:{}_v1_c1", candidate.candidate_rule_key);
    Ok(RuleCandidateBacktestResult {
        candidate_rule_key: candidate.candidate_rule_key.clone(),
        source_feature: candidate.source_feature.clone(),
        selected_operator: ">=".into(),
        selected_threshold: threshold,
        threshold_selection_split: if rows.iter().any(|row| row.split_name == "train") {
            "train".into()
        } else {
            "all_rows".into()
        },
        rule_library_writeback_template: serde_json::json!({
            "rule_id": candidate.candidate_rule_key,
            "version": 1,
            "name": format!("Model pattern: {}", candidate.source_feature),
            "review_mode": "both",
            "scheme_family": "high_risk_claim",
            "conditions": [
                {
                    "field": candidate.source_feature,
                    "operator": ">=",
                    "value": threshold
                }
            ],
            "action": {
                "score": 20,
                "alert_code": format!("MODEL_PATTERN_{}", safe_id_segment(&candidate.source_feature).to_uppercase()),
                "recommended_action": "ManualReview",
                "action_class": "manual_review",
                "required_evidence": [],
                "reason": "Explainable model pattern candidate; not publishable before deterministic backtest, false-positive review, human approval, and policy governance approval."
            }
        }),
        condition_refs: vec![condition_ref.clone()],
        metrics_by_split,
        gate_status: "backtested_but_blocked_until_human_review".into(),
        required_before_rule_library_writeback: vec![
            "false_positive_review".into(),
            "human_rule_promotion_review".into(),
            "customer_policy_or_model_governance_approval".into(),
            "shadow_or_limited_rollout_if_high_impact".into(),
        ],
        evidence_refs: vec![
            format!("rule_candidate_backtest:{}", candidate.candidate_rule_key),
            condition_ref,
            format!("source_feature:{}", candidate.source_feature),
        ],
    })
}

fn select_threshold(feature: &str, rows: &[&RuleBacktestRow]) -> anyhow::Result<f64> {
    let mut thresholds = rows
        .iter()
        .filter_map(|row| row.features.get(feature).copied())
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    if thresholds.is_empty() {
        bail!("no numeric values available for candidate feature {feature}");
    }
    thresholds.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    thresholds.dedup_by(|left, right| (*left - *right).abs() < f64::EPSILON);

    thresholds
        .into_iter()
        .map(|threshold| {
            let metrics = compute_split_metrics(feature, threshold, rows);
            (threshold, metrics)
        })
        .max_by(
            |(left_threshold, left_metrics), (right_threshold, right_metrics)| {
                left_metrics
                    .f1
                    .partial_cmp(&right_metrics.f1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        left_metrics
                            .precision
                            .partial_cmp(&right_metrics.precision)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .then_with(|| {
                        right_threshold
                            .partial_cmp(left_threshold)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            },
        )
        .map(|(threshold, _)| threshold)
        .ok_or_else(|| anyhow!("no threshold selected for candidate feature {feature}"))
}

fn compute_split_metrics(
    feature: &str,
    threshold: f64,
    rows: &[&RuleBacktestRow],
) -> RuleCandidateSplitMetrics {
    let mut true_positive = 0_u64;
    let mut false_positive = 0_u64;
    let mut true_negative = 0_u64;
    let mut false_negative = 0_u64;

    for row in rows {
        let hit = row
            .features
            .get(feature)
            .is_some_and(|value| *value >= threshold);
        match (hit, row.label) {
            (true, true) => true_positive += 1,
            (true, false) => false_positive += 1,
            (false, true) => false_negative += 1,
            (false, false) => true_negative += 1,
        }
    }

    let row_count = rows.len() as u64;
    let hit_count = true_positive + false_positive;
    let positive_count = true_positive + false_negative;
    let precision = ratio(true_positive, true_positive + false_positive);
    let recall = ratio(true_positive, true_positive + false_negative);
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };

    RuleCandidateSplitMetrics {
        row_count,
        positive_count,
        hit_count,
        hit_rate: ratio(hit_count, row_count),
        true_positive,
        false_positive,
        true_negative,
        false_negative,
        precision,
        recall,
        f1,
        manual_review_capacity_impact: ratio(hit_count, row_count),
    }
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn parse_label(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "positive" => Some(true),
        "0" | "false" | "no" | "negative" => Some(false),
        _ => None,
    }
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn read_feature_importance(path: &Path) -> anyhow::Result<Vec<FeatureImportanceRow>> {
    ensure_parquet_path(path)?;
    let file = File::open(path)
        .with_context(|| format!("open feature importance parquet {}", path.display()))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .with_context(|| format!("read feature importance metadata {}", path.display()))?;
    let mut reader = builder.with_batch_size(4096).build()?;
    let mut rows = Vec::new();
    for batch in &mut reader {
        let batch = batch?;
        let feature_index = batch
            .schema()
            .index_of("feature")
            .context("feature importance missing feature column")?;
        let importance_index = batch
            .schema()
            .index_of("importance")
            .context("feature importance missing importance column")?;
        let kind_index = batch
            .schema()
            .index_of("importance_kind")
            .context("feature importance missing importance_kind column")?;
        let feature_values = column_values(batch.column(feature_index).as_ref());
        let importance_values = column_values(batch.column(importance_index).as_ref());
        let kind_values = column_values(batch.column(kind_index).as_ref());
        for row_index in 0..batch.num_rows() {
            let Some(feature) = feature_values.get(row_index) else {
                continue;
            };
            let Some(importance) = importance_values
                .get(row_index)
                .and_then(|value| value.parse::<f64>().ok())
            else {
                continue;
            };
            let importance_kind = kind_values
                .get(row_index)
                .cloned()
                .unwrap_or_else(|| "unknown".into());
            rows.push(FeatureImportanceRow {
                feature: feature.clone(),
                importance,
                importance_kind,
            });
        }
    }
    rows.sort_by(|left, right| {
        right
            .importance
            .partial_cmp(&left.importance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.feature.cmp(&right.feature))
    });
    Ok(rows)
}

fn build_automl_candidate_rank(
    validation_report_uri: &str,
    report: &serde_json::Value,
) -> anyhow::Result<AutoMlCandidateRank> {
    let model_key = required_manifest_str(report, "model_key")?.to_string();
    let candidate_model_version =
        required_manifest_str(report, "candidate_model_version")?.to_string();
    let algorithm = required_manifest_str(report, "algorithm")?.to_string();
    let metrics = report
        .get("metrics_json")
        .and_then(|value| value.as_object())
        .ok_or_else(|| anyhow!("validation report missing metrics_json"))?;
    let validation_metrics = report
        .get("validation_metrics")
        .unwrap_or(&serde_json::Value::Null);
    let algorithm_family = metrics
        .get("algorithm_family")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let validation_auc = metric_at(validation_metrics, "auc");
    let out_of_time_auc = metric_object_value(metrics, "out_of_time_auc");
    let out_of_time_average_precision =
        metric_object_value(metrics, "out_of_time_average_precision");
    let out_of_time_precision = metric_object_value(metrics, "out_of_time_precision");
    let out_of_time_recall = metric_object_value(metrics, "out_of_time_recall");
    let score_psi =
        metric_object_value(metrics, "score_psi").or_else(|| metric_object_value(metrics, "psi"));
    let max_feature_psi = metric_object_value(metrics, "max_feature_psi");
    let permutation_importance_passed =
        automl_permutation_importance_passed(metrics) && automl_has_permutation_importance(metrics);
    let feature_reproducibility_passed = automl_feature_reproducibility_passed(metrics);

    let blocking_reasons = automl_blocking_reasons(metrics);
    let gate_status = if blocking_reasons.is_empty() {
        "passed"
    } else {
        "blocked"
    }
    .to_string();
    let recommended_action = if gate_status == "passed" {
        "open_human_review"
    } else {
        "keep_blocked"
    }
    .to_string();

    let mut evidence_refs = vec![
        format!("model_validation_reports:{validation_report_uri}"),
        format!(
            "model_evaluations:{}",
            safe_id_segment(&candidate_model_version)
        ),
    ];
    if let Some(feature_search_report_uri) = metrics
        .get("automl_feature_search_report_uri")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
    {
        evidence_refs.push(format!(
            "automl_feature_search_reports:{feature_search_report_uri}"
        ));
    }
    if let Some(factor_ranking_report_uri) = metrics
        .get("automl_factor_ranking_report_uri")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
    {
        evidence_refs.push(format!(
            "automl_factor_rankings:{factor_ranking_report_uri}"
        ));
    }

    Ok(AutoMlCandidateRank {
        rank: 0,
        model_key,
        candidate_model_version: candidate_model_version.clone(),
        algorithm,
        algorithm_family,
        dataset_key: report
            .get("dataset_key")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        dataset_version: report
            .get("dataset_version")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        validation_report_uri: validation_report_uri.into(),
        ranking_score: automl_ranking_score(
            out_of_time_auc,
            out_of_time_average_precision,
            out_of_time_precision,
            out_of_time_recall,
            score_psi,
            max_feature_psi,
            permutation_importance_passed,
            feature_reproducibility_passed,
            &gate_status,
        ),
        validation_auc,
        out_of_time_auc,
        out_of_time_average_precision,
        out_of_time_precision,
        out_of_time_recall,
        score_psi,
        max_feature_psi,
        overfitting_penalty: automl_overfitting_penalty(
            score_psi,
            max_feature_psi,
            permutation_importance_passed,
            feature_reproducibility_passed,
        ),
        gate_status,
        blocking_reasons,
        recommended_action,
        evidence_refs,
    })
}

fn automl_blocking_reasons(metrics: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    let required_statuses = [
        "time_group_split_status",
        "leakage_check_status",
        "out_of_time_validation_status",
        "score_stability_status",
        "feature_stability_status",
        "overfitting_diagnostics_status",
        "shadow_comparison_status",
        "serving_version_lock_status",
        "artifact_integrity_status",
        "feature_store_materialization_status",
        "segment_fairness_status",
        "label_provenance_status",
    ];
    let mut reasons = Vec::new();
    for key in required_statuses {
        let status = metrics
            .get(key)
            .and_then(|value| value.as_str())
            .unwrap_or("missing");
        if status != "passed" {
            reasons.push(format!("{key}:{status}"));
        }
    }
    if metrics
        .get("rust_feature_set_status")
        .and_then(|value| value.as_str())
        != Some("passed")
    {
        reasons.push("rust_feature_set_status:missing_or_failed".into());
    }
    if !metrics
        .get("rust_feature_set_manifest_uri")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty())
    {
        reasons.push("rust_feature_set_manifest_uri:missing".into());
    }
    if !automl_feature_search_passed(metrics) {
        reasons.push("automl_feature_search_status:missing_or_failed".into());
    }
    if !automl_has_feature_search_report(metrics) {
        reasons.push("automl_feature_search_report_uri:missing".into());
    }
    if metric_object_value(metrics, "automl_selected_feature_count").unwrap_or(0.0) <= 0.0 {
        reasons.push("automl_selected_feature_count:missing_or_zero".into());
    }
    if !automl_factor_ranking_passed(metrics) {
        reasons.push("automl_factor_ranking_status:missing_or_failed".into());
    }
    if !automl_has_factor_ranking_report(metrics) {
        reasons.push("automl_factor_ranking_report_uri:missing".into());
    }
    if metric_object_value(metrics, "automl_ranked_factor_count").unwrap_or(0.0) <= 0.0 {
        reasons.push("automl_ranked_factor_count:missing_or_zero".into());
    }
    if !automl_rust_serving_evaluation_passed(metrics) {
        reasons.push("model_artifact_evaluation_status:missing_or_failed".into());
    }
    if automl_requires_onnx_parity(metrics) && !automl_onnx_parity_passed(metrics) {
        reasons.push("onnx_parity_status:missing_or_failed".into());
    }
    if automl_requires_onnx_parity(metrics)
        && !metrics
            .get("onnx_parity_report_uri")
            .and_then(|value| value.as_str())
            .is_some_and(|value| !value.trim().is_empty())
    {
        reasons.push("onnx_parity_report_uri:missing".into());
    }
    if metric_object_value(metrics, "out_of_time_auc").unwrap_or(0.0) < 0.5 {
        reasons.push("out_of_time_auc:below_0_5".into());
    }
    if metric_object_value(metrics, "out_of_time_recall").unwrap_or(0.0) <= 0.0 {
        reasons.push("out_of_time_recall:missing_or_zero".into());
    }
    if !automl_has_time_group_split_fields(metrics) {
        reasons.push("time_group_split_fields:missing".into());
    }
    if !automl_permutation_importance_passed(metrics) {
        reasons.push("permutation_importance_status:missing_or_failed".into());
    }
    if !automl_has_permutation_importance(metrics) {
        reasons.push("permutation_importance_uri:missing".into());
    }
    if !metrics
        .get("overfitting_diagnostics_report_uri")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty())
    {
        reasons.push("overfitting_diagnostics_report_uri:missing".into());
    }
    if metric_object_value(metrics, "score_psi")
        .or_else(|| metric_object_value(metrics, "psi"))
        .is_none()
    {
        reasons.push("score_psi:missing".into());
    }
    if metric_object_value(metrics, "max_feature_psi").is_none() {
        reasons.push("max_feature_psi:missing".into());
    }
    if metric_object_value(metrics, "score_psi")
        .or_else(|| metric_object_value(metrics, "psi"))
        .is_some_and(|value| value >= 0.25)
    {
        reasons.push("score_psi:drift".into());
    }
    if metric_object_value(metrics, "max_feature_psi").is_some_and(|value| value >= 0.25) {
        reasons.push("max_feature_psi:drift".into());
    }
    if !automl_feature_reproducibility_passed(metrics) {
        reasons.push("feature_reproducibility_hash:missing".into());
    }
    reasons
}

fn automl_has_time_group_split_fields(
    metrics: &serde_json::Map<String, serde_json::Value>,
) -> bool {
    let has_time = metrics
        .get("time_split_field")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty());
    let has_group = metrics
        .get("group_split_fields")
        .and_then(|value| value.as_array())
        .is_some_and(|fields| {
            fields
                .iter()
                .any(|field| field.as_str().is_some_and(|value| !value.trim().is_empty()))
        });
    has_time && has_group
}

fn automl_permutation_importance_passed(
    metrics: &serde_json::Map<String, serde_json::Value>,
) -> bool {
    metrics
        .get("permutation_importance_status")
        .and_then(|value| value.as_str())
        == Some("passed")
}

fn automl_has_permutation_importance(metrics: &serde_json::Map<String, serde_json::Value>) -> bool {
    metrics
        .get("permutation_importance_uri")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty())
}

fn automl_feature_reproducibility_passed(
    metrics: &serde_json::Map<String, serde_json::Value>,
) -> bool {
    metrics
        .get("feature_reproducibility_hash")
        .and_then(|value| value.as_str())
        .is_some_and(|value| value.starts_with("sha256:") && value.len() > "sha256:".len())
}

fn automl_feature_search_passed(metrics: &serde_json::Map<String, serde_json::Value>) -> bool {
    metrics
        .get("automl_feature_search_status")
        .and_then(|value| value.as_str())
        == Some("passed")
}

fn automl_has_feature_search_report(metrics: &serde_json::Map<String, serde_json::Value>) -> bool {
    metrics
        .get("automl_feature_search_report_uri")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty())
}

fn automl_factor_ranking_passed(metrics: &serde_json::Map<String, serde_json::Value>) -> bool {
    metrics
        .get("automl_factor_ranking_status")
        .and_then(|value| value.as_str())
        == Some("passed")
}

fn automl_has_factor_ranking_report(metrics: &serde_json::Map<String, serde_json::Value>) -> bool {
    metrics
        .get("automl_factor_ranking_report_uri")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty())
}

fn automl_requires_onnx_parity(metrics: &serde_json::Map<String, serde_json::Value>) -> bool {
    matches!(
        metrics.get("algorithm").and_then(|value| value.as_str()),
        Some("xgboost" | "lightgbm")
    ) || matches!(
        metrics.get("runtime_kind").and_then(|value| value.as_str()),
        Some("xgboost_onnx" | "lightgbm_onnx" | "deep_learning_onnx")
    )
}

fn automl_onnx_parity_passed(metrics: &serde_json::Map<String, serde_json::Value>) -> bool {
    metrics
        .get("onnx_parity_gate_status")
        .and_then(|value| value.as_str())
        == Some("passed")
        || metrics
            .get("onnx_parity_status")
            .and_then(|value| value.as_str())
            == Some("passed")
}

fn automl_rust_serving_evaluation_passed(
    metrics: &serde_json::Map<String, serde_json::Value>,
) -> bool {
    if metrics
        .get("model_artifact_evaluation_status")
        .and_then(|value| value.as_str())
        == Some("passed")
    {
        return true;
    }
    if metrics
        .get("model_artifact_evaluation_gate_status")
        .and_then(|value| value.as_str())
        == Some("passed")
    {
        return true;
    }
    metrics
        .get("model_artifact_evaluation")
        .is_some_and(|value| {
            value.get("report_kind").and_then(|value| value.as_str())
                == Some("model_artifact_evaluation")
                && value.get("gate_status").and_then(|value| value.as_str()) == Some("passed")
        })
}

fn automl_ranking_score(
    out_of_time_auc: Option<f64>,
    average_precision: Option<f64>,
    precision: Option<f64>,
    recall: Option<f64>,
    score_psi: Option<f64>,
    max_feature_psi: Option<f64>,
    permutation_importance_passed: bool,
    feature_reproducibility_passed: bool,
    gate_status: &str,
) -> f64 {
    let score = out_of_time_auc.unwrap_or(0.0) * 60.0
        + average_precision.unwrap_or(0.0) * 20.0
        + precision.unwrap_or(0.0) * 10.0
        + recall.unwrap_or(0.0) * 10.0;
    let penalty = automl_overfitting_penalty(
        score_psi,
        max_feature_psi,
        permutation_importance_passed,
        feature_reproducibility_passed,
    ) + if gate_status == "passed" { 0.0 } else { 100.0 };
    ((score - penalty) * 10_000.0).round() / 10_000.0
}

fn automl_overfitting_penalty(
    score_psi: Option<f64>,
    max_feature_psi: Option<f64>,
    permutation_importance_passed: bool,
    feature_reproducibility_passed: bool,
) -> f64 {
    let stability_penalty = score_psi.unwrap_or(1.0) * 25.0 + max_feature_psi.unwrap_or(1.0) * 15.0;
    let permutation_penalty = if permutation_importance_passed {
        0.0
    } else {
        20.0
    };
    let reproducibility_penalty = if feature_reproducibility_passed {
        0.0
    } else {
        20.0
    };
    ((stability_penalty + permutation_penalty + reproducibility_penalty) * 10_000.0).round()
        / 10_000.0
}

fn eligible_sort_key(candidate: &AutoMlCandidateRank) -> f64 {
    if candidate.gate_status == "passed" {
        candidate.ranking_score
    } else {
        candidate.ranking_score - 1_000.0
    }
}

fn metric_at(value: &serde_json::Value, key: &str) -> Option<f64> {
    value.get(key).and_then(metric_value)
}

fn metric_object_value(
    metrics: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<f64> {
    metrics.get(key).and_then(metric_value)
}

fn metric_value(value: &serde_json::Value) -> Option<f64> {
    if let Some(value) = value.as_f64() {
        return Some(value);
    }
    value.as_str().and_then(|value| value.parse::<f64>().ok())
}

fn required_non_empty<'a>(field: &str, value: &'a str) -> anyhow::Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        bail!("{field} is required");
    }
    Ok(value)
}

fn required_optional<'a>(field: &str, value: Option<&'a str>) -> anyhow::Result<&'a str> {
    value
        .map(|value| required_non_empty(field, value))
        .transpose()?
        .with_context(|| format!("{field} is required when API submission is requested"))
}

fn artifact_parent_uri(artifact_uri: &str) -> &str {
    artifact_uri
        .trim()
        .rsplit_once('/')
        .map(|(parent, _)| parent)
        .unwrap_or_else(|| artifact_uri.trim())
}

fn artifact_parent_path(artifact_uri: &str) -> PathBuf {
    let parent_uri = artifact_parent_uri(artifact_uri);
    let local_parent = parent_uri
        .strip_prefix("artifact://")
        .or_else(|| parent_uri.strip_prefix("file://"))
        .unwrap_or(parent_uri);
    PathBuf::from(local_parent)
}

fn required_manifest_str<'a>(
    manifest: &'a serde_json::Value,
    key: &str,
) -> anyhow::Result<&'a str> {
    manifest
        .get(key)
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("training manifest missing {key}"))
}

pub(crate) fn build_training_retraining_output(
    job: &ClaimedRetrainingJob,
    actor: &str,
    artifact_base_uri: &str,
    training_manifest: &str,
    trainer_python: &str,
    trainer_workdir: Option<&str>,
    algorithm: Option<&str>,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    let training_command = build_training_command(
        trainer_python,
        training_manifest,
        artifact_base_uri,
        job,
        actor,
        trainer_workdir,
        algorithm,
    );
    let mut command = Command::new(&training_command.program);
    command.args(&training_command.args);
    if let Some(workdir) = &training_command.workdir {
        command.current_dir(workdir);
    }
    let output = command
        .output()
        .with_context(|| format!("run model training command {}", training_command.program))?;
    if !output.status.success() {
        bail!(
            "model training command failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = serde_json::from_slice::<CompleteRetrainingJobPayload>(&output.stdout)
        .context("parse model training output")?;
    enrich_retraining_output_with_rust_feature_set(output, training_manifest)
}

fn enrich_retraining_output_with_rust_feature_set(
    mut output: CompleteRetrainingJobPayload,
    training_manifest: &str,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    let feature_set_output_dir =
        artifact_parent_path(&output.artifact_uri).join("rust_feature_set");
    let feature_set_id = format!(
        "{}:{}",
        output.candidate_model_version, "rust_feature_set_v1"
    );
    let feature_set = build_feature_set(
        training_manifest,
        &feature_set_output_dir,
        Some(&feature_set_id),
    )
    .context("build Rust feature set for retraining output")?;
    let feature_set_manifest_uri = feature_set_output_dir
        .join("feature_set_manifest.json")
        .to_string_lossy()
        .into_owned();

    let Some(metrics) = output.metrics_json.as_object_mut() else {
        bail!("training output metrics_json must be an object");
    };
    if let Some(existing_hash) = metrics
        .get("feature_reproducibility_hash")
        .and_then(|value| value.as_str())
    {
        metrics.insert(
            "trainer_feature_reproducibility_hash".into(),
            serde_json::Value::String(existing_hash.to_string()),
        );
    }
    metrics.insert(
        "feature_reproducibility_hash".into(),
        serde_json::Value::String(feature_set.feature_reproducibility_hash.clone()),
    );
    metrics.insert(
        "rust_feature_set_manifest_uri".into(),
        serde_json::Value::String(feature_set_manifest_uri.clone()),
    );
    metrics.insert(
        "rust_feature_set_status".into(),
        serde_json::Value::String("passed".into()),
    );
    metrics.insert(
        "feature_store_materialization_status".into(),
        serde_json::Value::String("passed".into()),
    );

    let evidence_ref = format!("feature_set_manifests:{feature_set_manifest_uri}");
    if !output
        .evidence_refs
        .iter()
        .any(|reference| reference == &evidence_ref)
    {
        output.evidence_refs.push(evidence_ref);
    }
    Ok(output)
}

pub(crate) async fn enrich_retraining_output_with_model_artifact_evaluation(
    mut output: CompleteRetrainingJobPayload,
    training_manifest: &str,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    let Some(serving_manifest_uri) = output.serving_manifest_uri.clone() else {
        return Ok(output);
    };
    let artifact_eval_output_dir =
        artifact_parent_path(&output.artifact_uri).join("artifact-evaluation");
    let report = evaluate_model_artifact(
        &serving_manifest_uri,
        training_manifest,
        "validation",
        &artifact_eval_output_dir,
        None,
        0.0001,
        100,
        100,
        Some(&model_artifact_evaluation_signing_key()),
    )
    .await
    .context("evaluate Rust serving artifact before retraining registration")?;
    let onnx_parity = validate_onnx_parity_for_runtime(
        report.runtime_kind.as_str(),
        output.onnx_parity_report_uri.as_deref(),
    )?;
    let gate_status = if report.gate_status == "passed"
        && onnx_parity
            .as_ref()
            .is_none_or(|parity| parity.gate_status == "passed")
    {
        "passed".to_string()
    } else {
        "blocked".to_string()
    };
    let report_uri = artifact_eval_output_dir
        .join("model_artifact_evaluation_report.json")
        .to_string_lossy()
        .into_owned();

    let Some(metrics) = output.metrics_json.as_object_mut() else {
        bail!("training output metrics_json must be an object");
    };
    metrics.insert(
        "model_artifact_evaluation_status".into(),
        serde_json::Value::String(gate_status.clone()),
    );
    metrics.insert(
        "model_artifact_evaluation_gate_status".into(),
        serde_json::Value::String(gate_status.clone()),
    );
    metrics.insert(
        "model_artifact_evaluation_report_uri".into(),
        serde_json::Value::String(report_uri.clone()),
    );
    metrics.insert(
        "rust_serving_status".into(),
        serde_json::Value::String(report.rust_serving_status),
    );
    metrics.insert(
        "rust_serving_latency_status".into(),
        serde_json::Value::String(report.latency_status),
    );
    metrics.insert(
        "rust_serving_p95_latency_ms".into(),
        serde_json::json!(report.p95_latency_ms),
    );
    metrics.insert(
        "rust_serving_runtime_kind".into(),
        serde_json::Value::String(report.runtime_kind),
    );
    if let Some(parity) = &onnx_parity {
        metrics.insert(
            "onnx_parity_gate_status".into(),
            serde_json::Value::String(parity.gate_status.clone()),
        );
        metrics.insert(
            "onnx_parity_status".into(),
            serde_json::Value::String(parity.status.clone()),
        );
        metrics.insert(
            "onnx_parity_report_uri".into(),
            serde_json::Value::String(parity.report_uri.clone()),
        );
        metrics.insert(
            "onnx_serving_runtime_kind".into(),
            serde_json::Value::String(parity.serving_runtime_kind.clone()),
        );
        metrics.insert(
            "onnx_max_abs_probability_delta".into(),
            serde_json::json!(parity.max_abs_probability_delta),
        );
        metrics.insert(
            "onnx_probability_tolerance".into(),
            serde_json::json!(parity.tolerance),
        );
    }

    let evidence_ref = format!("model_artifact_evaluations:{report_uri}");
    if !output
        .evidence_refs
        .iter()
        .any(|reference| reference == &evidence_ref)
    {
        output.evidence_refs.push(evidence_ref);
    }
    if let Some(parity) = onnx_parity {
        let evidence_ref = format!("model_onnx_parity_reports:{}", parity.report_uri);
        if !output
            .evidence_refs
            .iter()
            .any(|reference| reference == &evidence_ref)
        {
            output.evidence_refs.push(evidence_ref);
        }
    }
    Ok(output)
}

pub(crate) fn enrich_retraining_output_with_rule_candidate_workflow(
    mut output: CompleteRetrainingJobPayload,
    training_manifest: &str,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    let Some(feature_importance_uri) = output.feature_importance_uri.clone() else {
        return Ok(output);
    };
    let training_platform_candidates = output.mined_rule_candidates.clone();
    let training_platform_candidate_count = training_platform_candidates.len();
    let rule_candidate_dir = artifact_parent_path(&output.artifact_uri).join("rule-candidates");
    let plan = mine_rule_candidates(
        &output.validation_report_uri,
        &feature_importance_uri,
        &rule_candidate_dir,
    )
    .context("mine explainable rule candidates before retraining output registration")?;
    let candidate_plan_uri = rule_candidate_dir
        .join("rule_candidate_mining_plan.json")
        .to_string_lossy()
        .into_owned();
    let candidate_review_tasks_uri = rule_candidate_dir
        .join("rule_candidate_review_tasks.json")
        .to_string_lossy()
        .into_owned();
    let backtest_dir = rule_candidate_dir.join("backtest");
    let backtest =
        run_rule_candidate_backtest(&candidate_plan_uri, training_manifest, &backtest_dir)
            .context(
                "backtest explainable rule candidates before retraining output registration",
            )?;
    let backtest_report_uri = backtest_dir
        .join("rule_candidate_backtest_report.json")
        .to_string_lossy()
        .into_owned();
    let backtest_review_tasks_uri = backtest_dir
        .join("rule_candidate_backtest_review_tasks.json")
        .to_string_lossy()
        .into_owned();
    let mut rule_candidates = training_platform_candidates;
    let mut existing_rule_ids = rule_candidates
        .iter()
        .filter_map(|candidate| {
            candidate
                .get("rule_id")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .collect::<BTreeSet<_>>();
    let backtested_rule_candidates = backtest
        .candidate_results
        .iter()
        .map(|result| result.rule_library_writeback_template.clone())
        .collect::<Vec<_>>();
    let backtested_rule_candidate_count = backtested_rule_candidates.len();
    for candidate in backtested_rule_candidates {
        let Some(rule_id) = candidate.get("rule_id").and_then(|value| value.as_str()) else {
            continue;
        };
        if existing_rule_ids.insert(rule_id.to_string()) {
            rule_candidates.push(candidate);
        }
    }
    output.mined_rule_candidates = rule_candidates;

    let Some(metrics) = output.metrics_json.as_object_mut() else {
        bail!("training output metrics_json must be an object");
    };
    metrics.insert(
        "rule_candidate_mining_status".into(),
        serde_json::Value::String("passed".into()),
    );
    metrics.insert(
        "rule_candidate_mining_plan_uri".into(),
        serde_json::Value::String(candidate_plan_uri.clone()),
    );
    metrics.insert(
        "rule_candidate_source_count".into(),
        serde_json::json!(plan.candidate_rules.len()),
    );
    metrics.insert(
        "rule_candidate_backtest_status".into(),
        serde_json::Value::String("passed".into()),
    );
    metrics.insert(
        "rule_candidate_backtest_report_uri".into(),
        serde_json::Value::String(backtest_report_uri.clone()),
    );
    metrics.insert(
        "rule_candidate_review_tasks_uri".into(),
        serde_json::Value::String(backtest_review_tasks_uri.clone()),
    );
    metrics.insert(
        "rule_candidate_review_task_count".into(),
        serde_json::json!(backtest.review_tasks.len()),
    );
    metrics.insert(
        "mined_rule_candidates_source".into(),
        serde_json::Value::String(
            "training_platform_and_deterministic_rule_candidate_backtest".into(),
        ),
    );
    metrics.insert(
        "training_platform_mined_rule_candidate_count".into(),
        serde_json::json!(training_platform_candidate_count),
    );
    metrics.insert(
        "mined_rule_candidates_backtested_count".into(),
        serde_json::json!(backtested_rule_candidate_count),
    );
    metrics.insert(
        "rule_library_writeback_status".into(),
        serde_json::Value::String(backtest.rule_library_writeback_status.clone()),
    );
    metrics.insert(
        "rule_candidate_workflow_boundary".into(),
        serde_json::Value::String(
            "rule candidates are backtested and handed to human review only; worker must not write active rules".into(),
        ),
    );

    push_unique_evidence_ref(
        &mut output.evidence_refs,
        format!("rule_candidate_mining_plans:{candidate_plan_uri}"),
    );
    push_unique_evidence_ref(
        &mut output.evidence_refs,
        format!("rule_candidate_review_tasks:{candidate_review_tasks_uri}"),
    );
    push_unique_evidence_ref(
        &mut output.evidence_refs,
        format!("rule_candidate_backtests:{backtest_report_uri}"),
    );
    push_unique_evidence_ref(
        &mut output.evidence_refs,
        format!("rule_candidate_review_tasks:{backtest_review_tasks_uri}"),
    );
    Ok(output)
}

fn push_unique_evidence_ref(evidence_refs: &mut Vec<String>, evidence_ref: String) {
    if !evidence_refs
        .iter()
        .any(|reference| reference == &evidence_ref)
    {
        evidence_refs.push(evidence_ref);
    }
}

#[derive(Debug, Clone, PartialEq)]
struct OnnxParityGate {
    report_uri: String,
    gate_status: String,
    status: String,
    serving_runtime_kind: String,
    max_abs_probability_delta: Option<f64>,
    tolerance: Option<f64>,
}

fn validate_onnx_parity_for_runtime(
    runtime_kind: &str,
    onnx_parity_report_uri: Option<&str>,
) -> anyhow::Result<Option<OnnxParityGate>> {
    if !runtime_kind.ends_with("_onnx") {
        return Ok(None);
    }
    let report_uri = onnx_parity_report_uri
        .filter(|uri| !uri.trim().is_empty())
        .ok_or_else(|| anyhow!("ONNX runtime {runtime_kind} requires onnx_parity_report_uri"))?;
    let report = read_json_report(report_uri)?;
    if report["report_kind"] != "onnx_probability_parity" {
        bail!("ONNX parity report {report_uri} has invalid report_kind");
    }
    let serving_runtime_kind =
        json_string(&report, "serving_runtime_kind").unwrap_or_else(|| "missing".into());
    let status = json_string(&report, "status").unwrap_or_else(|| "missing".into());
    let max_abs_probability_delta = metric_at(&report, "max_abs_probability_delta");
    let tolerance = metric_at(&report, "tolerance");
    let gate_status = if status == "passed" && serving_runtime_kind == runtime_kind {
        "passed"
    } else {
        "blocked"
    }
    .to_string();
    Ok(Some(OnnxParityGate {
        report_uri: report_uri.into(),
        gate_status,
        status,
        serving_runtime_kind,
        max_abs_probability_delta,
        tolerance,
    }))
}

fn model_artifact_evaluation_signing_key() -> String {
    std::env::var("FWA_MODEL_SIGNATURE_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "local-dev-model-signing-key".into())
}

pub(crate) fn build_mock_retraining_output(
    job: &ClaimedRetrainingJob,
    actor: &str,
    artifact_base_uri: &str,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    if artifact_base_uri.trim().is_empty() {
        bail!("artifact_base_uri is required");
    }
    let safe_model_key = safe_path_segment(&job.model_key);
    let candidate_model_version = format!(
        "{}-candidate-{}",
        safe_path_segment(&job.model_version),
        safe_path_segment(&job.job_id)
    );
    let artifact_root = artifact_base_uri.trim().trim_end_matches('/');
    let artifact_uri =
        format!("{artifact_root}/{safe_model_key}/{candidate_model_version}/model.onnx");
    let validation_report_uri =
        format!("{artifact_root}/{safe_model_key}/{candidate_model_version}/validation.json");
    let feature_importance_uri = format!(
        "{artifact_root}/{safe_model_key}/{candidate_model_version}/feature_importance.parquet"
    );
    let permutation_importance_uri = format!(
        "{artifact_root}/{safe_model_key}/{candidate_model_version}/permutation_importance.parquet"
    );
    let artifact_evaluation_report_uri = format!(
        "{artifact_root}/{safe_model_key}/{candidate_model_version}/artifact-evaluation/model_artifact_evaluation_report.json"
    );
    let rule_backtest_report_uri = format!(
        "{artifact_root}/{safe_model_key}/{candidate_model_version}/rule-candidates/backtest/rule_candidate_backtest_report.json"
    );
    let rule_review_tasks_uri = format!(
        "{artifact_root}/{safe_model_key}/{candidate_model_version}/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json"
    );
    let evaluation_run_id = format!(
        "eval_{}_{}",
        safe_id_segment(&job.model_key),
        safe_id_segment(&candidate_model_version)
    );
    let evidence_refs = vec![
        format!("model_retraining_jobs:{}", job.job_id),
        format!("model_artifacts:{artifact_uri}"),
        format!("model_validation_reports:{validation_report_uri}"),
        format!("model_feature_importance:{feature_importance_uri}"),
        format!("model_permutation_importance:{permutation_importance_uri}"),
        format!("model_artifact_evaluations:{artifact_evaluation_report_uri}"),
        format!("rule_candidate_backtests:{rule_backtest_report_uri}"),
        format!("rule_candidate_review_tasks:{rule_review_tasks_uri}"),
        format!("model_evaluations:{evaluation_run_id}"),
    ];

    Ok(CompleteRetrainingJobPayload {
        actor: actor.to_string(),
        notes: "Candidate model and validation report registered by worker.".into(),
        candidate_model_version,
        artifact_uri,
        artifact_sha256: None,
        training_artifact_uri: None,
        training_artifact_sha256: None,
        serving_manifest_uri: None,
        onnx_parity_report_uri: None,
        endpoint_url: None,
        validation_report_uri,
        evaluation_run_id,
        auc: Some("0.86".into()),
        ks: Some("0.48".into()),
        precision: Some("0.78".into()),
        recall: Some("0.71".into()),
        f1: Some("0.74".into()),
        accuracy: Some("0.79".into()),
        threshold: Some("0.52".into()),
        confusion_matrix_json: serde_json::json!({
            "tp": 24,
            "fp": 6,
            "tn": 52,
            "fn": 8
        }),
        feature_importance_uri: Some(feature_importance_uri),
        permutation_importance_uri: Some(permutation_importance_uri),
        metrics_json: serde_json::json!({
            "out_of_time_auc": 0.82,
            "out_of_time_precision": 0.76,
            "out_of_time_recall": 0.71,
            "score_psi": 0.04,
            "max_feature_psi": 0.08,
            "leakage_check_status": "passed",
            "time_group_split_status": "passed",
            "time_split_field": "service_date",
            "group_split_fields": ["member_id", "policy_id", "provider_id"],
            "feature_reproducibility_hash": "sha256:demo-retraining-feature-reproducibility",
            "label_provenance_status": "passed",
            "label_reviewer_source": "investigation_results",
            "pilot_validation_status": "passed",
            "shadow_comparison_status": "passed",
            "serving_version_lock_status": "passed",
            "artifact_integrity_status": "passed",
            "feature_store_materialization_status": "passed",
            "segment_fairness_status": "passed",
            "review_capacity_threshold_status": "passed",
            "model_artifact_evaluation_status": "passed",
            "model_artifact_evaluation_report_uri": artifact_evaluation_report_uri,
            "rule_candidate_backtest_status": "passed",
            "rule_candidate_backtest_report_uri": rule_backtest_report_uri,
            "rule_candidate_review_tasks_uri": rule_review_tasks_uri,
            "rule_library_writeback_status": "blocked_pending_human_review_and_policy_governance_approval"
        }),
        evidence_refs,
        mined_rule_owner: None,
        mined_rule_candidates: Vec::new(),
    })
}

fn safe_path_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if sanitized.is_empty() {
        "unknown".into()
    } else {
        sanitized
    }
}

fn safe_id_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if sanitized.is_empty() {
        "unknown".into()
    } else {
        sanitized
    }
}

fn resolve_parquet_files(base_dir: &Path, data_uri: &str) -> anyhow::Result<Vec<PathBuf>> {
    let path = PathBuf::from(data_uri);
    let path = if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    };

    if path.is_file() {
        ensure_parquet_path(&path)?;
        return Ok(vec![path]);
    }

    if path.is_dir() {
        let mut files = fs::read_dir(&path)
            .with_context(|| format!("read parquet directory {}", path.display()))?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_file())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "parquet")
            })
            .collect::<Vec<_>>();
        files.sort();
        return Ok(files);
    }

    Err(anyhow!(
        "parquet data_uri does not exist: {}",
        path.display()
    ))
}

fn ensure_parquet_path(path: &Path) -> anyhow::Result<()> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("parquet") => Ok(()),
        _ => bail!("data_uri file must end with .parquet: {}", path.display()),
    }
}

fn column_value_at(array: &dyn arrow_array::Array, index: usize) -> Option<String> {
    use arrow_array::{
        BooleanArray, Float64Array, Int16Array, Int32Array, Int64Array, Int8Array,
        LargeStringArray, StringArray, UInt16Array, UInt32Array, UInt64Array, UInt8Array,
    };

    if array.is_null(index) {
        return None;
    }
    if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<LargeStringArray>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<BooleanArray>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int8Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int16Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int32Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt8Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt16Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt32Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt64Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        return Some(values.value(index).to_string());
    }
    None
}

fn column_values(array: &dyn arrow_array::Array) -> Vec<String> {
    use arrow_array::{
        Array, BooleanArray, Float64Array, Int16Array, Int32Array, Int64Array, Int8Array,
        LargeStringArray, StringArray, UInt16Array, UInt32Array, UInt64Array, UInt8Array,
    };

    if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        return (0..values.len())
            .filter(|index| !values.is_null(*index))
            .map(|index| values.value(index).to_string())
            .collect();
    }
    if let Some(values) = array.as_any().downcast_ref::<LargeStringArray>() {
        return (0..values.len())
            .filter(|index| !values.is_null(*index))
            .map(|index| values.value(index).to_string())
            .collect();
    }
    if let Some(values) = array.as_any().downcast_ref::<BooleanArray>() {
        return (0..values.len())
            .filter(|index| !values.is_null(*index))
            .map(|index| values.value(index).to_string())
            .collect();
    }
    if let Some(values) = array.as_any().downcast_ref::<Int8Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int16Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int32Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt8Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt16Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt32Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt64Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        return primitive_values(values, |value| value.to_string());
    }

    Vec::new()
}

fn primitive_values<T, F>(array: &arrow_array::PrimitiveArray<T>, format: F) -> Vec<String>
where
    T: arrow_array::ArrowPrimitiveType,
    F: Fn(T::Native) -> String,
{
    use arrow_array::Array;

    (0..array.len())
        .filter(|index| !array.is_null(*index))
        .map(|index| format(array.value(index)))
        .collect()
}

#[cfg(test)]
mod tests;
