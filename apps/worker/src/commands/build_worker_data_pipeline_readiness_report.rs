use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let plan = take_flag_value(&mut args, "--plan")?;
    let readiness_input = take_flag_value(&mut args, "--readiness-input")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report =
        crate::build_worker_data_pipeline_readiness_report(&plan, &readiness_input, output_dir)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
