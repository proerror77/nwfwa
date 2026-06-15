use super::flags::{take_flag_value, take_optional_flag_value};

pub async fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let api_url = take_flag_value(&mut args, "--api-url")?;
    let api_key = take_flag_value(&mut args, "--api-key")?;
    let report = take_flag_value(&mut args, "--report")?;
    let published_report_uri = take_optional_flag_value(&mut args, "--published-report-uri")?
        .unwrap_or_else(|| report.clone());
    let materialization_id = take_flag_value(&mut args, "--materialization-id")?;
    let actor = take_flag_value(&mut args, "--actor")?;
    let notes = take_flag_value(&mut args, "--notes")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let response = crate::submit_scoring_feature_context_materialization_with_published_uri(
        &api_url,
        &api_key,
        &report,
        &published_report_uri,
        &materialization_id,
        &actor,
        &notes,
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
