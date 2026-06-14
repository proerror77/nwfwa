use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let object_storage_uri = take_flag_value(&mut args, "--object-storage-uri")?;
    let clickhouse_url = take_flag_value(&mut args, "--clickhouse-url")?;
    let customer_scope_id = take_flag_value(&mut args, "--customer-scope-id")?;
    let cron = take_flag_value(&mut args, "--cron")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let plan = crate::build_analytics_export_plan(
        &object_storage_uri,
        &clickhouse_url,
        &customer_scope_id,
        &cron,
    )?;
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}
