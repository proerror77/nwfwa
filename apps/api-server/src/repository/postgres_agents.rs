use super::*;

pub(super) async fn save_agent_run(
    repository: &PostgresScoringRepository,
    run: PersistedAgentRun,
) -> anyhow::Result<()> {
    let mut tx = repository.pool.begin().await?;
    sqlx::query(
        "INSERT INTO agent_runs
             (agent_run_id, claim_id, status, decision_boundary, output_json, evidence_refs, completed_at)
             VALUES ($1, $2, $3, $4, $5, $6, now())
             ON CONFLICT (agent_run_id) DO UPDATE
             SET status = EXCLUDED.status,
                 decision_boundary = EXCLUDED.decision_boundary,
                 output_json = EXCLUDED.output_json,
                 evidence_refs = EXCLUDED.evidence_refs,
                 completed_at = EXCLUDED.completed_at",
    )
    .bind(&run.agent_run_id)
    .bind(&run.claim_id)
    .bind(&run.status)
    .bind(&run.decision_boundary)
    .bind(&run.output_json)
    .bind(Value::Array(run.evidence_refs.clone()))
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM agent_steps WHERE agent_run_id = $1")
        .bind(&run.agent_run_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM agent_context_snapshots WHERE agent_run_id = $1")
        .bind(&run.agent_run_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM tool_results WHERE agent_run_id = $1")
        .bind(&run.agent_run_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM agent_policy_checks WHERE agent_run_id = $1")
        .bind(&run.agent_run_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM tool_calls WHERE agent_run_id = $1")
        .bind(&run.agent_run_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM agent_approvals WHERE agent_run_id = $1")
        .bind(&run.agent_run_id)
        .execute(&mut *tx)
        .await?;

    for step in &run.steps {
        sqlx::query(
            "INSERT INTO agent_steps
                 (agent_run_id, step_name, status, output_json, evidence_refs)
                 VALUES ($1, $2, 'succeeded', $3, $4)",
        )
        .bind(&run.agent_run_id)
        .bind(step["step_name"].as_str().unwrap_or("investigate"))
        .bind(step)
        .bind(step["evidence_refs"].clone())
        .execute(&mut *tx)
        .await?;
    }
    for snapshot in &run.context_snapshots {
        sqlx::query(
            "INSERT INTO agent_context_snapshots
                 (snapshot_id, agent_run_id, redaction_status, context_json, source_refs, checksum)
                 VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&snapshot.snapshot_id)
        .bind(&run.agent_run_id)
        .bind(&snapshot.redaction_status)
        .bind(&snapshot.context_json)
        .bind(string_values(&snapshot.source_refs))
        .bind(&snapshot.checksum)
        .execute(&mut *tx)
        .await?;
    }
    for call in &run.tool_calls {
        sqlx::query(
            "INSERT INTO tool_calls
                 (tool_call_id, agent_run_id, tool_name, status, input_json, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&call.tool_call_id)
        .bind(&run.agent_run_id)
        .bind(&call.tool_name)
        .bind(&call.status)
        .bind(&call.input_json)
        .bind(string_values(&call.evidence_refs))
        .execute(&mut *tx)
        .await?;
    }
    for check in &run.policy_checks {
        sqlx::query(
            "INSERT INTO agent_policy_checks
                 (policy_check_id, agent_run_id, tool_call_id, tool_name, policy_name, decision, reason, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&check.policy_check_id)
        .bind(&run.agent_run_id)
        .bind(&check.tool_call_id)
        .bind(&check.tool_name)
        .bind(&check.policy_name)
        .bind(&check.decision)
        .bind(&check.reason)
        .bind(string_values(&check.evidence_refs))
        .execute(&mut *tx)
        .await?;
    }
    for result in &run.tool_results {
        sqlx::query(
            "INSERT INTO tool_results
                 (tool_result_id, tool_call_id, agent_run_id, tool_name, status, output_json, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&result.tool_result_id)
        .bind(&result.tool_call_id)
        .bind(&run.agent_run_id)
        .bind(&result.tool_name)
        .bind(&result.status)
        .bind(&result.output_json)
        .bind(string_values(&result.evidence_refs))
        .execute(&mut *tx)
        .await?;
    }
    for approval in &run.approvals {
        sqlx::query(
            "INSERT INTO agent_approvals
                 (approval_id, agent_run_id, proposed_action, decision, approver, reason, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&approval.approval_id)
        .bind(&run.agent_run_id)
        .bind(&approval.proposed_action)
        .bind(&approval.decision)
        .bind(&approval.approver)
        .bind(&approval.reason)
        .bind(string_values(&approval.evidence_refs))
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub(super) async fn list_agent_runs(
    repository: &PostgresScoringRepository,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<AgentRunLogRecord>> {
    let rows: Vec<(
        String,
        String,
        String,
        String,
        Value,
        Value,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
    )> = sqlx::query_as(
        "SELECT agent_run_id, claim_id, status, decision_boundary, output_json, evidence_refs, created_at, completed_at
             FROM agent_runs ar
             WHERE (
               $1::text IS NULL OR EXISTS (
                 SELECT 1
                 FROM audit_events ae
                 LEFT JOIN claims c ON c.id = ae.claim_id
                 WHERE ae.payload ->> 'customer_scope_id' = $1
                   AND (
                     ae.payload ->> 'claim_id' = ar.claim_id
                     OR c.external_claim_id = ar.claim_id
                     OR ae.claim_id::text = ar.claim_id
                   )
               )
             )
             ORDER BY created_at DESC, agent_run_id",
    )
    .bind(customer_scope_id)
    .fetch_all(&repository.pool)
    .await?;

    let mut runs = Vec::with_capacity(rows.len());
    for (
        agent_run_id,
        claim_id,
        status,
        decision_boundary,
        output_json,
        evidence_refs,
        created_at,
        completed_at,
    ) in rows
    {
        let steps: Vec<(Value,)> = sqlx::query_as(
            "SELECT output_json
                 FROM agent_steps
                 WHERE agent_run_id = $1
                 ORDER BY created_at, id",
        )
        .bind(&agent_run_id)
        .fetch_all(&repository.pool)
        .await?;
        let context_snapshots = repository
            .load_agent_context_snapshots(&agent_run_id)
            .await?;
        let policy_checks = repository.load_agent_policy_checks(&agent_run_id).await?;
        let tool_calls = repository.load_agent_tool_calls(&agent_run_id).await?;
        let tool_results = repository.load_agent_tool_results(&agent_run_id).await?;
        let approvals = repository.load_agent_approvals(&agent_run_id).await?;
        runs.push(AgentRunLogRecord {
            agent_run_id,
            claim_id,
            status,
            decision_boundary,
            output_json,
            evidence_refs: json_array_to_strings(evidence_refs),
            steps: steps.into_iter().map(|row| row.0).collect(),
            context_snapshots,
            policy_checks,
            tool_calls,
            tool_results,
            approvals,
            created_at: Some(created_at.to_rfc3339()),
            completed_at: completed_at.map(|value| value.to_rfc3339()),
        });
    }

    Ok(runs)
}

pub(super) async fn save_agent_approval(
    repository: &PostgresScoringRepository,
    approval: AgentApprovalRecord,
) -> anyhow::Result<AgentApprovalRecord> {
    sqlx::query(
        "INSERT INTO agent_approvals
             (approval_id, agent_run_id, proposed_action, decision, approver, reason, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (approval_id) DO UPDATE
             SET decision = EXCLUDED.decision,
                 approver = EXCLUDED.approver,
                 reason = EXCLUDED.reason,
                 evidence_refs = EXCLUDED.evidence_refs",
    )
    .bind(&approval.approval_id)
    .bind(&approval.agent_run_id)
    .bind(&approval.proposed_action)
    .bind(&approval.decision)
    .bind(&approval.approver)
    .bind(&approval.reason)
    .bind(string_values(&approval.evidence_refs))
    .execute(&repository.pool)
    .await?;
    Ok(approval)
}
