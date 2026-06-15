use super::flags::take_flag_value;

pub async fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let api_url = take_flag_value(&mut args, "--api-url")?;
    let api_key = take_flag_value(&mut args, "--api-key")?;
    let report = take_flag_value(&mut args, "--report")?;
    let published_report_uri = take_flag_value(&mut args, "--published-report-uri")?;
    let published_input_uri = take_flag_value(&mut args, "--published-input-uri")?;
    let published_label_uri = take_flag_value(&mut args, "--published-label-uri")?;
    let actor = take_flag_value(&mut args, "--actor")?;
    let notes = take_flag_value(&mut args, "--notes")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let response = crate::submit_probability_calibration_report_with_published_uris(
        &api_url,
        &api_key,
        &report,
        &published_report_uri,
        &published_input_uri,
        &published_label_uri,
        &actor,
        &notes,
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
