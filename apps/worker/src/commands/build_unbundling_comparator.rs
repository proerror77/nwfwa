use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let input_uri = take_flag_value(&mut args, "--input-uri")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::build_unbundling_comparator_report(&input_uri, output_dir)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
