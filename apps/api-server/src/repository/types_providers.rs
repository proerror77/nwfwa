use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSanctionUpsertInput {
    pub sanction_key: String,
    pub list: String,
    pub provider_id: Option<String>,
    pub npi: Option<String>,
    pub provider_name: String,
    pub sanction_type: Option<String>,
    pub effective_date: Option<String>,
    pub source_ref: Option<String>,
    pub risk_feature: String,
    pub risk_score: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveProviderSanctionsInput {
    pub customer_scope_id: String,
    pub source_report_uri: String,
    pub submitted_by: String,
    pub notes: String,
    pub provider_upserts: Vec<ProviderSanctionUpsertInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSanctionRecord {
    pub customer_scope_id: String,
    pub sanction_key: String,
    pub list: String,
    pub provider_id: Option<String>,
    pub npi: Option<String>,
    pub provider_name: String,
    pub sanction_type: Option<String>,
    pub effective_date: Option<String>,
    pub source_ref: Option<String>,
    pub risk_feature: String,
    pub risk_score: u8,
    pub source_report_uri: String,
    pub submitted_by: String,
    pub notes: String,
}
