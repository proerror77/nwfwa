mod config_model;
mod config_validation;

use fwa_auth::ApiKeyConfig;

use config_validation::{api_key_principal_from_spec, api_key_principal_specs};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub api_key: String,
    pub api_key_principals: Vec<String>,
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
        let config = Self {
            api_key: std::env::var("FWA_API_KEY").unwrap_or_else(|_| "dev-secret".into()),
            api_key_principals: api_key_principal_specs(),
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
        };
        config.validate_environment();
        config
    }

    pub fn api_key_config(&self) -> ApiKeyConfig {
        self.api_key_config_from_specs(self.api_key_principals.clone())
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

impl Default for AppConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config_validation::{api_key_principal_configuration_status, api_key_principal_from_spec};
    use std::sync::{Mutex, MutexGuard, OnceLock};

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
        let api_key = "customer-pilot-secret".to_string();

        assert_eq!(
            api_key_principal_configuration_status(
                &[
                    "ops-secret|ops-console|fwa_operator|ops-studio|customer-beta".into(),
                    "broken|entry".into(),
                ],
                &api_key,
            ),
            "invalid_api_key_principals"
        );
    }

    #[test]
    fn api_key_config_disables_default_dev_key_when_principals_are_configured() {
        let config = AppConfig {
            api_key: "dev-secret".into(),
            api_key_principals: vec![],
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
    fn rejects_default_dev_key_outside_development() {
        let _guard = env_guard();
        let previous_env = std::env::var_os("FWA_ENV");
        let previous_api_key = std::env::var_os("FWA_API_KEY");
        let previous_principals = std::env::var_os("FWA_API_KEY_PRINCIPALS");
        std::env::set_var("FWA_ENV", "production");
        std::env::remove_var("FWA_API_KEY");
        std::env::remove_var("FWA_API_KEY_PRINCIPALS");

        let result = std::panic::catch_unwind(AppConfig::from_env);

        restore_env("FWA_ENV", previous_env);
        restore_env("FWA_API_KEY", previous_api_key);
        restore_env("FWA_API_KEY_PRINCIPALS", previous_principals);

        let panic_message = result
            .unwrap_err()
            .downcast::<String>()
            .map(|message| *message)
            .unwrap_or_else(|payload| {
                payload
                    .downcast::<&'static str>()
                    .map(|message| (*message).to_string())
                    .unwrap_or_default()
            });
        assert!(panic_message
            .contains("FWA_API_KEY or FWA_API_KEY_PRINCIPALS must be set outside development"));
    }

    #[test]
    fn allows_principal_map_outside_development_without_legacy_key() {
        let _guard = env_guard();
        let previous_env = std::env::var_os("FWA_ENV");
        let previous_api_key = std::env::var_os("FWA_API_KEY");
        let previous_principals = std::env::var_os("FWA_API_KEY_PRINCIPALS");
        std::env::set_var("FWA_ENV", "production");
        std::env::remove_var("FWA_API_KEY");
        std::env::set_var(
            "FWA_API_KEY_PRINCIPALS",
            "ops-secret|ops-console|fwa_operator|ops-studio|customer-beta",
        );

        let config = AppConfig::from_env();

        assert_eq!(config.api_key, "dev-secret");
        assert_eq!(config.api_key_config().key, "");

        restore_env("FWA_ENV", previous_env);
        restore_env("FWA_API_KEY", previous_api_key);
        restore_env("FWA_API_KEY_PRINCIPALS", previous_principals);
    }

    #[test]
    fn rejects_heuristic_model_scorer_outside_development() {
        let _guard = env_guard();
        let previous_env = std::env::var_os("FWA_ENV");
        let previous_api_key = std::env::var_os("FWA_API_KEY");
        let previous_principals = std::env::var_os("FWA_API_KEY_PRINCIPALS");
        let previous_model_service_url = std::env::var_os("FWA_MODEL_SERVICE_URL");
        std::env::set_var("FWA_ENV", "production");
        std::env::remove_var("FWA_API_KEY");
        std::env::set_var(
            "FWA_API_KEY_PRINCIPALS",
            "ops-secret|ops-console|fwa_operator|ops-studio|customer-beta",
        );
        std::env::set_var("FWA_MODEL_SERVICE_URL", "heuristic://local");

        let result = std::panic::catch_unwind(AppConfig::from_env);

        restore_env("FWA_ENV", previous_env);
        restore_env("FWA_API_KEY", previous_api_key);
        restore_env("FWA_API_KEY_PRINCIPALS", previous_principals);
        restore_env("FWA_MODEL_SERVICE_URL", previous_model_service_url);

        let panic_message = result
            .unwrap_err()
            .downcast::<String>()
            .map(|message| *message)
            .unwrap_or_else(|payload| {
                payload
                    .downcast::<&'static str>()
                    .map(|message| (*message).to_string())
                    .unwrap_or_default()
            });
        assert!(panic_message.contains(
            "FWA_MODEL_SERVICE_URL must point to a customer-approved scorer outside development"
        ));
    }

    #[test]
    fn rust_artifact_model_runtime_counts_as_configured_model_service() {
        let _guard = env_guard();
        let previous_artifact_uri = std::env::var_os("FWA_MODEL_ARTIFACT_URI");
        std::env::set_var(
            "FWA_MODEL_ARTIFACT_URI",
            "s3://customer-models/baseline_fwa/model.json",
        );
        let config = AppConfig {
            api_key: "dev-secret".into(),
            api_key_principals: vec![],
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
            api_key_principals: vec![],
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

    fn restore_env(name: &str, value: Option<std::ffi::OsString>) {
        if let Some(value) = value {
            std::env::set_var(name, value);
        } else {
            std::env::remove_var(name);
        }
    }

    fn env_guard() -> MutexGuard<'static, ()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("config env test lock poisoned")
    }
}
