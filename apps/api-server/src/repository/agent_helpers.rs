use super::{
    evidence_values_to_strings, AgentAuditEventRecord, AgentInvestigationRecord,
    AgentRegistryRecord, AgentRunLogRecord, AuditEventId, PersistedAgentRun,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(super) const DEFAULT_AGENT_IDENTITY_ID: &str = "agent_identity:deterministic_investigator:v1";
pub(super) const DEFAULT_AGENT_KIND: &str = "deterministic_investigator";
pub(super) const DEFAULT_AGENT_VERSION: u32 = 1;
pub(super) const DEFAULT_ORCHESTRATOR_VERSION: &str = "deterministic_orchestrator_v1";

pub(super) fn agent_run_log_from_persisted(run: &PersistedAgentRun) -> AgentRunLogRecord {
    AgentRunLogRecord {
        agent_run_id: run.agent_run_id.clone(),
        investigation_id: run.investigation_id.clone(),
        agent_identity_id: DEFAULT_AGENT_IDENTITY_ID.into(),
        agent_kind: DEFAULT_AGENT_KIND.into(),
        agent_version: DEFAULT_AGENT_VERSION,
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

pub(super) fn agent_audit_event_from_run(
    run: &PersistedAgentRun,
    investigation_id: &str,
    previous_event_hash: Option<String>,
) -> AgentAuditEventRecord {
    let audit_event_id = AuditEventId::new().to_string();
    let findings_count = run
        .output_json
        .get("findings")
        .and_then(Value::as_array)
        .map_or(run.steps.len(), Vec::len);
    let evidence_sufficiency = run
        .output_json
        .get("evidence_sufficiency")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let human_review_required = run
        .approvals
        .iter()
        .any(|approval| approval.proposed_action == "manual_review_required");
    let input_digest = sha256_json(&serde_json::json!({
        "investigation_id": run.investigation_id,
        "agent_run_id": run.agent_run_id,
        "claim_id": run.claim_id,
        "decision_boundary": run.decision_boundary,
        "context_snapshot_checksums": run
            .context_snapshots
            .iter()
            .map(|snapshot| snapshot.checksum.clone())
            .collect::<Vec<_>>(),
        "tool_call_ids": run
            .tool_calls
            .iter()
            .map(|call| call.tool_call_id.clone())
            .collect::<Vec<_>>(),
    }));
    let payload = serde_json::json!({
        "investigation_id": run.investigation_id,
        "agent_identity_id": DEFAULT_AGENT_IDENTITY_ID,
        "status": run.status,
        "evidence_ref_count": run.evidence_refs.len(),
        "context_snapshot_count": run.context_snapshots.len(),
        "policy_check_count": run.policy_checks.len(),
        "tool_result_count": run.tool_results.len(),
        "approval_count": run.approvals.len(),
    });
    let event_hash = sha256_json(&serde_json::json!({
        "audit_event_id": audit_event_id,
        "investigation_id": investigation_id,
        "agent_run_id": run.agent_run_id,
        "agent_identity_id": DEFAULT_AGENT_IDENTITY_ID,
        "agent_kind": DEFAULT_AGENT_KIND,
        "agent_version": DEFAULT_AGENT_VERSION,
        "action_type": "run_completed",
        "input_digest": input_digest,
        "decision_boundary": run.decision_boundary,
        "findings_count": findings_count,
        "evidence_sufficiency": evidence_sufficiency,
        "tool_call_count": run.tool_calls.len(),
        "human_review_required": human_review_required,
        "previous_event_hash": previous_event_hash,
        "payload": payload,
    }));

    AgentAuditEventRecord {
        audit_event_id,
        investigation_id: investigation_id.into(),
        agent_run_id: run.agent_run_id.clone(),
        agent_kind: DEFAULT_AGENT_KIND.into(),
        agent_version: DEFAULT_AGENT_VERSION,
        actor_id: "agent-case-investigator".into(),
        actor_role: "agent".into(),
        action_type: "run_completed".into(),
        input_digest,
        decision_boundary: run.decision_boundary.clone(),
        findings_count,
        evidence_sufficiency,
        tool_call_count: run.tool_calls.len(),
        human_review_required,
        phi_fields_accessed: vec![
            "claim_id".into(),
            "risk_score".into(),
            "rag".into(),
            "diagnosis_code".into(),
            "provider_region".into(),
        ],
        payload,
        previous_event_hash,
        event_hash,
    }
}

pub(super) fn default_agent_registry_record() -> AgentRegistryRecord {
    AgentRegistryRecord {
        agent_identity_id: DEFAULT_AGENT_IDENTITY_ID.into(),
        agent_kind: DEFAULT_AGENT_KIND.into(),
        agent_version: DEFAULT_AGENT_VERSION,
        capability_scope: vec![
            "knowledge.search_similar".into(),
            "agent.investigation.package".into(),
        ],
        phi_fields_allowed: default_agent_phi_fields_allowed(),
        status: "active".into(),
    }
}

pub(super) fn agent_investigation_record(
    claim_id: &str,
    investigation_id: &str,
) -> AgentInvestigationRecord {
    AgentInvestigationRecord {
        investigation_id: investigation_id.into(),
        claim_id: claim_id.into(),
        status: "open".into(),
        orchestrator_version: DEFAULT_ORCHESTRATOR_VERSION.into(),
    }
}

pub(super) fn stable_investigation_id_for_claim(claim_id: &str) -> String {
    let digest = Sha256::digest(claim_id.trim().as_bytes());
    format!("investigation:{digest:x}")
}

fn default_agent_phi_fields_allowed() -> Vec<String> {
    vec![
        "claim_id".into(),
        "risk_score".into(),
        "rag".into(),
        "diagnosis_code".into(),
        "provider_region".into(),
    ]
}

fn sha256_json(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_else(|_| value.to_string().into_bytes());
    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_audit_event_uses_digest_without_raw_output_payload() {
        let run = PersistedAgentRun {
            investigation_id: "investigation_test_01HX".into(),
            agent_run_id: "agent_01HX".into(),
            claim_id: "CLM-0287".into(),
            status: "succeeded".into(),
            decision_boundary: "assistive_only".into(),
            output_json: serde_json::json!({
                "findings": [{"finding": "peer outlier"}],
                "evidence_sufficiency": "sufficient"
            }),
            evidence_refs: vec![Value::String("agent_run:agent_01HX".into())],
            steps: vec![],
            context_snapshots: vec![],
            policy_checks: vec![],
            tool_calls: vec![],
            tool_results: vec![],
            approvals: vec![],
        };

        let investigation_id = run.investigation_id.clone();
        let event =
            agent_audit_event_from_run(&run, &investigation_id, Some("sha256:previous".into()));

        assert!(event.input_digest.starts_with("sha256:"));
        assert_eq!(event.investigation_id, investigation_id);
        assert_eq!(event.agent_kind, DEFAULT_AGENT_KIND);
        assert_eq!(event.findings_count, 1);
        assert_eq!(
            event.previous_event_hash.as_deref(),
            Some("sha256:previous")
        );
        assert!(!event.payload.to_string().contains("CLM-0287"));
    }
}
