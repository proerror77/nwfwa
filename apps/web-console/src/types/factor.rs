use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct FactorReadinessResponse {
    pub(crate) dataset_count: u32,
    pub(crate) factor_count: u32,
    pub(crate) data_quality_score: f64,
    pub(crate) data_quality_status: String,
    pub(crate) online_ready_count: u32,
    pub(crate) rule_convertible_count: u32,
    pub(crate) ready_factor_count: u32,
    pub(crate) review_factor_count: u32,
    pub(crate) scheme_readiness: Vec<FactorSchemeReadiness>,
    pub(crate) factor_cards: Vec<FactorCard>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct FactorSchemeReadiness {
    pub(crate) scheme_family: String,
    pub(crate) factor_count: u32,
    pub(crate) ready_factor_count: u32,
    pub(crate) review_factor_count: u32,
    pub(crate) online_ready_count: u32,
    pub(crate) rule_convertible_count: u32,
    pub(crate) readiness_issue_counts: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct FactorCard {
    pub(crate) dataset_key: String,
    pub(crate) factor_name: String,
    pub(crate) scheme_family: String,
    pub(crate) chinese_name: String,
    pub(crate) entity_type: String,
    pub(crate) business_meaning: String,
    pub(crate) readiness_status: String,
    pub(crate) owner: String,
    pub(crate) online_available: bool,
    pub(crate) rule_convertible: bool,
}
