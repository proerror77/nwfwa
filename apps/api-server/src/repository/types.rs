use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use super::types_agents::*;
pub use super::types_cases::*;
pub use super::types_core::*;
pub use super::types_dashboard::*;
pub use super::types_datasets::*;
pub use super::types_evidence::*;
pub use super::types_knowledge::*;
pub use super::types_models::*;
pub use super::types_rules::*;

pub(super) const GOVERNANCE_AUDIT_EVENT_TYPES: &[&str] = &[
    "dataset.registered",
    "dataset.field_mapping.added",
    "feature_set.registered",
    "model_dataset.registered",
    "model_evaluation.registered",
    "rule.candidate.saved",
    "rule.status.changed",
    "rule.rollback.completed",
    "rule.promotion.reviewed",
    "model.promotion.reviewed",
    "model.activation.completed",
    "model.rollback.completed",
    "agent.approval.decided",
    "audit_sample.created",
    "qa.feedback.status.updated",
    "routing_policy.candidate.saved",
    "routing_policy.status.changed",
    "routing_policy.activation.completed",
    "routing_policy.rollback.completed",
    "evidence.document.registered",
    "evidence.document_chunk.registered",
    "evidence.ocr_output.registered",
    "evidence.embedding_job.registered",
    "evidence.retrieval_audit.recorded",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationResultRecord {
    pub investigation_id: String,
    pub case_id: Option<String>,
    pub claim_id: String,
    pub outcome: String,
    pub confirmed_fwa: bool,
    pub financial_impact_type: Option<String>,
    pub saving_amount: Option<Decimal>,
    pub currency: Option<String>,
    pub notes: String,
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub customer_scope_id: Option<String>,
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub actor_role: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct SavingAttributionRecord {
    pub(super) attribution_id: String,
    pub(super) claim_id: String,
    pub(super) investigation_id: String,
    pub(super) source_type: String,
    pub(super) source_id: String,
    pub(super) financial_impact_type: String,
    pub(super) action: String,
    pub(super) saving_amount: Decimal,
    pub(super) currency: String,
    pub(super) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct AuditSampleStrataContext {
    pub(super) provider_type: String,
    pub(super) provider_region: String,
    pub(super) policy_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaReviewRecord {
    pub qa_case_id: String,
    pub claim_id: String,
    pub qa_conclusion: String,
    pub issue_type: String,
    pub feedback_target: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub customer_scope_id: Option<String>,
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub actor_role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaFeedbackItemRecord {
    pub feedback_id: String,
    pub qa_case_id: String,
    pub claim_id: String,
    pub feedback_target: String,
    pub issue_type: String,
    pub qa_conclusion: String,
    pub source: String,
    pub status: String,
    pub priority: String,
    pub summary: String,
    pub note_present: bool,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
    pub status_updated_by: Option<String>,
    pub status_audit_id: Option<String>,
    pub status_updated_at: Option<String>,
    pub status_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct QaFeedbackStatusUpdate {
    pub(super) status: String,
    pub(super) actor_id: Option<String>,
    pub(super) audit_id: String,
    pub(super) updated_at: Option<String>,
    pub(super) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateQaFeedbackStatusInput {
    pub status: String,
    pub actor_id: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_deserializing)]
    pub customer_scope_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateQaFeedbackStatusRecord {
    pub item: QaFeedbackItemRecord,
    pub audit_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeLabelRecord {
    pub label_id: String,
    pub claim_id: String,
    pub label_name: String,
    pub label_value: String,
    pub source_type: String,
    pub source_id: String,
    pub governance_status: String,
    pub feedback_target: String,
    pub currency: Option<String>,
    pub evidence_refs: Vec<String>,
}

pub fn canonical_feedback_target(feedback_target: &str) -> &str {
    match feedback_target {
        "models" => "model",
        value => value,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditHistoryEventRecord {
    pub audit_id: String,
    pub run_id: String,
    pub actor_role: String,
    pub event_type: String,
    pub event_status: String,
    pub summary: String,
    pub payload: Value,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEventRecord {
    pub event_id: String,
    pub event_type: String,
    pub source_event_type: String,
    pub source_audit_id: String,
    pub customer_scope_id: String,
    pub claim_id: String,
    pub run_id: String,
    pub delivery_status: String,
    pub retry_count: u32,
    pub max_attempts: u32,
    pub next_attempt_at: Option<String>,
    pub last_attempt_at: Option<String>,
    pub last_response_status_code: Option<u16>,
    pub last_error_message: Option<String>,
    pub idempotency_key: String,
    pub signature_key_id: String,
    pub signature_algorithm: String,
    pub signature_base_string: String,
    pub payload: Value,
    pub evidence_refs: Vec<String>,
    pub occurred_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDeliveryAttemptInput {
    pub event_id: String,
    pub delivery_status: String,
    pub response_status_code: Option<u16>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDeliveryAttemptRecord {
    pub event_id: String,
    pub attempt_number: u32,
    pub delivery_status: String,
    pub response_status_code: Option<u16>,
    pub error_message: Option<String>,
    pub next_attempt_at: Option<String>,
    pub attempted_at: Option<String>,
}
