use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let api_base_url = take_flag_value(&mut args, "--api-base-url")?;
    let object_storage_uri = take_flag_value(&mut args, "--object-storage-uri")?;
    let customer_scope_id = take_flag_value(&mut args, "--customer-scope-id")?;
    let daily_cron = take_flag_value(&mut args, "--daily-cron")?;
    let monthly_cron = take_flag_value(&mut args, "--monthly-cron")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let plan = crate::build_worker_data_pipeline_plan(
        &api_base_url,
        &object_storage_uri,
        &customer_scope_id,
        &daily_cron,
        &monthly_cron,
    )?;
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}
