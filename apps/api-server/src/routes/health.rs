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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<&'static str>,
}

pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let checks = vec![
        HealthCheck {
            name: "http_router",
            status: "ok",
            runtime_kind: None,
            remediation: None,
        },
        HealthCheck {
            name: "openapi_contract",
            status: "ok",
            runtime_kind: None,
            remediation: None,
        },
        HealthCheck {
            name: "model_scorer",
            status: "ok",
            runtime_kind: Some(state.config.model_runtime_kind()),
            remediation: None,
        },
        config_check(
            "model_service_configuration",
            state.config.model_service_configuration_status(),
            "Set FWA_MODEL_SERVICE_URL to the customer-approved model endpoint or configure a signed model artifact.",
        ),
        config_check(
            "api_key_configuration",
            state.config.api_key_configuration_status(),
            "Set customer API principals through FWA_API_KEY_PRINCIPALS and avoid the local dev key.",
        ),
        config_check(
            "source_system_configuration",
            state.config.source_system_configuration_status(),
            "Set FWA_SOURCE_SYSTEM to the customer-approved source system identifier.",
        ),
        config_check(
            "database_configuration",
            state.config.database_configuration_status(),
            "Set DATABASE_URL to the pilot database endpoint and credentials managed outside the repo.",
        ),
        config_check(
            "object_storage_configuration",
            state.config.object_storage_configuration_status(),
            "Set FWA_OBJECT_STORAGE_URI to the pilot artifact bucket or object storage prefix.",
        ),
        config_check(
            "customer_scope_configuration",
            state.config.customer_scope_configuration_status(),
            "Set FWA_CUSTOMER_SCOPE_ID and matching principal customer scopes for the pilot customer.",
        ),
        config_check(
            "retention_policy_configuration",
            state.config.retention_policy_configuration_status(),
            "Set FWA_RETENTION_POLICY_ID to the approved customer retention policy.",
        ),
        config_check(
            "backup_restore_configuration",
            state.config.backup_restore_configuration_status(),
            "Set FWA_BACKUP_RESTORE_PLAN_ID to the approved backup and restore plan.",
        ),
        config_check(
            "pii_masking_configuration",
            state.config.pii_masking_configuration_status(),
            "Set FWA_PII_MASKING_POLICY_ID to the approved PII masking policy.",
        ),
        config_check(
            "key_rotation_configuration",
            state.config.key_rotation_configuration_status(),
            "Set FWA_KEY_ROTATION_POLICY_ID to the approved key rotation policy.",
        ),
        config_check(
            "network_allowlist_configuration",
            state.config.network_allowlist_configuration_status(),
            "Set FWA_NETWORK_ALLOWLIST_ID after customer network allowlists or private connectivity are approved.",
        ),
        config_check(
            "alert_routing_configuration",
            state.config.alert_routing_configuration_status(),
            "Set FWA_ALERT_ROUTING_POLICY_ID to the approved customer alert routing policy.",
        ),
        config_check(
            "observability_exporter_configuration",
            state.config.observability_exporter_configuration_status(),
            "Set FWA_OBSERVABILITY_EXPORTER_ENDPOINT to the pilot OpenTelemetry collector endpoint.",
        ),
        config_check(
            "agent_policy_configuration",
            state.config.agent_policy_configuration_status(),
            "Set FWA_AGENT_POLICY_ID to the approved Agent tool, evidence, and approval policy.",
        ),
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

fn config_check(
    name: &'static str,
    status: &'static str,
    remediation: &'static str,
) -> HealthCheck {
    HealthCheck {
        name,
        status,
        runtime_kind: None,
        remediation: (status != "configured").then_some(remediation),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_check_includes_remediation_only_for_blockers() {
        let blocked = config_check(
            "api_key_configuration",
            "local_dev_key",
            "Set customer API principals.",
        );
        assert_eq!(blocked.remediation, Some("Set customer API principals."));

        let ready = config_check(
            "api_key_configuration",
            "configured",
            "Set customer API principals.",
        );
        assert_eq!(ready.remediation, None);
    }

    #[test]
    fn pilot_readiness_preserves_blocker_remediation() {
        let checks = vec![config_check(
            "object_storage_configuration",
            "local_demo_object_storage",
            "Set FWA_OBJECT_STORAGE_URI.",
        )];
        let readiness = pilot_readiness(&checks);

        assert_eq!(readiness.status, "not_ready");
        assert_eq!(readiness.blocking_check_count, 1);
        assert_eq!(
            readiness.blocking_checks[0].remediation,
            Some("Set FWA_OBJECT_STORAGE_URI.")
        );
    }
}
