use super::*;

impl PostgresScoringRepository {
    pub(super) async fn load_agent_tool_calls(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<Vec<AgentToolCallRecord>> {
        let rows: Vec<(String, String, String, Value, Value)> = sqlx::query_as(
            "SELECT tool_call_id, tool_name, status, input_json, evidence_refs
             FROM tool_calls
             WHERE agent_run_id = $1
             ORDER BY created_at, id",
        )
        .bind(agent_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(tool_call_id, tool_name, status, input_json, evidence_refs)| {
                    AgentToolCallRecord {
                        tool_call_id,
                        tool_name,
                        status,
                        input_json,
                        evidence_refs: json_array_to_strings(evidence_refs),
                    }
                },
            )
            .collect())
    }

    pub(super) async fn load_agent_context_snapshots(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<Vec<AgentContextSnapshotRecord>> {
        let rows: Vec<(String, String, Value, Value, String)> = sqlx::query_as(
            "SELECT snapshot_id, redaction_status, context_json, source_refs, checksum
             FROM agent_context_snapshots
             WHERE agent_run_id = $1
             ORDER BY created_at, id",
        )
        .bind(agent_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(snapshot_id, redaction_status, context_json, source_refs, checksum)| {
                    AgentContextSnapshotRecord {
                        snapshot_id,
                        redaction_status,
                        context_json,
                        source_refs: json_array_to_strings(source_refs),
                        checksum,
                    }
                },
            )
            .collect())
    }

    pub(super) async fn load_agent_policy_checks(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<Vec<AgentPolicyCheckRecord>> {
        let rows: Vec<AgentPolicyCheckRow> = sqlx::query_as(
            "SELECT policy_check_id, tool_call_id, tool_name, policy_name, decision, reason, evidence_refs, created_at
             FROM agent_policy_checks
             WHERE agent_run_id = $1
             ORDER BY created_at, id",
        )
        .bind(agent_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| AgentPolicyCheckRecord {
                policy_check_id: row.policy_check_id,
                agent_run_id: agent_run_id.to_string(),
                tool_call_id: row.tool_call_id,
                tool_name: row.tool_name,
                policy_name: row.policy_name,
                decision: row.decision,
                reason: row.reason,
                evidence_refs: json_array_to_strings(row.evidence_refs),
                created_at: Some(row.created_at.to_rfc3339()),
            })
            .collect())
    }

    pub(super) async fn load_agent_tool_results(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<Vec<AgentToolResultRecord>> {
        let rows: Vec<(String, String, String, String, Value, Value)> = sqlx::query_as(
            "SELECT tool_result_id, tool_call_id, tool_name, status, output_json, evidence_refs
             FROM tool_results
             WHERE agent_run_id = $1
             ORDER BY created_at, id",
        )
        .bind(agent_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(tool_result_id, tool_call_id, tool_name, status, output_json, evidence_refs)| {
                    AgentToolResultRecord {
                        tool_result_id,
                        tool_call_id,
                        tool_name,
                        status,
                        output_json,
                        evidence_refs: json_array_to_strings(evidence_refs),
                    }
                },
            )
            .collect())
    }

    pub(super) async fn load_agent_approvals(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<Vec<AgentApprovalRecord>> {
        let rows: Vec<AgentApprovalRow> = sqlx::query_as(
            "SELECT approval_id, proposed_action, decision, approver, reason, evidence_refs, created_at
             FROM agent_approvals
             WHERE agent_run_id = $1
             ORDER BY created_at, id",
        )
        .bind(agent_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| AgentApprovalRecord {
                approval_id: row.approval_id,
                agent_run_id: agent_run_id.to_string(),
                proposed_action: row.proposed_action,
                decision: row.decision,
                approver: row.approver,
                reason: row.reason,
                evidence_refs: json_array_to_strings(row.evidence_refs),
                created_at: Some(row.created_at.to_rfc3339()),
            })
            .collect())
    }
}
