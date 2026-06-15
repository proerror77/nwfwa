use super::flags::{take_flag_value, take_optional_flag_value};

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let input_uri = take_flag_value(&mut args, "--input-uri")?;
    let score_response_uri = take_optional_flag_value(&mut args, "--score-response-uri")?;
    let published_report_uri = take_optional_flag_value(&mut args, "--published-report-uri")?;
    let published_input_uri = take_optional_flag_value(&mut args, "--published-input-uri")?;
    let published_score_request_uri =
        take_optional_flag_value(&mut args, "--published-score-request-uri")?;
    let published_score_response_uri =
        take_optional_flag_value(&mut args, "--published-score-response-uri")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::build_scoring_readback_report_with_published_uris(
        &input_uri,
        score_response_uri.as_deref(),
        published_report_uri.as_deref(),
        published_input_uri.as_deref(),
        published_score_request_uri.as_deref(),
        published_score_response_uri.as_deref(),
        output_dir,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
