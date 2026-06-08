use super::{evidence_values_to_strings, AgentRunLogRecord, PersistedAgentRun};

pub(super) fn agent_run_log_from_persisted(run: &PersistedAgentRun) -> AgentRunLogRecord {
    AgentRunLogRecord {
        agent_run_id: run.agent_run_id.clone(),
        claim_id: run.claim_id.clone(),
        status: run.status.clone(),
        decision_boundary: run.decision_boundary.clone(),
        output_json: run.output_json.clone(),
        evidence_refs: evidence_values_to_strings(&run.evidence_refs),
        steps: run.steps.clone(),
        context_snapshots: run.context_snapshots.clone(),
        policy_checks: run.policy_checks.clone(),
        tool_calls: run.tool_calls.clone(),
        tool_results: run.tool_results.clone(),
        approvals: run.approvals.clone(),
        created_at: None,
        completed_at: None,
    }
}
