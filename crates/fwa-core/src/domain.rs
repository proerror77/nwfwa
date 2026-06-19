use crate::{ClaimId, MemberId, Money, PolicyId, ProviderId};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActorContext {
    pub actor_id: String,
    pub actor_role: String,
    pub source_system: String,
    pub customer_scope_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub id: MemberId,
    pub external_member_id: String,
    pub dob: Option<NaiveDate>,
    pub gender: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: PolicyId,
    pub external_policy_id: String,
    pub member_id: MemberId,
    pub product_code: String,
    pub coverage_start_date: NaiveDate,
    pub coverage_end_date: NaiveDate,
    pub coverage_limit: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: ProviderId,
    pub external_provider_id: String,
    pub name: String,
    pub provider_type: String,
    pub region: String,
    pub risk_tier: ProviderRiskTier,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderRiskTier {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: ClaimId,
    pub external_claim_id: String,
    pub member_id: MemberId,
    pub policy_id: PolicyId,
    pub provider_id: ProviderId,
    /// Primary diagnosis code (ICD-10 or scheme-specific).  Required; used
    /// as the single-code key in knowledge search, unbundling, and DB indexes.
    pub diagnosis_code: String,
    /// Additional secondary diagnosis codes.  Optional; empty in legacy/demo
    /// payloads.  Checked alongside `diagnosis_code` in rule conditions and
    /// clinical evidence matching to avoid false negatives on multi-diagnosis
    /// claims.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnosis_codes: Vec<String>,
    pub service_date: NaiveDate,
    pub amount: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimItem {
    pub item_code: String,
    pub item_type: String,
    pub description: String,
    pub quantity: u32,
    pub unit_amount: Money,
    pub total_amount: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimContext {
    pub claim: Claim,
    pub items: Vec<ClaimItem>,
    pub member: Member,
    pub policy: Policy,
    pub provider: Provider,
}
