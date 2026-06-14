use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct PersistedScoringRun {
    pub run_id: String,
    pub audit_id: String,
    pub claim_id: String,
    pub source_system: String,
    pub actor_id: String,
    pub risk_score: u8,
    pub rag: String,
    pub risk_level: String,
    pub recommended_action: String,
    pub confidence_score: u8,
    pub confidence: String,
    pub routing_reason: String,
    pub routing_policy: Value,
    pub score_breakdown: Value,
    pub feature_values: Vec<Value>,
    pub rule_runs: Vec<Value>,
    pub model_score: Value,
    pub audit_event: Value,
    pub evidence_refs: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberProfileSummaryRecord {
    pub member_id: String,
    pub claim_count: u32,
    pub policy_count: u32,
    pub total_claim_amount: Decimal,
    pub currency: String,
    pub high_risk_claim_count: u32,
    pub latest_claim_id: Option<String>,
    pub risk_level_summary: String,
    pub profile_summary: String,
    pub evidence_refs: Vec<String>,
}

pub(super) struct MemberProfileSummaryInput {
    pub(super) member_id: String,
    pub(super) claim_count: u32,
    pub(super) policy_count: u32,
    pub(super) total_claim_amount: Decimal,
    pub(super) currency: String,
    pub(super) high_risk_claim_count: u32,
    pub(super) latest_claim_id: Option<String>,
    pub(super) evidence_refs: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub struct PersistedAuditEvent {
    pub audit_id: String,
    pub run_id: String,
    pub claim_id: String,
    pub source_system: String,
    pub actor_id: String,
    pub actor_role: String,
    pub event_type: String,
    pub event_status: String,
    pub summary: String,
    pub payload: Value,
    pub evidence_refs: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct PersistedInboxClaimRun {
    pub run_id: String,
    pub audit_id: String,
    pub external_message_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub external_message_fingerprint: Option<String>,
    pub raw_payload_checksum: String,
    pub raw_payload_ref: Option<String>,
    pub mapping_version: String,
    pub validation_result: String,
    pub scoring_ready: bool,
    pub claim_id: String,
    pub source_system: String,
    pub customer_scope_id: String,
    pub canonical_claim_context: Value,
    pub validation_errors: Value,
    pub data_quality_signals: Value,
    pub evidence_refs: Value,
}

#[derive(Debug, Clone, Default)]
pub struct AuditEventListFilter {
    pub limit: u32,
    pub event_group: Option<String>,
    pub event_type: Option<String>,
    pub actor_id: Option<String>,
    pub run_id: Option<String>,
    pub claim_id: Option<String>,
    pub rule_id: Option<String>,
    pub rule_version: Option<String>,
    pub model_key: Option<String>,
    pub model_version: Option<String>,
    pub routing_policy_id: Option<String>,
    pub routing_policy_version: Option<String>,
    pub review_mode: Option<String>,
    pub feedback_id: Option<String>,
    pub qa_case_id: Option<String>,
    pub sample_id: Option<String>,
    pub agent_run_id: Option<String>,
    pub dataset_id: Option<String>,
    pub feature_set_id: Option<String>,
    pub model_dataset_id: Option<String>,
    pub evaluation_run_id: Option<String>,
    pub has_canonical_trace: Option<bool>,
    pub customer_scope_id: Option<String>,
}
