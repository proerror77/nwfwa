use super::flags::{take_bool_flag, take_flag_value, take_optional_flag_value};

pub async fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let api_url = take_flag_value(&mut args, "--api-url")?;
    let api_key = take_optional_flag_value(&mut args, "--api-key")?;
    let require_ready = take_bool_flag(&mut args, "--require-ready");
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::check_pilot_readiness(&api_url, api_key.as_deref()).await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if require_ready && !report.ready_for_customer_pilot {
        anyhow::bail!(
            "customer pilot readiness blocked: {}",
            report.remediation_summary.join(", ")
        );
    }
    Ok(())
}
