use super::flags::{take_flag_value, take_optional_flag_value};

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let plan = take_flag_value(&mut args, "--plan")?;
    let run_status = take_flag_value(&mut args, "--run-status")?;
    let published_plan_uri = take_optional_flag_value(&mut args, "--published-plan-uri")?;
    let published_run_status_uri =
        take_optional_flag_value(&mut args, "--published-run-status-uri")?;
    let published_readiness_report_uri =
        take_optional_flag_value(&mut args, "--published-readiness-report-uri")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::build_worker_data_pipeline_execution_report_with_published_uris(
        &plan,
        &run_status,
        output_dir,
        published_plan_uri.as_deref(),
        published_run_status_uri.as_deref(),
        published_readiness_report_uri.as_deref(),
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
