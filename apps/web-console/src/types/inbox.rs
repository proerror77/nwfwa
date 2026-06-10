use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CorrectionHint {
    pub(crate) field_path: String,
    pub(crate) severity: String,
    pub(crate) blocks_scoring: bool,
    pub(crate) next_action: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct MemberProfileSummary {
    pub(crate) member_id: String,
    pub(crate) claim_count: u32,
    pub(crate) policy_count: u32,
    pub(crate) total_claim_amount: Value,
    pub(crate) currency: String,
    pub(crate) high_risk_claim_count: u32,
    pub(crate) latest_claim_id: Option<String>,
    pub(crate) risk_level_summary: String,
    pub(crate) profile_summary: String,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ProviderRiskSummary {
    pub(crate) provider_count: u32,
    pub(crate) review_required_count: u32,
    pub(crate) high_risk_count: u32,
    pub(crate) providers: Vec<ProviderRiskItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ProviderRiskItem {
    pub(crate) provider_id: String,
    pub(crate) risk_score: u8,
    pub(crate) risk_tier: String,
    pub(crate) review_required: bool,
    pub(crate) review_route: String,
    pub(crate) claim_count: u32,
    pub(crate) specialty: Option<String>,
    pub(crate) network_status: Option<String>,
    pub(crate) review_failure_count: u32,
    pub(crate) confirmed_fwa_count: u32,
    pub(crate) false_positive_count: u32,
    pub(crate) network_risk_score: Option<u8>,
    pub(crate) latest_claim_id: Option<String>,
    pub(crate) outlier_flags: Vec<String>,
    pub(crate) graph_reasons: Vec<String>,
    pub(crate) evidence_refs: Vec<String>,
}
