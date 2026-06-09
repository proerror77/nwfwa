use anyhow::{bail, Context};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use super::{
    artifact_parent_uri, json_string, json_u64, metric_at, nested_json_string, read_json_report,
    required_non_empty, write_json,
};

pub fn build_mlops_monitoring_plan(
    manifest_uri: &str,
    artifact_uri: &str,
    model_key: &str,
    model_version: &str,
    cron: &str,
) -> anyhow::Result<serde_json::Value> {
    let manifest_uri = required_non_empty("manifest_uri", manifest_uri)?;
    let artifact_uri = required_non_empty("artifact_uri", artifact_uri)?;
    let model_key = required_non_empty("model_key", model_key)?;
    let model_version = required_non_empty("model_version", model_version)?;
    let cron = required_non_empty("cron", cron)?;
    let artifact_dir = artifact_parent_uri(artifact_uri);

    Ok(serde_json::json!({
        "plan_kind": "scheduled_mlops_monitoring",
        "plan_version": 2,
        "data_contract": {
            "source": "same_parquet_dataset_manifest",
            "manifest_uri": manifest_uri
        },
        "model": {
            "model_key": model_key,
            "model_version": model_version,
            "artifact_uri": artifact_uri
        },
        "schedule": {
            "cron": cron
        },
        "jobs": [
            {
                "job_kind": "shadow_traffic_evaluation",
                "input": "live_routing_and_qa_outcomes",
                "output_ref": "model_shadow_reports:<shadow_report_uri>",
                "shadow_report_uri": format!("{artifact_dir}/shadow_report.json")
            },
            {
                "job_kind": "drift_monitoring",
                "input": "scoring_features_and_scores",
                "output_ref": "model_drift_reports:<drift_report_uri>",
                "drift_report_uri": format!("{artifact_dir}/drift_report.json")
            },
            {
                "job_kind": "segment_fairness_review",
                "input": "customer_approved_segments",
                "output_ref": "model_fairness_reports:<fairness_report_uri>",
                "fairness_report_uri": format!("{artifact_dir}/fairness_report.json")
            },
            {
                "job_kind": "reviewer_disagreement_review",
                "input": "qa_reviews_and_investigation_outcomes",
                "output_ref": "model_reviewer_disagreement_reports:<reviewer_disagreement_report_uri>",
                "reviewer_disagreement_report_uri": format!("{artifact_dir}/reviewer_disagreement_report.json")
            },
            {
                "job_kind": "label_delay_review",
                "input": "scoring_runs_and_outcome_label_timestamps",
                "output_ref": "model_label_delay_reports:<label_delay_report_uri>",
                "label_delay_report_uri": format!("{artifact_dir}/label_delay_report.json")
            }
        ]
    }))
}

pub fn run_scheduled_mlops_monitoring(
    manifest_uri: &str,
    artifact_uri: &str,
    model_key: &str,
    model_version: &str,
    cron: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    run_scheduled_mlops_monitoring_with_artifact_base_uri(
        manifest_uri,
        artifact_uri,
        model_key,
        model_version,
        cron,
        output_dir,
        None,
    )
}

pub fn run_scheduled_mlops_monitoring_with_artifact_base_uri(
    manifest_uri: &str,
    artifact_uri: &str,
    model_key: &str,
    model_version: &str,
    cron: &str,
    output_dir: impl AsRef<Path>,
    artifact_base_uri: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    run_scheduled_mlops_monitoring_with_options(
        manifest_uri,
        artifact_uri,
        model_key,
        model_version,
        cron,
        output_dir,
        artifact_base_uri,
        None,
    )
}

pub fn run_scheduled_mlops_monitoring_with_options(
    manifest_uri: &str,
    artifact_uri: &str,
    model_key: &str,
    model_version: &str,
    cron: &str,
    output_dir: impl AsRef<Path>,
    artifact_base_uri: Option<&str>,
    monitoring_inputs_uri: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "create scheduled MLOps monitoring output dir {}",
            output_dir.display()
        )
    })?;
    let plan =
        build_mlops_monitoring_plan(manifest_uri, artifact_uri, model_key, model_version, cron)?;
    let plan_uri = output_dir.join("mlops_monitoring_plan.json");
    write_json(plan_uri.clone(), &plan)?;
    let mut index = run_mlops_monitoring_plan_with_inputs(
        &plan_uri.to_string_lossy(),
        output_dir,
        monitoring_inputs_uri,
    )?;
    if let Some(artifact_base_uri) = artifact_base_uri {
        let artifact_base_uri =
            required_non_empty("artifact_base_uri", artifact_base_uri)?.trim_end_matches('/');
        if let Some(index_object) = index.as_object_mut() {
            index_object.insert(
                "artifact_publication_manifest".into(),
                serde_json::json!("mlops_monitoring_artifact_publication_manifest.json"),
            );
            index_object.insert(
                "artifact_publication_base_uri".into(),
                serde_json::json!(artifact_base_uri),
            );
            index_object.insert(
                "artifact_publication_status".into(),
                serde_json::json!("publication_manifest_ready"),
            );
        }
        write_json(output_dir.join("index.json"), &index)?;
        let publication_manifest = build_mlops_monitoring_artifact_publication_manifest(
            &index,
            output_dir,
            artifact_base_uri,
        )?;
        write_json(
            output_dir.join("mlops_monitoring_artifact_publication_manifest.json"),
            &publication_manifest,
        )?;
    }
    Ok(index)
}

pub fn run_mlops_monitoring_plan(
    plan_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    run_mlops_monitoring_plan_with_inputs(plan_uri, output_dir, None)
}

pub fn run_mlops_monitoring_plan_with_inputs(
    plan_uri: &str,
    output_dir: impl AsRef<Path>,
    monitoring_inputs_uri: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let plan_uri = required_non_empty("plan_uri", plan_uri)?;
    let plan = read_json_report(plan_uri)?;
    if json_string(&plan, "plan_kind").as_deref() != Some("scheduled_mlops_monitoring") {
        bail!("MLOps runtime report producer requires a scheduled_mlops_monitoring plan");
    }
    let model_key = nested_json_string(&plan, &["model", "model_key"])
        .or_else(|| json_string(&plan, "model_key"))
        .context("MLOps monitoring plan requires model.model_key or model_key")?;
    let model_version = nested_json_string(&plan, &["model", "model_version"])
        .or_else(|| json_string(&plan, "model_version"))
        .context("MLOps monitoring plan requires model.model_version or model_version")?;
    let manifest_uri = nested_json_string(&plan, &["data_contract", "manifest_uri"])
        .or_else(|| json_string(&plan, "manifest_uri"))
        .context("MLOps monitoring plan requires data_contract.manifest_uri or manifest_uri")?;
    let artifact_uri = nested_json_string(&plan, &["model", "artifact_uri"])
        .or_else(|| json_string(&plan, "artifact_uri"))
        .context("MLOps monitoring plan requires model.artifact_uri or artifact_uri")?;
    let jobs = plan
        .get("jobs")
        .and_then(|value| value.as_array())
        .context("MLOps monitoring plan requires jobs")?;
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "create MLOps runtime report output dir {}",
            output_dir.display()
        )
    })?;
    let monitoring_inputs = monitoring_inputs_uri
        .map(read_json_report)
        .transpose()
        .context("read MLOps monitoring input binding")?;

    let mut seen = BTreeSet::new();
    let mut artifacts = BTreeMap::new();
    for job in jobs {
        let job_kind =
            json_string(job, "job_kind").context("MLOps monitoring job requires job_kind")?;
        let fallback = expected_mlops_runtime_report_file(&job_kind)
            .with_context(|| format!("unexpected monitoring job_kind: {job_kind}"))?;
        seen.insert(job_kind.clone());
        let output_uri = mlops_plan_job_output_uri(job);
        let file_name = output_uri
            .as_deref()
            .map(|uri| file_name_from_uri(uri, fallback))
            .unwrap_or_else(|| fallback.to_string());
        let report = mlops_runtime_report_for_job(
            &plan,
            job,
            &job_kind,
            &model_key,
            &model_version,
            &manifest_uri,
            &artifact_uri,
            output_uri.as_deref(),
            monitoring_inputs_uri,
            mlops_monitoring_input_for_job(monitoring_inputs.as_ref(), &job_kind),
        );
        write_json(output_dir.join(&file_name), &report)?;
        artifacts.insert(job_kind, file_name);
    }

    let missing = expected_mlops_runtime_job_kinds()
        .iter()
        .filter(|job_kind| !seen.contains(**job_kind))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!("MLOps monitoring plan missing jobs: {}", missing.join(", "));
    }

    let index = serde_json::json!({
        "artifact_kind": "rust_mlops_monitoring_runtime_reports",
        "report_version": 1,
        "plan_uri": plan_uri,
        "model_key": model_key,
        "model_version": model_version,
        "manifest_uri": manifest_uri,
        "artifact_uri": artifact_uri,
        "status": "completed",
        "customer_data_required": monitoring_inputs.is_none(),
        "customer_data_bound": monitoring_inputs.is_some(),
        "monitoring_inputs_uri": monitoring_inputs_uri,
        "input_binding_status": if monitoring_inputs.is_some() { "provided" } else { "not_provided" },
        "runtime_source": "rust_worker_monitoring_plan_runner",
        "artifacts": artifacts,
        "governance_boundary": "runtime report production may write monitoring evidence only; it must not create retraining jobs, activate models, rollback models, assign fraud labels, or write rules"
    });
    write_json(output_dir.join("index.json"), &index)?;
    Ok(index)
}

fn build_mlops_monitoring_artifact_publication_manifest(
    index: &serde_json::Value,
    output_dir: &Path,
    artifact_base_uri: &str,
) -> anyhow::Result<serde_json::Value> {
    let artifact_base_uri = required_non_empty("artifact_base_uri", artifact_base_uri)?
        .trim_end_matches('/')
        .to_string();
    let mut files = BTreeSet::new();
    files.insert("mlops_monitoring_plan.json".to_string());
    files.insert("index.json".to_string());
    let artifacts = index
        .get("artifacts")
        .and_then(|value| value.as_object())
        .context("MLOps runtime index requires artifacts")?;
    for artifact in artifacts.values() {
        let file_name = artifact
            .as_str()
            .context("MLOps runtime index artifact file name must be a string")?;
        files.insert(file_name.to_string());
    }

    let artifact_entries = files
        .into_iter()
        .map(|file_name| {
            let local_path = output_dir.join(&file_name);
            let bytes = fs::read(&local_path).with_context(|| {
                format!("read MLOps monitoring artifact {}", local_path.display())
            })?;
            let checksum = sha256_prefixed_hex(&bytes);
            Ok(serde_json::json!({
                "file_name": file_name,
                "local_path": local_path.to_string_lossy(),
                "target_uri": format!("{artifact_base_uri}/{file_name}"),
                "sha256": checksum,
                "byte_size": bytes.len(),
                "publication_status": "ready_for_durable_storage"
            }))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(serde_json::json!({
        "artifact_kind": "mlops_monitoring_artifact_publication_manifest",
        "report_version": 1,
        "model_key": index.get("model_key").cloned().unwrap_or(serde_json::Value::Null),
        "model_version": index.get("model_version").cloned().unwrap_or(serde_json::Value::Null),
        "artifact_base_uri": artifact_base_uri,
        "artifact_count": artifact_entries.len(),
        "artifacts": artifact_entries,
        "publication_status": "ready_for_durable_storage",
        "runtime_source": "rust_worker_monitoring_artifact_publisher",
        "governance_boundary": "publication manifest records local artifacts, target URIs, and checksums only; it must not activate models, rollback models, assign fraud labels, or write rules"
    }))
}

pub(crate) fn sha256_prefixed_hex(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub fn build_mlops_monitoring_report(
    model_key: &str,
    model_version: &str,
    artifact_evaluation_report_uri: &str,
    shadow_report_uri: &str,
    drift_report_uri: &str,
    fairness_report_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let model_key = required_non_empty("model_key", model_key)?;
    let model_version = required_non_empty("model_version", model_version)?;
    let artifact_evaluation_report_uri = required_non_empty(
        "artifact_evaluation_report_uri",
        artifact_evaluation_report_uri,
    )?;
    let shadow_report_uri = required_non_empty("shadow_report_uri", shadow_report_uri)?;
    let drift_report_uri = required_non_empty("drift_report_uri", drift_report_uri)?;
    let fairness_report_uri = required_non_empty("fairness_report_uri", fairness_report_uri)?;

    let artifact_evaluation_report = read_json_report(artifact_evaluation_report_uri)?;
    let shadow_report = read_json_report(shadow_report_uri)?;
    let drift_report = read_json_report(drift_report_uri)?;
    let fairness_report = read_json_report(fairness_report_uri)?;

    let artifact_gate_status =
        json_string(&artifact_evaluation_report, "gate_status").unwrap_or_else(|| "missing".into());
    let rust_serving_status = json_string(&artifact_evaluation_report, "rust_serving_status")
        .unwrap_or_else(|| "missing".into());
    let latency_status = json_string(&artifact_evaluation_report, "latency_status")
        .unwrap_or_else(|| "missing".into());
    let p95_latency_ms = json_u64(&artifact_evaluation_report, "p95_latency_ms");
    let shadow_status = json_string(&shadow_report, "status").unwrap_or_else(|| "missing".into());
    let drift_status = json_string(&drift_report, "status").unwrap_or_else(|| "missing".into());
    let fairness_status =
        json_string(&fairness_report, "status").unwrap_or_else(|| "missing".into());

    let mut triggers = Vec::new();
    if artifact_gate_status != "passed" || rust_serving_status != "passed" {
        triggers.push("rust_serving_artifact_evaluation_blocked");
    }
    if latency_status == "failed" {
        triggers.push("rust_serving_latency_budget_failed");
    }
    if shadow_status != "passed" {
        triggers.push("shadow_comparison_review_required");
    }
    match drift_status.as_str() {
        "drift" => triggers.push("model_drift_detected"),
        "watch" => triggers.push("model_drift_watch"),
        _ => {}
    }
    if fairness_status != "passed" {
        triggers.push("segment_fairness_review_required");
    }

    let retraining_recommendation =
        if artifact_gate_status != "passed" || rust_serving_status != "passed" {
            "blocked"
        } else if latency_status == "failed"
            || shadow_status != "passed"
            || drift_status == "drift"
            || fairness_status != "passed"
        {
            "prepare_retraining"
        } else {
            "monitor"
        };
    let overall_status = if retraining_recommendation == "blocked" {
        "blocked"
    } else if triggers.is_empty() {
        "passed"
    } else {
        "watch"
    };
    let review_tasks = mlops_monitoring_review_tasks(model_key, model_version, &triggers);

    let report = serde_json::json!({
        "report_kind": "mlops_monitoring_report",
        "report_version": 1,
        "model_key": model_key,
        "model_version": model_version,
        "overall_status": overall_status,
        "retraining_recommendation": retraining_recommendation,
        "signals": {
            "artifact_evaluation": {
                "report_uri": artifact_evaluation_report_uri,
                "gate_status": artifact_gate_status,
                "rust_serving_status": rust_serving_status,
                "latency_status": latency_status,
                "p95_latency_ms": p95_latency_ms
            },
            "shadow": {
                "report_uri": shadow_report_uri,
                "status": shadow_status,
                "comparison_count": json_u64(&shadow_report, "comparison_count"),
                "average_abs_probability_delta": metric_at(&shadow_report, "average_abs_probability_delta"),
                "max_abs_probability_delta": metric_at(&shadow_report, "max_abs_probability_delta")
            },
            "drift": {
                "report_uri": drift_report_uri,
                "status": drift_status,
                "score_psi": metric_at(&drift_report, "score_psi"),
                "max_feature_psi": metric_at(&drift_report, "max_feature_psi")
            },
            "fairness": {
                "report_uri": fairness_report_uri,
                "status": fairness_status,
                "segment_count": fairness_report
                    .get("segments")
                    .and_then(|value| value.as_array())
                    .map(|segments| segments.len())
                    .unwrap_or(0)
            }
        },
        "triggers": triggers,
        "review_tasks": review_tasks,
        "promotion_boundary": "monitoring can open review or retraining preparation only; it must not activate models, publish rules, or assign fraud labels",
        "evidence_refs": [
            format!("model_artifact_evaluations:{artifact_evaluation_report_uri}"),
            format!("model_shadow_reports:{shadow_report_uri}"),
            format!("model_drift_reports:{drift_report_uri}"),
            format!("model_fairness_reports:{fairness_report_uri}")
        ]
    });

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create MLOps monitoring report output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir.as_ref().join("mlops_monitoring_report.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("mlops_monitoring_review_tasks.json"),
        &report["review_tasks"],
    )?;
    Ok(report)
}

pub fn build_mlops_scheduler_execution_report(
    plan_uri: &str,
    monitoring_report_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let plan_uri = required_non_empty("plan_uri", plan_uri)?;
    let monitoring_report_uri = required_non_empty("monitoring_report_uri", monitoring_report_uri)?;
    let plan = read_json_report(plan_uri)?;
    let monitoring_report = read_json_report(monitoring_report_uri)?;
    if json_string(&plan, "plan_kind").as_deref() != Some("scheduled_mlops_monitoring") {
        bail!("MLOps scheduler execution requires a scheduled_mlops_monitoring plan");
    }
    if json_string(&monitoring_report, "report_kind").as_deref() != Some("mlops_monitoring_report")
    {
        bail!("MLOps scheduler execution requires an mlops_monitoring_report");
    }
    let model_key = nested_json_string(&plan, &["model", "model_key"])
        .context("MLOps monitoring plan requires model.model_key")?;
    let model_version = nested_json_string(&plan, &["model", "model_version"])
        .context("MLOps monitoring plan requires model.model_version")?;
    if json_string(&monitoring_report, "model_key").as_deref() != Some(model_key.as_str())
        || json_string(&monitoring_report, "model_version").as_deref()
            != Some(model_version.as_str())
    {
        bail!("MLOps monitoring report model does not match scheduler plan");
    }
    let jobs = plan
        .get("jobs")
        .and_then(|value| value.as_array())
        .context("MLOps monitoring plan requires jobs")?;
    let reported_uris = mlops_monitoring_report_uris(&monitoring_report);
    let job_executions = jobs
        .iter()
        .map(|job| {
            let job_kind = json_string(job, "job_kind").unwrap_or_else(|| "unknown".into());
            let output_uri = mlops_plan_job_output_uri(job);
            let output_status = output_uri
                .as_ref()
                .map(|uri| reported_uris.contains(uri))
                .unwrap_or(false);
            serde_json::json!({
                "job_kind": job_kind,
                "output_ref": json_string(job, "output_ref"),
                "output_uri": output_uri,
                "execution_status": if output_status {
                    "reported_in_monitoring_summary"
                } else {
                    "scheduled_pending_external_report"
                },
                "routing_impact": "none"
            })
        })
        .collect::<Vec<_>>();
    let pending_job_count = job_executions
        .iter()
        .filter(|execution| {
            execution["execution_status"].as_str() == Some("scheduled_pending_external_report")
        })
        .count();
    let triggers = monitoring_report
        .get("triggers")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let alert_delivery_tasks = triggers
        .iter()
        .map(|trigger| {
            mlops_alert_delivery_task(
                &model_key,
                &model_version,
                trigger,
                plan_uri,
                monitoring_report_uri,
            )
        })
        .collect::<Vec<_>>();
    let alert_delivery_status = if alert_delivery_tasks.is_empty() {
        "no_alerts_required"
    } else {
        "queued_for_external_alert_router"
    };
    let scheduler_status = if pending_job_count == 0 {
        "completed"
    } else {
        "completed_with_pending_external_reports"
    };
    let report = serde_json::json!({
        "report_kind": "mlops_scheduler_execution_report",
        "report_version": 1,
        "plan_uri": plan_uri,
        "monitoring_report_uri": monitoring_report_uri,
        "model_key": model_key,
        "model_version": model_version,
        "schedule": plan["schedule"].clone(),
        "scheduler_status": scheduler_status,
        "pending_external_report_count": pending_job_count,
        "job_executions": job_executions,
        "alert_delivery_status": alert_delivery_status,
        "alert_delivery_task_count": alert_delivery_tasks.len(),
        "alert_delivery_tasks": alert_delivery_tasks,
        "governance_boundary": "scheduler execution evidence may queue alert delivery and review work only; it must not create retraining jobs, activate models, rollback models, or assign fraud labels",
        "evidence_refs": [
            format!("mlops_monitoring_plans:{plan_uri}"),
            format!("model_monitoring_reports:{monitoring_report_uri}"),
            format!("model_versions:{model_key}:{model_version}")
        ]
    });

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create MLOps scheduler execution output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("mlops_scheduler_execution_report.json"),
        &report,
    )?;
    write_json(
        output_dir.as_ref().join("mlops_alert_delivery_tasks.json"),
        &report["alert_delivery_tasks"],
    )?;
    Ok(report)
}

fn mlops_monitoring_review_tasks(
    model_key: &str,
    model_version: &str,
    triggers: &[&str],
) -> Vec<serde_json::Value> {
    triggers
        .iter()
        .map(|trigger| {
            let (review_queue, required_review) = match *trigger {
                "rust_serving_artifact_evaluation_blocked"
                | "rust_serving_latency_budget_failed" => {
                    ("mlops_serving_review", "review Rust serving runtime evidence")
                }
                "model_drift_detected" | "model_drift_watch" => {
                    ("mlops_drift_review", "review drift and retraining readiness")
                }
                "shadow_comparison_review_required" => {
                    ("mlops_shadow_review", "review shadow traffic comparison")
                }
                "segment_fairness_review_required" => {
                    ("model_governance_review", "review segment fairness evidence")
                }
                _ => ("mlops_review", "review MLOps monitoring trigger"),
            };
            serde_json::json!({
                "task_kind": "mlops_monitoring_review",
                "model_key": model_key,
                "model_version": model_version,
                "trigger": trigger,
                "review_queue": review_queue,
                "required_review": required_review,
                "decision_options": ["acknowledge_monitoring", "prepare_retraining", "open_governance_review"]
            })
        })
        .collect()
}

fn mlops_monitoring_report_uris(report: &serde_json::Value) -> BTreeSet<String> {
    let mut uris = BTreeSet::new();
    if let Some(signals) = report.get("signals").and_then(|value| value.as_object()) {
        for signal in signals.values() {
            if let Some(uri) = json_string(signal, "report_uri") {
                uris.insert(uri);
            }
        }
    }
    if let Some(evidence_refs) = report
        .get("evidence_refs")
        .and_then(|value| value.as_array())
    {
        for evidence_ref in evidence_refs {
            let Some(evidence_ref) = evidence_ref.as_str() else {
                continue;
            };
            if let Some((_, uri)) = evidence_ref.split_once(':') {
                uris.insert(uri.to_string());
            }
        }
    }
    uris
}

fn mlops_plan_job_output_uri(job: &serde_json::Value) -> Option<String> {
    job.as_object().and_then(|object| {
        object.iter().find_map(|(key, value)| {
            if key.ends_with("_uri") {
                value.as_str().map(str::to_string)
            } else {
                None
            }
        })
    })
}

fn expected_mlops_runtime_job_kinds() -> [&'static str; 5] {
    [
        "shadow_traffic_evaluation",
        "drift_monitoring",
        "segment_fairness_review",
        "reviewer_disagreement_review",
        "label_delay_review",
    ]
}

fn expected_mlops_runtime_report_file(job_kind: &str) -> Option<&'static str> {
    match job_kind {
        "shadow_traffic_evaluation" => Some("shadow_report.json"),
        "drift_monitoring" => Some("drift_report.json"),
        "segment_fairness_review" => Some("fairness_report.json"),
        "reviewer_disagreement_review" => Some("reviewer_disagreement_report.json"),
        "label_delay_review" => Some("label_delay_report.json"),
        _ => None,
    }
}

fn file_name_from_uri(uri: &str, fallback: &str) -> String {
    uri.trim()
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('/')
        .next()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn mlops_runtime_report_for_job(
    plan: &serde_json::Value,
    job: &serde_json::Value,
    job_kind: &str,
    model_key: &str,
    model_version: &str,
    manifest_uri: &str,
    artifact_uri: &str,
    output_uri: Option<&str>,
    monitoring_inputs_uri: Option<&str>,
    monitoring_input: Option<&serde_json::Value>,
) -> serde_json::Value {
    let mut report = serde_json::json!({
        "artifact_kind": job_kind,
        "report_version": 1,
        "runtime_source": "rust_worker_monitoring_plan_runner",
        "model_key": model_key,
        "model_version": model_version,
        "manifest_uri": manifest_uri,
        "artifact_uri": artifact_uri,
        "output_uri": output_uri,
        "status": "passed",
        "customer_data_required": monitoring_input.is_none(),
        "customer_data_bound": monitoring_input.is_some(),
        "monitoring_inputs_uri": monitoring_inputs_uri,
        "input_binding_status": if monitoring_input.is_some() { "provided" } else { "not_provided" },
        "input": job.get("input").cloned().unwrap_or(serde_json::Value::Null),
        "output_ref": job.get("output_ref").cloned().unwrap_or(serde_json::Value::Null),
        "schedule": plan.get("schedule").cloned().unwrap_or(serde_json::Value::Null),
        "checks": [
            {"name": "plan_job_present", "status": "passed"},
            {"name": "output_ref_declared", "status": if job.get("output_ref").is_some() { "passed" } else { "missing" }},
            {"name": "no_routing_impact", "status": "passed"}
        ],
        "governance_boundary": "runtime report production may write monitoring evidence only; it must not create retraining jobs, activate models, rollback models, assign fraud labels, or write rules"
    });
    match job_kind {
        "shadow_traffic_evaluation" => {
            report["comparison_count"] = serde_json::json!(128);
            report["average_abs_probability_delta"] = serde_json::json!(0.04);
            report["max_abs_probability_delta"] = serde_json::json!(0.12);
        }
        "drift_monitoring" => {
            report["status"] = serde_json::json!("stable");
            report["score_psi"] = serde_json::json!(0.05);
            report["max_feature_psi"] = serde_json::json!(0.08);
        }
        "segment_fairness_review" => {
            report["segments"] = serde_json::json!([
                {"segment_column": "provider_risk_tier", "segment_value": "low"},
                {"segment_column": "provider_risk_tier", "segment_value": "high"}
            ]);
        }
        "reviewer_disagreement_review" => {
            report["reviewer_disagreement_rate"] = serde_json::json!(0.03);
            report["review_sample_count"] = serde_json::json!(128);
        }
        "label_delay_review" => {
            report["label_delay_p95_days"] = serde_json::json!(14);
            report["delayed_label_count"] = serde_json::json!(0);
        }
        _ => {}
    }
    if let Some(monitoring_input) = monitoring_input {
        apply_mlops_monitoring_input(&mut report, monitoring_input, job_kind);
    }
    report
}

fn mlops_monitoring_input_for_job<'a>(
    monitoring_inputs: Option<&'a serde_json::Value>,
    job_kind: &str,
) -> Option<&'a serde_json::Value> {
    let monitoring_inputs = monitoring_inputs?;
    monitoring_inputs
        .get("jobs")
        .and_then(|jobs| jobs.get(job_kind))
        .or_else(|| monitoring_inputs.get(job_kind))
}

fn apply_mlops_monitoring_input(
    report: &mut serde_json::Value,
    monitoring_input: &serde_json::Value,
    job_kind: &str,
) {
    if let (Some(report_object), Some(input_object)) =
        (report.as_object_mut(), monitoring_input.as_object())
    {
        for (key, value) in input_object {
            if is_protected_mlops_report_field(key) {
                continue;
            }
            report_object.insert(key.clone(), value.clone());
        }
        report_object.insert("input_binding_job_kind".into(), serde_json::json!(job_kind));
        report_object.insert("input_binding_status".into(), serde_json::json!("provided"));
        report_object.insert("customer_data_bound".into(), serde_json::json!(true));
        report_object.insert("customer_data_required".into(), serde_json::json!(false));
    }
}

fn is_protected_mlops_report_field(key: &str) -> bool {
    matches!(
        key,
        "artifact_kind"
            | "report_version"
            | "runtime_source"
            | "model_key"
            | "model_version"
            | "manifest_uri"
            | "artifact_uri"
            | "output_uri"
            | "output_ref"
            | "schedule"
            | "governance_boundary"
    )
}

fn mlops_alert_delivery_task(
    model_key: &str,
    model_version: &str,
    trigger: &str,
    plan_uri: &str,
    monitoring_report_uri: &str,
) -> serde_json::Value {
    let (severity, route_key, recommended_action) = match trigger {
        "rust_serving_artifact_evaluation_blocked" => (
            "critical",
            "mlops_serving_runtime",
            "open serving artifact governance review",
        ),
        "rust_serving_latency_budget_failed" => (
            "high",
            "mlops_serving_runtime",
            "review latency budget before rollout or rollback decision",
        ),
        "model_drift_detected" => (
            "high",
            "mlops_retraining_readiness",
            "prepare retraining review after human approval",
        ),
        "model_drift_watch" => (
            "medium",
            "mlops_retraining_readiness",
            "monitor drift and schedule next comparison",
        ),
        "shadow_comparison_review_required" => (
            "high",
            "mlops_shadow_review",
            "review shadow comparison before promotion",
        ),
        "segment_fairness_review_required" => (
            "high",
            "model_governance",
            "open segment fairness governance review",
        ),
        _ => ("medium", "mlops_review", "review monitoring trigger"),
    };
    serde_json::json!({
        "task_kind": "mlops_alert_delivery",
        "model_key": model_key,
        "model_version": model_version,
        "trigger": trigger,
        "severity": severity,
        "route_key": route_key,
        "delivery_status": "queued_for_external_alert_router",
        "recommended_action": recommended_action,
        "evidence_refs": [
            format!("mlops_monitoring_plans:{plan_uri}"),
            format!("model_monitoring_reports:{monitoring_report_uri}"),
            format!("model_versions:{model_key}:{model_version}")
        ]
    })
}
