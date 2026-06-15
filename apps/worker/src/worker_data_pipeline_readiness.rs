use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

use crate::{
    api_url, contains_non_production_artifact_reference, json_string, read_json_report,
    required_non_empty,
    worker_data_pipeline_execution::{
        canonical_required_evidence_prefixes, validate_worker_data_pipeline_job_kinds,
        validate_worker_data_pipeline_plan, worker_data_pipeline_submit_job_contract,
        worker_data_pipeline_submit_job_required_flags, REPORT_VERSION,
    },
    write_json,
};

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
    pub coverage_window_days: Option<u64>,
    pub data_quality_status: Option<String>,
    pub source_freshness_status: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct WorkerDataPipelineReadinessReportSubmission {
    pub actor: String,
    pub notes: String,
    pub source_report_uri: String,
    pub report_kind: String,
    pub plan_uri: String,
    pub readiness_input_uri: String,
    pub readiness_status: String,
    pub job_count: usize,
    pub ready_job_count: usize,
    pub blocked_job_count: usize,
    pub review_task_count: usize,
    pub job_readiness: Vec<serde_json::Value>,
    pub review_tasks: Vec<serde_json::Value>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

pub fn build_worker_data_pipeline_readiness_input_template(
    plan_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let plan_uri = required_non_empty("plan_uri", plan_uri)?;
    let plan = read_json_report(plan_uri)?;
    let (customer_scope_id, jobs) = validate_worker_data_pipeline_plan(&plan)?;
    let checks = jobs
        .iter()
        .map(|job| {
            let job_kind = json_string(job, "job_kind").unwrap_or_else(|| "unknown".into());
            serde_json::json!({
                "job_kind": job_kind,
                "cadence": json_string(job, "cadence"),
                "build_command": json_string(job, "build_command"),
                "submit_command": json_string(job, "submit_command"),
                "required_submit_flags": job
                    .get("required_submit_flags")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
                "score_response_capture_command": json_string(job, "score_response_capture_command"),
                "source_input": json_string(job, "source_input"),
                "api_path": json_string(job, "api_path"),
                "required_permission": json_string(job, "required_permission"),
                "required_evidence_prefixes": job
                    .get("required_evidence_prefixes")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
                "depends_on": job
                    .get("depends_on")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
                "artifact_uri": json_string(job, "report_uri"),
                "customer_approved": false,
                "external_fetch_configured": false,
                "row_count": serde_json::Value::Null,
                "minimum_row_count": 1,
                "coverage_window_days": serde_json::Value::Null,
                "data_quality_status": "pending_customer_validation",
                "source_freshness_status": "pending_customer_validation",
                "evidence_refs": []
            })
        })
        .collect::<Vec<_>>();
    let template = serde_json::json!({
        "report_kind": "worker_data_pipeline_readiness_input_template",
        "report_version": REPORT_VERSION,
        "template_only": true,
        "plan_uri": plan_uri,
        "customer_scope_id": customer_scope_id,
        "checks": checks,
        "governance_boundary": "readiness input templates collect customer prerequisite evidence only; they must not fetch external data, submit artifacts, score claims, assign labels, activate models, or change routing policy"
    });

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create worker data pipeline readiness input template output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("worker_data_pipeline_readiness_input_template.json"),
        &template,
    )?;
    Ok(template)
}

pub fn build_worker_data_pipeline_readiness_report(
    plan_uri: &str,
    readiness_input_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    build_worker_data_pipeline_readiness_report_with_published_uris(
        plan_uri,
        readiness_input_uri,
        output_dir,
        None,
        None,
    )
}

pub fn build_worker_data_pipeline_readiness_report_with_published_uris(
    plan_uri: &str,
    readiness_input_uri: &str,
    output_dir: impl AsRef<Path>,
    published_plan_uri: Option<&str>,
    published_readiness_input_uri: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let plan_uri = required_non_empty("plan_uri", plan_uri)?;
    let readiness_input_uri = required_non_empty("readiness_input_uri", readiness_input_uri)?;
    let plan = read_json_report(plan_uri)?;
    let input: WorkerDataPipelineReadinessInput =
        serde_json::from_value(read_json_report(readiness_input_uri)?)
            .context("parse worker data pipeline readiness input")?;
    let (customer_scope_id, jobs) = validate_worker_data_pipeline_plan(&plan)?;
    if published_plan_uri.is_some() != published_readiness_input_uri.is_some() {
        bail!("published_plan_uri and published_readiness_input_uri must be supplied together");
    }
    let published_plan_uri =
        output_lineage_uri("published_plan_uri", plan_uri, published_plan_uri)?;
    let published_readiness_input_uri = output_lineage_uri(
        "published_readiness_input_uri",
        readiness_input_uri,
        published_readiness_input_uri,
    )?;
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
            let required_evidence_prefixes = required_evidence_prefixes(job);
            let blockers = readiness_blockers(&job_kind, check, &required_evidence_prefixes);
            serde_json::json!({
                "job_kind": job_kind,
                "cadence": json_string(job, "cadence"),
                "source_input": json_string(job, "source_input"),
                "api_path": json_string(job, "api_path"),
                "required_permission": json_string(job, "required_permission"),
                "submit_command": json_string(job, "submit_command"),
                "required_submit_flags": job
                    .get("required_submit_flags")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
                "required_evidence_prefixes": required_evidence_prefixes,
                "artifact_uri": check.and_then(|check| check.artifact_uri.clone()),
                "customer_approved": check.map(|check| check.customer_approved).unwrap_or(false),
                "external_fetch_configured": check
                    .map(|check| check.external_fetch_configured)
                    .unwrap_or(false),
                "row_count": check.and_then(|check| check.row_count),
                "minimum_row_count": check.and_then(|check| check.minimum_row_count),
                "coverage_window_days": check.and_then(|check| check.coverage_window_days),
                "data_quality_status": check.and_then(|check| check.data_quality_status.clone()),
                "source_freshness_status": check
                    .and_then(|check| check.source_freshness_status.clone()),
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
                "api_path": job["api_path"].clone(),
                "required_permission": job["required_permission"].clone(),
                "submit_command": job["submit_command"].clone(),
                "required_submit_flags": job["required_submit_flags"].clone(),
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
        "report_version": REPORT_VERSION,
        "plan_uri": published_plan_uri,
        "readiness_input_uri": published_readiness_input_uri,
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
            format!("worker_data_pipeline_plans:{published_plan_uri}"),
            format!("worker_data_pipeline_readiness_inputs:{published_readiness_input_uri}")
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

fn output_lineage_uri(
    field: &str,
    local_uri: &str,
    published_uri: Option<&str>,
) -> anyhow::Result<String> {
    let Some(published_uri) = published_uri else {
        return Ok(local_uri.to_string());
    };
    let published_uri = required_non_empty(field, published_uri)?;
    ensure_production_lineage_uri(field, published_uri)?;
    Ok(published_uri.into())
}

pub fn build_worker_data_pipeline_readiness_submission(
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<WorkerDataPipelineReadinessReportSubmission> {
    build_worker_data_pipeline_readiness_submission_with_published_uri(
        report_uri, actor, notes, report_uri,
    )
}

pub fn build_worker_data_pipeline_readiness_submission_with_published_uri(
    report_uri: &str,
    actor: &str,
    notes: &str,
    published_report_uri: &str,
) -> anyhow::Result<WorkerDataPipelineReadinessReportSubmission> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    let published_report_uri = required_non_empty("published_report_uri", published_report_uri)?;
    ensure_published_report_uri(
        "worker data pipeline readiness published_report_uri",
        published_report_uri,
    )?;
    let report = read_json_report(report_uri)?;
    if json_string(&report, "report_kind").as_deref()
        != Some("worker_data_pipeline_readiness_report")
    {
        bail!("report_kind must be worker_data_pipeline_readiness_report");
    }
    let job_readiness = report
        .get("job_readiness")
        .and_then(|value| value.as_array())
        .cloned()
        .context("worker data pipeline readiness report requires job_readiness")?;
    validate_worker_data_pipeline_job_kinds(&job_readiness, "job_readiness")?;
    let review_tasks = report
        .get("review_tasks")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let job_count = json_usize(&report, "job_count")?;
    let ready_job_count = json_usize(&report, "ready_job_count")?;
    let blocked_job_count = json_usize(&report, "blocked_job_count")?;
    let review_task_count = json_usize(&report, "review_task_count")?;
    let readiness_status = json_string(&report, "readiness_status")
        .context("worker data pipeline readiness report requires readiness_status")?;
    validate_worker_data_pipeline_readiness_counts(
        &job_readiness,
        &review_tasks,
        &readiness_status,
        job_count,
        ready_job_count,
        blocked_job_count,
        review_task_count,
    )?;
    validate_worker_data_pipeline_readiness_review_tasks(&job_readiness, &review_tasks)?;
    let mut evidence_refs = report
        .get("evidence_refs")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let plan_uri = json_string(&report, "plan_uri")
        .context("worker data pipeline readiness report requires plan_uri")?;
    let readiness_input_uri = json_string(&report, "readiness_input_uri")
        .context("worker data pipeline readiness report requires readiness_input_uri")?;
    ensure_production_lineage_uri("plan_uri", &plan_uri)?;
    ensure_production_lineage_uri("readiness_input_uri", &readiness_input_uri)?;
    ensure_production_evidence_refs(&evidence_refs)?;
    for required_ref in [
        format!("worker_data_pipeline_plans:{plan_uri}"),
        format!("worker_data_pipeline_readiness_inputs:{readiness_input_uri}"),
    ] {
        if !evidence_refs
            .iter()
            .any(|reference| reference.trim() == required_ref)
        {
            bail!("worker data pipeline readiness report requires {required_ref} evidence");
        }
    }
    validate_worker_data_pipeline_readiness_job_evidence(&job_readiness)?;
    evidence_refs.push(format!(
        "worker_data_pipeline_readiness_reports:{published_report_uri}"
    ));
    Ok(WorkerDataPipelineReadinessReportSubmission {
        actor: actor.into(),
        notes: notes.into(),
        source_report_uri: published_report_uri.into(),
        report_kind: "worker_data_pipeline_readiness_report".into(),
        plan_uri,
        readiness_input_uri,
        readiness_status,
        job_count,
        ready_job_count,
        blocked_job_count,
        review_task_count,
        job_readiness,
        review_tasks,
        evidence_refs,
        governance_boundary: json_string(&report, "governance_boundary")
            .context("worker data pipeline readiness report requires governance_boundary")?,
    })
}

fn validate_worker_data_pipeline_readiness_review_tasks(
    job_readiness: &[serde_json::Value],
    review_tasks: &[serde_json::Value],
) -> anyhow::Result<()> {
    for job in job_readiness {
        let job_kind = json_string(job, "job_kind").context("job_readiness requires job_kind")?;
        let readiness_status = json_string(job, "readiness_status")
            .context("job_readiness requires readiness_status")?;
        if readiness_status != "blocked" {
            continue;
        }
        let has_matching_review = review_tasks.iter().any(|task| {
            json_string(task, "task_kind").as_deref()
                == Some("worker_data_pipeline_readiness_review")
                && json_string(task, "job_kind").as_deref() == Some(job_kind.as_str())
        });
        if !has_matching_review {
            bail!("blocked job requires matching worker_data_pipeline_readiness_review task");
        }
    }
    for task in review_tasks {
        if json_string(task, "task_kind").as_deref()
            != Some("worker_data_pipeline_readiness_review")
        {
            bail!("readiness review task kind must be worker_data_pipeline_readiness_review");
        }
        let job_kind = json_string(task, "job_kind")
            .context("worker_data_pipeline_readiness_review requires job_kind")?;
        validate_worker_data_pipeline_submit_contract_fields(&job_kind, task)?;
        let matches_blocked_job = job_readiness.iter().any(|job| {
            json_string(job, "job_kind").as_deref() == Some(job_kind.as_str())
                && json_string(job, "readiness_status").as_deref() == Some("blocked")
        });
        if !matches_blocked_job {
            bail!("worker_data_pipeline_readiness_review task must match a blocked job");
        }
    }
    Ok(())
}

fn validate_worker_data_pipeline_readiness_counts(
    job_readiness: &[serde_json::Value],
    review_tasks: &[serde_json::Value],
    readiness_status: &str,
    job_count: usize,
    ready_job_count: usize,
    blocked_job_count: usize,
    review_task_count: usize,
) -> anyhow::Result<()> {
    if job_count != job_readiness.len() {
        bail!("job_count must match job_readiness length");
    }
    let mut actual_ready = 0usize;
    let mut actual_blocked = 0usize;
    for job in job_readiness {
        match json_string(job, "readiness_status").as_deref() {
            Some("ready") => actual_ready += 1,
            Some("blocked") => actual_blocked += 1,
            _ => bail!("job_readiness readiness_status must be ready or blocked"),
        }
    }
    if ready_job_count != actual_ready || blocked_job_count != actual_blocked {
        bail!("ready_job_count and blocked_job_count must match job_readiness statuses");
    }
    match (readiness_status, actual_blocked) {
        ("ready", 0) => {}
        ("blocked", blocked) if blocked > 0 => {}
        _ => bail!("readiness_status must match whether any job is blocked"),
    }
    if review_task_count != review_tasks.len() {
        bail!("review_task_count must match review_tasks length");
    }
    Ok(())
}

fn validate_worker_data_pipeline_readiness_job_evidence(
    job_readiness: &[serde_json::Value],
) -> anyhow::Result<()> {
    for job in job_readiness {
        let job_kind = json_string(job, "job_kind").context("job_readiness requires job_kind")?;
        match json_string(job, "readiness_status").as_deref() {
            Some("ready") => validate_ready_worker_data_pipeline_job(&job_kind, job)?,
            Some("blocked") => {
                let has_blockers = job
                    .get("blockers")
                    .and_then(|value| value.as_array())
                    .is_some_and(|blockers| {
                        !blockers.is_empty()
                            && blockers.iter().all(|blocker| {
                                blocker
                                    .as_str()
                                    .is_some_and(|value| !value.trim().is_empty())
                            })
                    });
                if !has_blockers {
                    bail!("blocked job readiness records require non-empty blockers");
                }
            }
            _ => bail!("job_readiness readiness_status must be ready or blocked"),
        }
    }
    Ok(())
}

fn validate_ready_worker_data_pipeline_job(
    job_kind: &str,
    job: &serde_json::Value,
) -> anyhow::Result<()> {
    if job
        .get("blockers")
        .is_some_and(|value| value.as_array().is_none_or(|blockers| !blockers.is_empty()))
    {
        bail!("ready job readiness records must not include blockers");
    }
    if job
        .get("coverage_window_days")
        .and_then(|value| value.as_u64())
        .unwrap_or(0)
        == 0
    {
        bail!("ready job readiness records require positive coverage_window_days");
    }
    if job
        .get("source_freshness_status")
        .and_then(|value| value.as_str())
        != Some("fresh")
    {
        bail!("ready job readiness records require source_freshness_status fresh");
    }
    if !json_string(job, "artifact_uri").is_some_and(|value| is_production_artifact_uri(&value)) {
        bail!(
            "ready job readiness records require a production artifact_uri, not a local dry-run or placeholder URI"
        );
    }
    let evidence_refs = job
        .get("evidence_refs")
        .and_then(|value| value.as_array())
        .context("ready job readiness records require non-empty evidence_refs")?;
    if evidence_refs.is_empty()
        || evidence_refs.iter().any(|reference| {
            reference
                .as_str()
                .is_none_or(|value| value.trim().is_empty())
        })
    {
        bail!("ready job readiness records require non-empty evidence_refs");
    }
    if evidence_refs.iter().any(|reference| {
        reference
            .as_str()
            .is_some_and(evidence_ref_is_non_production)
    }) {
        bail!("ready job evidence_refs must not use local dry-run or placeholder evidence");
    }
    let required_evidence_prefixes = job
        .get("required_evidence_prefixes")
        .and_then(|value| value.as_array())
        .context("ready job readiness records require non-empty required_evidence_prefixes")?;
    if required_evidence_prefixes.is_empty()
        || required_evidence_prefixes
            .iter()
            .any(|prefix| prefix.as_str().is_none_or(|value| value.trim().is_empty()))
    {
        bail!("ready job readiness records require non-empty required_evidence_prefixes");
    }
    let Some(canonical_prefixes) = canonical_required_evidence_prefixes(job_kind) else {
        return Ok(());
    };
    for prefix in canonical_prefixes {
        if !required_evidence_prefixes
            .iter()
            .any(|value| value.as_str() == Some(prefix))
        {
            bail!("{job_kind} required_evidence_prefixes must include {prefix}");
        }
        if !evidence_refs.iter().any(|value| {
            value
                .as_str()
                .is_some_and(|reference| reference.starts_with(prefix))
        }) {
            bail!("ready job evidence_refs must include required prefix {prefix}");
        }
    }
    validate_worker_data_pipeline_submit_contract_fields(job_kind, job)?;
    Ok(())
}

fn validate_worker_data_pipeline_submit_contract_fields(
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

pub async fn submit_worker_data_pipeline_readiness_report(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    submit_worker_data_pipeline_readiness_report_with_published_uri(
        api_base_url,
        api_key,
        report_uri,
        actor,
        notes,
        report_uri,
    )
    .await
}

pub async fn submit_worker_data_pipeline_readiness_report_with_published_uri(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    actor: &str,
    notes: &str,
    published_report_uri: &str,
) -> anyhow::Result<serde_json::Value> {
    let payload = build_worker_data_pipeline_readiness_submission_with_published_uri(
        report_uri,
        actor,
        notes,
        published_report_uri,
    )?;
    let client = reqwest::Client::new();
    let response = client
        .post(api_url(
            api_base_url,
            "/api/v1/ops/worker-data-pipeline-readiness",
        ))
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .context("submit worker data pipeline readiness report")?;
    let status = response.status();
    let body = response.text().await.context("read submit response")?;
    if !status.is_success() {
        bail!("submit worker data pipeline readiness report failed with {status}: {body}");
    }
    serde_json::from_str(&body).context("parse worker data pipeline readiness response")
}

fn readiness_blockers(
    job_kind: &str,
    check: Option<&WorkerDataPipelineReadinessCheck>,
    required_evidence_prefixes: &[String],
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
    if check
        .artifact_uri
        .as_deref()
        .is_some_and(|value| value.trim().starts_with("local://template"))
    {
        blockers.push("template_artifact_uri_not_replaced".into());
    } else if check
        .artifact_uri
        .as_deref()
        .is_some_and(|value| !is_production_artifact_uri(value))
    {
        blockers.push("non_production_artifact_uri".into());
    }
    if !check.customer_approved {
        blockers.push("customer_approval_missing".into());
    }
    if job_kind == "oig_sam_sanctions_snapshot_fetch" && !check.external_fetch_configured {
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
    if check.coverage_window_days.unwrap_or(0) == 0 {
        blockers.push("missing_coverage_window".into());
    }
    if check.source_freshness_status.as_deref() != Some("fresh") {
        blockers.push("source_freshness_not_confirmed".into());
    }
    if check.evidence_refs.is_empty()
        || check
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        blockers.push("missing_evidence_refs".into());
    }
    if check
        .evidence_refs
        .iter()
        .any(|reference| evidence_ref_is_non_production(reference))
    {
        blockers.push("non_production_evidence_refs".into());
    }
    if required_evidence_prefixes
        .iter()
        .any(|prefix| prefix.trim().is_empty())
    {
        blockers.push("blank_required_evidence_prefixes".into());
    } else if required_evidence_prefixes.iter().any(|prefix| {
        !check
            .evidence_refs
            .iter()
            .any(|reference| reference.starts_with(prefix))
    }) {
        blockers.push("missing_required_evidence_prefixes".into());
    }
    blockers
}

fn required_evidence_prefixes(job: &serde_json::Value) -> Vec<String> {
    job.get("required_evidence_prefixes")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .map(str::to_string)
        .collect()
}

fn is_production_artifact_uri(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && value.contains("://") && !contains_non_production_artifact_reference(value)
}

fn evidence_ref_is_non_production(value: &str) -> bool {
    contains_non_production_artifact_reference(value)
}

fn ensure_production_lineage_uri(field: &str, value: &str) -> anyhow::Result<()> {
    if !is_production_artifact_uri(value) {
        bail!("{field} must use production evidence, not local dry-run or placeholder URI");
    }
    Ok(())
}

fn ensure_published_report_uri(field: &str, value: &str) -> anyhow::Result<()> {
    let value = value.trim();
    if value.is_empty()
        || !value.contains("://")
        || contains_non_production_artifact_reference(value)
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
        bail!("worker data pipeline readiness evidence_refs must not use local dry-run or placeholder evidence");
    }
    Ok(())
}

fn json_usize(value: &serde_json::Value, key: &'static str) -> anyhow::Result<usize> {
    value
        .get(key)
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .with_context(|| format!("worker data pipeline readiness report requires {key}"))
}
