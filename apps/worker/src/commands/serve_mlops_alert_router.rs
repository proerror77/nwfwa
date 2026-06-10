use super::flags::take_optional_flag_value;

pub async fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let bind_addr = take_optional_flag_value(&mut args, "--bind-addr")?
        .or_else(|| std::env::var("FWA_MLOPS_ALERT_ROUTER_BIND_ADDR").ok())
        .unwrap_or_else(|| "0.0.0.0:8080".into());
    let api_url = take_optional_flag_value(&mut args, "--api-url")?
        .or_else(|| std::env::var("FWA_API_BASE_URL").ok())
        .ok_or_else(|| anyhow::anyhow!("missing --api-url or FWA_API_BASE_URL"))?;
    let api_key = take_optional_flag_value(&mut args, "--api-key")?
        .or_else(|| std::env::var("FWA_API_KEY").ok())
        .ok_or_else(|| anyhow::anyhow!("missing --api-key or FWA_API_KEY"))?;
    let alertmanager_webhook_token =
        take_optional_flag_value(&mut args, "--alertmanager-webhook-token")?
            .or_else(|| std::env::var("FWA_MLOPS_ALERT_ROUTER_TOKEN").ok())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "missing --alertmanager-webhook-token or FWA_MLOPS_ALERT_ROUTER_TOKEN"
                )
            })?;
    let model_key = take_optional_flag_value(&mut args, "--model-key")?
        .or_else(|| std::env::var("FWA_MLOPS_ALERT_MODEL_KEY").ok())
        .unwrap_or_else(|| "baseline_fwa".into());
    let model_version = take_optional_flag_value(&mut args, "--model-version")?
        .or_else(|| std::env::var("FWA_MLOPS_ALERT_MODEL_VERSION").ok())
        .ok_or_else(|| {
            anyhow::anyhow!("missing --model-version or FWA_MLOPS_ALERT_MODEL_VERSION")
        })?;
    let scheduler_report = take_optional_flag_value(&mut args, "--scheduler-report-uri")?
        .or_else(|| std::env::var("FWA_MLOPS_SCHEDULER_REPORT_URI").ok())
        .ok_or_else(|| {
            anyhow::anyhow!("missing --scheduler-report-uri or FWA_MLOPS_SCHEDULER_REPORT_URI")
        })?;
    let actor = take_optional_flag_value(&mut args, "--actor")?
        .or_else(|| std::env::var("FWA_MLOPS_ALERT_ROUTER_ACTOR").ok())
        .unwrap_or_else(|| "mlops-alert-router".into());
    let notes = take_optional_flag_value(&mut args, "--notes")?
        .or_else(|| std::env::var("FWA_MLOPS_ALERT_ROUTER_NOTES").ok())
        .unwrap_or_else(|| "Alertmanager webhook converted by MLOps alert-router adapter.".into());
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    crate::serve_mlops_alert_router(crate::MlopsAlertRouterConfig {
        bind_addr,
        api_base_url: api_url,
        api_key,
        alertmanager_webhook_token: Some(alertmanager_webhook_token),
        model_key,
        model_version,
        scheduler_execution_report_uri: scheduler_report,
        actor,
        notes,
    })
    .await?;
    Ok(())
}
