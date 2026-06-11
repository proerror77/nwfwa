use serde::Deserialize;
use serde_json::Value;

/// Combined data for the force-directed network graph
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GraphNetworkData {
    pub(crate) leads: Vec<LeadRecord>,
    pub(crate) providers: Vec<crate::types::ProviderRiskItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct LeadListResponse {
    pub(crate) leads: Vec<LeadRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct LeadRecord {
    pub(crate) lead_id: String,
    pub(crate) run_id: String,
    pub(crate) claim_id: String,
    pub(crate) member_id: String,
    pub(crate) provider_id: String,
    pub(crate) source_system: String,
    pub(crate) review_mode: String,
    pub(crate) scheme_family: String,
    pub(crate) lead_source: String,
    pub(crate) status: String,
    pub(crate) disposition: String,
    pub(crate) risk_score: u8,
    pub(crate) rag: String,
    pub(crate) reason: String,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct CaseListResponse {
    pub(crate) cases: Vec<CaseRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct CaseRecord {
    pub(crate) case_id: String,
    pub(crate) lead_id: String,
    pub(crate) claim_id: String,
    pub(crate) member_id: String,
    pub(crate) provider_id: String,
    pub(crate) source_system: String,
    pub(crate) review_mode: String,
    pub(crate) scheme_family: String,
    pub(crate) lead_source: String,
    pub(crate) status: String,
    pub(crate) assignee: String,
    pub(crate) reviewer: String,
    pub(crate) priority: String,
    pub(crate) routing_reason: String,
    pub(crate) evidence_package: Value,
    pub(crate) sla_target_hours: u32,
    pub(crate) sla_status: String,
    pub(crate) time_to_triage_hours: f64,
    pub(crate) time_to_closure_hours: Option<f64>,
    pub(crate) final_outcome: Option<String>,
    pub(crate) reviewer_notes: Option<String>,
    pub(crate) investigation_result_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct TriageLeadRecord {
    pub(crate) lead: LeadRecord,
    pub(crate) case: Option<CaseRecord>,
    pub(crate) audit_id: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct UpdateCaseStatusRecord {
    pub(crate) case: CaseRecord,
    pub(crate) audit_id: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct PilotWritebackResponse {
    pub(crate) claim_id: String,
    pub(crate) event_type: String,
    pub(crate) event_status: String,
    pub(crate) audit_id: String,
    pub(crate) run_id: String,
    pub(crate) idempotency_key: String,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LeadsCasesSnapshot {
    pub(crate) leads: Vec<LeadRecord>,
    pub(crate) cases: Vec<CaseRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct MedicalReviewQueueResponse {
    pub(crate) items: Vec<MedicalReviewQueueItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct MedicalReviewQueueItem {
    pub(crate) claim_id: String,
    pub(crate) run_id: String,
    pub(crate) audit_id: String,
    pub(crate) medical_reasonableness_score: u8,
    pub(crate) review_route: String,
    pub(crate) evidence_status: String,
    pub(crate) missing_evidence: Vec<String>,
    pub(crate) item_finding_count: u32,
    pub(crate) first_item_code: Option<String>,
    pub(crate) first_issue_type: Option<String>,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) canonical_source_refs: Vec<String>,
    pub(crate) canonical_evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
    pub(crate) review_status: String,
    pub(crate) review_audit_id: Option<String>,
    pub(crate) review_decision: Option<String>,
    pub(crate) reviewer: Option<String>,
    pub(crate) reviewed_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct MedicalReviewResultResponse {
    pub(crate) claim_id: String,
    pub(crate) event_type: String,
    pub(crate) event_status: String,
    pub(crate) audit_id: String,
    pub(crate) run_id: String,
    pub(crate) review_status: String,
    pub(crate) clinical_outcomes: Vec<String>,
    pub(crate) evidence_refs: Vec<String>,
}

/// All data needed for the 7-layer investigation workbench.
/// Loaded when a case is selected.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InvestigationContext {
    pub(crate) case: CaseRecord,
    pub(crate) lead: Option<LeadRecord>,
    pub(crate) member: Option<crate::types::MemberProfileSummary>,
    pub(crate) providers: Vec<crate::types::ProviderRiskItem>,
    pub(crate) audit_events: Vec<crate::types::AuditEventRecord>,
    pub(crate) similar_cases: Vec<SimilarCaseItem>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub(crate) struct ClaimAuditHistoryResponse {
    pub(crate) claim_id: String,
    pub(crate) events: Vec<crate::types::AuditEventRecord>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub(crate) struct SimilarCaseItem {
    pub(crate) case_id: String,
    pub(crate) scheme_family: String,
    pub(crate) similarity_score: f64,
    pub(crate) final_outcome: Option<String>,
    pub(crate) tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub(crate) struct SimilarCasesResponse {
    pub(crate) cases: Vec<SimilarCaseItem>,
}
