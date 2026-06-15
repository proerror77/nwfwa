use anyhow::{bail, Context};
use std::{fs, path::Path};

use crate::{json_string, read_json_report, required_non_empty, write_json};

pub fn build_worker_data_pipeline_run_status_template(
    plan_uri: &str,
    readiness_report_uri: &str,
    run_id: &str,
    execution_date: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let plan_uri = required_non_empty("plan_uri", plan_uri)?;
    let readiness_report_uri = required_non_empty("readiness_report_uri", readiness_report_uri)?;
    let run_id = required_non_empty("run_id", run_id)?;
    let execution_date = required_non_empty("execution_date", execution_date)?;
    let plan = read_json_report(plan_uri)?;
    if json_string(&plan, "plan_kind").as_deref() != Some("scheduled_worker_data_pipeline") {
        bail!("worker data pipeline run status template requires a scheduled_worker_data_pipeline plan");
    }
    let customer_scope_id = json_string(&plan, "customer_scope_id")
        .context("worker data pipeline plan requires customer_scope_id")?;
    let jobs = plan
        .get("jobs")
        .and_then(|value| value.as_array())
        .context("worker data pipeline plan requires jobs")?;
    let job_statuses = jobs
        .iter()
        .map(|job| {
            serde_json::json!({
                "job_kind": json_string(job, "job_kind").unwrap_or_else(|| "unknown".into()),
                "cadence": json_string(job, "cadence"),
                "build_command": json_string(job, "build_command"),
                "score_response_capture_command": json_string(job, "score_response_capture_command"),
                "source_input": json_string(job, "source_input"),
                "required_permission": json_string(job, "required_permission"),
                "depends_on": job
                    .get("depends_on")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
                "artifact_kind": json_string(job, "artifact_kind"),
                "planned_report_uri": json_string(job, "report_uri"),
                "submit_command": json_string(job, "submit_command"),
                "required_submit_flags": job
                    .get("required_submit_flags")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
                "api_path": json_string(job, "api_path"),
                "required_evidence_prefixes": job
                    .get("required_evidence_prefixes")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
                "status": "scheduled_pending_customer_execution",
                "artifact_uri": serde_json::Value::Null,
                "evidence_refs": [],
                "submitted": false
            })
        })
        .collect::<Vec<_>>();
    let report = serde_json::json!({
        "report_kind": "worker_data_pipeline_run_status",
        "report_version": 1,
        "run_status_template": true,
        "plan_uri": plan_uri,
        "readiness_report_uri": readiness_report_uri,
        "customer_scope_id": customer_scope_id,
        "run_id": run_id,
        "execution_date": execution_date,
        "job_count": job_statuses.len(),
        "job_statuses": job_statuses,
        "governance_boundary": "run status templates are scheduler input contracts only; they must not execute jobs, submit artifacts, score claims, assign labels, deny claims, activate models, or change routing policy",
        "evidence_refs": [
            format!("worker_data_pipeline_plans:{plan_uri}"),
            format!("worker_data_pipeline_readiness_reports:{readiness_report_uri}")
        ]
    });

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create worker data pipeline run status output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("worker_data_pipeline_run_status_template.json"),
        &report,
    )?;
    Ok(report)
}
