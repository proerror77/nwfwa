use super::flags::{take_flag_value, take_optional_flag_value};

pub async fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let plan = take_flag_value(&mut args, "--plan")?;
    let artifact_evaluation_report = take_flag_value(&mut args, "--artifact-evaluation-report")?;
    let shadow_report = take_flag_value(&mut args, "--shadow-report")?;
    let drift_report = take_flag_value(&mut args, "--drift-report")?;
    let fairness_report = take_flag_value(&mut args, "--fairness-report")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    let api_url = take_optional_flag_value(&mut args, "--api-url")?;
    let api_key = take_optional_flag_value(&mut args, "--api-key")?;
    let actor = take_optional_flag_value(&mut args, "--actor")?;
    let notes = take_optional_flag_value(&mut args, "--notes")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::run_mlops_monitoring_cycle(
        &plan,
        &artifact_evaluation_report,
        &shadow_report,
        &drift_report,
        &fairness_report,
        output_dir,
        api_url.as_deref(),
        api_key.as_deref(),
        actor.as_deref(),
        notes.as_deref(),
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
