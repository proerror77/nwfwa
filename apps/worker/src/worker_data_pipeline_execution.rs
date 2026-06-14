use anyhow::{bail, Context};
use std::{collections::BTreeMap, fs, path::Path};

use crate::{json_string, read_json_report, required_non_empty, write_json};

pub fn build_worker_data_pipeline_execution_report(
    plan_uri: &str,
    run_status_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let plan_uri = required_non_empty("plan_uri", plan_uri)?;
    let run_status_uri = required_non_empty("run_status_uri", run_status_uri)?;
    let plan = read_json_report(plan_uri)?;
    let run_status = read_json_report(run_status_uri)?;
    if json_string(&plan, "plan_kind").as_deref() != Some("scheduled_worker_data_pipeline") {
        bail!("worker data pipeline execution requires a scheduled_worker_data_pipeline plan");
    }
    if json_string(&run_status, "report_kind").as_deref() != Some("worker_data_pipeline_run_status")
    {
        bail!("worker data pipeline execution requires a worker_data_pipeline_run_status report");
    }

    let customer_scope_id = json_string(&plan, "customer_scope_id")
        .context("worker data pipeline plan requires customer_scope_id")?;
    let run_id = json_string(&run_status, "run_id")
        .context("worker data pipeline run status requires run_id")?;
    let execution_date = json_string(&run_status, "execution_date")
        .context("worker data pipeline run status requires execution_date")?;
    let jobs = plan
        .get("jobs")
        .and_then(|value| value.as_array())
        .context("worker data pipeline plan requires jobs")?;
    let reported_jobs = reported_job_statuses(&run_status);
    let job_executions = jobs
        .iter()
        .map(|job| {
            let job_kind = json_string(job, "job_kind").unwrap_or_else(|| "unknown".into());
            let reported = reported_jobs.get(&job_kind);
            serde_json::json!({
                "job_kind": job_kind,
                "cadence": json_string(job, "cadence"),
                "build_command": json_string(job, "build_command"),
                "submit_command": json_string(job, "submit_command"),
                "api_path": json_string(job, "api_path"),
                "planned_report_uri": json_string(job, "report_uri"),
                "reported_status": reported.and_then(|status| json_string(status, "status")),
                "reported_artifact_uri": reported.and_then(|status| json_string(status, "artifact_uri")),
                "submitted": reported
                    .and_then(|status| status.get("submitted"))
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false),
                "execution_status": execution_status(reported)
            })
        })
        .collect::<Vec<_>>();
    let pending_job_count = job_executions
        .iter()
        .filter(|execution| execution["execution_status"].as_str() != Some("completed"))
        .count();
    let scheduler_status = if pending_job_count == 0 {
        "completed"
    } else {
        "completed_with_pending_or_failed_jobs"
    };
    let review_tasks = job_executions
        .iter()
        .filter(|execution| execution["execution_status"].as_str() != Some("completed"))
        .map(|execution| {
            serde_json::json!({
                "task_kind": "worker_data_pipeline_execution_review",
                "customer_scope_id": customer_scope_id,
                "run_id": run_id,
                "job_kind": execution["job_kind"].clone(),
                "execution_status": execution["execution_status"].clone(),
                "review_queue": "worker_data_pipeline_ops",
                "required_review": "review missing, failed, or unsubmitted worker data pipeline artifact before downstream scoring use"
            })
        })
        .collect::<Vec<_>>();
    let report = serde_json::json!({
        "report_kind": "worker_data_pipeline_execution_report",
        "report_version": 1,
        "plan_uri": plan_uri,
        "run_status_uri": run_status_uri,
        "customer_scope_id": customer_scope_id,
        "run_id": run_id,
        "execution_date": execution_date,
        "schedule": plan["schedule"].clone(),
        "scheduler_status": scheduler_status,
        "job_count": jobs.len(),
        "pending_or_failed_job_count": pending_job_count,
        "job_executions": job_executions,
        "review_task_count": review_tasks.len(),
        "review_tasks": review_tasks,
        "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only; it must not score claims, assign labels, deny claims, activate models, or change routing policy",
        "evidence_refs": [
            format!("worker_data_pipeline_plans:{plan_uri}"),
            format!("worker_data_pipeline_run_status:{run_status_uri}")
        ]
    });

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create worker data pipeline execution output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("worker_data_pipeline_execution_report.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("worker_data_pipeline_execution_review_tasks.json"),
        &report["review_tasks"],
    )?;
    Ok(report)
}

fn reported_job_statuses(run_status: &serde_json::Value) -> BTreeMap<String, serde_json::Value> {
    run_status
        .get("job_statuses")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|status| {
            let job_kind = json_string(status, "job_kind")?;
            Some((job_kind, status.clone()))
        })
        .collect()
}

fn execution_status(reported: Option<&serde_json::Value>) -> &'static str {
    let Some(reported) = reported else {
        return "scheduled_pending_customer_execution";
    };
    if json_string(reported, "status").as_deref() == Some("succeeded")
        && reported
            .get("submitted")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    {
        "completed"
    } else if json_string(reported, "status").as_deref() == Some("failed") {
        "failed"
    } else {
        "artifact_pending_submission"
    }
}
