use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let candidate_plan = take_flag_value(&mut args, "--candidate-plan")?;
    let dataset_manifest = take_flag_value(&mut args, "--dataset-manifest")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report =
        crate::run_rule_candidate_backtest(&candidate_plan, &dataset_manifest, output_dir)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
