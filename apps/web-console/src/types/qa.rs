use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AuditSampleListResponse {
    pub(crate) samples: Vec<AuditSampleRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AuditSampleRecord {
    pub(crate) sample_id: String,
    pub(crate) sample_mode: String,
    pub(crate) population_definition: String,
    pub(crate) inclusion_criteria: Value,
    pub(crate) deterministic_seed: Option<String>,
    pub(crate) selection_method: String,
    pub(crate) sample_size: usize,
    pub(crate) reviewer: String,
    pub(crate) assignment_queue: String,
    pub(crate) selected_leads: Vec<AuditSampleLead>,
    pub(crate) outcome_distribution: Value,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AuditSampleLead {
    pub(crate) lead_id: String,
    pub(crate) claim_id: String,
    pub(crate) scheme_family: String,
    pub(crate) review_mode: String,
    pub(crate) provider_id: String,
    pub(crate) provider_type: String,
    pub(crate) provider_region: String,
    pub(crate) policy_type: String,
    pub(crate) risk_band: String,
    pub(crate) strata_key: String,
    pub(crate) prior_reviewer_sample_count: u32,
    pub(crate) risk_score: u8,
    pub(crate) rag: String,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct QaQueueListResponse {
    pub(crate) items: Vec<QaQueueItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct QaQueueItem {
    pub(crate) qa_case_id: String,
    pub(crate) sample_id: String,
    pub(crate) lead_id: String,
    pub(crate) claim_id: String,
    pub(crate) scheme_family: String,
    pub(crate) rag: String,
    pub(crate) risk_score: u8,
    pub(crate) reviewer: String,
    pub(crate) assignment_queue: String,
    pub(crate) status: String,
    pub(crate) qa_conclusion: Option<String>,
    pub(crate) issue_type: Option<String>,
    pub(crate) feedback_target: Option<String>,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) canonical_source_refs: Vec<String>,
    pub(crate) canonical_evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct QaQueueSummary {
    pub(crate) open_count: u32,
    pub(crate) in_progress_count: u32,
    pub(crate) resolved_count: u32,
    pub(crate) dismissed_count: u32,
    pub(crate) unresolved_count: u32,
    pub(crate) rules_feedback_count: u32,
    pub(crate) models_feedback_count: u32,
    pub(crate) features_feedback_count: u32,
    pub(crate) provider_profile_feedback_count: u32,
    pub(crate) workflow_feedback_count: u32,
    pub(crate) tpa_feedback_count: u32,
    pub(crate) high_priority_count: u32,
    pub(crate) evidence_backed_count: u32,
    pub(crate) highest_priority: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct QaFeedbackItemListResponse {
    pub(crate) items: Vec<QaFeedbackItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct QaFeedbackItem {
    pub(crate) feedback_id: String,
    pub(crate) qa_case_id: String,
    pub(crate) claim_id: String,
    pub(crate) feedback_target: String,
    pub(crate) issue_type: String,
    pub(crate) qa_conclusion: String,
    pub(crate) source: String,
    pub(crate) status: String,
    pub(crate) priority: String,
    pub(crate) summary: String,
    pub(crate) note_present: bool,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
    pub(crate) status_updated_by: Option<String>,
    pub(crate) status_audit_id: Option<String>,
    pub(crate) status_updated_at: Option<String>,
    pub(crate) status_evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct QaReviewSnapshot {
    pub(crate) queue: Vec<QaQueueItem>,
    pub(crate) summary: QaQueueSummary,
    pub(crate) feedback_items: Vec<QaFeedbackItem>,
}
