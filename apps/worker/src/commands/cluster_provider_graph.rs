use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let manifest = take_flag_value(&mut args, "--manifest")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::cluster_provider_graph_communities(&manifest, output_dir)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
