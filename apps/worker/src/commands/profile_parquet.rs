use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let manifest = take_flag_value(&mut args, "--manifest")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let result = crate::profile_manifest_file(manifest, output_dir)?;
    tracing::info!(
        dataset_key = %result.schema.dataset_key,
        dataset_version = %result.schema.dataset_version,
        field_count = result.schema.fields.len(),
        "parquet profile written"
    );
    Ok(())
}
