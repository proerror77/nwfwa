use crate::repository::{OutcomeLabelRecord, QaFeedbackItemRecord, WebhookEventRecord};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct PilotWritebackResponse {
    pub claim_id: String,
    pub event_type: String,
    pub event_status: String,
    pub audit_id: String,
    pub run_id: String,
    pub idempotency_key: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ClaimAuditHistoryResponse {
    pub claim_id: String,
    pub events: Vec<crate::repository::AuditHistoryEventRecord>,
}

#[derive(Debug, Serialize)]
pub struct WebhookEventListResponse {
    pub events: Vec<WebhookEventRecord>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitWebhookDeliveryAttemptRequest {
    pub delivery_status: String,
    pub response_status_code: Option<u16>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpsAlertRecord {
    pub alert_id: String,
    pub alert_type: String,
    pub severity: String,
    pub status: String,
    pub claim_id: String,
    pub lead_id: Option<String>,
    pub case_id: Option<String>,
    pub scheme_family: String,
    pub message: String,
    pub recommended_action: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct OpsAlertListResponse {
    pub alerts: Vec<OpsAlertRecord>,
}

#[derive(Debug, Serialize)]
pub struct QaFeedbackItemListResponse {
    pub items: Vec<QaFeedbackItemRecord>,
}

#[derive(Debug, Default, Deserialize)]
pub struct QaFeedbackItemListQuery {
    pub status: Option<String>,
    pub feedback_target: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QaQueueItemResponse {
    pub qa_case_id: String,
    pub sample_id: String,
    pub lead_id: String,
    pub claim_id: String,
    pub scheme_family: String,
    pub rag: String,
    pub risk_score: u8,
    pub reviewer: String,
    pub assignment_queue: String,
    pub status: String,
    pub qa_conclusion: Option<String>,
    pub issue_type: Option<String>,
    pub feedback_target: Option<String>,
    pub evidence_refs: Vec<String>,
    pub canonical_source_refs: Vec<String>,
    pub canonical_evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct QaQueueListResponse {
    pub items: Vec<QaQueueItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct QaQueueSummaryResponse {
    pub open_count: u32,
    pub in_progress_count: u32,
    pub resolved_count: u32,
    pub dismissed_count: u32,
    pub unresolved_count: u32,
    pub rules_feedback_count: u32,
    pub models_feedback_count: u32,
    pub features_feedback_count: u32,
    pub provider_profile_feedback_count: u32,
    pub workflow_feedback_count: u32,
    pub tpa_feedback_count: u32,
    pub high_priority_count: u32,
    pub evidence_backed_count: u32,
    pub highest_priority: String,
}

#[derive(Debug, Serialize)]
pub struct OutcomeLabelListResponse {
    pub labels: Vec<OutcomeLabelRecord>,
}
