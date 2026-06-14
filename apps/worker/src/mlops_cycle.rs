use anyhow::{bail, Context};
use std::{fs, path::Path};

use super::{
    build_mlops_monitoring_report, build_mlops_scheduler_execution_report, json_string, json_u64,
    nested_json_string, read_json_report, required_non_empty, required_optional,
    submit_mlops_alert_delivery_tasks, submit_mlops_monitoring_report, write_json,
};

pub fn build_mlops_monitoring_cycle_evidence(
    plan_uri: &str,
    artifact_evaluation_report_uri: &str,
    shadow_report_uri: &str,
    drift_report_uri: &str,
    fairness_report_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let plan_uri = required_non_empty("plan_uri", plan_uri)?;
    let plan = read_json_report(plan_uri)?;
    if json_string(&plan, "plan_kind").as_deref() != Some("scheduled_mlops_monitoring") {
        bail!("MLOps monitoring cycle requires a scheduled_mlops_monitoring plan");
    }
    let model_key = nested_json_string(&plan, &["model", "model_key"])
        .context("MLOps monitoring plan requires model.model_key")?;
    let model_version = nested_json_string(&plan, &["model", "model_version"])
        .context("MLOps monitoring plan requires model.model_version")?;

    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "create MLOps monitoring cycle output dir {}",
            output_dir.display()
        )
    })?;
    let monitoring_dir = output_dir.join("monitoring");
    let scheduler_dir = output_dir.join("scheduler");

    build_mlops_monitoring_report(
        &model_key,
        &model_version,
        artifact_evaluation_report_uri,
        shadow_report_uri,
        drift_report_uri,
        fairness_report_uri,
        &monitoring_dir,
    )?;
    let monitoring_report_uri = monitoring_dir.join("mlops_monitoring_report.json");
    build_mlops_scheduler_execution_report(
        plan_uri,
        &monitoring_report_uri.to_string_lossy(),
        &scheduler_dir,
    )?;
    let scheduler_execution_report_uri =
        scheduler_dir.join("mlops_scheduler_execution_report.json");
    let scheduler_execution = read_json_report(&scheduler_execution_report_uri.to_string_lossy())?;
    let monitoring_report = read_json_report(&monitoring_report_uri.to_string_lossy())?;
    let monitoring_report_uri_string = monitoring_report_uri.to_string_lossy().to_string();
    let scheduler_execution_report_uri_string =
        scheduler_execution_report_uri.to_string_lossy().to_string();
    let alert_delivery_task_count =
        json_u64(&scheduler_execution, "alert_delivery_task_count").unwrap_or(0);
    let cycle_status = if monitoring_report["overall_status"] == "blocked" {
        "completed_with_blocked_monitoring"
    } else if alert_delivery_task_count > 0 {
        "completed_with_alert_handoff_ready"
    } else {
        "completed_no_alerts_required"
    };
    let report = serde_json::json!({
        "report_kind": "mlops_monitoring_cycle_execution",
        "report_version": 1,
        "plan_uri": plan_uri,
        "model_key": model_key,
        "model_version": model_version,
        "monitoring_report_uri": monitoring_report_uri_string,
        "scheduler_execution_report_uri": scheduler_execution_report_uri_string,
        "cycle_status": cycle_status,
        "monitoring_status": monitoring_report["overall_status"].clone(),
        "retraining_recommendation": monitoring_report["retraining_recommendation"].clone(),
        "scheduler_status": scheduler_execution["scheduler_status"].clone(),
        "alert_delivery_status": scheduler_execution["alert_delivery_status"].clone(),
        "alert_delivery_task_count": alert_delivery_task_count,
        "api_submission_status": "not_requested",
        "governance_boundary": "monitoring cycle execution may create reports and submit governance handoffs only; it must not create retraining jobs, activate models, rollback models, assign fraud labels, or write rules",
        "evidence_refs": [
            format!("mlops_monitoring_plans:{plan_uri}"),
            format!("model_monitoring_reports:{monitoring_report_uri_string}"),
            format!(
                "mlops_scheduler_execution_reports:{scheduler_execution_report_uri_string}"
            )
        ]
    });
    write_json(
        output_dir.join("mlops_monitoring_cycle_report.json"),
        &report,
    )?;
    Ok(report)
}

pub async fn run_mlops_monitoring_cycle(
    plan_uri: &str,
    artifact_evaluation_report_uri: &str,
    shadow_report_uri: &str,
    drift_report_uri: &str,
    fairness_report_uri: &str,
    output_dir: impl AsRef<Path>,
    api_base_url: Option<&str>,
    api_key: Option<&str>,
    actor: Option<&str>,
    notes: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let output_dir = output_dir.as_ref().to_path_buf();
    let mut report = build_mlops_monitoring_cycle_evidence(
        plan_uri,
        artifact_evaluation_report_uri,
        shadow_report_uri,
        drift_report_uri,
        fairness_report_uri,
        &output_dir,
    )?;
    let submission_requested =
        api_base_url.is_some() || api_key.is_some() || actor.is_some() || notes.is_some();
    if !submission_requested {
        return Ok(report);
    }
    let api_base_url = required_optional("api_base_url", api_base_url)?;
    let api_key = required_optional("api_key", api_key)?;
    let actor = required_optional("actor", actor)?;
    let notes = required_optional("notes", notes)?;
    let monitoring_report_uri = json_string(&report, "monitoring_report_uri")
        .context("MLOps monitoring cycle report requires monitoring_report_uri")?;
    let scheduler_execution_report_uri = json_string(&report, "scheduler_execution_report_uri")
        .context("MLOps monitoring cycle report requires scheduler_execution_report_uri")?;
    let monitoring_submission =
        submit_mlops_monitoring_report(api_base_url, api_key, &monitoring_report_uri, actor, notes)
            .await?;
    let alert_delivery_submission = submit_mlops_alert_delivery_tasks(
        api_base_url,
        api_key,
        &scheduler_execution_report_uri,
        actor,
        notes,
    )
    .await?;
    report["api_submission_status"] = serde_json::json!("submitted");
    report["api_submissions"] = serde_json::json!({
        "monitoring_report": monitoring_submission,
        "alert_delivery": alert_delivery_submission
    });
    write_json(
        output_dir.join("mlops_monitoring_cycle_report.json"),
        &report,
    )?;
    Ok(report)
}
