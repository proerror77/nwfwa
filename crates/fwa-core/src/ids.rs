use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use ulid::Ulid;

macro_rules! id_type {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new() -> Self {
                Self(format!("{}_{}", $prefix, Ulid::new()))
            }

            pub fn from_external(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id_type!(ClaimId, "claim");
id_type!(MemberId, "member");
id_type!(PolicyId, "policy");
id_type!(ProviderId, "provider");
id_type!(ScoringRunId, "run");
id_type!(AuditEventId, "aud");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_run_id_has_expected_prefix() {
        let id = ScoringRunId::new();
        assert!(id.as_str().starts_with("run_"));
    }

    #[test]
    fn external_claim_id_is_preserved() {
        let id = ClaimId::from_external("CLM-0287");
        assert_eq!(id.as_str(), "CLM-0287");
    }
}
