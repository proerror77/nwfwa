use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let source_uri = take_flag_value(&mut args, "--source-uri")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::build_anomaly_upgrade_readiness_report(&source_uri, output_dir)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
