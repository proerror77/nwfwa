use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let model_key = take_flag_value(&mut args, "--model-key")?;
    let model_version = take_flag_value(&mut args, "--model-version")?;
    let artifact_evaluation_report = take_flag_value(&mut args, "--artifact-evaluation-report")?;
    let shadow_report = take_flag_value(&mut args, "--shadow-report")?;
    let drift_report = take_flag_value(&mut args, "--drift-report")?;
    let fairness_report = take_flag_value(&mut args, "--fairness-report")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::build_mlops_monitoring_report(
        &model_key,
        &model_version,
        &artifact_evaluation_report,
        &shadow_report,
        &drift_report,
        &fairness_report,
        output_dir,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
