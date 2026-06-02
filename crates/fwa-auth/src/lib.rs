use fwa_audit::ActorContext;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuthError {
    #[error("invalid api key")]
    InvalidApiKey,
}

#[derive(Debug, Clone)]
pub struct ApiKeyConfig {
    pub key: String,
    pub source_system: String,
    pub customer_scope_id: String,
    pub principals: Vec<ApiKeyPrincipal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiKeyPrincipal {
    pub key: String,
    pub actor_id: String,
    pub actor_role: String,
    pub source_system: String,
    pub customer_scope_id: String,
}

pub fn validate_api_key(
    provided_key: Option<&str>,
    config: &ApiKeyConfig,
) -> Result<ActorContext, AuthError> {
    if let Some(value) = provided_key {
        if let Some(principal) = config
            .principals
            .iter()
            .find(|principal| principal.key == value)
        {
            return Ok(ActorContext {
                actor_id: principal.actor_id.clone(),
                actor_role: principal.actor_role.clone(),
                source_system: principal.source_system.clone(),
                customer_scope_id: principal.customer_scope_id.clone(),
            });
        }
        if value == config.key {
            return Ok(ActorContext {
                actor_id: config.source_system.clone(),
                actor_role: "tpa_system".into(),
                source_system: config.source_system.clone(),
                customer_scope_id: config.customer_scope_id.clone(),
            });
        }
    }
    Err(AuthError::InvalidApiKey)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_key_returns_actor_context() {
        let config = ApiKeyConfig {
            key: "secret".into(),
            source_system: "tpa-demo".into(),
            customer_scope_id: "customer-alpha".into(),
            principals: Vec::new(),
        };
        let actor = validate_api_key(Some("secret"), &config).unwrap();
        assert_eq!(actor.source_system, "tpa-demo");
        assert_eq!(actor.customer_scope_id, "customer-alpha");
        assert_eq!(actor.actor_role, "tpa_system");
    }

    #[test]
    fn valid_principal_key_returns_principal_context() {
        let config = ApiKeyConfig {
            key: "legacy-secret".into(),
            source_system: "tpa-demo".into(),
            customer_scope_id: "customer-alpha".into(),
            principals: vec![ApiKeyPrincipal {
                key: "ops-secret".into(),
                actor_id: "ops-console".into(),
                actor_role: "fwa_operator".into(),
                source_system: "ops-studio".into(),
                customer_scope_id: "customer-beta".into(),
            }],
        };

        let actor = validate_api_key(Some("ops-secret"), &config).unwrap();

        assert_eq!(actor.actor_id, "ops-console");
        assert_eq!(actor.actor_role, "fwa_operator");
        assert_eq!(actor.source_system, "ops-studio");
        assert_eq!(actor.customer_scope_id, "customer-beta");
    }

    #[test]
    fn empty_legacy_key_does_not_accept_dev_secret() {
        let config = ApiKeyConfig {
            key: String::new(),
            source_system: "tpa-demo".into(),
            customer_scope_id: "customer-alpha".into(),
            principals: vec![ApiKeyPrincipal {
                key: "ops-secret".into(),
                actor_id: "ops-console".into(),
                actor_role: "fwa_operator".into(),
                source_system: "ops-studio".into(),
                customer_scope_id: "customer-beta".into(),
            }],
        };

        assert_eq!(
            validate_api_key(Some("dev-secret"), &config).unwrap_err(),
            AuthError::InvalidApiKey
        );
    }

    #[test]
    fn invalid_key_is_rejected() {
        let config = ApiKeyConfig {
            key: "secret".into(),
            source_system: "tpa-demo".into(),
            customer_scope_id: "customer-alpha".into(),
            principals: Vec::new(),
        };
        assert_eq!(
            validate_api_key(Some("wrong"), &config).unwrap_err(),
            AuthError::InvalidApiKey
        );
    }
}
