use crate::app::AppState;
use axum::extract::State;
use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
    pub pilot_readiness: PilotReadiness,
    pub checks: Vec<HealthCheck>,
}

#[derive(Debug, Serialize)]
pub struct PilotReadiness {
    pub status: &'static str,
    pub required_check_names: Vec<&'static str>,
    pub required_check_count: usize,
    pub ready_check_count: usize,
    pub blocking_check_count: usize,
    pub ready_checks: Vec<HealthCheck>,
    pub blocking_checks: Vec<HealthCheck>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct HealthCheck {
    pub name: &'static str,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_kind: Option<&'static str>,
}

pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let checks = vec![
        HealthCheck {
            name: "http_router",
            status: "ok",
            runtime_kind: None,
        },
        HealthCheck {
            name: "openapi_contract",
            status: "ok",
            runtime_kind: None,
        },
        HealthCheck {
            name: "model_scorer",
            status: "ok",
            runtime_kind: Some(state.config.model_runtime_kind()),
        },
        HealthCheck {
            name: "model_service_configuration",
            status: state.config.model_service_configuration_status(),
            runtime_kind: None,
        },
        HealthCheck {
            name: "api_key_configuration",
            status: state.config.api_key_configuration_status(),
            runtime_kind: None,
        },
        HealthCheck {
            name: "source_system_configuration",
            status: state.config.source_system_configuration_status(),
            runtime_kind: None,
        },
        HealthCheck {
            name: "database_configuration",
            status: state.config.database_configuration_status(),
            runtime_kind: None,
        },
        HealthCheck {
            name: "object_storage_configuration",
            status: state.config.object_storage_configuration_status(),
            runtime_kind: None,
        },
        HealthCheck {
            name: "customer_scope_configuration",
            status: state.config.customer_scope_configuration_status(),
            runtime_kind: None,
        },
        HealthCheck {
            name: "retention_policy_configuration",
            status: state.config.retention_policy_configuration_status(),
            runtime_kind: None,
        },
        HealthCheck {
            name: "backup_restore_configuration",
            status: state.config.backup_restore_configuration_status(),
            runtime_kind: None,
        },
        HealthCheck {
            name: "pii_masking_configuration",
            status: state.config.pii_masking_configuration_status(),
            runtime_kind: None,
        },
        HealthCheck {
            name: "key_rotation_configuration",
            status: state.config.key_rotation_configuration_status(),
            runtime_kind: None,
        },
        HealthCheck {
            name: "network_allowlist_configuration",
            status: state.config.network_allowlist_configuration_status(),
            runtime_kind: None,
        },
        HealthCheck {
            name: "alert_routing_configuration",
            status: state.config.alert_routing_configuration_status(),
            runtime_kind: None,
        },
        HealthCheck {
            name: "observability_exporter_configuration",
            status: state.config.observability_exporter_configuration_status(),
            runtime_kind: None,
        },
        HealthCheck {
            name: "agent_policy_configuration",
            status: state.config.agent_policy_configuration_status(),
            runtime_kind: None,
        },
    ];
    let pilot_readiness = pilot_readiness(&checks);
    Json(HealthResponse {
        status: "ok",
        service: "api-server",
        version: env!("CARGO_PKG_VERSION"),
        pilot_readiness,
        checks,
    })
}

fn pilot_readiness(checks: &[HealthCheck]) -> PilotReadiness {
    let required_checks: Vec<HealthCheck> = checks
        .iter()
        .copied()
        .filter(|check| check.name.ends_with("_configuration"))
        .collect();
    let ready_checks: Vec<HealthCheck> = required_checks
        .iter()
        .copied()
        .filter(|check| check.status == "configured")
        .collect();
    let blocking_checks: Vec<HealthCheck> = checks
        .iter()
        .copied()
        .filter(|check| check.name.ends_with("_configuration") && check.status != "configured")
        .collect();
    let status = if blocking_checks.is_empty() {
        "ready"
    } else {
        "not_ready"
    };
    PilotReadiness {
        status,
        required_check_names: required_checks.iter().map(|check| check.name).collect(),
        required_check_count: required_checks.len(),
        ready_check_count: ready_checks.len(),
        blocking_check_count: blocking_checks.len(),
        ready_checks,
        blocking_checks,
    }
}
