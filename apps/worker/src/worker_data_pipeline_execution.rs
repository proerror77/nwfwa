use anyhow::{bail, Context};
use serde::Serialize;
use std::{collections::BTreeMap, fs, path::Path};

use crate::{api_url, json_string, read_json_report, required_non_empty, write_json};

#[derive(Debug, Serialize)]
pub struct WorkerDataPipelineExecutionReportSubmission {
    pub actor: String,
    pub notes: String,
    pub source_report_uri: String,
    pub report_kind: String,
    pub plan_uri: String,
    pub run_status_uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readiness_report_uri: Option<String>,
    pub readiness_gate_status: String,
    pub run_id: String,
    pub execution_date: String,
    pub job_count: usize,
    pub pending_or_failed_job_count: usize,
    pub review_task_count: usize,
    pub job_executions: Vec<serde_json::Value>,
    pub review_tasks: Vec<serde_json::Value>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

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
    let readiness_report_uri = json_string(&run_status, "readiness_report_uri");
    let readiness_gate_status = readiness_gate_status(readiness_report_uri.as_deref())?;
    let jobs = plan
        .get("jobs")
        .and_then(|value| value.as_array())
        .context("worker data pipeline plan requires jobs")?;
    let reported_jobs = reported_job_statuses(&run_status);
    let jobs_by_kind = jobs
        .iter()
        .filter_map(|job| Some((json_string(job, "job_kind")?, job)))
        .collect::<BTreeMap<_, _>>();
    let job_executions = jobs
        .iter()
        .map(|job| {
            let job_kind = json_string(job, "job_kind").unwrap_or_else(|| "unknown".into());
            let reported = reported_jobs.get(&job_kind);
            let blocked_dependencies = blocked_dependencies(job, &jobs_by_kind, &reported_jobs);
            let execution_status = execution_status(job, reported, &blocked_dependencies);
            serde_json::json!({
                "job_kind": job_kind,
                "cadence": json_string(job, "cadence"),
                "build_command": json_string(job, "build_command"),
                "submit_command": json_string(job, "submit_command"),
                "api_path": json_string(job, "api_path"),
                "required_permission": json_string(job, "required_permission"),
                "planned_report_uri": json_string(job, "report_uri"),
                "reported_status": reported.and_then(|status| json_string(status, "status")),
                "reported_artifact_uri": reported.and_then(|status| json_string(status, "artifact_uri")),
                "evidence_refs": reported
                    .and_then(|status| status.get("evidence_refs"))
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
                "submitted": reported
                    .and_then(|status| status.get("submitted"))
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false),
                "blocked_dependencies": blocked_dependencies,
                "execution_status": execution_status
            })
        })
        .collect::<Vec<_>>();
    let pending_job_count = job_executions
        .iter()
        .filter(|execution| execution["execution_status"].as_str() != Some("completed"))
        .count();
    let scheduler_status = if pending_job_count == 0 {
        if readiness_gate_status == "ready" {
            "completed"
        } else {
            "completed_with_pending_or_failed_jobs"
        }
    } else {
        "completed_with_pending_or_failed_jobs"
    };
    let mut review_tasks = job_executions
        .iter()
        .filter(|execution| execution["execution_status"].as_str() != Some("completed"))
        .map(|execution| {
            serde_json::json!({
                "task_kind": "worker_data_pipeline_execution_review",
                "customer_scope_id": customer_scope_id,
                "run_id": run_id,
                "job_kind": execution["job_kind"].clone(),
                "execution_status": execution["execution_status"].clone(),
                "api_path": execution["api_path"].clone(),
                "required_permission": execution["required_permission"].clone(),
                "review_queue": "worker_data_pipeline_ops",
                "required_review": "review missing, failed, or unsubmitted worker data pipeline artifact before downstream scoring use"
            })
        })
        .collect::<Vec<_>>();
    if readiness_gate_status != "ready" {
        review_tasks.push(serde_json::json!({
            "task_kind": "worker_data_pipeline_readiness_gate_review",
            "customer_scope_id": customer_scope_id,
            "run_id": run_id,
            "readiness_report_uri": readiness_report_uri,
            "readiness_gate_status": readiness_gate_status,
            "review_queue": "worker_data_pipeline_ops",
            "required_review": "resolve worker data pipeline readiness gate before downstream scoring use"
        }));
    }
    let mut evidence_refs = vec![
        format!("worker_data_pipeline_plans:{plan_uri}"),
        format!("worker_data_pipeline_run_status:{run_status_uri}"),
    ];
    if let Some(readiness_report_uri) = &readiness_report_uri {
        evidence_refs.push(format!(
            "worker_data_pipeline_readiness_reports:{readiness_report_uri}"
        ));
    }
    let report = serde_json::json!({
        "report_kind": "worker_data_pipeline_execution_report",
        "report_version": 1,
        "plan_uri": plan_uri,
        "run_status_uri": run_status_uri,
        "readiness_report_uri": readiness_report_uri,
        "readiness_gate_status": readiness_gate_status,
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
        "evidence_refs": evidence_refs
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

pub fn build_worker_data_pipeline_execution_submission(
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<WorkerDataPipelineExecutionReportSubmission> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    let report = read_json_report(report_uri)?;
    if json_string(&report, "report_kind").as_deref()
        != Some("worker_data_pipeline_execution_report")
    {
        bail!("report_kind must be worker_data_pipeline_execution_report");
    }
    let job_executions = report
        .get("job_executions")
        .and_then(|value| value.as_array())
        .cloned()
        .context("worker data pipeline execution report requires job_executions")?;
    let review_tasks = report
        .get("review_tasks")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let mut evidence_refs = report
        .get("evidence_refs")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    evidence_refs.push(format!(
        "worker_data_pipeline_execution_reports:{report_uri}"
    ));
    Ok(WorkerDataPipelineExecutionReportSubmission {
        actor: actor.into(),
        notes: notes.into(),
        source_report_uri: report_uri.into(),
        report_kind: "worker_data_pipeline_execution_report".into(),
        plan_uri: json_string(&report, "plan_uri")
            .context("worker data pipeline execution report requires plan_uri")?,
        run_status_uri: json_string(&report, "run_status_uri")
            .context("worker data pipeline execution report requires run_status_uri")?,
        readiness_report_uri: json_string(&report, "readiness_report_uri"),
        readiness_gate_status: json_string(&report, "readiness_gate_status")
            .unwrap_or_else(|| "missing".into()),
        run_id: json_string(&report, "run_id")
            .context("worker data pipeline execution report requires run_id")?,
        execution_date: json_string(&report, "execution_date")
            .context("worker data pipeline execution report requires execution_date")?,
        job_count: report
            .get("job_count")
            .and_then(|value| value.as_u64())
            .context("worker data pipeline execution report requires job_count")?
            as usize,
        pending_or_failed_job_count: report
            .get("pending_or_failed_job_count")
            .and_then(|value| value.as_u64())
            .context("worker data pipeline execution report requires pending_or_failed_job_count")?
            as usize,
        review_task_count: report
            .get("review_task_count")
            .and_then(|value| value.as_u64())
            .context("worker data pipeline execution report requires review_task_count")?
            as usize,
        job_executions,
        review_tasks,
        evidence_refs,
        governance_boundary: json_string(&report, "governance_boundary")
            .context("worker data pipeline execution report requires governance_boundary")?,
    })
}

pub async fn submit_worker_data_pipeline_execution_report(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    let payload = build_worker_data_pipeline_execution_submission(report_uri, actor, notes)?;
    let client = reqwest::Client::new();
    let response = client
        .post(api_url(
            api_base_url,
            "/api/v1/ops/worker-data-pipeline-executions",
        ))
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .context("submit worker data pipeline execution report")?;
    let status = response.status();
    let body = response.text().await.context("read submit response")?;
    if !status.is_success() {
        bail!("submit worker data pipeline execution report failed with {status}: {body}");
    }
    serde_json::from_str(&body).context("parse worker data pipeline execution response")
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

fn readiness_gate_status(readiness_report_uri: Option<&str>) -> anyhow::Result<&'static str> {
    let Some(readiness_report_uri) = readiness_report_uri else {
        return Ok("missing");
    };
    let readiness_report = read_json_report(readiness_report_uri)?;
    if json_string(&readiness_report, "report_kind").as_deref()
        != Some("worker_data_pipeline_readiness_report")
    {
        bail!("readiness_report_uri must point to a worker_data_pipeline_readiness_report");
    }
    if json_string(&readiness_report, "readiness_status").as_deref() == Some("ready") {
        Ok("ready")
    } else {
        Ok("blocked")
    }
}

fn blocked_dependencies(
    job: &serde_json::Value,
    jobs_by_kind: &BTreeMap<String, &serde_json::Value>,
    reported_jobs: &BTreeMap<String, serde_json::Value>,
) -> Vec<String> {
    job.get("depends_on")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|dependency| dependency.as_str())
        .filter(|dependency| {
            let Some(dependency_job) = jobs_by_kind.get(*dependency) else {
                return true;
            };
            let dependency_reported = reported_jobs.get(*dependency);
            base_execution_status(dependency_job, dependency_reported) != "completed"
        })
        .map(str::to_string)
        .collect()
}

fn execution_status(
    job: &serde_json::Value,
    reported: Option<&serde_json::Value>,
    blocked_dependencies: &[String],
) -> &'static str {
    if !blocked_dependencies.is_empty() {
        return "dependency_not_completed";
    }
    base_execution_status(job, reported)
}

fn base_execution_status(
    job: &serde_json::Value,
    reported: Option<&serde_json::Value>,
) -> &'static str {
    let Some(reported) = reported else {
        return "scheduled_pending_customer_execution";
    };
    if json_string(reported, "status").as_deref() == Some("failed") {
        "failed"
    } else if json_string(reported, "status").as_deref() == Some("succeeded") {
        if json_string(job, "submit_command").is_some() {
            if reported
                .get("submitted")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
            {
                "completed"
            } else {
                "artifact_pending_submission"
            }
        } else if json_string(reported, "artifact_uri")
            .is_some_and(|value| !value.trim().is_empty())
        {
            "completed"
        } else {
            "artifact_pending_submission"
        }
    } else {
        "artifact_pending_submission"
    }
}
