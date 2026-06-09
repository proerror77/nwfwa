use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateHistoricalBackfillRequest {
    pub job_id: Option<String>,
    #[serde(default)]
    pub dataset_refs: Vec<String>,
    #[serde(default)]
    pub rule_refs: Vec<String>,
    pub reviewer: Option<String>,
    pub notes: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct HistoricalBackfillResponse {
    pub job: HistoricalBackfillJobRecord,
}

#[derive(Debug, Serialize)]
pub struct HistoricalBackfillListResponse {
    pub jobs: Vec<HistoricalBackfillJobRecord>,
}

#[derive(Debug, Serialize)]
pub struct HistoricalBackfillLeadResponse {
    pub job_id: String,
    pub leads: Vec<HistoricalBackfillLeadRecord>,
}

#[derive(Debug, Serialize, Clone)]
pub struct HistoricalBackfillJobRecord {
    pub job_id: String,
    pub status: String,
    pub dataset_refs: Vec<String>,
    pub rule_refs: Vec<String>,
    pub candidate_count: u32,
    pub leads: Vec<HistoricalBackfillLeadRecord>,
    pub reviewer: Option<String>,
    pub notes: Option<String>,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct HistoricalBackfillLeadRecord {
    pub lead_id: String,
    pub claim_id: String,
    pub scheme_family: String,
    pub risk_score: u8,
    pub rag: String,
    pub status: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateEvidenceRequestsRequest {
    pub claim_id: Option<String>,
    pub scoring_audit_id: Option<String>,
    pub requested_by: Option<String>,
    pub reviewer_queue: Option<String>,
    pub notes: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEvidenceRequestStatusRequest {
    pub status: String,
    pub actor_id: String,
    pub notes: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EvidenceRequestListResponse {
    pub requests: Vec<EvidenceRequestRecord>,
}

#[derive(Debug, Serialize)]
pub struct EvidenceRequestGenerateResponse {
    pub requests: Vec<EvidenceRequestRecord>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EvidenceRequestRecord {
    pub request_id: String,
    pub claim_id: String,
    pub scoring_audit_id: String,
    pub status: String,
    pub request_reason: String,
    pub missing_evidence: Vec<String>,
    pub items: Vec<EvidenceRequestItemRecord>,
    pub reviewer_queue: String,
    pub requested_by: String,
    pub notes: Option<String>,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EvidenceRequestItemRecord {
    pub item_id: String,
    pub document_type: String,
    pub status: String,
    pub reason: String,
    pub blocking: bool,
    pub policy_authority_ref: Option<String>,
    pub exception_check: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewLabelBootstrapItemRequest {
    pub reviewer: String,
    pub label_name: String,
    pub label_value: String,
    pub governance_status: String,
    pub feedback_target: String,
    pub notes: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LabelBootstrapQueueResponse {
    pub items: Vec<LabelBootstrapItemRecord>,
}

#[derive(Debug, Serialize, Clone)]
pub struct LabelBootstrapItemRecord {
    pub item_id: String,
    pub claim_id: String,
    pub source_type: String,
    pub source_id: String,
    pub suggested_label_name: String,
    pub suggested_label_value: String,
    pub governance_status: String,
    pub training_eligible: bool,
    pub review_status: String,
    pub review_audit_id: Option<String>,
    pub reviewer: Option<String>,
    pub feedback_target: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LabelBootstrapReviewResponse {
    pub item: LabelBootstrapItemRecord,
    pub audit_id: String,
}
