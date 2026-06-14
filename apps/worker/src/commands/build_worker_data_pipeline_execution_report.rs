use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let plan = take_flag_value(&mut args, "--plan")?;
    let run_status = take_flag_value(&mut args, "--run-status")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report =
        crate::build_worker_data_pipeline_execution_report(&plan, &run_status, output_dir)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
