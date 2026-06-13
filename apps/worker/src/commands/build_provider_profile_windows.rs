use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let claims_uri = take_flag_value(&mut args, "--claims-uri")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::build_provider_profile_window_rollup(&claims_uri, output_dir)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
