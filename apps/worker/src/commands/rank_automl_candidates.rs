use super::flags::{take_flag_value, take_repeated_flag_value};

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let reports = take_repeated_flag_value(&mut args, "--validation-report")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let ranking = crate::rank_automl_candidates(&reports, output_dir)?;
    println!("{}", serde_json::to_string_pretty(&ranking)?);
    Ok(())
}
