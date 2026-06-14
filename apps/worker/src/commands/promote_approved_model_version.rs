use super::flags::take_flag_value;

pub async fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let api_url = take_flag_value(&mut args, "--api-url")?;
    let api_key = take_flag_value(&mut args, "--api-key")?;
    let model_key = take_flag_value(&mut args, "--model-key")?;
    let model_version = take_flag_value(&mut args, "--model-version")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let result =
        crate::promote_approved_model_version(&api_url, &api_key, &model_key, &model_version)
            .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
