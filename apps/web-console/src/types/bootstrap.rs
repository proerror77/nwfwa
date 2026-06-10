use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct HistoricalBackfillListResponse {
    pub(crate) jobs: Vec<HistoricalBackfillJob>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct HistoricalBackfillResponse {
    pub(crate) job: HistoricalBackfillJob,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct HistoricalBackfillJob {
    pub(crate) job_id: String,
    pub(crate) status: String,
    pub(crate) dataset_refs: Vec<String>,
    pub(crate) rule_refs: Vec<String>,
    pub(crate) candidate_count: u32,
    pub(crate) leads: Vec<HistoricalBackfillLead>,
    pub(crate) reviewer: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct HistoricalBackfillLead {
    pub(crate) lead_id: String,
    pub(crate) claim_id: String,
    pub(crate) scheme_family: String,
    pub(crate) risk_score: u8,
    pub(crate) rag: String,
    pub(crate) status: String,
    pub(crate) reason: String,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceRequestListResponse {
    pub(crate) requests: Vec<EvidenceRequestRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceRequestGenerateResponse {
    pub(crate) requests: Vec<EvidenceRequestRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceRequestRecord {
    pub(crate) request_id: String,
    pub(crate) claim_id: String,
    pub(crate) scoring_audit_id: String,
    pub(crate) status: String,
    pub(crate) request_reason: String,
    pub(crate) missing_evidence: Vec<String>,
    pub(crate) items: Vec<EvidenceRequestItem>,
    pub(crate) reviewer_queue: String,
    pub(crate) requested_by: String,
    pub(crate) notes: Option<String>,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
    pub(crate) updated_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EvidenceRequestItem {
    pub(crate) item_id: String,
    pub(crate) document_type: String,
    pub(crate) status: String,
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) blocking: bool,
    pub(crate) policy_authority_ref: Option<String>,
    pub(crate) exception_check: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct LabelBootstrapQueueResponse {
    pub(crate) items: Vec<LabelBootstrapItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct LabelBootstrapReviewResponse {
    pub(crate) item: LabelBootstrapItem,
    pub(crate) audit_id: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct LabelBootstrapItem {
    pub(crate) item_id: String,
    pub(crate) claim_id: String,
    pub(crate) source_type: String,
    pub(crate) source_id: String,
    pub(crate) suggested_label_name: String,
    pub(crate) suggested_label_value: String,
    pub(crate) governance_status: String,
    pub(crate) training_eligible: bool,
    pub(crate) review_status: String,
    pub(crate) review_audit_id: Option<String>,
    pub(crate) reviewer: Option<String>,
    pub(crate) feedback_target: String,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BootstrapOpsSnapshot {
    pub(crate) backfills: Vec<HistoricalBackfillJob>,
    pub(crate) evidence_requests: Vec<EvidenceRequestRecord>,
    pub(crate) label_items: Vec<LabelBootstrapItem>,
}
