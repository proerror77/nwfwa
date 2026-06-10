use super::flags::{take_flag_value, take_optional_flag_value};

pub async fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let api_url = take_flag_value(&mut args, "--api-url")?;
    let api_key = take_flag_value(&mut args, "--api-key")?;
    let actor = take_flag_value(&mut args, "--actor")?;
    let artifact_base_uri = take_flag_value(&mut args, "--artifact-base-uri")?;
    let model_key = take_optional_flag_value(&mut args, "--model-key")?;
    let training_manifest = take_optional_flag_value(&mut args, "--training-manifest")?;
    let algorithm = take_optional_flag_value(&mut args, "--algorithm")?;
    let trainer_python =
        take_optional_flag_value(&mut args, "--trainer-python")?.unwrap_or_else(|| "python".into());
    let trainer_workdir = take_optional_flag_value(&mut args, "--trainer-workdir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let result = crate::run_one_retraining_job(
        &api_url,
        &api_key,
        &actor,
        model_key.as_deref(),
        &artifact_base_uri,
        training_manifest.as_deref(),
        &trainer_python,
        trainer_workdir.as_deref(),
        algorithm.as_deref(),
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
