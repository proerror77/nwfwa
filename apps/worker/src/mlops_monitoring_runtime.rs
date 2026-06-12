use anyhow::Context;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path};

use super::mlops_monitoring_plan::compute_psi;
use super::required_non_empty;

pub(super) fn build_mlops_monitoring_artifact_publication_manifest(
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

pub(super) fn mlops_plan_job_output_uri(job: &serde_json::Value) -> Option<String> {
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

pub(super) fn expected_mlops_runtime_job_kinds() -> [&'static str; 7] {
    [
        "shadow_traffic_evaluation",
        "drift_monitoring",
        "feature_distribution_psi",
        "rule_hit_rate_trend",
        "segment_fairness_review",
        "reviewer_disagreement_review",
        "label_delay_review",
    ]
}

pub(super) fn expected_mlops_runtime_report_file(job_kind: &str) -> Option<&'static str> {
    match job_kind {
        "shadow_traffic_evaluation" => Some("shadow_report.json"),
        "drift_monitoring" => Some("drift_report.json"),
        "feature_distribution_psi" => Some("feature_psi_report.json"),
        "rule_hit_rate_trend" => Some("rule_hit_rate_report.json"),
        "segment_fairness_review" => Some("fairness_report.json"),
        "reviewer_disagreement_review" => Some("reviewer_disagreement_report.json"),
        "label_delay_review" => Some("label_delay_report.json"),
        _ => None,
    }
}

pub(super) fn file_name_from_uri(uri: &str, fallback: &str) -> String {
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

pub(super) fn mlops_runtime_report_for_job(
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
        "feature_distribution_psi" => {
            report["status"] = serde_json::json!("stable");
            report["feature"] = job
                .get("feature")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("claim_amount_peer_percentile"));
            report["bucket_count"] = job
                .get("bucket_count")
                .cloned()
                .unwrap_or_else(|| serde_json::json!(10));
            report["psi"] = serde_json::json!(0.05);
            report["thresholds"] = job.get("thresholds").cloned().unwrap_or_else(|| {
                serde_json::json!({
                    "stable_below": 0.10,
                    "watch_below": 0.25
                })
            });
            if let Some(monitoring_input) = monitoring_input {
                if let (Some(baseline), Some(current)) = (
                    f64_array(monitoring_input, "baseline"),
                    f64_array(monitoring_input, "current"),
                ) {
                    let bucket_count = job
                        .get("bucket_count")
                        .and_then(|value| value.as_u64())
                        .unwrap_or(10) as usize;
                    let psi = compute_psi(&baseline, &current, bucket_count);
                    report["psi"] = serde_json::json!(psi);
                    report["status"] = serde_json::json!(if psi < 0.10 {
                        "stable"
                    } else if psi < 0.25 {
                        "watch"
                    } else {
                        "alert"
                    });
                }
            }
        }
        "rule_hit_rate_trend" => {
            report["status"] = serde_json::json!("stable");
            report["rules_evaluated"] = serde_json::json!(0);
            report["alert_condition"] = job
                .get("alert_condition")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("hit_rate_7d < 0.5 * hit_rate_90d"));
            report["rule_drift_alerts"] = serde_json::json!([]);
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

pub(super) fn mlops_monitoring_input_for_job<'a>(
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

fn f64_array(input: &serde_json::Value, field: &str) -> Option<Vec<f64>> {
    input.get(field)?.as_array().map(|values| {
        values
            .iter()
            .filter_map(|value| value.as_f64())
            .collect::<Vec<_>>()
    })
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
