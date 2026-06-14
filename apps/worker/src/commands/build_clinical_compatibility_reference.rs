use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let reference_uri = take_flag_value(&mut args, "--reference-uri")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::build_clinical_compatibility_reference_report(&reference_uri, output_dir)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
