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
        }
    }

    pub fn model_runtime_kind(&self) -> &'static str {
        if self.model_service_url == "heuristic"
            || self.model_service_url.starts_with("heuristic://")
        {
            "heuristic"
        } else {
            "python_http"
        }
    }

    pub fn model_service_configuration_status(&self) -> &'static str {
        if self.model_service_url == "heuristic"
            || self.model_service_url.starts_with("heuristic://")
        {
            "heuristic_model_scorer"
        } else if self.model_service_url == "http://127.0.0.1:8001" {
            "local_dev_model_service"
        } else {
            "configured"
        }
    }

    pub fn api_key_configuration_status(&self) -> &'static str {
        if self.api_key == "dev-secret" {
            "local_dev_key"
        } else {
            "configured"
        }
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
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
