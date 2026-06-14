use anyhow::{bail, Context};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use super::mlops_monitoring_runtime::{
    build_mlops_monitoring_artifact_publication_manifest, expected_mlops_runtime_job_kinds,
    expected_mlops_runtime_report_file, file_name_from_uri, mlops_monitoring_input_for_job,
    mlops_plan_job_output_uri, mlops_runtime_report_for_job,
};
use super::{json_string, nested_json_string, read_json_report, required_non_empty, write_json};

pub use super::mlops_monitoring_plan::build_mlops_monitoring_plan;
pub use super::mlops_monitoring_report::{
    build_mlops_monitoring_report, build_mlops_scheduler_execution_report,
};

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
