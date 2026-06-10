use super::flags::take_flag_value;

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let object_storage_uri = take_flag_value(&mut args, "--object-storage-uri")?;
    let database_ref = take_flag_value(&mut args, "--database-ref")?;
    let customer_scope_id = take_flag_value(&mut args, "--customer-scope-id")?;
    let retention_policy_id = take_flag_value(&mut args, "--retention-policy-id")?;
    let backup_restore_plan_id = take_flag_value(&mut args, "--backup-restore-plan-id")?;
    let legal_hold_policy_id = take_flag_value(&mut args, "--legal-hold-policy-id")?;
    let cron = take_flag_value(&mut args, "--cron")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let plan = crate::build_governance_ops_plan(
        &object_storage_uri,
        &database_ref,
        &customer_scope_id,
        &retention_policy_id,
        &backup_restore_plan_id,
        &legal_hold_policy_id,
        &cron,
    )?;
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}
