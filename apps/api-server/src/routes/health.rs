use crate::app::AppState;
use axum::extract::State;
use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
    pub checks: Vec<HealthCheck>,
}

#[derive(Debug, Serialize)]
pub struct HealthCheck {
    pub name: &'static str,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_kind: Option<&'static str>,
}

pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "api-server",
        version: env!("CARGO_PKG_VERSION"),
        checks: vec![
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
        ],
    })
}
