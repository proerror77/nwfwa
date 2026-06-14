use super::flags::{take_flag_value, take_optional_flag_value, take_optional_u64_flag};

pub async fn run(mut args: Vec<String>) -> anyhow::Result<()> {
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
    let report = crate::deliver_mlops_alert_receiver_webhook(
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
    Ok(())
}
