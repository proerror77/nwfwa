use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let claims_uri = take_flag_value(&mut args, "--claims-uri")?;
    let episode_rollups_uri = take_flag_value(&mut args, "--episode-rollups-uri")?;
    let peer_benchmarks_uri = take_flag_value(&mut args, "--peer-benchmarks-uri")?;
    let clinical_compatibility_uri = take_flag_value(&mut args, "--clinical-compatibility-uri")?;
    let unbundling_candidates_uri = take_flag_value(&mut args, "--unbundling-candidates-uri")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::build_scoring_feature_context_report(
        &claims_uri,
        &episode_rollups_uri,
        &peer_benchmarks_uri,
        &clinical_compatibility_uri,
        &unbundling_candidates_uri,
        output_dir,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
