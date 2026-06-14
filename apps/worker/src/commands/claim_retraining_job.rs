use super::flags::{take_flag_value, take_optional_flag_value};

pub async fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let api_url = take_flag_value(&mut args, "--api-url")?;
    let api_key = take_flag_value(&mut args, "--api-key")?;
    let actor = take_flag_value(&mut args, "--actor")?;
    let model_key = take_optional_flag_value(&mut args, "--model-key")?;
    let notes = take_optional_flag_value(&mut args, "--notes")?
        .unwrap_or_else(|| "Worker claimed retraining job.".into());
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let job =
        crate::claim_next_retraining_job(&api_url, &api_key, &actor, model_key.as_deref(), &notes)
            .await?;
    println!("{}", serde_json::to_string_pretty(&job)?);
    Ok(())
}
