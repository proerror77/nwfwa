mod flags;

mod build_ai_evidence_execution_plan;
mod build_analytics_export_plan;
mod build_anomaly_upgrade_readiness;
mod build_audit_retention_scan;
mod build_automl_lifecycle_closure_report;
mod build_clinical_compatibility_reference;
mod build_demo_automl_lifecycle_evidence;
mod build_demo_ml_datasets;
mod build_episode_aggregation;
mod build_feature_set;
mod build_governance_ops_plan;
mod build_mlops_monitoring_plan;
mod build_mlops_monitoring_report;
mod build_mlops_scheduler_execution_report;
mod build_model_promotion_orchestration_report;
mod build_peer_benchmarks;
mod build_probability_calibration_report;
mod build_provider_graph_signals;
mod build_provider_profile_windows;
mod build_scoring_feature_contexts;
mod build_training_handoff;
mod build_unbundling_comparator;
mod check_pilot_readiness;
mod claim_retraining_job;
mod cluster_claim_entities;
mod cluster_provider_graph;
mod cluster_provider_peers;
mod deliver_mlops_alert_receiver_webhook;
mod evaluate_model_artifact;
mod health;
mod mine_rule_candidates;
mod profile_parquet;
mod promote_approved_model_version;
mod rank_automl_candidates;
mod run_mlops_monitoring_cycle;
mod run_mlops_monitoring_plan;
mod run_retraining_job;
mod run_rule_candidate_backtest;
mod run_scheduled_mlops_monitoring;
mod serve_mlops_alert_router;
mod submit_anomaly_clustering_report;
mod submit_clinical_compatibility_reference;
mod submit_episode_aggregation;
mod submit_mlops_alert_delivery_tasks;
mod submit_mlops_monitoring_report;
mod submit_peer_benchmark;
mod submit_provider_graph_signal_rollup;
mod submit_provider_profile_window_rollup;
mod submit_sanctions_sync_report;
mod submit_scoring_feature_contexts;
mod sync_oig_sam_sanctions;
mod verify_demo_automl_lifecycle;

pub async fn dispatch(mut args: Vec<String>) -> anyhow::Result<()> {
    if args.is_empty() {
        tracing::info!("worker skeleton ready");
        return Ok(());
    }

    match args.remove(0).as_str() {
        "health" => health::run(args),
        "check-pilot-readiness" => check_pilot_readiness::run(args).await,
        "profile-parquet" => profile_parquet::run(args),
        "build-feature-set" => build_feature_set::run(args),
        "build-demo-ml-datasets" => build_demo_ml_datasets::run(args),
        "build-demo-automl-lifecycle-evidence" => build_demo_automl_lifecycle_evidence::run(args),
        "verify-demo-automl-lifecycle" => verify_demo_automl_lifecycle::run(args),
        "build-training-handoff" => build_training_handoff::run(args),
        "build-audit-retention-scan" => build_audit_retention_scan::run(args),
        "build-anomaly-upgrade-readiness" => build_anomaly_upgrade_readiness::run(args),
        "build-clinical-compatibility-reference" => {
            build_clinical_compatibility_reference::run(args)
        }
        "submit-clinical-compatibility-reference" => {
            submit_clinical_compatibility_reference::run(args).await
        }
        "build-mlops-monitoring-plan" => build_mlops_monitoring_plan::run(args),
        "run-scheduled-mlops-monitoring" => run_scheduled_mlops_monitoring::run(args),
        "build-mlops-monitoring-report" => build_mlops_monitoring_report::run(args),
        "run-mlops-monitoring-plan" => run_mlops_monitoring_plan::run(args),
        "submit-mlops-monitoring-report" => submit_mlops_monitoring_report::run(args).await,
        "build-mlops-scheduler-execution-report" => {
            build_mlops_scheduler_execution_report::run(args)
        }
        "build-model-promotion-orchestration-report" => {
            build_model_promotion_orchestration_report::run(args)
        }
        "build-episode-aggregation" => build_episode_aggregation::run(args),
        "submit-episode-aggregation" => submit_episode_aggregation::run(args).await,
        "build-peer-benchmarks" => build_peer_benchmarks::run(args),
        "submit-peer-benchmark" => submit_peer_benchmark::run(args).await,
        "build-probability-calibration-report" => build_probability_calibration_report::run(args),
        "build-provider-graph-signals" => build_provider_graph_signals::run(args),
        "build-provider-profile-windows" => build_provider_profile_windows::run(args),
        "submit-provider-graph-signal-rollup" => {
            submit_provider_graph_signal_rollup::run(args).await
        }
        "build-scoring-feature-contexts" => build_scoring_feature_contexts::run(args),
        "submit-provider-profile-window-rollup" => {
            submit_provider_profile_window_rollup::run(args).await
        }
        "submit-scoring-feature-contexts" => submit_scoring_feature_contexts::run(args).await,
        "build-unbundling-comparator" => build_unbundling_comparator::run(args),
        "submit-mlops-alert-delivery-tasks" => submit_mlops_alert_delivery_tasks::run(args).await,
        "submit-anomaly-clustering-report" => submit_anomaly_clustering_report::run(args).await,
        "submit-sanctions-sync-report" => submit_sanctions_sync_report::run(args).await,
        "sync-oig-sam-sanctions" => sync_oig_sam_sanctions::run(args),
        "deliver-mlops-alert-receiver-webhook" => {
            deliver_mlops_alert_receiver_webhook::run(args).await
        }
        "serve-mlops-alert-router" => serve_mlops_alert_router::run(args).await,
        "run-mlops-monitoring-cycle" => run_mlops_monitoring_cycle::run(args).await,
        "build-automl-lifecycle-closure-report" => build_automl_lifecycle_closure_report::run(args),
        "rank-automl-candidates" => rank_automl_candidates::run(args),
        "evaluate-model-artifact" => evaluate_model_artifact::run(args).await,
        "mine-rule-candidates" => mine_rule_candidates::run(args),
        "run-rule-candidate-backtest" => run_rule_candidate_backtest::run(args),
        "cluster-provider-peers" => cluster_provider_peers::run(args),
        "cluster-claim-entities" => cluster_claim_entities::run(args),
        "cluster-provider-graph" => cluster_provider_graph::run(args),
        "build-analytics-export-plan" => build_analytics_export_plan::run(args),
        "build-ai-evidence-execution-plan" => build_ai_evidence_execution_plan::run(args),
        "build-governance-ops-plan" => build_governance_ops_plan::run(args),
        "claim-retraining-job" => claim_retraining_job::run(args).await,
        "run-retraining-job" => run_retraining_job::run(args).await,
        "promote-approved-model-version" => promote_approved_model_version::run(args).await,
        command => anyhow::bail!("unknown worker command: {command}"),
    }
}
