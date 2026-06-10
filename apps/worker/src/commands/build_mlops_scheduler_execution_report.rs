use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let plan = take_flag_value(&mut args, "--plan")?;
    let monitoring_report = take_flag_value(&mut args, "--monitoring-report")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report =
        crate::build_mlops_scheduler_execution_report(&plan, &monitoring_report, output_dir)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
