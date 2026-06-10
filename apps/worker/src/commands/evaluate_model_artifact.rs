use super::flags::{
    take_flag_value, take_optional_f64_flag, take_optional_flag_value, take_optional_u64_flag,
    take_optional_usize_flag,
};

pub async fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let serving_manifest = take_flag_value(&mut args, "--serving-manifest")?;
    let dataset_manifest = take_flag_value(&mut args, "--dataset-manifest")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    let split =
        take_optional_flag_value(&mut args, "--split")?.unwrap_or_else(|| "validation".into());
    let expected_probability_column =
        take_optional_flag_value(&mut args, "--expected-probability-column")?;
    let probability_tolerance =
        take_optional_f64_flag(&mut args, "--probability-tolerance")?.unwrap_or(0.0001);
    let latency_budget_ms =
        take_optional_u64_flag(&mut args, "--latency-budget-ms")?.unwrap_or(100);
    let max_rows = take_optional_usize_flag(&mut args, "--max-rows")?.unwrap_or(100);
    let signing_key = take_optional_flag_value(&mut args, "--signing-key")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::evaluate_model_artifact(
        &serving_manifest,
        &dataset_manifest,
        &split,
        output_dir,
        expected_probability_column.as_deref(),
        probability_tolerance,
        latency_budget_ms,
        max_rows,
        signing_key.as_deref(),
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
