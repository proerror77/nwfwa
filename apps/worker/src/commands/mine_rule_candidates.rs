use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let validation_report = take_flag_value(&mut args, "--validation-report")?;
    let feature_importance = take_flag_value(&mut args, "--feature-importance")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let candidates =
        crate::mine_rule_candidates(&validation_report, &feature_importance, output_dir)?;
    println!("{}", serde_json::to_string_pretty(&candidates)?);
    Ok(())
}
