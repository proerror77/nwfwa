use super::*;

pub(super) async fn save_agent_registry(
    repository: &PostgresScoringRepository,
    record: AgentRegistryRecord,
) -> anyhow::Result<AgentRegistryRecord> {
    sqlx::query(
        "INSERT INTO agent_registry
             (agent_identity_id, agent_kind, agent_version, capability_scope, phi_fields_allowed, status)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (agent_identity_id) DO UPDATE
             SET agent_kind = EXCLUDED.agent_kind,
                 agent_version = EXCLUDED.agent_version,
                 capability_scope = EXCLUDED.capability_scope,
                 phi_fields_allowed = EXCLUDED.phi_fields_allowed,
                 status = EXCLUDED.status",
    )
    .bind(&record.agent_identity_id)
    .bind(&record.agent_kind)
    .bind(record.agent_version as i32)
    .bind(string_values(&record.capability_scope))
    .bind(string_values(&record.phi_fields_allowed))
    .bind(&record.status)
    .execute(&repository.pool)
    .await?;
    Ok(record)
}

pub(super) async fn active_agent_registry(
    repository: &PostgresScoringRepository,
    agent_kind: &str,
    agent_version: u32,
) -> anyhow::Result<Option<AgentRegistryRecord>> {
    let row: Option<(String, String, i32, Value, Value, String)> = sqlx::query_as(
        "SELECT agent_identity_id, agent_kind, agent_version, capability_scope, phi_fields_allowed, status
             FROM agent_registry
             WHERE agent_kind = $1
               AND agent_version = $2
               AND status = 'active'
               AND deprovisioned_at IS NULL
             LIMIT 1",
    )
    .bind(agent_kind)
    .bind(agent_version as i32)
    .fetch_optional(&repository.pool)
    .await?;
    Ok(row.map(
        |(
            agent_identity_id,
            agent_kind,
            agent_version,
            capability_scope,
            phi_fields_allowed,
            status,
        )| AgentRegistryRecord {
            agent_identity_id,
            agent_kind,
            agent_version: agent_version as u32,
            capability_scope: json_array_to_strings(capability_scope),
            phi_fields_allowed: json_array_to_strings(phi_fields_allowed),
            status,
        },
    ))
}

pub(super) async fn save_agent_run(
    repository: &PostgresScoringRepository,
    run: PersistedAgentRun,
) -> anyhow::Result<()> {
    let mut tx = repository.pool.begin().await?;
    let registry = default_agent_registry_record();
    let investigation = agent_investigation_record(&run.claim_id, &run.investigation_id);
    sqlx::query(
        "INSERT INTO agent_registry
             (agent_identity_id, agent_kind, agent_version, capability_scope, phi_fields_allowed, status)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (agent_identity_id) DO NOTHING",
    )
    .bind(&registry.agent_identity_id)
    .bind(&registry.agent_kind)
    .bind(registry.agent_version as i32)
    .bind(string_values(&registry.capability_scope))
    .bind(string_values(&registry.phi_fields_allowed))
    .bind(&registry.status)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO investigations
             (investigation_id, claim_id, status, orchestrator_version)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (investigation_id) DO UPDATE
             SET status = CASE
                   WHEN investigations.closed_at IS NULL THEN EXCLUDED.status
                   ELSE investigations.status
                 END",
    )
    .bind(&investigation.investigation_id)
    .bind(&investigation.claim_id)
    .bind(&investigation.status)
    .bind(&investigation.orchestrator_version)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO agent_runs
             (investigation_id, agent_run_id, claim_id, status, decision_boundary, output_json, evidence_refs, completed_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, now())
             ON CONFLICT (agent_run_id) DO UPDATE
             SET investigation_id = EXCLUDED.investigation_id,
                 status = EXCLUDED.status,
                 decision_boundary = EXCLUDED.decision_boundary,
                 output_json = EXCLUDED.output_json,
                 evidence_refs = EXCLUDED.evidence_refs,
                 completed_at = EXCLUDED.completed_at",
    )
    .bind(&run.investigation_id)
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

    let previous_event_hash: Option<String> = sqlx::query_scalar(
        "SELECT event_hash
             FROM agent_audit_events
             WHERE agent_run_id = $1
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
    )
    .bind(&run.agent_run_id)
    .fetch_optional(&mut *tx)
    .await?;
    let audit_event =
        agent_audit_event_from_run(&run, &investigation.investigation_id, previous_event_hash);
    sqlx::query(
        "INSERT INTO agent_audit_events
             (audit_event_id, investigation_id, agent_run_id, agent_kind, agent_version,
              actor_id, actor_role, action_type, input_digest, decision_boundary,
              findings_count, evidence_sufficiency, tool_call_count, human_review_required,
              phi_fields_accessed, payload, previous_event_hash, event_hash)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                     $11, $12, $13, $14, $15, $16, $17, $18)",
    )
    .bind(&audit_event.audit_event_id)
    .bind(&audit_event.investigation_id)
    .bind(&audit_event.agent_run_id)
    .bind(&audit_event.agent_kind)
    .bind(audit_event.agent_version as i32)
    .bind(&audit_event.actor_id)
    .bind(&audit_event.actor_role)
    .bind(&audit_event.action_type)
    .bind(&audit_event.input_digest)
    .bind(&audit_event.decision_boundary)
    .bind(audit_event.findings_count as i32)
    .bind(&audit_event.evidence_sufficiency)
    .bind(audit_event.tool_call_count as i32)
    .bind(audit_event.human_review_required)
    .bind(string_values(&audit_event.phi_fields_accessed))
    .bind(&audit_event.payload)
    .bind(&audit_event.previous_event_hash)
    .bind(&audit_event.event_hash)
    .execute(&mut *tx)
    .await?;

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
        Option<String>,
        Option<String>,
        String,
        String,
        Value,
        Value,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
    )> = sqlx::query_as(
        "SELECT ar.agent_run_id, ar.claim_id, ar.investigation_id, latest_event.investigation_id, ar.status,
                ar.decision_boundary, ar.output_json, ar.evidence_refs, ar.created_at, ar.completed_at
             FROM agent_runs ar
             LEFT JOIN LATERAL (
               SELECT investigation_id
               FROM agent_audit_events aae
               WHERE aae.agent_run_id = ar.agent_run_id
               ORDER BY created_at DESC
               LIMIT 1
             ) latest_event ON true
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
        investigation_id,
        audit_investigation_id,
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
            investigation_id: investigation_id
                .or(audit_investigation_id)
                .unwrap_or_else(|| stable_investigation_id_for_claim(&claim_id)),
            agent_identity_id: DEFAULT_AGENT_IDENTITY_ID.into(),
            agent_kind: DEFAULT_AGENT_KIND.into(),
            agent_version: DEFAULT_AGENT_VERSION,
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

pub(super) async fn cancel_agent_run(
    repository: &PostgresScoringRepository,
    agent_run_id: &str,
) -> anyhow::Result<()> {
    let result = sqlx::query(
        "UPDATE agent_runs
             SET status = 'cancelled',
                 completed_at = COALESCE(completed_at, NOW())
             WHERE agent_run_id = $1",
    )
    .bind(agent_run_id)
    .execute(&repository.pool)
    .await?;

    if result.rows_affected() == 0 {
        anyhow::bail!("agent run not found: {agent_run_id}");
    }
    Ok(())
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
