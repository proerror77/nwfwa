use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let demo_root = take_flag_value(&mut args, "--demo-root")?;
    let evidence_dir = take_flag_value(&mut args, "--evidence-dir")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::verify_demo_automl_lifecycle(&demo_root, &evidence_dir, &output_dir)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if report["verification_status"] != "passed" {
        anyhow::bail!("Rust Auto MLOps demo lifecycle verification blocked");
    }
    Ok(())
}
