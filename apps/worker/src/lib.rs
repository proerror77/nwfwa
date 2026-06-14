#![allow(clippy::too_many_arguments)]

use anyhow::{bail, Context};
use arrow_array::RecordBatch;
#[cfg(test)]
use arrow_array::{Float64Array, Int8Array, StringArray};
use arrow_schema::Schema;
#[cfg(test)]
use arrow_schema::{DataType, Field};
use hmac::Hmac;
use parquet::arrow::ArrowWriter;
use serde::Serialize;
use sha2::Sha256;
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    sync::Arc,
};

type HmacSha256 = Hmac<Sha256>;

mod dataset_types;
pub use dataset_types::*;

mod demo_datasets;
pub use demo_datasets::build_demo_ml_datasets;

mod demo_lifecycle;
pub use demo_lifecycle::{build_demo_automl_lifecycle_evidence, verify_demo_automl_lifecycle};

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
mod mlops_monitoring_plan;
mod mlops_monitoring_report;
pub use mlops_monitoring::{
    build_mlops_monitoring_plan, build_mlops_monitoring_report,
    build_mlops_scheduler_execution_report, run_mlops_monitoring_plan,
    run_mlops_monitoring_plan_with_inputs, run_scheduled_mlops_monitoring,
    run_scheduled_mlops_monitoring_with_artifact_base_uri,
    run_scheduled_mlops_monitoring_with_options,
};
#[cfg(test)]
pub(crate) use mlops_monitoring_runtime::sha256_prefixed_hex;
mod mlops_monitoring_runtime;

mod mlops_cycle;
pub use mlops_cycle::{build_mlops_monitoring_cycle_evidence, run_mlops_monitoring_cycle};

mod anomaly_upgrade;
pub use anomaly_upgrade::{
    build_anomaly_upgrade_readiness_report, AnomalyUpgradeReadinessInput,
    AnomalyUpgradeReadinessReport, AnomalyUpgradeReviewTask,
};

mod audit_retention;
pub use audit_retention::{
    build_audit_retention_scan_report, AuditRetentionCandidate, AuditRetentionRecord,
    AuditRetentionReviewTask, AuditRetentionScanInput, AuditRetentionScanReport,
};

mod probability_calibration;
pub use probability_calibration::{
    build_probability_calibration_report, ProbabilityCalibrationBin, ProbabilityCalibrationInput,
    ProbabilityCalibrationReport, ProbabilityCalibrationReviewTask, ProbabilityCalibrationRow,
};

mod model_artifact_evaluation;
pub use model_artifact_evaluation::evaluate_model_artifact;

mod rule_candidates;
pub use rule_candidates::{mine_rule_candidates, run_rule_candidate_backtest};

mod clustering;
mod clustering_data;
mod clustering_math;
mod clustering_types;
pub use clustering::{
    cluster_claim_entities, cluster_provider_graph_communities, cluster_provider_peers,
};
pub use clustering_types::{
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

mod automl_ranking;
pub use automl_ranking::rank_automl_candidates;
pub(crate) use automl_ranking::{metric_at, read_feature_importance, round4};

mod ops_plans;
pub use ops_plans::{
    build_ai_evidence_execution_plan, build_analytics_export_plan, build_governance_ops_plan,
};

mod sanctions;
pub use sanctions::{
    build_sanctions_sync_report, build_sanctions_sync_report_submission,
    submit_sanctions_sync_report, SanctionsProviderUpsert, SanctionsSourceRecord,
    SanctionsSourceSnapshot, SanctionsSyncReport, SanctionsSyncReportSubmission,
    SanctionsSyncReviewTask,
};

mod provider_profile_rollup;
pub use provider_profile_rollup::{
    build_provider_profile_window_rollup, build_provider_profile_window_rollup_submission,
    submit_provider_profile_window_rollup, ProviderProfileClaimInput, ProviderProfileRollup,
    ProviderProfileRollupInput, ProviderProfileWindowOutput, ProviderProfileWindowRollupReport,
    ProviderProfileWindowRollupSubmission,
};

mod provider_graph_rollup;
pub use provider_graph_rollup::{
    build_provider_graph_signal_rollup, build_provider_graph_signal_rollup_submission,
    submit_provider_graph_signal_rollup, ProviderGraphClaimInput, ProviderGraphRollupInput,
    ProviderGraphSignalRollup, ProviderGraphSignalRollupReport,
    ProviderGraphSignalRollupSubmission, ProviderReferralInput,
};

mod peer_benchmark;
pub use peer_benchmark::{
    build_peer_percentile_benchmark, PeerBenchmarkClaimInput, PeerBenchmarkGroup,
    PeerBenchmarkInput, PeerBenchmarkReport,
};

mod episode_rollup;
pub use episode_rollup::{
    build_episode_aggregation_report, EpisodeAggregationReport, EpisodeClaimInput,
    EpisodeRollupInput, EpisodeWindowRollup, MemberProviderEpisodeRollup,
};

mod clinical_compatibility;
pub use clinical_compatibility::{
    build_clinical_compatibility_reference_report, ClinicalCompatibilityRecord,
    ClinicalCompatibilityReferenceInput, ClinicalCompatibilityReferenceReport,
    ClinicalCompatibilityReferenceRow, ClinicalCompatibilityReviewTask,
};

mod unbundling_comparator;
pub use unbundling_comparator::{
    build_unbundling_comparator_report, UnbundlingComparatorCandidate, UnbundlingComparatorInput,
    UnbundlingComparatorReport, UnbundlingEpisodeInput, UnbundlingRuleInput,
};

mod scoring_feature_context;
pub use scoring_feature_context::{
    build_scoring_feature_context_materialization_submission, build_scoring_feature_context_report,
    submit_scoring_feature_context_materialization, ClaimScoringFeatureContext,
    ScoringFeatureContextClaimInput, ScoringFeatureContextInput,
    ScoringFeatureContextMaterializationSubmission, ScoringFeatureContextReport,
    ScoringFeatureContextSourceUris,
};

mod parquet_utils;
pub(crate) use parquet_utils::{
    column_value_at, column_values, ensure_parquet_path, resolve_parquet_files,
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

mod retraining_output;
pub(crate) use retraining_output::{
    build_mock_retraining_output, build_training_retraining_output,
    enrich_retraining_output_with_model_artifact_evaluation,
    enrich_retraining_output_with_rule_candidate_workflow, required_manifest_str, safe_id_segment,
    safe_path_segment,
};
#[cfg(test)]
pub(crate) use retraining_output::{
    enrich_retraining_output_with_rust_feature_set, validate_onnx_parity_for_runtime,
};

pub mod commands;

#[cfg(test)]
mod tests;
