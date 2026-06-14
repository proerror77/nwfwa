use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let plan = take_flag_value(&mut args, "--plan")?;
    let readiness_report = take_flag_value(&mut args, "--readiness-report")?;
    let run_id = take_flag_value(&mut args, "--run-id")?;
    let execution_date = take_flag_value(&mut args, "--execution-date")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::build_worker_data_pipeline_run_status_template(
        &plan,
        &readiness_report,
        &run_id,
        &execution_date,
        output_dir,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
