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
        "build-training-handoff" => {
            let manifest = take_flag_value(&mut args, "--manifest")?;
            let artifact_base_uri = take_flag_value(&mut args, "--artifact-base-uri")?;
            let model_key = take_flag_value(&mut args, "--model-key")?;
            let base_model_version = take_flag_value(&mut args, "--base-model-version")?;
            let job_id = take_flag_value(&mut args, "--job-id")?;
            let actor = take_flag_value(&mut args, "--actor")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let handoff = worker::build_training_handoff(
                manifest,
                &artifact_base_uri,
                &model_key,
                &base_model_version,
                &job_id,
                &actor,
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
            let trainer_python = take_optional_flag_value(&mut args, "--trainer-python")?
                .unwrap_or_else(|| "python".into());
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
