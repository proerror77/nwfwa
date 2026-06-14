use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

use super::api_url;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WorkerHealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
    pub checks: Vec<WorkerHealthCheck>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WorkerHealthCheck {
    pub name: &'static str,
    pub status: &'static str,
}

pub fn worker_health() -> WorkerHealthResponse {
    WorkerHealthResponse {
        status: "ok",
        service: "worker",
        version: env!("CARGO_PKG_VERSION"),
        checks: vec![
            WorkerHealthCheck {
                name: "cli_commands",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "parquet_profiler",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "feature_set_builder",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "demo_ml_dataset_builder",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "automl_candidate_ranker",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "mlops_monitoring_plan_runner",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "rule_candidate_miner",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "rule_candidate_backtester",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "provider_peer_clusterer",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "claim_entity_clusterer",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "provider_graph_clusterer",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "automl_lifecycle_closure_reporter",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "demo_automl_lifecycle_evidence_builder",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "demo_automl_lifecycle_verifier",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "model_promotion_orchestrator",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "mlops_monitoring_report_submitter",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "mlops_scheduler_execution_reporter",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "mlops_alert_delivery_submitter",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "mlops_monitoring_cycle_executor",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "mlops_alert_receiver_webhook_sender",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "model_artifact_evaluator",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "retraining_job_runner",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "pilot_readiness_checker",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "analytics_export_plan",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "ai_evidence_execution_plan",
                status: "ok",
            },
            WorkerHealthCheck {
                name: "governance_ops_plan",
                status: "ok",
            },
        ],
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ApiHealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
    pub pilot_readiness: ApiPilotReadiness,
    pub checks: Vec<ApiHealthCheck>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ApiPilotReadiness {
    pub status: String,
    #[serde(default)]
    pub required_check_names: Vec<String>,
    #[serde(default)]
    pub required_check_count: usize,
    #[serde(default)]
    pub ready_check_count: usize,
    #[serde(default)]
    pub blocking_check_count: usize,
    #[serde(default)]
    pub ready_checks: Vec<ApiHealthCheck>,
    #[serde(default)]
    pub blocking_checks: Vec<ApiHealthCheck>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ApiHealthCheck {
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PilotReadinessReport {
    pub status: String,
    pub ready_for_customer_pilot: bool,
    pub api_status: String,
    pub api_service: String,
    pub api_version: String,
    pub required_check_count: usize,
    pub ready_check_count: usize,
    pub blocking_check_count: usize,
    pub model_runtime_kind: Option<String>,
    pub ready_checks: Vec<ApiHealthCheck>,
    pub blocking_checks: Vec<ApiHealthCheck>,
    pub remediation_summary: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub async fn check_pilot_readiness(
    api_base_url: &str,
    api_key: Option<&str>,
) -> anyhow::Result<PilotReadinessReport> {
    let mut request = reqwest::Client::new().get(api_url(api_base_url, "/api/v1/health"));
    if let Some(api_key) = api_key {
        request = request.header("x-api-key", api_key);
    }
    let response = request.send().await.context("fetch API health")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("fetch API health failed with {status}: {body}");
    }
    let health = response
        .json::<ApiHealthResponse>()
        .await
        .context("parse API health response")?;
    Ok(build_pilot_readiness_report(health))
}

pub fn build_pilot_readiness_report(health: ApiHealthResponse) -> PilotReadinessReport {
    let model_runtime_kind = health
        .checks
        .iter()
        .find(|check| check.name == "model_scorer")
        .and_then(|check| check.runtime_kind.clone());
    let remediation_summary = health
        .pilot_readiness
        .blocking_checks
        .iter()
        .map(|check| {
            check
                .remediation
                .clone()
                .unwrap_or_else(|| format!("{}={}", check.name, check.status))
        })
        .collect::<Vec<_>>();
    let evidence_refs = vec![
        "api_health:/api/v1/health".to_string(),
        "pilot_readiness:/api/v1/health#pilot_readiness".to_string(),
    ];
    PilotReadinessReport {
        status: health.pilot_readiness.status.clone(),
        ready_for_customer_pilot: health.pilot_readiness.status == "ready"
            && health.pilot_readiness.blocking_checks.is_empty(),
        api_status: health.status,
        api_service: health.service,
        api_version: health.version,
        required_check_count: health.pilot_readiness.required_check_count,
        ready_check_count: health.pilot_readiness.ready_check_count,
        blocking_check_count: health.pilot_readiness.blocking_check_count,
        model_runtime_kind,
        ready_checks: health.pilot_readiness.ready_checks,
        blocking_checks: health.pilot_readiness.blocking_checks,
        remediation_summary,
        evidence_refs,
    }
}
