use super::case_rows::{load_case_in_tx, load_cases, load_lead_in_tx, load_leads};
use super::*;

pub(super) async fn list_leads(
    repository: &PostgresScoringRepository,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<LeadRecord>> {
    load_leads(&repository.pool, customer_scope_id).await
}

pub(super) async fn triage_lead(
    repository: &PostgresScoringRepository,
    lead_id: &str,
    input: TriageLeadInput,
) -> anyhow::Result<Option<TriageLeadRecord>> {
    let mut tx = repository.pool.begin().await?;
    let lead = load_lead_in_tx(&mut tx, lead_id, input.customer_scope_id.as_deref()).await?;
    let Some(mut lead) = lead else {
        return Ok(None);
    };
    if input.decision == "merge_lead" && merge_target_lead_in_tx(&mut tx, &input).await?.is_none() {
        return Ok(None);
    }
    lead.status = triage_status_for_decision(&input.decision).into();
    lead.disposition = triage_disposition_for_decision(&input.decision).into();
    let case = (input.decision == "open_case").then(|| case_from_lead(&lead, &input));
    sqlx::query(
        "UPDATE fwa_leads
             SET status = $2, disposition = $3, updated_at = now()
             WHERE lead_id = $1",
    )
    .bind(&lead.lead_id)
    .bind(&lead.status)
    .bind(&lead.disposition)
    .execute(&mut *tx)
    .await?;
    if let Some(case) = &case {
        sqlx::query(
            "INSERT INTO investigation_cases
                 (case_id, lead_id, claim_id, member_id, provider_id, source_system, review_mode, scheme_family, lead_source, status, assignee, reviewer, priority, routing_reason, evidence_package_json)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                 ON CONFLICT (case_id) DO UPDATE
                 SET status = EXCLUDED.status,
                     review_mode = EXCLUDED.review_mode,
                     assignee = EXCLUDED.assignee,
                     reviewer = EXCLUDED.reviewer,
                     priority = EXCLUDED.priority,
                     routing_reason = EXCLUDED.routing_reason,
                     evidence_package_json = EXCLUDED.evidence_package_json,
                     updated_at = now()",
        )
        .bind(&case.case_id)
        .bind(&case.lead_id)
        .bind(&case.claim_id)
        .bind(&case.member_id)
        .bind(&case.provider_id)
        .bind(&case.source_system)
        .bind(&case.review_mode)
        .bind(&case.scheme_family)
        .bind(&case.lead_source)
        .bind(&case.status)
        .bind(&case.assignee)
        .bind(&case.reviewer)
        .bind(&case.priority)
        .bind(&case.routing_reason)
        .bind(&case.evidence_package)
        .execute(&mut *tx)
        .await?;
    }

    let audit_id = AuditEventId::new().to_string();
    insert_audit_event(
        &mut tx,
        &PersistedAuditEvent {
            audit_id: audit_id.clone(),
            run_id: lead.run_id.clone(),
            claim_id: lead.claim_id.clone(),
            source_system: lead.source_system.clone(),
            actor_id: input.assignee.clone(),
            actor_role: "fwa_operator".into(),
            event_type: "lead.triaged".into(),
            event_status: "succeeded".into(),
            summary: format!("Lead triaged: {}", input.decision),
            payload: triage_audit_payload(&lead, &input, case.as_ref()),
            evidence_refs: input
                .evidence_refs
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        },
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(Some(TriageLeadRecord {
        lead,
        case,
        audit_id,
    }))
}

pub(super) async fn list_cases(
    repository: &PostgresScoringRepository,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<CaseRecord>> {
    load_cases(&repository.pool, customer_scope_id).await
}

pub(super) async fn update_case_status(
    repository: &PostgresScoringRepository,
    case_id: &str,
    input: UpdateCaseStatusInput,
) -> anyhow::Result<Option<UpdateCaseStatusRecord>> {
    let mut tx = repository.pool.begin().await?;
    let case = load_case_in_tx(&mut tx, case_id, input.customer_scope_id.as_deref()).await?;
    let Some(mut case) = case else {
        return Ok(None);
    };
    let audit_run_id = load_lead_in_tx(&mut tx, &case.lead_id, input.customer_scope_id.as_deref())
        .await?
        .map(|lead| lead.run_id)
        .unwrap_or_else(|| format!("case_status_{}", case.case_id));
    let from_status = case.status.clone();
    case.status = input.status.clone();
    sqlx::query(
        "UPDATE investigation_cases
             SET status = $2, updated_at = now()
             WHERE case_id = $1",
    )
    .bind(&case.case_id)
    .bind(&case.status)
    .execute(&mut *tx)
    .await?;
    let case = load_case_in_tx(&mut tx, case_id, input.customer_scope_id.as_deref())
        .await?
        .expect("case should exist after status update");

    let audit_id = AuditEventId::new().to_string();
    insert_audit_event(
        &mut tx,
        &PersistedAuditEvent {
            audit_id: audit_id.clone(),
            run_id: audit_run_id,
            claim_id: case.claim_id.clone(),
            source_system: case.source_system.clone(),
            actor_id: input.actor_id.clone(),
            actor_role: "fwa_operator".into(),
            event_type: "case.status.updated".into(),
            event_status: "succeeded".into(),
            summary: format!("Case status updated: {} -> {}", from_status, case.status),
            payload: serde_json::json!({
                "claim_id": case.claim_id,
                "case_id": case.case_id,
                "lead_id": case.lead_id,
                "from_status": from_status,
                "to_status": case.status,
                "notes": input.notes,
                "customer_scope_id": input.customer_scope_id
            }),
            evidence_refs: input
                .evidence_refs
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        },
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(Some(UpdateCaseStatusRecord { case, audit_id }))
}
