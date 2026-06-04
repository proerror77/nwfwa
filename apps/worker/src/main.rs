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

fn take_bool_flag(args: &mut Vec<String>, flag: &str) -> bool {
    let Some(index) = args.iter().position(|arg| arg == flag) else {
        return false;
    };
    args.remove(index);
    true
}
