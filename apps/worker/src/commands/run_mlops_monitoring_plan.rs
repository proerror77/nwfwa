use super::flags::{take_flag_value, take_optional_flag_value};

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let plan = take_flag_value(&mut args, "--plan")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    let monitoring_inputs = take_optional_flag_value(&mut args, "--monitoring-inputs")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let index = crate::run_mlops_monitoring_plan_with_inputs(
        &plan,
        output_dir,
        monitoring_inputs.as_deref(),
    )?;
    println!("{}", serde_json::to_string_pretty(&index)?);
    Ok(())
}
