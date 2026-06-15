use super::flags::{take_flag_value, take_optional_flag_value};

pub async fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let api_url = take_flag_value(&mut args, "--api-url")?;
    let api_key = take_flag_value(&mut args, "--api-key")?;
    let report = take_flag_value(&mut args, "--report")?;
    let published_report_uri = take_optional_flag_value(&mut args, "--published-report-uri")?
        .unwrap_or_else(|| report.clone());
    let published_source_uri = take_optional_flag_value(&mut args, "--published-source-uri")?;
    let actor = take_flag_value(&mut args, "--actor")?;
    let notes = take_flag_value(&mut args, "--notes")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    if published_source_uri.is_none() && published_report_uri != report {
        anyhow::bail!("--published-source-uri is required when --published-report-uri is set");
    }
    let response = if let Some(published_source_uri) = published_source_uri {
        crate::submit_clinical_compatibility_reference_with_published_uris(
            &api_url,
            &api_key,
            &report,
            &published_report_uri,
            &published_source_uri,
            &actor,
            &notes,
        )
        .await?
    } else {
        crate::submit_clinical_compatibility_reference(&api_url, &api_key, &report, &actor, &notes)
            .await?
    };
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
