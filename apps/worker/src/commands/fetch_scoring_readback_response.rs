use super::flags::take_flag_value;

pub async fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let api_url = take_flag_value(&mut args, "--api-url")?;
    let api_key = take_flag_value(&mut args, "--api-key")?;
    let score_request_uri = take_flag_value(&mut args, "--score-request-uri")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let response =
        crate::fetch_scoring_readback_response(&api_url, &api_key, &score_request_uri, output_dir)
            .await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
