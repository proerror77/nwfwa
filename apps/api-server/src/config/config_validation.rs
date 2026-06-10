use fwa_auth::ApiKeyPrincipal;

use super::AppConfig;

impl AppConfig {
    pub fn validate_environment(&self) {
        let env = std::env::var("FWA_ENV").unwrap_or_else(|_| "development".into());
        let principal_specs = api_key_principal_specs();
        if env != "development" && self.api_key == "dev-secret" && principal_specs.is_empty() {
            panic!("FWA_API_KEY or FWA_API_KEY_PRINCIPALS must be set outside development");
        }
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
}

pub(super) fn api_key_principal_specs() -> Vec<String> {
    std::env::var("FWA_API_KEY_PRINCIPALS")
        .unwrap_or_default()
        .split(';')
        .map(str::trim)
        .filter(|spec| !spec.is_empty())
        .map(str::to_string)
        .collect()
}

pub(super) fn api_key_principal_configuration_status(
    specs: &[String],
    legacy_key: &str,
) -> &'static str {
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

pub(super) fn api_key_principal_from_spec(spec: &str) -> Option<ApiKeyPrincipal> {
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
