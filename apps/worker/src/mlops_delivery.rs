use anyhow::{bail, Context};
use hmac::Mac;
use serde::Serialize;
use std::{collections::BTreeSet, fs, path::Path};

use super::{api_url, json_string, read_json_report, required_non_empty, write_json, HmacSha256};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MlopsMonitoringReportSubmission {
    pub actor: String,
    pub notes: String,
    pub report_uri: String,
    pub report_kind: String,
    pub model_version: String,
    pub overall_status: String,
    pub retraining_recommendation: String,
    pub triggers: Vec<String>,
    pub review_tasks: Vec<serde_json::Value>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MlopsAlertDeliverySubmission {
    pub actor: String,
    pub notes: String,
    pub scheduler_execution_report_uri: String,
    pub report_kind: String,
    pub model_version: String,
    pub alert_delivery_status: String,
    pub alert_delivery_tasks: Vec<serde_json::Value>,
    pub evidence_refs: Vec<String>,
}

pub fn build_mlops_monitoring_report_submission(
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<(String, MlopsMonitoringReportSubmission)> {
    build_mlops_monitoring_report_submission_with_published_uri(
        report_uri, report_uri, actor, notes,
    )
}

pub fn build_mlops_monitoring_report_submission_with_published_uri(
    report_uri: &str,
    published_report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<(String, MlopsMonitoringReportSubmission)> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let published_report_uri = required_non_empty("published_report_uri", published_report_uri)?;
    ensure_published_artifact_uri(
        "MLOps monitoring published_report_uri",
        published_report_uri,
    )?;
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    let report = read_json_report(report_uri)?;
    let model_key = json_string(&report, "model_key")
        .filter(|value| !value.trim().is_empty())
        .context("MLOps monitoring report requires model_key")?;
    let model_version = json_string(&report, "model_version")
        .filter(|value| !value.trim().is_empty())
        .context("MLOps monitoring report requires model_version")?;
    let report_kind = json_string(&report, "report_kind")
        .filter(|value| value == "mlops_monitoring_report")
        .context("MLOps monitoring report_kind must be mlops_monitoring_report")?;
    let overall_status = json_string(&report, "overall_status")
        .filter(|value| !value.trim().is_empty())
        .context("MLOps monitoring report requires overall_status")?;
    let retraining_recommendation = json_string(&report, "retraining_recommendation")
        .filter(|value| !value.trim().is_empty())
        .context("MLOps monitoring report requires retraining_recommendation")?;
    let triggers = report
        .get("triggers")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();
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
        .filter(|reference| evidence_ref_is_production(reference))
        .collect::<Vec<_>>();
    evidence_refs.push(format!("model_versions:{model_key}:{model_version}"));
    evidence_refs.push(format!("model_monitoring_reports:{published_report_uri}"));
    evidence_refs.sort();
    evidence_refs.dedup();

    Ok((
        model_key,
        MlopsMonitoringReportSubmission {
            actor: actor.into(),
            notes: notes.into(),
            report_uri: published_report_uri.into(),
            report_kind,
            model_version,
            overall_status,
            retraining_recommendation,
            triggers,
            review_tasks,
            evidence_refs,
        },
    ))
}

pub async fn submit_mlops_monitoring_report(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    submit_mlops_monitoring_report_with_published_uri(
        api_base_url,
        api_key,
        report_uri,
        report_uri,
        actor,
        notes,
    )
    .await
}

pub async fn submit_mlops_monitoring_report_with_published_uri(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    published_report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    let (model_key, payload) = build_mlops_monitoring_report_submission_with_published_uri(
        report_uri,
        published_report_uri,
        actor,
        notes,
    )?;
    let response = reqwest::Client::new()
        .post(api_url(
            api_base_url,
            &format!("/api/v1/ops/models/{model_key}/mlops-monitoring-reports"),
        ))
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .context("submit MLOps monitoring report")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("submit MLOps monitoring report failed with {status}: {body}");
    }
    response
        .json::<serde_json::Value>()
        .await
        .context("parse MLOps monitoring report response")
}

pub fn build_mlops_alert_delivery_submission(
    scheduler_execution_report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<(String, MlopsAlertDeliverySubmission)> {
    build_mlops_alert_delivery_submission_with_published_uri(
        scheduler_execution_report_uri,
        scheduler_execution_report_uri,
        actor,
        notes,
    )
}

pub fn build_mlops_alert_delivery_submission_with_published_uri(
    scheduler_execution_report_uri: &str,
    published_scheduler_execution_report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<(String, MlopsAlertDeliverySubmission)> {
    let scheduler_execution_report_uri = required_non_empty(
        "scheduler_execution_report_uri",
        scheduler_execution_report_uri,
    )?;
    let published_scheduler_execution_report_uri = required_non_empty(
        "published_scheduler_execution_report_uri",
        published_scheduler_execution_report_uri,
    )?;
    ensure_published_artifact_uri(
        "MLOps alert delivery published_scheduler_execution_report_uri",
        published_scheduler_execution_report_uri,
    )?;
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    let report = read_json_report(scheduler_execution_report_uri)?;
    let model_key = json_string(&report, "model_key")
        .filter(|value| !value.trim().is_empty())
        .context("MLOps scheduler execution report requires model_key")?;
    let model_version = json_string(&report, "model_version")
        .filter(|value| !value.trim().is_empty())
        .context("MLOps scheduler execution report requires model_version")?;
    let report_kind = json_string(&report, "report_kind")
        .filter(|value| value == "mlops_scheduler_execution_report")
        .context("report_kind must be mlops_scheduler_execution_report")?;
    let alert_delivery_status = json_string(&report, "alert_delivery_status")
        .filter(|value| !value.trim().is_empty())
        .context("MLOps scheduler execution report requires alert_delivery_status")?;
    let alert_delivery_tasks = report
        .get("alert_delivery_tasks")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let mut evidence_refs = report
        .get("evidence_refs")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .filter(|reference| evidence_ref_is_production(reference))
        .collect::<Vec<_>>();
    evidence_refs.push(format!("model_versions:{model_key}:{model_version}"));
    evidence_refs.push(format!(
        "mlops_scheduler_execution_reports:{published_scheduler_execution_report_uri}"
    ));
    evidence_refs.sort();
    evidence_refs.dedup();

    Ok((
        model_key,
        MlopsAlertDeliverySubmission {
            actor: actor.into(),
            notes: notes.into(),
            scheduler_execution_report_uri: published_scheduler_execution_report_uri.into(),
            report_kind,
            model_version,
            alert_delivery_status,
            alert_delivery_tasks,
            evidence_refs,
        },
    ))
}

pub async fn submit_mlops_alert_delivery_tasks(
    api_base_url: &str,
    api_key: &str,
    scheduler_execution_report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    submit_mlops_alert_delivery_tasks_with_published_uri(
        api_base_url,
        api_key,
        scheduler_execution_report_uri,
        scheduler_execution_report_uri,
        actor,
        notes,
    )
    .await
}

pub async fn submit_mlops_alert_delivery_tasks_with_published_uri(
    api_base_url: &str,
    api_key: &str,
    scheduler_execution_report_uri: &str,
    published_scheduler_execution_report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    let (model_key, payload) = build_mlops_alert_delivery_submission_with_published_uri(
        scheduler_execution_report_uri,
        published_scheduler_execution_report_uri,
        actor,
        notes,
    )?;
    let response = reqwest::Client::new()
        .post(api_url(
            api_base_url,
            &format!("/api/v1/ops/models/{model_key}/mlops-alert-deliveries"),
        ))
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .context("submit MLOps alert delivery tasks")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("submit MLOps alert delivery tasks failed with {status}: {body}");
    }
    response
        .json::<serde_json::Value>()
        .await
        .context("parse MLOps alert delivery response")
}

fn ensure_published_artifact_uri(field: &str, value: &str) -> anyhow::Result<()> {
    let value = value.trim();
    if value.is_empty()
        || value.starts_with("local://")
        || value.starts_with("file://")
        || !value.contains("://")
        || value.contains('{')
        || value.contains('}')
    {
        bail!("{field} must use a published production artifact URI");
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

fn evidence_ref_is_production(reference: &str) -> bool {
    let reference = reference.trim();
    if reference.is_empty()
        || reference.contains("local://")
        || reference.contains("file://")
        || reference.contains('{')
        || reference.contains('}')
    {
        return false;
    }
    if reference.starts_with("model_versions:") {
        return true;
    }
    reference
        .split_once(':')
        .is_some_and(|(_, uri)| uri.contains("://"))
}

pub fn build_mlops_alert_receiver_payload(
    scheduler_execution_report_uri: &str,
    receiver_id: &str,
) -> anyhow::Result<serde_json::Value> {
    let scheduler_execution_report_uri = required_non_empty(
        "scheduler_execution_report_uri",
        scheduler_execution_report_uri,
    )?;
    let receiver_id = required_non_empty("receiver_id", receiver_id)?;
    let report = read_json_report(scheduler_execution_report_uri)?;
    if json_string(&report, "report_kind").as_deref() != Some("mlops_scheduler_execution_report") {
        bail!("alert receiver payload requires an mlops_scheduler_execution_report");
    }
    let model_key = json_string(&report, "model_key")
        .filter(|value| !value.trim().is_empty())
        .context("MLOps scheduler execution report requires model_key")?;
    let model_version = json_string(&report, "model_version")
        .filter(|value| !value.trim().is_empty())
        .context("MLOps scheduler execution report requires model_version")?;
    let alert_delivery_status = json_string(&report, "alert_delivery_status")
        .filter(|value| !value.trim().is_empty())
        .context("MLOps scheduler execution report requires alert_delivery_status")?;
    let alert_delivery_tasks = report
        .get("alert_delivery_tasks")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let evidence_refs = report
        .get("evidence_refs")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .chain(std::iter::once(format!(
            "mlops_scheduler_execution_reports:{scheduler_execution_report_uri}"
        )))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    Ok(serde_json::json!({
        "event_kind": "mlops_alert_receiver_delivery",
        "event_version": 1,
        "receiver_id": receiver_id,
        "model_key": model_key,
        "model_version": model_version,
        "scheduler_execution_report_uri": scheduler_execution_report_uri,
        "alert_delivery_status": alert_delivery_status,
        "alert_delivery_task_count": alert_delivery_tasks.len(),
        "alert_delivery_tasks": alert_delivery_tasks,
        "evidence_refs": evidence_refs,
        "governance_boundary": "alert receiver delivery may notify an external receiver only; it must not create retraining jobs, activate models, rollback models, assign fraud labels, or write rules"
    }))
}

pub async fn deliver_mlops_alert_receiver_webhook(
    scheduler_execution_report_uri: &str,
    receiver_url: &str,
    receiver_id: &str,
    receiver_token: Option<&str>,
    receiver_secret: Option<&str>,
    max_attempts: u32,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let receiver_url = required_non_empty("receiver_url", receiver_url)?;
    let receiver_token = receiver_token
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let receiver_secret = receiver_secret
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let max_attempts = max_attempts.clamp(1, 5);
    let payload = build_mlops_alert_receiver_payload(scheduler_execution_report_uri, receiver_id)?;
    let payload_body =
        serde_json::to_string(&payload).context("serialize MLOps alert receiver payload")?;
    let signature = receiver_secret
        .map(|secret| mlops_alert_receiver_signature(secret, payload_body.as_bytes()))
        .transpose()?;
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "create MLOps alert receiver output dir {}",
            output_dir.display()
        )
    })?;
    write_json(
        output_dir.join("mlops_alert_receiver_payload.json"),
        &payload,
    )?;

    let task_count = payload["alert_delivery_task_count"].as_u64().unwrap_or(0);
    let mut report = serde_json::json!({
        "report_kind": "mlops_alert_receiver_delivery_report",
        "report_version": 1,
        "receiver_id": payload["receiver_id"].clone(),
        "model_key": payload["model_key"].clone(),
        "model_version": payload["model_version"].clone(),
        "scheduler_execution_report_uri": payload["scheduler_execution_report_uri"].clone(),
        "alert_delivery_task_count": task_count,
        "receiver_url_configured": true,
        "receiver_auth_configured": receiver_token.is_some(),
        "receiver_signature_configured": signature.is_some(),
        "max_attempts": max_attempts,
        "attempt_count": 0,
        "delivery_status": "skipped_no_alerts_required",
        "http_status": serde_json::Value::Null,
        "response_body_excerpt": serde_json::Value::Null,
        "governance_boundary": payload["governance_boundary"].clone(),
        "evidence_refs": payload["evidence_refs"].clone()
    });
    if task_count > 0 {
        let client = reqwest::Client::new();
        for attempt in 1..=max_attempts {
            report["attempt_count"] = serde_json::json!(attempt);
            let mut request = client
                .post(receiver_url)
                .header("content-type", "application/json")
                .header("x-fwa-event-kind", "mlops_alert_receiver_delivery")
                .header("x-fwa-delivery-attempt", attempt.to_string())
                .header(
                    "x-fwa-model-key",
                    payload["model_key"].as_str().unwrap_or(""),
                );
            if let Some(token) = receiver_token {
                request = request.bearer_auth(token);
            }
            if let Some(signature) = &signature {
                request = request.header("x-fwa-signature-sha256", signature);
            }
            let response = request.body(payload_body.clone()).send().await;
            match response {
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    let body_excerpt = body.chars().take(256).collect::<String>();
                    report["delivery_status"] = serde_json::json!(if status.is_success() {
                        "delivered"
                    } else {
                        "failed"
                    });
                    report["http_status"] = serde_json::json!(status.as_u16());
                    report["response_body_excerpt"] = serde_json::json!(body_excerpt);
                    if status.is_success() {
                        break;
                    }
                }
                Err(error) => {
                    report["delivery_status"] = serde_json::json!("failed");
                    report["response_body_excerpt"] =
                        serde_json::json!(error.to_string().chars().take(256).collect::<String>());
                }
            }
        }
    }
    write_json(
        output_dir.join("mlops_alert_receiver_delivery_report.json"),
        &report,
    )?;
    Ok(report)
}

fn mlops_alert_receiver_signature(secret: &str, body: &[u8]) -> anyhow::Result<String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .context("create MLOps alert receiver HMAC")?;
    mac.update(body);
    let bytes = mac.finalize().into_bytes();
    Ok(format!("hmac-sha256={}", lowercase_hex(&bytes)))
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
