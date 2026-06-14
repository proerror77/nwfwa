use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

use crate::{json_string, read_json_report, required_non_empty, write_json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerDataPipelineReadinessInput {
    #[serde(default)]
    pub checks: Vec<WorkerDataPipelineReadinessCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerDataPipelineReadinessCheck {
    pub job_kind: String,
    pub artifact_uri: Option<String>,
    #[serde(default)]
    pub customer_approved: bool,
    #[serde(default)]
    pub external_fetch_configured: bool,
    pub row_count: Option<u64>,
    pub minimum_row_count: Option<u64>,
    pub data_quality_status: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

pub fn build_worker_data_pipeline_readiness_report(
    plan_uri: &str,
    readiness_input_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let plan_uri = required_non_empty("plan_uri", plan_uri)?;
    let readiness_input_uri = required_non_empty("readiness_input_uri", readiness_input_uri)?;
    let plan = read_json_report(plan_uri)?;
    if json_string(&plan, "plan_kind").as_deref() != Some("scheduled_worker_data_pipeline") {
        bail!("worker data pipeline readiness requires a scheduled_worker_data_pipeline plan");
    }
    let input: WorkerDataPipelineReadinessInput =
        serde_json::from_value(read_json_report(readiness_input_uri)?)
            .context("parse worker data pipeline readiness input")?;
    let customer_scope_id = json_string(&plan, "customer_scope_id")
        .context("worker data pipeline plan requires customer_scope_id")?;
    let jobs = plan
        .get("jobs")
        .and_then(|value| value.as_array())
        .context("worker data pipeline plan requires jobs")?;
    let checks_by_job = input
        .checks
        .iter()
        .map(|check| (check.job_kind.as_str(), check))
        .collect::<BTreeMap<_, _>>();
    let job_readiness = jobs
        .iter()
        .map(|job| {
            let job_kind = json_string(job, "job_kind").unwrap_or_else(|| "unknown".into());
            let check = checks_by_job.get(job_kind.as_str()).copied();
            let blockers = readiness_blockers(&job_kind, check);
            serde_json::json!({
                "job_kind": job_kind,
                "cadence": json_string(job, "cadence"),
                "source_input": json_string(job, "source_input"),
                "artifact_uri": check.and_then(|check| check.artifact_uri.clone()),
                "customer_approved": check.map(|check| check.customer_approved).unwrap_or(false),
                "external_fetch_configured": check
                    .map(|check| check.external_fetch_configured)
                    .unwrap_or(false),
                "row_count": check.and_then(|check| check.row_count),
                "minimum_row_count": check.and_then(|check| check.minimum_row_count),
                "data_quality_status": check.and_then(|check| check.data_quality_status.clone()),
                "readiness_status": if blockers.is_empty() { "ready" } else { "blocked" },
                "blockers": blockers,
                "evidence_refs": check
                    .map(|check| check.evidence_refs.clone())
                    .unwrap_or_default()
            })
        })
        .collect::<Vec<_>>();
    let blocked_jobs = job_readiness
        .iter()
        .filter(|job| job["readiness_status"].as_str() == Some("blocked"))
        .collect::<Vec<_>>();
    let review_tasks = blocked_jobs
        .iter()
        .map(|job| {
            serde_json::json!({
                "task_kind": "worker_data_pipeline_readiness_review",
                "customer_scope_id": customer_scope_id,
                "job_kind": job["job_kind"].clone(),
                "blockers": job["blockers"].clone(),
                "review_queue": "worker_data_pipeline_ops",
                "required_review": "resolve customer data, approval, quality, or external-fetch readiness before scheduled writes"
            })
        })
        .collect::<Vec<_>>();
    let readiness_status = if blocked_jobs.is_empty() {
        "ready"
    } else {
        "blocked"
    };
    let report = serde_json::json!({
        "report_kind": "worker_data_pipeline_readiness_report",
        "report_version": 1,
        "plan_uri": plan_uri,
        "readiness_input_uri": readiness_input_uri,
        "customer_scope_id": customer_scope_id,
        "readiness_status": readiness_status,
        "job_count": jobs.len(),
        "ready_job_count": job_readiness.len() - blocked_jobs.len(),
        "blocked_job_count": blocked_jobs.len(),
        "job_readiness": job_readiness,
        "review_task_count": review_tasks.len(),
        "review_tasks": review_tasks,
        "governance_boundary": "readiness report validates customer data prerequisites only; it must not fetch external data, submit artifacts, score claims, assign labels, activate models, or change routing policy",
        "evidence_refs": [
            format!("worker_data_pipeline_plans:{plan_uri}"),
            format!("worker_data_pipeline_readiness_inputs:{readiness_input_uri}")
        ]
    });

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create worker data pipeline readiness output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("worker_data_pipeline_readiness_report.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("worker_data_pipeline_readiness_review_tasks.json"),
        &report["review_tasks"],
    )?;
    Ok(report)
}

fn readiness_blockers(
    job_kind: &str,
    check: Option<&WorkerDataPipelineReadinessCheck>,
) -> Vec<String> {
    let Some(check) = check else {
        return vec!["missing_customer_readiness_check".into()];
    };
    let mut blockers = Vec::new();
    if check
        .artifact_uri
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        blockers.push("missing_artifact_uri".into());
    }
    if !check.customer_approved {
        blockers.push("customer_approval_missing".into());
    }
    if job_kind == "oig_sam_sanctions_sync" && !check.external_fetch_configured {
        blockers.push("external_oig_sam_fetch_not_configured".into());
    }
    if let Some(minimum) = check.minimum_row_count {
        if check.row_count.unwrap_or(0) < minimum {
            blockers.push("row_count_below_minimum".into());
        }
    }
    if check
        .data_quality_status
        .as_deref()
        .is_some_and(|status| matches!(status, "blocked" | "failed"))
    {
        blockers.push("data_quality_status_blocked".into());
    }
    if check.evidence_refs.is_empty()
        || check
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        blockers.push("missing_evidence_refs".into());
    }
    blockers
}
