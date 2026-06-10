use super::flags::{take_flag_value, take_optional_flag_value};

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let manifest = take_flag_value(&mut args, "--manifest")?;
    let artifact_base_uri = take_flag_value(&mut args, "--artifact-base-uri")?;
    let model_key = take_flag_value(&mut args, "--model-key")?;
    let base_model_version = take_flag_value(&mut args, "--base-model-version")?;
    let job_id = take_flag_value(&mut args, "--job-id")?;
    let actor = take_flag_value(&mut args, "--actor")?;
    let algorithm = take_optional_flag_value(&mut args, "--algorithm")?
        .unwrap_or_else(|| "logistic_regression".into());
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let handoff = crate::build_training_handoff_with_algorithm(
        manifest,
        &artifact_base_uri,
        &model_key,
        &base_model_version,
        &job_id,
        &actor,
        &algorithm,
    )?;
    println!("{}", serde_json::to_string_pretty(&handoff)?);
    Ok(())
}
