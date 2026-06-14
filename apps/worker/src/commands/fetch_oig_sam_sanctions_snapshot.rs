use super::flags::{take_flag_value, take_optional_flag_value};

pub async fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let oig_url = take_optional_flag_value(&mut args, "--oig-url")?;
    let sam_url = take_optional_flag_value(&mut args, "--sam-url")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    let source_date = take_optional_flag_value(&mut args, "--source-date")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let snapshot = crate::fetch_oig_sam_sanctions_snapshot(
        oig_url.as_deref(),
        sam_url.as_deref(),
        output_dir,
        source_date.as_deref(),
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&snapshot)?);
    Ok(())
}
