use super::flags::{take_flag_value, take_optional_flag_value};

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let plan = take_flag_value(&mut args, "--plan")?;
    let readiness_input = take_flag_value(&mut args, "--readiness-input")?;
    let published_plan_uri = take_optional_flag_value(&mut args, "--published-plan-uri")?;
    let published_readiness_input_uri =
        take_optional_flag_value(&mut args, "--published-readiness-input-uri")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::build_worker_data_pipeline_readiness_report_with_published_uris(
        &plan,
        &readiness_input,
        output_dir,
        published_plan_uri.as_deref(),
        published_readiness_input_uri.as_deref(),
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
