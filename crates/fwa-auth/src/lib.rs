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
}

pub fn validate_api_key(
    provided_key: Option<&str>,
    config: &ApiKeyConfig,
) -> Result<ActorContext, AuthError> {
    match provided_key {
        Some(value) if value == config.key => Ok(ActorContext {
            actor_id: config.source_system.clone(),
            actor_role: "tpa_system".into(),
            source_system: config.source_system.clone(),
        }),
        _ => Err(AuthError::InvalidApiKey),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_key_returns_actor_context() {
        let config = ApiKeyConfig {
            key: "secret".into(),
            source_system: "tpa-demo".into(),
        };
        let actor = validate_api_key(Some("secret"), &config).unwrap();
        assert_eq!(actor.source_system, "tpa-demo");
    }

    #[test]
    fn invalid_key_is_rejected() {
        let config = ApiKeyConfig {
            key: "secret".into(),
            source_system: "tpa-demo".into(),
        };
        assert_eq!(
            validate_api_key(Some("wrong"), &config).unwrap_err(),
            AuthError::InvalidApiKey
        );
    }
}
