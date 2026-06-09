use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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
