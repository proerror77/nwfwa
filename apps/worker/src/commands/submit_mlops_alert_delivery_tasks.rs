use super::flags::take_flag_value;

pub async fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let api_url = take_flag_value(&mut args, "--api-url")?;
    let api_key = take_flag_value(&mut args, "--api-key")?;
    let scheduler_report = take_flag_value(&mut args, "--scheduler-report")?;
    let published_scheduler_report =
        take_flag_value(&mut args, "--published-scheduler-report-uri")?;
    let actor = take_flag_value(&mut args, "--actor")?;
    let notes = take_flag_value(&mut args, "--notes")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let response = crate::submit_mlops_alert_delivery_tasks_with_published_uri(
        &api_url,
        &api_key,
        &scheduler_report,
        &published_scheduler_report,
        &actor,
        &notes,
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
