use fwa_auth::{ApiKeyConfig, ApiKeyPrincipal};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub api_key: String,
    pub source_system: String,
    pub database_url: String,
    pub model_service_url: String,
    pub object_storage_uri: String,
    pub customer_scope_id: String,
    pub retention_policy_id: String,
    pub backup_restore_plan_id: String,
    pub pii_masking_policy_id: String,
    pub key_rotation_policy_id: String,
    pub network_allowlist_id: String,
    pub alert_routing_policy_id: String,
    pub observability_exporter_endpoint: String,
    pub agent_policy_id: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("FWA_API_KEY").unwrap_or_else(|_| "dev-secret".into()),
            source_system: std::env::var("FWA_SOURCE_SYSTEM").unwrap_or_else(|_| "tpa-demo".into()),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/fwa".into()),
            model_service_url: std::env::var("FWA_MODEL_SERVICE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8001".into()),
            object_storage_uri: std::env::var("FWA_OBJECT_STORAGE_URI")
                .unwrap_or_else(|_| "local://demo-artifacts".into()),
            customer_scope_id: std::env::var("FWA_CUSTOMER_SCOPE_ID")
                .unwrap_or_else(|_| "demo-customer".into()),
            retention_policy_id: std::env::var("FWA_RETENTION_POLICY_ID")
                .unwrap_or_else(|_| "demo-retention-policy".into()),
            backup_restore_plan_id: std::env::var("FWA_BACKUP_RESTORE_PLAN_ID")
                .unwrap_or_else(|_| "demo-backup-restore-plan".into()),
            pii_masking_policy_id: std::env::var("FWA_PII_MASKING_POLICY_ID")
                .unwrap_or_else(|_| "demo-pii-masking-policy".into()),
            key_rotation_policy_id: std::env::var("FWA_KEY_ROTATION_POLICY_ID")
                .unwrap_or_else(|_| "demo-key-rotation-policy".into()),
            network_allowlist_id: std::env::var("FWA_NETWORK_ALLOWLIST_ID")
                .unwrap_or_else(|_| "demo-network-allowlist".into()),
            alert_routing_policy_id: std::env::var("FWA_ALERT_ROUTING_POLICY_ID")
                .unwrap_or_else(|_| "demo-alert-routing-policy".into()),
            observability_exporter_endpoint: std::env::var("FWA_OBSERVABILITY_EXPORTER_ENDPOINT")
                .unwrap_or_else(|_| "local://demo-observability".into()),
            agent_policy_id: std::env::var("FWA_AGENT_POLICY_ID")
                .unwrap_or_else(|_| "demo-agent-policy".into()),
        }
    }

    pub fn model_runtime_kind(&self) -> &'static str {
        if self.model_serving_manifest_uri().is_some() {
            "rust_serving_manifest"
        } else if self.model_artifact_uri().is_some() {
            "rust_artifact"
        } else if self.model_service_url == "heuristic"
            || self.model_service_url.starts_with("heuristic://")
        {
            "heuristic"
        } else {
            "python_http"
        }
    }

    pub fn model_service_configuration_status(&self) -> &'static str {
        if self.model_serving_manifest_uri().is_some() {
            "configured"
        } else if self.model_artifact_uri().is_some() {
            "configured"
        } else if self.model_service_url == "heuristic"
            || self.model_service_url.starts_with("heuristic://")
        {
            "heuristic_model_scorer"
        } else if self.model_service_url == "http://127.0.0.1:8001" {
            "local_dev_model_service"
        } else {
            "configured"
        }
    }

    pub fn model_artifact_uri(&self) -> Option<String> {
        configured_env_value("FWA_MODEL_ARTIFACT_URI")
    }

    pub fn model_serving_manifest_uri(&self) -> Option<String> {
        configured_env_value("FWA_MODEL_SERVING_MANIFEST_URI")
    }

    pub fn model_version_lock(&self) -> Option<String> {
        configured_env_value("FWA_MODEL_VERSION_LOCK")
    }

    pub fn model_artifact_sha256(&self) -> Option<String> {
        configured_env_value("FWA_MODEL_ARTIFACT_SHA256")
            .or_else(|| configured_env_value("FWA_MODEL_ARTIFACT_CHECKSUM"))
    }

    pub fn model_artifact_signature(&self) -> Option<String> {
        configured_env_value("FWA_MODEL_ARTIFACT_SIGNATURE")
    }

    pub fn model_signature_key(&self) -> Option<String> {
        configured_env_value("FWA_MODEL_SIGNATURE_KEY")
    }

    pub fn api_key_configuration_status(&self) -> &'static str {
        let principal_specs = api_key_principal_specs();
        api_key_principal_configuration_status(&principal_specs, &self.api_key)
    }

    pub fn source_system_configuration_status(&self) -> &'static str {
        if self.source_system == "tpa-demo" {
            "local_demo_source"
        } else {
            "configured"
        }
    }

    pub fn database_configuration_status(&self) -> &'static str {
        if self.database_url == "postgres://postgres:postgres@localhost:5432/fwa" {
            "local_dev_database"
        } else {
            "configured"
        }
    }

    pub fn object_storage_configuration_status(&self) -> &'static str {
        if self.object_storage_uri == "local://demo-artifacts" {
            "local_demo_object_storage"
        } else {
            "configured"
        }
    }

    pub fn customer_scope_configuration_status(&self) -> &'static str {
        if self.customer_scope_id == "demo-customer" {
            "local_demo_customer_scope"
        } else {
            "configured"
        }
    }

    pub fn retention_policy_configuration_status(&self) -> &'static str {
        if self.retention_policy_id == "demo-retention-policy" {
            "local_demo_retention_policy"
        } else {
            "configured"
        }
    }

    pub fn backup_restore_configuration_status(&self) -> &'static str {
        if self.backup_restore_plan_id == "demo-backup-restore-plan" {
            "local_demo_backup_restore"
        } else {
            "configured"
        }
    }

    pub fn pii_masking_configuration_status(&self) -> &'static str {
        if self.pii_masking_policy_id == "demo-pii-masking-policy" {
            "local_demo_pii_masking"
        } else {
            "configured"
        }
    }

    pub fn key_rotation_configuration_status(&self) -> &'static str {
        if self.key_rotation_policy_id == "demo-key-rotation-policy" {
            "local_demo_key_rotation"
        } else {
            "configured"
        }
    }

    pub fn network_allowlist_configuration_status(&self) -> &'static str {
        if self.network_allowlist_id == "demo-network-allowlist" {
            "local_demo_network_allowlist"
        } else {
            "configured"
        }
    }

    pub fn alert_routing_configuration_status(&self) -> &'static str {
        if self.alert_routing_policy_id == "demo-alert-routing-policy" {
            "local_demo_alert_routing"
        } else {
            "configured"
        }
    }

    pub fn observability_exporter_configuration_status(&self) -> &'static str {
        if self.observability_exporter_endpoint == "local://demo-observability" {
            "local_demo_observability_exporter"
        } else {
            "configured"
        }
    }

    pub fn agent_policy_configuration_status(&self) -> &'static str {
        if self.agent_policy_id == "demo-agent-policy" {
            "local_demo_agent_policy"
        } else {
            "configured"
        }
    }

    pub fn api_key_config(&self) -> ApiKeyConfig {
        self.api_key_config_from_specs(api_key_principal_specs())
    }

    fn api_key_config_from_specs(&self, specs: Vec<String>) -> ApiKeyConfig {
        let has_principal_specs = !specs.is_empty();
        let legacy_key = if has_principal_specs && self.api_key == "dev-secret" {
            String::new()
        } else {
            self.api_key.clone()
        };

        ApiKeyConfig {
            key: legacy_key,
            source_system: self.source_system.clone(),
            customer_scope_id: self.customer_scope_id.clone(),
            principals: specs
                .into_iter()
                .filter_map(|spec| api_key_principal_from_spec(&spec))
                .collect(),
        }
    }
}

fn api_key_principal_specs() -> Vec<String> {
    std::env::var("FWA_API_KEY_PRINCIPALS")
        .unwrap_or_default()
        .split(';')
        .map(str::trim)
        .filter(|spec| !spec.is_empty())
        .map(str::to_string)
        .collect()
}

fn configured_env_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn api_key_principal_configuration_status(specs: &[String], legacy_key: &str) -> &'static str {
    if !specs.is_empty() {
        if specs
            .iter()
            .all(|spec| api_key_principal_from_spec(spec).is_some())
        {
            "configured"
        } else {
            "invalid_api_key_principals"
        }
    } else if legacy_key == "dev-secret" {
        "local_dev_key"
    } else {
        "configured"
    }
}

fn api_key_principal_from_spec(spec: &str) -> Option<ApiKeyPrincipal> {
    let mut parts = spec.split('|').map(str::trim);
    let key = parts.next()?;
    let actor_id = parts.next()?;
    let actor_role = parts.next()?;
    let source_system = parts.next()?;
    let customer_scope_id = parts.next()?;
    let permissions = match parts.next() {
        Some(value) => permissions_from_spec(value),
        None => default_permissions_for_role(actor_role),
    };
    if parts.next().is_some()
        || key.is_empty()
        || actor_id.is_empty()
        || actor_role.is_empty()
        || source_system.is_empty()
        || customer_scope_id.is_empty()
        || permissions.is_empty()
    {
        return None;
    }
    Some(ApiKeyPrincipal {
        key: key.into(),
        actor_id: actor_id.into(),
        actor_role: actor_role.into(),
        source_system: source_system.into(),
        customer_scope_id: customer_scope_id.into(),
        permissions,
    })
}

fn permissions_from_spec(spec: &str) -> Vec<String> {
    spec.split(',')
        .map(str::trim)
        .filter(|permission| !permission.is_empty())
        .map(str::to_string)
        .collect()
}

fn default_permissions_for_role(actor_role: &str) -> Vec<String> {
    match actor_role {
        "tpa_system" => vec!["tpa:*".into()],
        "fwa_operator" => vec!["ops:*".into(), "audit:read".into()],
        "operations_reviewer" => vec!["ops:read".into(), "audit:read".into()],
        "medical_reviewer" => vec!["medical:*".into(), "audit:read".into()],
        "agent" => vec!["agent:*".into()],
        _ => Vec::new(),
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_api_key_principal_spec() {
        let principal = api_key_principal_from_spec(
            "ops-secret|ops-console|fwa_operator|ops-studio|customer-beta",
        )
        .unwrap();

        assert_eq!(principal.key, "ops-secret");
        assert_eq!(principal.actor_id, "ops-console");
        assert_eq!(principal.actor_role, "fwa_operator");
        assert_eq!(principal.source_system, "ops-studio");
        assert_eq!(principal.customer_scope_id, "customer-beta");
        assert_eq!(
            principal.permissions,
            vec!["ops:*".to_string(), "audit:read".to_string()]
        );
    }

    #[test]
    fn parses_api_key_principal_permissions_spec() {
        let principal = api_key_principal_from_spec(
            "ops-secret|ops-console|fwa_operator|ops-studio|customer-beta|ops:rules:publish,audit:read",
        )
        .unwrap();

        assert_eq!(
            principal.permissions,
            vec!["ops:rules:publish".to_string(), "audit:read".to_string()]
        );
    }

    #[test]
    fn rejects_malformed_api_key_principal_spec() {
        assert!(api_key_principal_from_spec("ops-secret|ops-console").is_none());
        assert!(api_key_principal_from_spec(
            "ops-secret|ops-console|fwa_operator|ops-studio|customer-beta|audit:read|extra",
        )
        .is_none());
        assert!(api_key_principal_from_spec(
            "ops-secret|ops-console|unknown_role|ops-studio|customer-beta",
        )
        .is_none());
    }

    #[test]
    fn api_key_configuration_status_rejects_any_malformed_principal() {
        let mut config = AppConfig::from_env();
        config.api_key = "customer-pilot-secret".into();

        assert_eq!(
            api_key_principal_configuration_status(
                &[
                    "ops-secret|ops-console|fwa_operator|ops-studio|customer-beta".into(),
                    "broken|entry".into(),
                ],
                &config.api_key,
            ),
            "invalid_api_key_principals"
        );
    }

    #[test]
    fn api_key_config_disables_default_dev_key_when_principals_are_configured() {
        let config = AppConfig {
            api_key: "dev-secret".into(),
            source_system: "tpa-demo".into(),
            database_url: "postgres://postgres:postgres@localhost:5432/fwa".into(),
            model_service_url: "http://127.0.0.1:8001".into(),
            object_storage_uri: "local://demo-artifacts".into(),
            customer_scope_id: "demo-customer".into(),
            retention_policy_id: "demo-retention-policy".into(),
            backup_restore_plan_id: "demo-backup-restore-plan".into(),
            pii_masking_policy_id: "demo-pii-masking-policy".into(),
            key_rotation_policy_id: "demo-key-rotation-policy".into(),
            network_allowlist_id: "demo-network-allowlist".into(),
            alert_routing_policy_id: "demo-alert-routing-policy".into(),
            observability_exporter_endpoint: "local://demo-observability".into(),
            agent_policy_id: "demo-agent-policy".into(),
        };

        let api_key_config = config.api_key_config_from_specs(vec![
            "ops-secret|ops-console|fwa_operator|ops-studio|customer-beta".into(),
        ]);

        assert_eq!(api_key_config.key, "");
        assert_eq!(api_key_config.principals.len(), 1);
    }

    #[test]
    fn rust_artifact_model_runtime_counts_as_configured_model_service() {
        let previous_artifact_uri = std::env::var_os("FWA_MODEL_ARTIFACT_URI");
        std::env::set_var(
            "FWA_MODEL_ARTIFACT_URI",
            "s3://customer-models/baseline_fwa/model.json",
        );
        let config = AppConfig {
            api_key: "dev-secret".into(),
            source_system: "tpa-demo".into(),
            database_url: "postgres://postgres:postgres@localhost:5432/fwa".into(),
            model_service_url: "http://127.0.0.1:8001".into(),
            object_storage_uri: "local://demo-artifacts".into(),
            customer_scope_id: "demo-customer".into(),
            retention_policy_id: "demo-retention-policy".into(),
            backup_restore_plan_id: "demo-backup-restore-plan".into(),
            pii_masking_policy_id: "demo-pii-masking-policy".into(),
            key_rotation_policy_id: "demo-key-rotation-policy".into(),
            network_allowlist_id: "demo-network-allowlist".into(),
            alert_routing_policy_id: "demo-alert-routing-policy".into(),
            observability_exporter_endpoint: "local://demo-observability".into(),
            agent_policy_id: "demo-agent-policy".into(),
        };

        assert_eq!(config.model_runtime_kind(), "rust_artifact");
        assert_eq!(config.model_service_configuration_status(), "configured");

        if let Some(previous_artifact_uri) = previous_artifact_uri {
            std::env::set_var("FWA_MODEL_ARTIFACT_URI", previous_artifact_uri);
        } else {
            std::env::remove_var("FWA_MODEL_ARTIFACT_URI");
        }
    }

    #[test]
    fn api_key_config_keeps_custom_legacy_principal_and_adds_configured_principals() {
        let config = AppConfig {
            api_key: "legacy-secret".into(),
            source_system: "tpa-demo".into(),
            database_url: "postgres://postgres:postgres@localhost:5432/fwa".into(),
            model_service_url: "http://127.0.0.1:8001".into(),
            object_storage_uri: "local://demo-artifacts".into(),
            customer_scope_id: "demo-customer".into(),
            retention_policy_id: "demo-retention-policy".into(),
            backup_restore_plan_id: "demo-backup-restore-plan".into(),
            pii_masking_policy_id: "demo-pii-masking-policy".into(),
            key_rotation_policy_id: "demo-key-rotation-policy".into(),
            network_allowlist_id: "demo-network-allowlist".into(),
            alert_routing_policy_id: "demo-alert-routing-policy".into(),
            observability_exporter_endpoint: "local://demo-observability".into(),
            agent_policy_id: "demo-agent-policy".into(),
        };

        let api_key_config = config.api_key_config_from_specs(vec![
            "ops-secret|ops-console|fwa_operator|ops-studio|customer-beta".into(),
        ]);

        assert_eq!(api_key_config.key, "legacy-secret");
        assert_eq!(api_key_config.source_system, "tpa-demo");
        assert_eq!(api_key_config.principals.len(), 1);
        assert_eq!(api_key_config.principals[0].actor_role, "fwa_operator");
        assert_eq!(
            api_key_config.principals[0].customer_scope_id,
            "customer-beta"
        );
    }
}
