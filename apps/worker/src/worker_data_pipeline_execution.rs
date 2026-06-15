use anyhow::{bail, Context};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use crate::{api_url, json_string, read_json_report, required_non_empty, write_json};

pub const REPORT_VERSION: u64 = 1;

pub fn validate_worker_data_pipeline_plan(
    plan: &serde_json::Value,
) -> anyhow::Result<(String, &[serde_json::Value])> {
    if json_string(plan, "plan_kind").as_deref() != Some("scheduled_worker_data_pipeline") {
        bail!("worker data pipeline requires a scheduled_worker_data_pipeline plan");
    }
    let customer_scope_id = json_string(plan, "customer_scope_id")
        .context("worker data pipeline plan requires customer_scope_id")?;
    let jobs = plan
        .get("jobs")
        .and_then(|value| value.as_array())
        .context("worker data pipeline plan requires jobs")?
        .as_slice();
    for job in jobs {
        if let Some(submit_command) = json_string(job, "submit_command") {
            let flags = job
                .get("required_submit_flags")
                .and_then(|value| value.as_array())
                .with_context(|| {
                    format!("{submit_command} requires non-empty required_submit_flags")
                })?;
            if flags.is_empty()
                || flags
                    .iter()
                    .any(|flag| flag.as_str().map(str::is_empty).unwrap_or(true))
            {
                bail!("{submit_command} requires non-empty required_submit_flags");
            }
        }
    }
    Ok((customer_scope_id, jobs))
}

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
    if json_string(&run_status, "report_kind").as_deref() != Some("worker_data_pipeline_run_status")
    {
        bail!("worker data pipeline execution requires a worker_data_pipeline_run_status report");
    }

    let (customer_scope_id, jobs) = validate_worker_data_pipeline_plan(&plan)?;
    let run_id = json_string(&run_status, "run_id")
        .context("worker data pipeline run status requires run_id")?;
    let execution_date = json_string(&run_status, "execution_date")
        .context("worker data pipeline run status requires execution_date")?;
    let readiness_report_uri = json_string(&run_status, "readiness_report_uri");
    let readiness_gate_status = readiness_gate_status(readiness_report_uri.as_deref())?;
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
                "required_submit_flags": job
                    .get("required_submit_flags")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
                "api_path": json_string(job, "api_path"),
                "required_permission": json_string(job, "required_permission"),
                "planned_report_uri": json_string(job, "report_uri"),
                "required_evidence_prefixes": job
                    .get("required_evidence_prefixes")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
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
                "required_submit_flags": execution["required_submit_flags"].clone(),
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
        "report_version": REPORT_VERSION,
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
    build_worker_data_pipeline_execution_submission_with_published_uri(
        report_uri, actor, notes, report_uri,
    )
}

pub fn build_worker_data_pipeline_execution_submission_with_published_uri(
    report_uri: &str,
    actor: &str,
    notes: &str,
    published_report_uri: &str,
) -> anyhow::Result<WorkerDataPipelineExecutionReportSubmission> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    let published_report_uri = required_non_empty("published_report_uri", published_report_uri)?;
    ensure_published_report_uri(
        "worker data pipeline execution published_report_uri",
        published_report_uri,
    )?;
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
    validate_worker_data_pipeline_job_kinds(&job_executions, "job_executions")?;
    validate_completed_job_evidence_prefixes(&job_executions)?;
    let review_tasks = report
        .get("review_tasks")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let job_count = report
        .get("job_count")
        .and_then(|value| value.as_u64())
        .context("worker data pipeline execution report requires job_count")?
        as usize;
    let pending_or_failed_job_count = report
        .get("pending_or_failed_job_count")
        .and_then(|value| value.as_u64())
        .context("worker data pipeline execution report requires pending_or_failed_job_count")?
        as usize;
    let review_task_count = report
        .get("review_task_count")
        .and_then(|value| value.as_u64())
        .context("worker data pipeline execution report requires review_task_count")?
        as usize;
    validate_worker_data_pipeline_execution_counts(
        &job_executions,
        &review_tasks,
        job_count,
        pending_or_failed_job_count,
        review_task_count,
    )?;
    let readiness_gate_status =
        json_string(&report, "readiness_gate_status").unwrap_or_else(|| "missing".into());
    let readiness_report_uri = json_string(&report, "readiness_report_uri");
    validate_worker_data_pipeline_execution_readiness_gate(
        &readiness_gate_status,
        readiness_report_uri.as_deref(),
    )?;
    validate_worker_data_pipeline_execution_review_tasks(
        &job_executions,
        &review_tasks,
        &readiness_gate_status,
    )?;
    let mut evidence_refs = report
        .get("evidence_refs")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let plan_uri = json_string(&report, "plan_uri")
        .context("worker data pipeline execution report requires plan_uri")?;
    let run_status_uri = json_string(&report, "run_status_uri")
        .context("worker data pipeline execution report requires run_status_uri")?;
    ensure_production_lineage_uri("plan_uri", &plan_uri)?;
    ensure_production_lineage_uri("run_status_uri", &run_status_uri)?;
    if let Some(readiness_report_uri) = readiness_report_uri.as_deref() {
        ensure_production_lineage_uri("readiness_report_uri", readiness_report_uri)?;
    }
    ensure_production_evidence_refs(&evidence_refs)?;
    let mut required_refs = vec![
        format!("worker_data_pipeline_plans:{plan_uri}"),
        format!("worker_data_pipeline_run_status:{run_status_uri}"),
    ];
    if let Some(readiness_report_uri) = readiness_report_uri.as_deref() {
        required_refs.push(format!(
            "worker_data_pipeline_readiness_reports:{readiness_report_uri}"
        ));
    }
    for required_ref in required_refs {
        if !evidence_refs
            .iter()
            .any(|reference| reference.trim() == required_ref)
        {
            bail!("worker data pipeline execution report requires {required_ref} evidence");
        }
    }
    validate_completed_job_scheduler_statuses(&job_executions)?;
    validate_completed_job_submit_contracts(&job_executions)?;
    evidence_refs.push(format!(
        "worker_data_pipeline_execution_reports:{published_report_uri}"
    ));
    Ok(WorkerDataPipelineExecutionReportSubmission {
        actor: actor.into(),
        notes: notes.into(),
        source_report_uri: published_report_uri.into(),
        report_kind: "worker_data_pipeline_execution_report".into(),
        plan_uri,
        run_status_uri,
        readiness_report_uri,
        readiness_gate_status,
        run_id: json_string(&report, "run_id")
            .context("worker data pipeline execution report requires run_id")?,
        execution_date: json_string(&report, "execution_date")
            .context("worker data pipeline execution report requires execution_date")?,
        job_count,
        pending_or_failed_job_count,
        review_task_count,
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
    submit_worker_data_pipeline_execution_report_with_published_uri(
        api_base_url,
        api_key,
        report_uri,
        actor,
        notes,
        report_uri,
    )
    .await
}

pub async fn submit_worker_data_pipeline_execution_report_with_published_uri(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    actor: &str,
    notes: &str,
    published_report_uri: &str,
) -> anyhow::Result<serde_json::Value> {
    let payload = build_worker_data_pipeline_execution_submission_with_published_uri(
        report_uri,
        actor,
        notes,
        published_report_uri,
    )?;
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
        if !has_reported_artifact_uri(reported) || !has_evidence_refs(reported) {
            return "artifact_missing_evidence";
        }
        if !has_production_artifact_uri(reported) {
            return "artifact_missing_evidence";
        }
        if has_template_artifact_uri(reported) || has_non_production_evidence_refs(reported) {
            return "artifact_missing_evidence";
        }
        if !has_required_evidence_prefixes(job, reported) {
            return "artifact_missing_evidence";
        }
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
        } else {
            "completed"
        }
    } else {
        "artifact_pending_submission"
    }
}

fn has_reported_artifact_uri(reported: &serde_json::Value) -> bool {
    json_string(reported, "artifact_uri").is_some_and(|value| !value.trim().is_empty())
}

fn has_template_artifact_uri(reported: &serde_json::Value) -> bool {
    json_string(reported, "artifact_uri")
        .is_some_and(|value| value.trim().starts_with("local://template"))
}

fn has_production_artifact_uri(reported: &serde_json::Value) -> bool {
    json_string(reported, "artifact_uri").is_some_and(|value| {
        let value = value.trim();
        !value.is_empty()
            && !value.starts_with("local://")
            && !value.starts_with("file://")
            && value.contains("://")
            && !value.contains('{')
            && !value.contains('}')
    })
}

fn has_evidence_refs(reported: &serde_json::Value) -> bool {
    reported
        .get("evidence_refs")
        .and_then(|value| value.as_array())
        .is_some_and(|references| {
            !references.is_empty()
                && references.iter().all(|reference| {
                    reference
                        .as_str()
                        .is_some_and(|value| !value.trim().is_empty())
                })
        })
}

fn has_non_production_evidence_refs(reported: &serde_json::Value) -> bool {
    reported
        .get("evidence_refs")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .any(|reference| {
            reference
                .as_str()
                .is_some_and(|value| evidence_ref_is_non_production(value))
        })
}

fn evidence_ref_is_non_production(value: &str) -> bool {
    let value = value.trim();
    value.contains("local://")
        || value.contains("file://")
        || value.contains('{')
        || value.contains('}')
}

fn ensure_production_lineage_uri(field: &str, value: &str) -> anyhow::Result<()> {
    if !is_production_lineage_uri(value) {
        bail!("{field} must use production evidence, not local dry-run or placeholder URI");
    }
    Ok(())
}

fn ensure_published_report_uri(field: &str, value: &str) -> anyhow::Result<()> {
    let value = value.trim();
    if value.is_empty()
        || value.starts_with("local://")
        || value.starts_with("file://")
        || !value.contains("://")
        || value.contains('{')
        || value.contains('}')
    {
        bail!("{field} must use production evidence, not local dry-run or placeholder URI");
    }
    let normalized = value
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !normalized.ends_with(".json") {
        bail!("{field} must point to a JSON report artifact");
    }
    Ok(())
}

fn ensure_production_evidence_refs(evidence_refs: &[String]) -> anyhow::Result<()> {
    if evidence_refs
        .iter()
        .any(|reference| evidence_ref_is_non_production(reference))
    {
        bail!("worker data pipeline execution evidence_refs must not use local dry-run or placeholder evidence");
    }
    Ok(())
}

fn is_production_lineage_uri(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && !value.starts_with("local://")
        && !value.starts_with("file://")
        && value.contains("://")
        && !value.contains('{')
        && !value.contains('}')
}

fn has_required_evidence_prefixes(job: &serde_json::Value, reported: &serde_json::Value) -> bool {
    let Some(required_prefixes) = job
        .get("required_evidence_prefixes")
        .and_then(|value| value.as_array())
    else {
        return true;
    };
    required_prefixes.iter().all(|prefix| {
        let Some(prefix) = prefix.as_str() else {
            return false;
        };
        if prefix.trim().is_empty() {
            return false;
        }
        reported
            .get("evidence_refs")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .any(|reference| {
                reference
                    .as_str()
                    .is_some_and(|value| value.starts_with(prefix))
            })
    })
}

fn validate_worker_data_pipeline_execution_review_tasks(
    job_executions: &[serde_json::Value],
    review_tasks: &[serde_json::Value],
    readiness_gate_status: &str,
) -> anyhow::Result<()> {
    if readiness_gate_status != "ready" {
        let has_matching_gate_review = review_tasks.iter().any(|task| {
            json_string(task, "task_kind").as_deref()
                == Some("worker_data_pipeline_readiness_gate_review")
                && json_string(task, "readiness_gate_status").as_deref()
                    == Some(readiness_gate_status)
        });
        if !has_matching_gate_review {
            bail!(
                "non-ready readiness_gate_status requires matching worker_data_pipeline_readiness_gate_review"
            );
        }
    }
    for job in job_executions {
        let job_kind = json_string(job, "job_kind").context("job_executions requires job_kind")?;
        let execution_status = json_string(job, "execution_status")
            .context("job_executions requires execution_status")?;
        if execution_status == "completed" {
            continue;
        }
        let has_matching_review = review_tasks.iter().any(|task| {
            json_string(task, "task_kind").as_deref()
                == Some("worker_data_pipeline_execution_review")
                && json_string(task, "job_kind").as_deref() == Some(job_kind.as_str())
                && json_string(task, "execution_status").as_deref()
                    == Some(execution_status.as_str())
        });
        if !has_matching_review {
            bail!("non-completed job requires matching worker_data_pipeline_execution_review task");
        }
    }
    for task in review_tasks {
        match json_string(task, "task_kind").as_deref() {
            Some("worker_data_pipeline_execution_review") => {
                let job_kind = json_string(task, "job_kind")
                    .context("worker_data_pipeline_execution_review requires job_kind")?;
                let execution_status = json_string(task, "execution_status")
                    .context("worker_data_pipeline_execution_review requires execution_status")?;
                validate_submit_job_contract_fields(&job_kind, task)?;
                let matches_non_completed_job = job_executions.iter().any(|job| {
                    json_string(job, "job_kind").as_deref() == Some(job_kind.as_str())
                        && json_string(job, "execution_status").as_deref()
                            == Some(execution_status.as_str())
                        && execution_status != "completed"
                });
                if !matches_non_completed_job {
                    bail!(
                        "worker_data_pipeline_execution_review task must match a non-completed job"
                    );
                }
            }
            Some("worker_data_pipeline_readiness_gate_review") => {
                let task_status = json_string(task, "readiness_gate_status")
                    .context("worker_data_pipeline_readiness_gate_review requires readiness_gate_status")?;
                if readiness_gate_status == "ready" || task_status != readiness_gate_status {
                    bail!(
                        "worker_data_pipeline_readiness_gate_review task must match non-ready readiness_gate_status"
                    );
                }
            }
            Some(_) | None => bail!(
                "review task kind must be worker_data_pipeline_execution_review or worker_data_pipeline_readiness_gate_review"
            ),
        }
    }
    Ok(())
}

fn validate_worker_data_pipeline_execution_readiness_gate(
    readiness_gate_status: &str,
    readiness_report_uri: Option<&str>,
) -> anyhow::Result<()> {
    if !matches!(readiness_gate_status, "ready" | "blocked" | "missing") {
        bail!("readiness_gate_status must be ready, blocked, or missing");
    }
    if readiness_report_uri.is_some() && !matches!(readiness_gate_status, "ready" | "blocked") {
        bail!(
            "readiness_gate_status must be ready or blocked when readiness_report_uri is supplied"
        );
    }
    if matches!(readiness_gate_status, "ready" | "blocked") && readiness_report_uri.is_none() {
        bail!("readiness_report_uri is required when readiness_gate_status is ready or blocked");
    }
    Ok(())
}

fn validate_worker_data_pipeline_execution_counts(
    job_executions: &[serde_json::Value],
    review_tasks: &[serde_json::Value],
    job_count: usize,
    pending_or_failed_job_count: usize,
    review_task_count: usize,
) -> anyhow::Result<()> {
    if job_count != job_executions.len() {
        bail!("job_count must match job_executions length");
    }
    let actual_pending_or_failed = job_executions
        .iter()
        .filter(|job| json_string(job, "execution_status").as_deref() != Some("completed"))
        .count();
    if pending_or_failed_job_count != actual_pending_or_failed {
        bail!("pending_or_failed_job_count must match non-completed job_executions");
    }
    if review_task_count != review_tasks.len() {
        bail!("review_task_count must match review_tasks length");
    }
    Ok(())
}

pub(crate) fn validate_worker_data_pipeline_job_kinds(
    records: &[serde_json::Value],
    field_name: &str,
) -> anyhow::Result<()> {
    let mut seen_job_kinds = BTreeSet::new();
    for record in records {
        let job_kind = json_string(record, "job_kind")
            .with_context(|| format!("{field_name} requires job_kind"))?;
        if canonical_required_evidence_prefixes(&job_kind).is_none() {
            bail!("unknown worker data pipeline job_kind {job_kind}");
        }
        if !seen_job_kinds.insert(job_kind.clone()) {
            bail!("duplicate worker data pipeline job_kind {job_kind}");
        }
    }
    Ok(())
}

fn validate_completed_job_evidence_prefixes(
    job_executions: &[serde_json::Value],
) -> anyhow::Result<()> {
    for job in job_executions {
        if json_string(job, "execution_status").as_deref() != Some("completed") {
            continue;
        }
        let Some(job_kind) = json_string(job, "job_kind") else {
            continue;
        };
        if json_string(job, "reported_artifact_uri")
            .is_some_and(|value| value.trim().starts_with("local://template"))
        {
            bail!("{job_kind} reported_artifact_uri must not use local://template evidence");
        }
        if job
            .get("evidence_refs")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .any(|value| {
                value
                    .as_str()
                    .is_some_and(|reference| evidence_ref_is_non_production(reference))
            })
        {
            bail!("{job_kind} evidence_refs must not use local or placeholder evidence");
        }
        let Some(required_prefixes) = canonical_required_evidence_prefixes(&job_kind) else {
            continue;
        };
        for prefix in required_prefixes {
            if !job
                .get("required_evidence_prefixes")
                .and_then(|value| value.as_array())
                .into_iter()
                .flatten()
                .any(|value| value.as_str() == Some(prefix))
            {
                bail!("{job_kind} required_evidence_prefixes must include {prefix}");
            }
            if !job
                .get("evidence_refs")
                .and_then(|value| value.as_array())
                .into_iter()
                .flatten()
                .any(|value| {
                    value
                        .as_str()
                        .is_some_and(|reference| reference.starts_with(prefix))
                })
            {
                bail!("{job_kind} evidence_refs must include {prefix}");
            }
        }
    }
    Ok(())
}

fn validate_completed_job_scheduler_statuses(
    job_executions: &[serde_json::Value],
) -> anyhow::Result<()> {
    for job in job_executions {
        if json_string(job, "execution_status").as_deref() != Some("completed") {
            continue;
        }
        if json_string(job, "reported_status").as_deref() != Some("succeeded") {
            bail!("completed job executions require reported_status succeeded");
        }
        if !json_string(job, "reported_artifact_uri")
            .is_some_and(|value| is_production_lineage_uri(&value))
        {
            bail!(
                "completed job executions require a production reported_artifact_uri, not a local dry-run or placeholder URI"
            );
        }
        let has_blocked_dependencies = job
            .get("blocked_dependencies")
            .and_then(|value| value.as_array())
            .is_some_and(|dependencies| !dependencies.is_empty());
        if has_blocked_dependencies {
            bail!("completed job executions must not include blocked_dependencies");
        }
    }
    Ok(())
}

fn validate_completed_job_submit_contracts(
    job_executions: &[serde_json::Value],
) -> anyhow::Result<()> {
    for job in job_executions {
        if json_string(job, "execution_status").as_deref() != Some("completed") {
            continue;
        }
        let Some(job_kind) = json_string(job, "job_kind") else {
            continue;
        };
        validate_completed_job_submit_contract(&job_kind, job)?;
    }
    Ok(())
}

fn validate_completed_job_submit_contract(
    job_kind: &str,
    job: &serde_json::Value,
) -> anyhow::Result<()> {
    validate_submit_job_contract_fields(job_kind, job)?;
    if worker_data_pipeline_submit_job_contract(job_kind).is_some()
        && job.get("submitted").and_then(|value| value.as_bool()) != Some(true)
    {
        bail!("completed governed submit job executions require submitted true");
    }
    Ok(())
}

fn validate_submit_job_contract_fields(
    job_kind: &str,
    job: &serde_json::Value,
) -> anyhow::Result<()> {
    let Some((expected_api_path, expected_permission)) =
        worker_data_pipeline_submit_job_contract(job_kind)
    else {
        return Ok(());
    };
    if json_string(job, "api_path").as_deref() != Some(expected_api_path) {
        bail!("{job_kind} requires api_path {expected_api_path}");
    }
    if json_string(job, "required_permission").as_deref() != Some(expected_permission) {
        bail!("{job_kind} requires required_permission {expected_permission}");
    }
    let Some(expected_flags) = worker_data_pipeline_submit_job_required_flags(job_kind) else {
        return Ok(());
    };
    let Some(required_submit_flags) = job
        .get("required_submit_flags")
        .and_then(|value| value.as_array())
    else {
        bail!("{job_kind} requires required_submit_flags {expected_flags:?}");
    };
    let submitted_flags = required_submit_flags
        .iter()
        .map(|value| value.as_str())
        .collect::<Option<Vec<_>>>();
    if submitted_flags.as_deref() != Some(expected_flags) {
        bail!("{job_kind} requires required_submit_flags {expected_flags:?}");
    }
    Ok(())
}

pub(crate) fn canonical_required_evidence_prefixes(
    job_kind: &str,
) -> Option<&'static [&'static str]> {
    match job_kind {
        "oig_sam_sanctions_snapshot_fetch" => Some(&["oig_sam_snapshot:"]),
        "oig_sam_sanctions_sync" => Some(&["sanctions_sync_reports:"]),
        "provider_profile_window_rollup" => Some(&[
            "provider_profile_window_rollups:",
            "provider_profile_claim_snapshot:",
        ]),
        "provider_graph_signal_rollup" => Some(&[
            "provider_graph_signal_rollups:",
            "provider_graph_claim_snapshot:",
        ]),
        "peer_percentile_benchmark" => {
            Some(&["peer_benchmarks:", "peer_benchmark_claim_snapshot:"])
        }
        "episode_aggregation" => Some(&["episode_rollups:", "episode_claim_snapshot:"]),
        "clinical_compatibility_reference" => Some(&[
            "clinical_compatibility_references:",
            "clinical_compatibility_reference:",
            "clinical_policy_authority:",
        ]),
        "unbundling_comparator" => Some(&[
            "unbundling_comparator_candidates:",
            "unbundling_comparator_input:",
        ]),
        "scoring_feature_context_materialization" => Some(&[
            "scoring_feature_contexts:",
            "scoring_feature_context_claim_snapshot:",
            "episode_rollups:",
            "peer_benchmarks:",
            "clinical_compatibility:",
            "unbundling_candidates:",
        ]),
        "scoring_online_readback" => Some(&[
            "scoring_readback_reports:",
            "scoring_readback_inputs:",
            "scoring_readback_score_requests:",
            "scoring_readback_score_responses:",
            "scoring_feature_contexts:",
            "provider_profile_window_rollups:",
            "sanctions_sync_reports:",
            "provider_graph_signal_rollups:",
            "peer_benchmarks:",
            "episode_rollups:",
            "clinical_compatibility:",
            "unbundling_candidates:",
        ]),
        "probability_calibration_evidence" => Some(&[
            "probability_calibration_reports:",
            "probability_calibration_input:",
            "calibration_labels:",
        ]),
        _ => None,
    }
}

pub(crate) fn worker_data_pipeline_submit_job_contract(
    job_kind: &str,
) -> Option<(&'static str, &'static str)> {
    match job_kind {
        "oig_sam_sanctions_sync" => Some((
            "/api/v1/ops/providers/sanctions-sync-reports",
            "ops:providers:write",
        )),
        "provider_profile_window_rollup" => Some((
            "/api/v1/ops/providers/profile-window-rollups",
            "ops:providers:write",
        )),
        "provider_graph_signal_rollup" => Some((
            "/api/v1/ops/providers/graph-signal-rollups",
            "ops:providers:write",
        )),
        "peer_percentile_benchmark" => Some((
            "/api/v1/ops/providers/peer-benchmarks",
            "ops:providers:write",
        )),
        "episode_aggregation" => Some((
            "/api/v1/ops/providers/episode-rollups",
            "ops:providers:write",
        )),
        "clinical_compatibility_reference" => Some((
            "/api/v1/ops/clinical-compatibility-references",
            "ops:datasets:write",
        )),
        "unbundling_comparator" => Some((
            "/api/v1/ops/unbundling-comparator-candidates",
            "ops:datasets:write",
        )),
        "scoring_feature_context_materialization" => Some((
            "/api/v1/ops/scoring-feature-context-materializations",
            "ops:datasets:write",
        )),
        "probability_calibration_evidence" => Some((
            "/api/v1/ops/models/{model_key}/probability-calibration-reports",
            "ops:models:review",
        )),
        _ => None,
    }
}

pub(crate) fn worker_data_pipeline_submit_job_required_flags(
    job_kind: &str,
) -> Option<&'static [&'static str]> {
    match job_kind {
        "oig_sam_sanctions_sync"
        | "provider_profile_window_rollup"
        | "provider_graph_signal_rollup"
        | "peer_percentile_benchmark"
        | "episode_aggregation"
        | "clinical_compatibility_reference"
        | "unbundling_comparator" => Some(&["--published-report-uri", "--published-source-uri"]),
        "scoring_feature_context_materialization" => Some(&["--published-report-uri"]),
        "probability_calibration_evidence" => Some(&[
            "--published-report-uri",
            "--published-input-uri",
            "--published-label-uri",
        ]),
        _ => None,
    }
}
