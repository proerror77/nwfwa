use super::flags::{take_flag_value, take_optional_flag_value};

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    let dataset_version = take_optional_flag_value(&mut args, "--dataset-version")?
        .unwrap_or_else(|| "2026-06-rust-automl-demo".into());
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let pack = crate::build_demo_ml_datasets(output_dir, &dataset_version)?;
    println!("{}", serde_json::to_string_pretty(&pack)?);
    Ok(())
}
