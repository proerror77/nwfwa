#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        tracing::info!("worker skeleton ready");
        return Ok(());
    }

    match args.remove(0).as_str() {
        "health" => {
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&worker::worker_health())?
            );
        }
        "check-pilot-readiness" => {
            let api_url = take_flag_value(&mut args, "--api-url")?;
            let api_key = take_optional_flag_value(&mut args, "--api-key")?;
            let require_ready = take_bool_flag(&mut args, "--require-ready");
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let report = worker::check_pilot_readiness(&api_url, api_key.as_deref()).await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            if require_ready && !report.ready_for_customer_pilot {
                anyhow::bail!(
                    "customer pilot readiness blocked: {}",
                    report.remediation_summary.join(", ")
                );
            }
        }
        "profile-parquet" => {
            let manifest = take_flag_value(&mut args, "--manifest")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let result = worker::profile_manifest_file(manifest, output_dir)?;
            tracing::info!(
                dataset_key = %result.schema.dataset_key,
                dataset_version = %result.schema.dataset_version,
                field_count = result.schema.fields.len(),
                "parquet profile written"
            );
        }
        "build-feature-set" => {
            let manifest = take_flag_value(&mut args, "--manifest")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            let feature_set_id = take_optional_flag_value(&mut args, "--feature-set-id")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let result =
                worker::build_feature_set(manifest, output_dir, feature_set_id.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        "build-demo-ml-datasets" => {
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            let dataset_version = take_optional_flag_value(&mut args, "--dataset-version")?
                .unwrap_or_else(|| "2026-06-rust-automl-demo".into());
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let pack = worker::build_demo_ml_datasets(output_dir, &dataset_version)?;
            println!("{}", serde_json::to_string_pretty(&pack)?);
        }
        "build-demo-automl-lifecycle-evidence" => {
            let demo_root = take_flag_value(&mut args, "--demo-root")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let index = worker::build_demo_automl_lifecycle_evidence(demo_root, output_dir)?;
            println!("{}", serde_json::to_string_pretty(&index)?);
        }
        "verify-demo-automl-lifecycle" => {
            let demo_root = take_flag_value(&mut args, "--demo-root")?;
            let evidence_dir = take_flag_value(&mut args, "--evidence-dir")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let report =
                worker::verify_demo_automl_lifecycle(&demo_root, &evidence_dir, &output_dir)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            if report["verification_status"] != "passed" {
                anyhow::bail!("Rust Auto MLOps demo lifecycle verification blocked");
            }
        }
        "build-training-handoff" => {
            let manifest = take_flag_value(&mut args, "--manifest")?;
            let artifact_base_uri = take_flag_value(&mut args, "--artifact-base-uri")?;
            let model_key = take_flag_value(&mut args, "--model-key")?;
            let base_model_version = take_flag_value(&mut args, "--base-model-version")?;
            let job_id = take_flag_value(&mut args, "--job-id")?;
            let actor = take_flag_value(&mut args, "--actor")?;
            let algorithm = take_optional_flag_value(&mut args, "--algorithm")?
                .unwrap_or_else(|| "logistic_regression".into());
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let handoff = worker::build_training_handoff_with_algorithm(
                manifest,
                &artifact_base_uri,
                &model_key,
                &base_model_version,
                &job_id,
                &actor,
                &algorithm,
            )?;
            println!("{}", serde_json::to_string_pretty(&handoff)?);
        }
        "build-mlops-monitoring-plan" => {
            let manifest_uri = take_flag_value(&mut args, "--manifest-uri")?;
            let artifact_uri = take_flag_value(&mut args, "--artifact-uri")?;
            let model_key = take_flag_value(&mut args, "--model-key")?;
            let model_version = take_flag_value(&mut args, "--model-version")?;
            let cron = take_flag_value(&mut args, "--cron")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let plan = worker::build_mlops_monitoring_plan(
                &manifest_uri,
                &artifact_uri,
                &model_key,
                &model_version,
                &cron,
            )?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        "run-scheduled-mlops-monitoring" => {
            let manifest_uri = take_flag_value(&mut args, "--manifest-uri")?;
            let artifact_uri = take_flag_value(&mut args, "--artifact-uri")?;
            let model_key = take_flag_value(&mut args, "--model-key")?;
            let model_version = take_flag_value(&mut args, "--model-version")?;
            let cron = take_flag_value(&mut args, "--cron")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            let artifact_base_uri = take_optional_flag_value(&mut args, "--artifact-base-uri")?;
            let monitoring_inputs = take_optional_flag_value(&mut args, "--monitoring-inputs")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let index = worker::run_scheduled_mlops_monitoring_with_options(
                &manifest_uri,
                &artifact_uri,
                &model_key,
                &model_version,
                &cron,
                output_dir,
                artifact_base_uri.as_deref(),
                monitoring_inputs.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&index)?);
        }
        "build-mlops-monitoring-report" => {
            let model_key = take_flag_value(&mut args, "--model-key")?;
            let model_version = take_flag_value(&mut args, "--model-version")?;
            let artifact_evaluation_report =
                take_flag_value(&mut args, "--artifact-evaluation-report")?;
            let shadow_report = take_flag_value(&mut args, "--shadow-report")?;
            let drift_report = take_flag_value(&mut args, "--drift-report")?;
            let fairness_report = take_flag_value(&mut args, "--fairness-report")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let report = worker::build_mlops_monitoring_report(
                &model_key,
                &model_version,
                &artifact_evaluation_report,
                &shadow_report,
                &drift_report,
                &fairness_report,
                output_dir,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "run-mlops-monitoring-plan" => {
            let plan = take_flag_value(&mut args, "--plan")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            let monitoring_inputs = take_optional_flag_value(&mut args, "--monitoring-inputs")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let index = worker::run_mlops_monitoring_plan_with_inputs(
                &plan,
                output_dir,
                monitoring_inputs.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&index)?);
        }
        "submit-mlops-monitoring-report" => {
            let api_url = take_flag_value(&mut args, "--api-url")?;
            let api_key = take_flag_value(&mut args, "--api-key")?;
            let report = take_flag_value(&mut args, "--report")?;
            let actor = take_flag_value(&mut args, "--actor")?;
            let notes = take_flag_value(&mut args, "--notes")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let response =
                worker::submit_mlops_monitoring_report(&api_url, &api_key, &report, &actor, &notes)
                    .await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        "build-mlops-scheduler-execution-report" => {
            let plan = take_flag_value(&mut args, "--plan")?;
            let monitoring_report = take_flag_value(&mut args, "--monitoring-report")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let report = worker::build_mlops_scheduler_execution_report(
                &plan,
                &monitoring_report,
                output_dir,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "submit-mlops-alert-delivery-tasks" => {
            let api_url = take_flag_value(&mut args, "--api-url")?;
            let api_key = take_flag_value(&mut args, "--api-key")?;
            let scheduler_report = take_flag_value(&mut args, "--scheduler-report")?;
            let actor = take_flag_value(&mut args, "--actor")?;
            let notes = take_flag_value(&mut args, "--notes")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let response = worker::submit_mlops_alert_delivery_tasks(
                &api_url,
                &api_key,
                &scheduler_report,
                &actor,
                &notes,
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        "submit-anomaly-clustering-report" => {
            let api_url = take_flag_value(&mut args, "--api-url")?;
            let api_key = take_flag_value(&mut args, "--api-key")?;
            let report = take_flag_value(&mut args, "--report")?;
            let actor = take_flag_value(&mut args, "--actor")?;
            let notes = take_flag_value(&mut args, "--notes")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let response = worker::submit_anomaly_clustering_report(
                &api_url, &api_key, &report, &actor, &notes,
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        "deliver-mlops-alert-receiver-webhook" => {
            let scheduler_report = take_flag_value(&mut args, "--scheduler-report")?;
            let receiver_url = take_flag_value(&mut args, "--receiver-url")?;
            let receiver_id = take_flag_value(&mut args, "--receiver-id")?;
            let receiver_token = take_optional_flag_value(&mut args, "--receiver-token")?;
            let receiver_secret = take_optional_flag_value(&mut args, "--receiver-secret")?;
            let max_attempts = take_optional_u64_flag(&mut args, "--max-attempts")?
                .unwrap_or(1)
                .try_into()
                .unwrap_or(u32::MAX);
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let report = worker::deliver_mlops_alert_receiver_webhook(
                &scheduler_report,
                &receiver_url,
                &receiver_id,
                receiver_token.as_deref(),
                receiver_secret.as_deref(),
                max_attempts,
                output_dir,
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "run-mlops-monitoring-cycle" => {
            let plan = take_flag_value(&mut args, "--plan")?;
            let artifact_evaluation_report =
                take_flag_value(&mut args, "--artifact-evaluation-report")?;
            let shadow_report = take_flag_value(&mut args, "--shadow-report")?;
            let drift_report = take_flag_value(&mut args, "--drift-report")?;
            let fairness_report = take_flag_value(&mut args, "--fairness-report")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            let api_url = take_optional_flag_value(&mut args, "--api-url")?;
            let api_key = take_optional_flag_value(&mut args, "--api-key")?;
            let actor = take_optional_flag_value(&mut args, "--actor")?;
            let notes = take_optional_flag_value(&mut args, "--notes")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let report = worker::run_mlops_monitoring_cycle(
                &plan,
                &artifact_evaluation_report,
                &shadow_report,
                &drift_report,
                &fairness_report,
                output_dir,
                api_url.as_deref(),
                api_key.as_deref(),
                actor.as_deref(),
                notes.as_deref(),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "build-automl-lifecycle-closure-report" => {
            let demo_index = take_flag_value(&mut args, "--demo-index")?;
            let candidate_ranking = take_flag_value(&mut args, "--candidate-ranking")?;
            let artifact_evaluation_reports =
                take_repeated_flag_value(&mut args, "--artifact-evaluation-report")?;
            let rule_backtest_report = take_flag_value(&mut args, "--rule-backtest-report")?;
            let provider_clustering_report =
                take_flag_value(&mut args, "--provider-clustering-report")?;
            let provider_graph_report = take_flag_value(&mut args, "--provider-graph-report")?;
            let claim_entity_clustering_report =
                take_flag_value(&mut args, "--claim-entity-clustering-report")?;
            let mlops_monitoring_report = take_flag_value(&mut args, "--mlops-monitoring-report")?;
            let mlops_scheduler_execution_report =
                take_flag_value(&mut args, "--mlops-scheduler-execution-report")?;
            let mlops_monitoring_cycle_report =
                take_flag_value(&mut args, "--mlops-monitoring-cycle-report")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let report = worker::build_automl_lifecycle_closure_report(
                &demo_index,
                &candidate_ranking,
                &artifact_evaluation_reports,
                &rule_backtest_report,
                &provider_clustering_report,
                &provider_graph_report,
                &claim_entity_clustering_report,
                &mlops_monitoring_report,
                &mlops_scheduler_execution_report,
                &mlops_monitoring_cycle_report,
                output_dir,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "rank-automl-candidates" => {
            let reports = take_repeated_flag_value(&mut args, "--validation-report")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let ranking = worker::rank_automl_candidates(&reports, output_dir)?;
            println!("{}", serde_json::to_string_pretty(&ranking)?);
        }
        "evaluate-model-artifact" => {
            let serving_manifest = take_flag_value(&mut args, "--serving-manifest")?;
            let dataset_manifest = take_flag_value(&mut args, "--dataset-manifest")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            let split = take_optional_flag_value(&mut args, "--split")?
                .unwrap_or_else(|| "validation".into());
            let expected_probability_column =
                take_optional_flag_value(&mut args, "--expected-probability-column")?;
            let probability_tolerance =
                take_optional_f64_flag(&mut args, "--probability-tolerance")?.unwrap_or(0.0001);
            let latency_budget_ms =
                take_optional_u64_flag(&mut args, "--latency-budget-ms")?.unwrap_or(100);
            let max_rows = take_optional_usize_flag(&mut args, "--max-rows")?.unwrap_or(100);
            let signing_key = take_optional_flag_value(&mut args, "--signing-key")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let report = worker::evaluate_model_artifact(
                &serving_manifest,
                &dataset_manifest,
                &split,
                output_dir,
                expected_probability_column.as_deref(),
                probability_tolerance,
                latency_budget_ms,
                max_rows,
                signing_key.as_deref(),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "mine-rule-candidates" => {
            let validation_report = take_flag_value(&mut args, "--validation-report")?;
            let feature_importance = take_flag_value(&mut args, "--feature-importance")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let candidates =
                worker::mine_rule_candidates(&validation_report, &feature_importance, output_dir)?;
            println!("{}", serde_json::to_string_pretty(&candidates)?);
        }
        "run-rule-candidate-backtest" => {
            let candidate_plan = take_flag_value(&mut args, "--candidate-plan")?;
            let dataset_manifest = take_flag_value(&mut args, "--dataset-manifest")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let report = worker::run_rule_candidate_backtest(
                &candidate_plan,
                &dataset_manifest,
                output_dir,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "cluster-provider-peers" => {
            let manifest = take_flag_value(&mut args, "--manifest")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let report = worker::cluster_provider_peers(&manifest, output_dir)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "cluster-claim-entities" => {
            let manifest = take_flag_value(&mut args, "--manifest")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let report = worker::cluster_claim_entities(&manifest, output_dir)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "cluster-provider-graph" => {
            let manifest = take_flag_value(&mut args, "--manifest")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let report = worker::cluster_provider_graph_communities(&manifest, output_dir)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "build-analytics-export-plan" => {
            let object_storage_uri = take_flag_value(&mut args, "--object-storage-uri")?;
            let clickhouse_url = take_flag_value(&mut args, "--clickhouse-url")?;
            let customer_scope_id = take_flag_value(&mut args, "--customer-scope-id")?;
            let cron = take_flag_value(&mut args, "--cron")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let plan = worker::build_analytics_export_plan(
                &object_storage_uri,
                &clickhouse_url,
                &customer_scope_id,
                &cron,
            )?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        "build-ai-evidence-execution-plan" => {
            let api_url = take_flag_value(&mut args, "--api-url")?;
            let object_storage_uri = take_flag_value(&mut args, "--object-storage-uri")?;
            let vector_store_kind = take_flag_value(&mut args, "--vector-store-kind")?;
            let vector_store_ref = take_flag_value(&mut args, "--vector-store-ref")?;
            let customer_scope_id = take_flag_value(&mut args, "--customer-scope-id")?;
            let cron = take_flag_value(&mut args, "--cron")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let plan = worker::build_ai_evidence_execution_plan(
                &api_url,
                &object_storage_uri,
                &vector_store_kind,
                &vector_store_ref,
                &customer_scope_id,
                &cron,
            )?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        "build-governance-ops-plan" => {
            let object_storage_uri = take_flag_value(&mut args, "--object-storage-uri")?;
            let database_ref = take_flag_value(&mut args, "--database-ref")?;
            let customer_scope_id = take_flag_value(&mut args, "--customer-scope-id")?;
            let retention_policy_id = take_flag_value(&mut args, "--retention-policy-id")?;
            let backup_restore_plan_id = take_flag_value(&mut args, "--backup-restore-plan-id")?;
            let legal_hold_policy_id = take_flag_value(&mut args, "--legal-hold-policy-id")?;
            let cron = take_flag_value(&mut args, "--cron")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let plan = worker::build_governance_ops_plan(
                &object_storage_uri,
                &database_ref,
                &customer_scope_id,
                &retention_policy_id,
                &backup_restore_plan_id,
                &legal_hold_policy_id,
                &cron,
            )?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        "claim-retraining-job" => {
            let api_url = take_flag_value(&mut args, "--api-url")?;
            let api_key = take_flag_value(&mut args, "--api-key")?;
            let actor = take_flag_value(&mut args, "--actor")?;
            let model_key = take_optional_flag_value(&mut args, "--model-key")?;
            let notes = take_optional_flag_value(&mut args, "--notes")?
                .unwrap_or_else(|| "Worker claimed retraining job.".into());
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let job = worker::claim_next_retraining_job(
                &api_url,
                &api_key,
                &actor,
                model_key.as_deref(),
                &notes,
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&job)?);
        }
        "run-retraining-job" => {
            let api_url = take_flag_value(&mut args, "--api-url")?;
            let api_key = take_flag_value(&mut args, "--api-key")?;
            let actor = take_flag_value(&mut args, "--actor")?;
            let artifact_base_uri = take_flag_value(&mut args, "--artifact-base-uri")?;
            let model_key = take_optional_flag_value(&mut args, "--model-key")?;
            let training_manifest = take_optional_flag_value(&mut args, "--training-manifest")?;
            let algorithm = take_optional_flag_value(&mut args, "--algorithm")?;
            let trainer_python = take_optional_flag_value(&mut args, "--trainer-python")?
                .unwrap_or_else(|| "python".into());
            let trainer_workdir = take_optional_flag_value(&mut args, "--trainer-workdir")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let result = worker::run_one_retraining_job(
                &api_url,
                &api_key,
                &actor,
                model_key.as_deref(),
                &artifact_base_uri,
                training_manifest.as_deref(),
                &trainer_python,
                trainer_workdir.as_deref(),
                algorithm.as_deref(),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        command => anyhow::bail!("unknown worker command: {command}"),
    }
    Ok(())
}

fn take_flag_value(args: &mut Vec<String>, flag: &str) -> anyhow::Result<String> {
    let Some(index) = args.iter().position(|arg| arg == flag) else {
        anyhow::bail!("missing required flag {flag}");
    };
    args.remove(index);
    if index >= args.len() {
        anyhow::bail!("missing value for flag {flag}");
    }
    Ok(args.remove(index))
}

fn take_optional_flag_value(args: &mut Vec<String>, flag: &str) -> anyhow::Result<Option<String>> {
    let Some(index) = args.iter().position(|arg| arg == flag) else {
        return Ok(None);
    };
    args.remove(index);
    if index >= args.len() {
        anyhow::bail!("missing value for flag {flag}");
    }
    Ok(Some(args.remove(index)))
}

fn take_optional_f64_flag(args: &mut Vec<String>, flag: &str) -> anyhow::Result<Option<f64>> {
    take_optional_flag_value(args, flag)?
        .map(|value| {
            value
                .parse::<f64>()
                .map_err(|error| anyhow::anyhow!("invalid {flag}: {error}"))
        })
        .transpose()
}

fn take_optional_u64_flag(args: &mut Vec<String>, flag: &str) -> anyhow::Result<Option<u64>> {
    take_optional_flag_value(args, flag)?
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| anyhow::anyhow!("invalid {flag}: {error}"))
        })
        .transpose()
}

fn take_optional_usize_flag(args: &mut Vec<String>, flag: &str) -> anyhow::Result<Option<usize>> {
    take_optional_flag_value(args, flag)?
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| anyhow::anyhow!("invalid {flag}: {error}"))
        })
        .transpose()
}

fn take_repeated_flag_value(args: &mut Vec<String>, flag: &str) -> anyhow::Result<Vec<String>> {
    let mut values = Vec::new();
    while let Some(index) = args.iter().position(|arg| arg == flag) {
        args.remove(index);
        if index >= args.len() {
            anyhow::bail!("missing value for flag {flag}");
        }
        values.push(args.remove(index));
    }
    if values.is_empty() {
        anyhow::bail!("missing required flag {flag}");
    }
    Ok(values)
}

fn take_bool_flag(args: &mut Vec<String>, flag: &str) -> bool {
    let Some(index) = args.iter().position(|arg| arg == flag) else {
        return false;
    };
    args.remove(index);
    true
}
