#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        tracing::info!("worker skeleton ready");
        return Ok(());
    }

    match args.remove(0).as_str() {
        "profile-parquet" => {
            let manifest = take_flag_value(&mut args, "--manifest")?;
            let output_dir = take_flag_value(&mut args, "--output-dir")?;
            if !args.is_empty() {
                anyhow::bail!("unexpected arguments: {}", args.join(" "));
            }
            let result = worker::profile_manifest_file(manifest, output_dir)?;
            tracing::info!(
                dataset_key = %result.schema.dataset_key,
                dataset_version = %result.schema.dataset_version,
                field_count = result.schema.fields.len(),
                "parquet profile written"
            );
        }
        command => anyhow::bail!("unknown worker command: {command}"),
    }
    Ok(())
}

fn take_flag_value(args: &mut Vec<String>, flag: &str) -> anyhow::Result<String> {
    let Some(index) = args.iter().position(|arg| arg == flag) else {
        anyhow::bail!("missing required flag {flag}");
    };
    args.remove(index);
    if index >= args.len() {
        anyhow::bail!("missing value for flag {flag}");
    }
    Ok(args.remove(index))
}
