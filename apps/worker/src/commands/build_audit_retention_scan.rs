use super::flags::{take_flag_value, take_optional_u64_flag};

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let source_uri = take_flag_value(&mut args, "--source-uri")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    let as_of_date = take_flag_value(&mut args, "--as-of-date")?;
    let retention_years = take_optional_u64_flag(&mut args, "--retention-years")?
        .map(u16::try_from)
        .transpose()
        .map_err(|_| anyhow::anyhow!("invalid --retention-years: out of range for u16"))?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::build_audit_retention_scan_report(
        &source_uri,
        output_dir,
        &as_of_date,
        retention_years,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
