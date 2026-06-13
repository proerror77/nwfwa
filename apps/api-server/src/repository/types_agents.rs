use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct PersistedAgentRun {
    pub agent_run_id: String,
    pub claim_id: String,
    pub status: String,
    pub decision_boundary: String,
    pub output_json: Value,
    pub evidence_refs: Vec<Value>,
    pub steps: Vec<Value>,
    pub context_snapshots: Vec<AgentContextSnapshotRecord>,
    pub policy_checks: Vec<AgentPolicyCheckRecord>,
    pub tool_calls: Vec<AgentToolCallRecord>,
    pub tool_results: Vec<AgentToolResultRecord>,
    pub approvals: Vec<AgentApprovalRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistryRecord {
    pub agent_identity_id: String,
    pub agent_kind: String,
    pub agent_version: u32,
    pub capability_scope: Vec<String>,
    pub phi_fields_allowed: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInvestigationRecord {
    pub investigation_id: String,
    pub claim_id: String,
    pub status: String,
    pub orchestrator_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAuditEventRecord {
    pub audit_event_id: String,
    pub investigation_id: String,
    pub agent_run_id: String,
    pub agent_kind: String,
    pub agent_version: u32,
    pub actor_id: String,
    pub actor_role: String,
    pub action_type: String,
    pub input_digest: String,
    pub decision_boundary: String,
    pub findings_count: usize,
    pub evidence_sufficiency: String,
    pub tool_call_count: usize,
    pub human_review_required: bool,
    pub phi_fields_accessed: Vec<String>,
    pub payload: Value,
    pub previous_event_hash: Option<String>,
    pub event_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunLogRecord {
    pub agent_run_id: String,
    pub investigation_id: String,
    pub agent_identity_id: String,
    pub agent_kind: String,
    pub agent_version: u32,
    pub claim_id: String,
    pub status: String,
    pub decision_boundary: String,
    pub output_json: Value,
    pub evidence_refs: Vec<String>,
    pub steps: Vec<Value>,
    pub context_snapshots: Vec<AgentContextSnapshotRecord>,
    pub policy_checks: Vec<AgentPolicyCheckRecord>,
    pub tool_calls: Vec<AgentToolCallRecord>,
    pub tool_results: Vec<AgentToolResultRecord>,
    pub approvals: Vec<AgentApprovalRecord>,
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContextSnapshotRecord {
    pub snapshot_id: String,
    pub redaction_status: String,
    pub context_json: Value,
    pub source_refs: Vec<String>,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCallRecord {
    pub tool_call_id: String,
    pub tool_name: String,
    pub status: String,
    pub input_json: Value,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPolicyCheckRecord {
    pub policy_check_id: String,
    pub agent_run_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    pub policy_name: String,
    pub decision: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolResultRecord {
    pub tool_result_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    pub status: String,
    pub output_json: Value,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentApprovalRecord {
    pub approval_id: String,
    pub agent_run_id: String,
    pub proposed_action: String,
    pub decision: String,
    pub approver: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}
