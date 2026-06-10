use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let demo_root = take_flag_value(&mut args, "--demo-root")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let index = crate::build_demo_automl_lifecycle_evidence(demo_root, output_dir)?;
    println!("{}", serde_json::to_string_pretty(&index)?);
    Ok(())
}
