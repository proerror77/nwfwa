pub fn run(args: Vec<String>) -> anyhow::Result<()> {
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    println!("{}", serde_json::to_string_pretty(&crate::worker_health())?);
    Ok(())
}
