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
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuthenticatedPrincipal {
    pub actor: ActorContext,
    pub permissions: Vec<String>,
}

impl AuthenticatedPrincipal {
    pub fn has_permission(&self, required: &str) -> bool {
        self.permissions.iter().any(|permission| {
            permission == "*"
                || permission == required
                || permission
                    .strip_suffix(":*")
                    .is_some_and(|prefix| required.starts_with(&format!("{prefix}:")))
        })
    }
}

pub fn validate_api_key(
    provided_key: Option<&str>,
    config: &ApiKeyConfig,
) -> Result<ActorContext, AuthError> {
    authenticate_api_key(provided_key, config).map(|principal| principal.actor)
}

pub fn authenticate_api_key(
    provided_key: Option<&str>,
    config: &ApiKeyConfig,
) -> Result<AuthenticatedPrincipal, AuthError> {
    if let Some(value) = provided_key {
        if let Some(principal) = config
            .principals
            .iter()
            .find(|principal| constant_time_eq(&principal.key, value))
        {
            return Ok(AuthenticatedPrincipal {
                actor: ActorContext {
                    actor_id: principal.actor_id.clone(),
                    actor_role: principal.actor_role.clone(),
                    source_system: principal.source_system.clone(),
                    customer_scope_id: principal.customer_scope_id.clone(),
                },
                permissions: principal.permissions.clone(),
            });
        }
        if constant_time_eq(value, &config.key) {
            return Ok(AuthenticatedPrincipal {
                actor: ActorContext {
                    actor_id: config.source_system.clone(),
                    actor_role: "tpa_system".into(),
                    source_system: config.source_system.clone(),
                    customer_scope_id: config.customer_scope_id.clone(),
                },
                permissions: vec!["*".into()],
            });
        }
    }
    Err(AuthError::InvalidApiKey)
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut diff = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        let left_byte = left.get(index).copied().unwrap_or(0);
        let right_byte = right.get(index).copied().unwrap_or(0);
        diff |= (left_byte ^ right_byte) as usize;
    }
    diff == 0
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
                permissions: vec!["ops:*".into()],
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
                permissions: vec!["ops:*".into()],
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

    #[test]
    fn api_key_comparison_requires_exact_full_match() {
        assert!(constant_time_eq("secret", "secret"));
        assert!(!constant_time_eq("secret", "secre"));
        assert!(!constant_time_eq("secret", "secret-extra"));
        assert!(!constant_time_eq("secret", "secRet"));
    }

    #[test]
    fn authenticated_principal_matches_exact_and_wildcard_permissions() {
        let principal = AuthenticatedPrincipal {
            actor: ActorContext {
                actor_id: "ops-console".into(),
                actor_role: "fwa_operator".into(),
                source_system: "ops-studio".into(),
                customer_scope_id: "customer-beta".into(),
            },
            permissions: vec!["audit:read".into(), "ops:*".into()],
        };

        assert!(principal.has_permission("audit:read"));
        assert!(principal.has_permission("ops:rules:publish"));
        assert!(!principal.has_permission("claims:score"));
    }

    #[test]
    fn authenticated_principal_matches_tpa_namespace_permissions() {
        let principal = AuthenticatedPrincipal {
            actor: ActorContext {
                actor_id: "customer-tpa".into(),
                actor_role: "tpa_system".into(),
                source_system: "customer-claims-system".into(),
                customer_scope_id: "customer-alpha".into(),
            },
            permissions: vec!["tpa:*".into(), "audit:read".into()],
        };

        assert!(principal.has_permission("tpa:claims:score"));
        assert!(principal.has_permission("tpa:inbox:normalize"));
        assert!(principal.has_permission("tpa:investigations:write"));
        assert!(principal.has_permission("tpa:qa:write"));
        assert!(principal.has_permission("tpa:audit:read"));
        assert!(!principal.has_permission("ops:rules:publish"));
    }

    #[test]
    fn legacy_key_gets_compatibility_wildcard_permission() {
        let config = ApiKeyConfig {
            key: "legacy-secret".into(),
            source_system: "tpa-demo".into(),
            customer_scope_id: "customer-alpha".into(),
            principals: Vec::new(),
        };

        let principal = authenticate_api_key(Some("legacy-secret"), &config).unwrap();

        assert!(principal.has_permission("claims:score"));
        assert_eq!(principal.actor.actor_role, "tpa_system");
    }
}
