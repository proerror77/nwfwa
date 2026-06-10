use super::flags::{take_flag_value, take_optional_flag_value};

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let manifest = take_flag_value(&mut args, "--manifest")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    let feature_set_id = take_optional_flag_value(&mut args, "--feature-set-id")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let result = crate::build_feature_set(manifest, output_dir, feature_set_id.as_deref())?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
