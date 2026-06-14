use anyhow::{bail, Context};
use serde::Deserialize;
use std::{collections::BTreeMap, sync::Arc};

use super::{api_url, required_non_empty};

#[derive(Debug, Clone)]
pub struct MlopsAlertRouterConfig {
    pub bind_addr: String,
    pub api_base_url: String,
    pub api_key: String,
    pub alertmanager_webhook_token: Option<String>,
    pub model_key: String,
    pub model_version: String,
    pub scheduler_execution_report_uri: String,
    pub actor: String,
    pub notes: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AlertmanagerWebhook {
    #[serde(default)]
    pub status: String,
    #[serde(default, rename = "groupKey")]
    pub group_key: String,
    #[serde(default)]
    pub alerts: Vec<AlertmanagerAlert>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AlertmanagerAlert {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default)]
    pub fingerprint: String,
    #[serde(default, rename = "startsAt")]
    pub starts_at: String,
}

pub fn build_alertmanager_mlops_alert_delivery_submission(
    config: &MlopsAlertRouterConfig,
    webhook: &AlertmanagerWebhook,
) -> anyhow::Result<serde_json::Value> {
    required_non_empty("api_base_url", &config.api_base_url)?;
    required_non_empty("api_key", &config.api_key)?;
    required_non_empty("model_key", &config.model_key)?;
    required_non_empty("model_version", &config.model_version)?;
    required_non_empty(
        "scheduler_execution_report_uri",
        &config.scheduler_execution_report_uri,
    )?;
    required_non_empty("actor", &config.actor)?;
    required_non_empty("notes", &config.notes)?;

    let alert_delivery_tasks = webhook
        .alerts
        .iter()
        .filter(|alert| alert.status.trim().is_empty() || alert.status == "firing")
        .map(|alert| alertmanager_alert_delivery_task(config, alert))
        .collect::<Vec<_>>();
    let alert_delivery_status = if alert_delivery_tasks.is_empty() {
        "no_alerts_required"
    } else {
        "queued_for_external_alert_router"
    };
    Ok(serde_json::json!({
        "actor": config.actor,
        "notes": config.notes,
        "scheduler_execution_report_uri": config.scheduler_execution_report_uri,
        "report_kind": "mlops_scheduler_execution_report",
        "model_version": config.model_version,
        "alert_delivery_status": alert_delivery_status,
        "alert_delivery_tasks": alert_delivery_tasks,
        "evidence_refs": [
            format!("mlops_scheduler_execution_reports:{}", config.scheduler_execution_report_uri),
            format!("model_versions:{}:{}", config.model_key, config.model_version)
        ]
    }))
}

pub async fn submit_alertmanager_webhook_to_fwa(
    config: &MlopsAlertRouterConfig,
    webhook: &AlertmanagerWebhook,
) -> anyhow::Result<serde_json::Value> {
    let payload = build_alertmanager_mlops_alert_delivery_submission(config, webhook)?;
    let response = reqwest::Client::new()
        .post(api_url(
            &config.api_base_url,
            &format!(
                "/api/v1/ops/models/{}/mlops-alert-deliveries",
                config.model_key
            ),
        ))
        .header("x-api-key", &config.api_key)
        .json(&payload)
        .send()
        .await
        .context("submit Alertmanager MLOps alert delivery")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        tracing::warn!(
            status = %status,
            upstream_body = %body,
            "Alertmanager MLOps alert delivery submission failed"
        );
        bail!("submit Alertmanager MLOps alert delivery failed with {status}");
    }
    response
        .json::<serde_json::Value>()
        .await
        .context("parse Alertmanager MLOps alert delivery response")
}

pub async fn serve_mlops_alert_router(config: MlopsAlertRouterConfig) -> anyhow::Result<()> {
    use axum::{
        extract::State,
        http::{header, HeaderMap, StatusCode},
        response::IntoResponse,
        routing::{get, post},
        Json, Router,
    };

    async fn health() -> impl IntoResponse {
        Json(serde_json::json!({
            "status": "ok",
            "service": "mlops-alert-router",
            "adapter_boundary": "alertmanager_to_fwa_mlops_alert_delivery"
        }))
    }

    async fn route_alertmanager_webhook(
        State(config): State<Arc<MlopsAlertRouterConfig>>,
        headers: HeaderMap,
        Json(webhook): Json<AlertmanagerWebhook>,
    ) -> impl IntoResponse {
        if !alertmanager_webhook_is_authorized(&config, &headers, header::AUTHORIZATION) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "status": "unauthorized",
                    "error": "invalid Alertmanager webhook credentials",
                    "adapter_boundary": "alertmanager_to_fwa_mlops_alert_delivery"
                })),
            );
        }
        match submit_alertmanager_webhook_to_fwa(&config, &webhook).await {
            Ok(response) => (StatusCode::ACCEPTED, Json(response)),
            Err(error) => {
                tracing::warn!(error = %error, "Alertmanager webhook routing failed");
                (
                    StatusCode::BAD_GATEWAY,
                    Json(serde_json::json!({
                        "status": "failed",
                        "error": "failed to submit Alertmanager webhook to FWA API",
                        "adapter_boundary": "alertmanager_to_fwa_mlops_alert_delivery"
                    })),
                )
            }
        }
    }

    required_non_empty("bind_addr", &config.bind_addr)?;
    required_non_empty(
        "alertmanager_webhook_token",
        config
            .alertmanager_webhook_token
            .as_deref()
            .unwrap_or_default(),
    )?;
    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .with_context(|| format!("bind MLOps alert router on {}", config.bind_addr))?;
    let router = Router::new()
        .route("/health", get(health))
        .route("/alertmanager/webhook", post(route_alertmanager_webhook))
        .with_state(Arc::new(config));
    axum::serve(listener, router)
        .await
        .context("serve MLOps alert router")
}

pub(crate) fn alertmanager_webhook_is_authorized(
    config: &MlopsAlertRouterConfig,
    headers: &axum::http::HeaderMap,
    authorization_header: axum::http::header::HeaderName,
) -> bool {
    let Some(expected_token) = config.alertmanager_webhook_token.as_deref() else {
        return false;
    };
    if expected_token.trim().is_empty() {
        return false;
    }
    let expected = format!("Bearer {expected_token}");
    headers
        .get(authorization_header)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|actual| actual.trim() == expected)
}

fn alertmanager_alert_delivery_task(
    config: &MlopsAlertRouterConfig,
    alert: &AlertmanagerAlert,
) -> serde_json::Value {
    let alert_name = safe_alertmanager_value(alert.labels.get("alertname"), "alertmanager_alert");
    let severity = safe_alertmanager_value(alert.labels.get("severity"), "warning");
    let service = safe_alertmanager_value(alert.labels.get("service"), "mlops");
    let fingerprint = safe_alertmanager_value(Some(&alert.fingerprint), "");
    let fallback_dedupe = format!(
        "{}:{}:{}",
        alert_name,
        service,
        safe_alertmanager_value(Some(&alert.starts_at), "unknown")
    );
    let dedupe_key = if fingerprint.is_empty() {
        format!("alertmanager:{fallback_dedupe}")
    } else {
        format!("alertmanager:{fingerprint}")
    };
    serde_json::json!({
        "task_kind": "mlops_alert_delivery",
        "model_key": config.model_key,
        "model_version": config.model_version,
        "trigger": alert_name,
        "severity": severity,
        "route_key": service,
        "dedupe_key": dedupe_key,
        "alertmanager_fingerprint": fingerprint,
        "alert_status": if alert.status.trim().is_empty() { "firing" } else { alert.status.as_str() },
        "starts_at": safe_alertmanager_value(Some(&alert.starts_at), "unknown"),
        "delivery_status": "queued_for_external_alert_router",
        "recommended_action": "review Alertmanager MLOps alert and confirm customer alert-router receipt",
        "evidence_refs": [
            format!("mlops_scheduler_execution_reports:{}", config.scheduler_execution_report_uri),
            format!("model_versions:{}:{}", config.model_key, config.model_version)
        ]
    })
}

fn safe_alertmanager_value(value: Option<&String>, fallback: &str) -> String {
    let cleaned = value
        .map(String::as_str)
        .unwrap_or(fallback)
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':' | '/') {
                ch
            } else {
                '_'
            }
        })
        .take(96)
        .collect::<String>();
    if cleaned.trim_matches('_').is_empty() {
        fallback.into()
    } else {
        cleaned
    }
}
