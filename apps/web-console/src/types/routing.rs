use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RoutingPolicyListResponse {
    pub(crate) policies: Vec<RoutingPolicyRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RoutingPolicyRecord {
    pub(crate) policy_id: String,
    pub(crate) version: u32,
    pub(crate) review_mode: String,
    pub(crate) status: String,
    pub(crate) owner: String,
    pub(crate) risk_thresholds: RoutingRiskThresholds,
    pub(crate) confidence_thresholds: RoutingConfidenceThresholds,
    pub(crate) provider_review_threshold: u8,
    pub(crate) activated_at: Option<String>,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RoutingRiskThresholds {
    pub(crate) low_max: u8,
    pub(crate) medium_min: u8,
    pub(crate) high_min: u8,
    pub(crate) critical_min: u8,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RoutingConfidenceThresholds {
    pub(crate) low_confidence_below: u8,
    pub(crate) high_confidence_min: u8,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RoutingPolicyPromotionGates {
    pub(crate) policy_id: String,
    pub(crate) version: u32,
    pub(crate) review_mode: String,
    pub(crate) status: String,
    pub(crate) decision: String,
    pub(crate) passed_count: u32,
    pub(crate) total_count: u32,
    pub(crate) gates: Vec<RoutingPolicyPromotionGate>,
    pub(crate) blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RoutingPolicyPromotionGate {
    pub(crate) label: String,
    pub(crate) passed: bool,
    pub(crate) blocker: String,
    pub(crate) evidence_source: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RoutingPolicySnapshot {
    pub(crate) policies: Vec<RoutingPolicyRecord>,
    pub(crate) gates: RoutingPolicyPromotionGates,
}
