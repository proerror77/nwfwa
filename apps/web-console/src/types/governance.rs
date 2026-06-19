use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AuditEventListResponse {
    pub(crate) events: Vec<AuditEventRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AuditEventRecord {
    pub(crate) audit_id: String,
    pub(crate) run_id: String,
    pub(crate) event_type: String,
    pub(crate) event_status: String,
    pub(crate) summary: String,
    pub(crate) payload: Value,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentRunListResponse {
    pub(crate) runs: Vec<AgentRunRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentRunRecord {
    pub(crate) agent_run_id: String,
    pub(crate) investigation_id: String,
    pub(crate) claim_id: String,
    pub(crate) status: String,
    pub(crate) decision_boundary: String,
    pub(crate) output_json: Value,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) steps: Vec<Value>,
    pub(crate) context_snapshots: Vec<Value>,
    pub(crate) policy_checks: Vec<Value>,
    pub(crate) tool_calls: Vec<Value>,
    pub(crate) tool_results: Vec<Value>,
    pub(crate) approvals: Vec<AgentApprovalView>,
    pub(crate) created_at: Option<String>,
    pub(crate) completed_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentApprovalView {
    pub(crate) approval_id: String,
    pub(crate) proposed_action: String,
    pub(crate) decision: String,
    pub(crate) approver: String,
    pub(crate) reason: String,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentInvestigationResponse {
    pub(crate) investigation_id: String,
    pub(crate) agent_run_id: String,
    pub(crate) decision_boundary: String,
    pub(crate) risk_summary: String,
    pub(crate) findings: Vec<AgentInvestigationFinding>,
    pub(crate) investigation_checklist: Vec<String>,
    pub(crate) similar_cases: Vec<AgentInvestigationSimilarCase>,
    pub(crate) qa_opinion_draft: String,
    pub(crate) evidence_sufficiency: AgentEvidenceSufficiency,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) evidence_refs_by_type: AgentEvidenceBuckets,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentInvestigationFinding {
    pub(crate) finding: String,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentInvestigationSimilarCase {
    pub(crate) case_id: String,
    pub(crate) similarity_score: f64,
    pub(crate) matched_signals: Vec<String>,
    pub(crate) provenance_refs: Vec<String>,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentEvidenceSufficiency {
    pub(crate) scheme_family: String,
    pub(crate) status: String,
    pub(crate) minimum_evidence: Vec<String>,
    pub(crate) present_evidence: Vec<String>,
    pub(crate) missing_evidence: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AgentEvidenceBuckets {
    pub(crate) claim: Vec<String>,
    pub(crate) rule: Vec<String>,
    pub(crate) model: Vec<String>,
    pub(crate) anomaly: Vec<String>,
    pub(crate) document: Vec<String>,
    pub(crate) similar_case: Vec<String>,
}
