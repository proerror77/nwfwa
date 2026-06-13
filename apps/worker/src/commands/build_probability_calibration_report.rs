use super::flags::{take_flag_value, take_optional_usize_flag};

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let source_uri = take_flag_value(&mut args, "--source-uri")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    let bin_count = take_optional_usize_flag(&mut args, "--bins")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::build_probability_calibration_report(&source_uri, output_dir, bin_count)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
