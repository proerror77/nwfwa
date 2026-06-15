use super::flags::{take_flag_value, take_optional_flag_value};

pub async fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let api_url = take_flag_value(&mut args, "--api-url")?;
    let api_key = take_flag_value(&mut args, "--api-key")?;
    let report = take_flag_value(&mut args, "--report")?;
    let published_report_uri = take_optional_flag_value(&mut args, "--published-report-uri")?;
    let published_input_uri = take_optional_flag_value(&mut args, "--published-input-uri")?;
    let published_label_uri = take_optional_flag_value(&mut args, "--published-label-uri")?;
    let actor = take_flag_value(&mut args, "--actor")?;
    let notes = take_flag_value(&mut args, "--notes")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let published_override_count = [
        published_report_uri.as_ref(),
        published_input_uri.as_ref(),
        published_label_uri.as_ref(),
    ]
    .into_iter()
    .filter(Option::is_some)
    .count();
    if published_override_count != 0 && published_override_count != 3 {
        anyhow::bail!(
            "--published-report-uri, --published-input-uri, and --published-label-uri must be provided together"
        );
    }
    let response = if let (
        Some(published_report_uri),
        Some(published_input_uri),
        Some(published_label_uri),
    ) = (
        published_report_uri,
        published_input_uri,
        published_label_uri,
    ) {
        crate::submit_probability_calibration_report_with_published_uris(
            &api_url,
            &api_key,
            &report,
            &published_report_uri,
            &published_input_uri,
            &published_label_uri,
            &actor,
            &notes,
        )
        .await?
    } else {
        crate::submit_probability_calibration_report(&api_url, &api_key, &report, &actor, &notes)
            .await?
    };
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
