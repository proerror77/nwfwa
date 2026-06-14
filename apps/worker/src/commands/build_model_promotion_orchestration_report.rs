use super::flags::{take_flag_value, take_repeated_flag_value};

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let candidate_ranking = take_flag_value(&mut args, "--candidate-ranking")?;
    let artifact_evaluation_reports =
        take_repeated_flag_value(&mut args, "--artifact-evaluation-report")?;
    let mlops_monitoring_report = take_flag_value(&mut args, "--mlops-monitoring-report")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::build_model_promotion_orchestration_report(
        &candidate_ranking,
        &artifact_evaluation_reports,
        &mlops_monitoring_report,
        output_dir,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
