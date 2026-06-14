use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let api_url = take_flag_value(&mut args, "--api-url")?;
    let object_storage_uri = take_flag_value(&mut args, "--object-storage-uri")?;
    let vector_store_kind = take_flag_value(&mut args, "--vector-store-kind")?;
    let vector_store_ref = take_flag_value(&mut args, "--vector-store-ref")?;
    let customer_scope_id = take_flag_value(&mut args, "--customer-scope-id")?;
    let cron = take_flag_value(&mut args, "--cron")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let plan = crate::build_ai_evidence_execution_plan(
        &api_url,
        &object_storage_uri,
        &vector_store_kind,
        &vector_store_ref,
        &customer_scope_id,
        &cron,
    )?;
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}
