use serde::{Deserialize, Serialize};
use serde_json::Value;

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
