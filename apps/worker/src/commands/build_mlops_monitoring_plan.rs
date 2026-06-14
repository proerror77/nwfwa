use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let manifest_uri = take_flag_value(&mut args, "--manifest-uri")?;
    let artifact_uri = take_flag_value(&mut args, "--artifact-uri")?;
    let model_key = take_flag_value(&mut args, "--model-key")?;
    let model_version = take_flag_value(&mut args, "--model-version")?;
    let cron = take_flag_value(&mut args, "--cron")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let plan = crate::build_mlops_monitoring_plan(
        &manifest_uri,
        &artifact_uri,
        &model_key,
        &model_version,
        &cron,
    )?;
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}
