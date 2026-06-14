use serde::{Deserialize, Serialize};
use serde_json::Value;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfileWindowUpsertInput {
    pub provider_id: String,
    pub specialty: Option<String>,
    pub network_status: Option<String>,
    pub windows: Vec<Value>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveProviderProfileWindowsInput {
    pub customer_scope_id: String,
    pub source_report_uri: String,
    pub as_of_date: String,
    pub submitted_by: String,
    pub notes: String,
    pub provider_profiles: Vec<ProviderProfileWindowUpsertInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfileWindowRecord {
    pub customer_scope_id: String,
    pub provider_id: String,
    pub specialty: Option<String>,
    pub network_status: Option<String>,
    pub as_of_date: String,
    pub windows: Vec<Value>,
    pub evidence_refs: Vec<String>,
    pub source_report_uri: String,
    pub submitted_by: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderGraphSignalUpsertInput {
    pub provider_id: String,
    pub high_risk_neighbor_ratio: Option<f64>,
    pub provider_patient_overlap_score: Option<f64>,
    pub referral_concentration_score: Option<f64>,
    pub billing_ring_membership: bool,
    pub temporal_co_billing_frequency_7d: f64,
    pub referral_concentration_entropy: Option<f64>,
    pub shared_member_provider_count: usize,
    pub connected_confirmed_fwa_count: Option<u32>,
    pub network_component_risk_score: Option<u8>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveProviderGraphSignalsInput {
    pub customer_scope_id: String,
    pub source_report_uri: String,
    pub as_of_date: String,
    pub submitted_by: String,
    pub notes: String,
    pub provider_relationships: Vec<ProviderGraphSignalUpsertInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderGraphSignalRecord {
    pub customer_scope_id: String,
    pub provider_id: String,
    pub as_of_date: String,
    pub high_risk_neighbor_ratio: Option<f64>,
    pub provider_patient_overlap_score: Option<f64>,
    pub referral_concentration_score: Option<f64>,
    pub billing_ring_membership: bool,
    pub temporal_co_billing_frequency_7d: f64,
    pub referral_concentration_entropy: Option<f64>,
    pub shared_member_provider_count: usize,
    pub connected_confirmed_fwa_count: Option<u32>,
    pub network_component_risk_score: Option<u8>,
    pub evidence_refs: Vec<String>,
    pub source_report_uri: String,
    pub submitted_by: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerBenchmarkGroupUpsertInput {
    pub peer_group_key: String,
    pub specialty: String,
    pub region: String,
    pub service_segment: String,
    pub claim_count: usize,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub p99: f64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavePeerBenchmarkGroupsInput {
    pub customer_scope_id: String,
    pub source_report_uri: String,
    pub benchmark_month: String,
    pub submitted_by: String,
    pub notes: String,
    pub peer_groups: Vec<PeerBenchmarkGroupUpsertInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerBenchmarkGroupRecord {
    pub customer_scope_id: String,
    pub peer_group_key: String,
    pub specialty: String,
    pub region: String,
    pub service_segment: String,
    pub benchmark_month: String,
    pub claim_count: usize,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub p99: f64,
    pub evidence_refs: Vec<String>,
    pub source_report_uri: String,
    pub submitted_by: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeRollupUpsertInput {
    pub episode_key: String,
    pub member_id: String,
    pub provider_id: String,
    pub windows: Vec<Value>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveEpisodeRollupsInput {
    pub customer_scope_id: String,
    pub source_report_uri: String,
    pub as_of_date: String,
    pub submitted_by: String,
    pub notes: String,
    pub episodes: Vec<EpisodeRollupUpsertInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeRollupRecord {
    pub customer_scope_id: String,
    pub episode_key: String,
    pub member_id: String,
    pub provider_id: String,
    pub as_of_date: String,
    pub windows: Vec<Value>,
    pub evidence_refs: Vec<String>,
    pub source_report_uri: String,
    pub submitted_by: String,
    pub notes: String,
}
