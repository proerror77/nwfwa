use super::flags::{take_flag_value, take_optional_flag_value};

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let manifest_uri = take_flag_value(&mut args, "--manifest-uri")?;
    let artifact_uri = take_flag_value(&mut args, "--artifact-uri")?;
    let model_key = take_flag_value(&mut args, "--model-key")?;
    let model_version = take_flag_value(&mut args, "--model-version")?;
    let cron = take_flag_value(&mut args, "--cron")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    let artifact_base_uri = take_optional_flag_value(&mut args, "--artifact-base-uri")?;
    let monitoring_inputs = take_optional_flag_value(&mut args, "--monitoring-inputs")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let index = crate::run_scheduled_mlops_monitoring_with_options(
        &manifest_uri,
        &artifact_uri,
        &model_key,
        &model_version,
        &cron,
        output_dir,
        artifact_base_uri.as_deref(),
        monitoring_inputs.as_deref(),
    )?;
    println!("{}", serde_json::to_string_pretty(&index)?);
    Ok(())
}
