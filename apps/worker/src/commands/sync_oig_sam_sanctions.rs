use super::flags::{take_bool_flag, take_flag_value};

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let source_uri = take_flag_value(&mut args, "--source-uri")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    let run_date = take_flag_value(&mut args, "--run-date")?;
    let dry_run = take_bool_flag(&mut args, "--dry-run");
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::build_sanctions_sync_report(&source_uri, output_dir, &run_date, dry_run)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
