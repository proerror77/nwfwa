use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let plan = take_flag_value(&mut args, "--plan")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let template = crate::build_worker_data_pipeline_readiness_input_template(&plan, output_dir)?;
    println!("{}", serde_json::to_string_pretty(&template)?);
    Ok(())
}
