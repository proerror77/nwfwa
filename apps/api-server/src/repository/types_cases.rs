use serde::{Deserialize, Serialize};
use serde_json::Value;

fn default_review_mode() -> String {
    "pre_payment".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadRecord {
    pub lead_id: String,
    pub run_id: String,
    pub claim_id: String,
    pub member_id: String,
    pub provider_id: String,
    pub source_system: String,
    #[serde(default = "default_review_mode")]
    pub review_mode: String,
    pub scheme_family: String,
    pub lead_source: String,
    pub status: String,
    pub disposition: String,
    pub risk_score: u8,
    pub rag: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseRecord {
    pub case_id: String,
    pub lead_id: String,
    pub claim_id: String,
    pub member_id: String,
    pub provider_id: String,
    pub source_system: String,
    #[serde(default = "default_review_mode")]
    pub review_mode: String,
    pub scheme_family: String,
    pub lead_source: String,
    pub status: String,
    pub assignee: String,
    pub reviewer: String,
    pub priority: String,
    pub routing_reason: String,
    pub evidence_package: Value,
    pub sla_target_hours: u32,
    pub sla_status: String,
    pub time_to_triage_hours: f64,
    pub time_to_closure_hours: Option<f64>,
    pub final_outcome: Option<String>,
    pub reviewer_notes: Option<String>,
    pub investigation_result_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageLeadInput {
    pub decision: String,
    pub merge_target_lead_id: Option<String>,
    pub assignee: String,
    pub reviewer: String,
    pub priority: String,
    pub notes: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_deserializing)]
    pub customer_scope_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageLeadRecord {
    pub lead: LeadRecord,
    pub case: Option<CaseRecord>,
    pub audit_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCaseStatusInput {
    pub status: String,
    pub actor_id: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_deserializing)]
    pub customer_scope_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCaseStatusRecord {
    pub case: CaseRecord,
    pub audit_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSampleLeadRecord {
    pub lead_id: String,
    pub claim_id: String,
    pub scheme_family: String,
    #[serde(default = "default_review_mode")]
    pub review_mode: String,
    #[serde(default)]
    pub provider_id: String,
    #[serde(default)]
    pub provider_type: String,
    #[serde(default)]
    pub provider_region: String,
    #[serde(default)]
    pub policy_type: String,
    #[serde(default)]
    pub risk_band: String,
    #[serde(default)]
    pub strata_key: String,
    #[serde(default)]
    pub prior_reviewer_sample_count: u32,
    pub risk_score: u8,
    pub rag: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuditSampleInput {
    pub sample_mode: String,
    pub population_definition: String,
    pub inclusion_criteria: Value,
    pub deterministic_seed: Option<String>,
    pub sample_size: usize,
    pub reviewer: String,
    pub assignment_queue: String,
    #[serde(default, skip_deserializing)]
    pub customer_scope_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSampleRecord {
    pub sample_id: String,
    pub customer_scope_id: String,
    pub sample_mode: String,
    pub population_definition: String,
    pub inclusion_criteria: Value,
    pub deterministic_seed: Option<String>,
    pub selection_method: String,
    pub sample_size: usize,
    pub reviewer: String,
    pub assignment_queue: String,
    pub selected_leads: Vec<AuditSampleLeadRecord>,
    pub outcome_distribution: Value,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct AuditSampleStrataContext {
    pub(super) provider_type: String,
    pub(super) provider_region: String,
    pub(super) policy_type: String,
}
