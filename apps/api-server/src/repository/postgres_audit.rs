use super::*;

pub(super) async fn claim_audit_history(
    repository: &PostgresScoringRepository,
    claim_id: &str,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
    let rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        Value,
        Value,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "SELECT ae.audit_id, ae.run_id, ae.actor_role, ae.event_type, ae.event_status, ae.summary, ae.payload, ae.evidence_refs, ae.created_at
                 FROM audit_events ae
                 LEFT JOIN claims c ON c.id = ae.claim_id
                 WHERE (payload ->> 'claim_id' = $1 OR c.external_claim_id = $1)
                   AND ($2::text IS NULL OR ae.payload ->> 'customer_scope_id' = $2)
                 ORDER BY ae.created_at, ae.audit_id",
    )
    .bind(claim_id)
    .bind(customer_scope_id)
    .fetch_all(&repository.pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(
                audit_id,
                run_id,
                actor_role,
                event_type,
                event_status,
                summary,
                payload,
                evidence_refs,
                created_at,
            )| AuditHistoryEventRecord {
                audit_id,
                run_id,
                actor_role,
                event_type,
                event_status,
                summary,
                payload,
                evidence_refs: json_array_to_strings(evidence_refs),
                created_at: Some(created_at.to_rfc3339()),
            },
        )
        .collect())
}

pub(super) async fn list_audit_events(
    repository: &PostgresScoringRepository,
    filter: AuditEventListFilter,
) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
    let rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        Value,
        Value,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "SELECT ae.audit_id, ae.run_id, ae.actor_role, ae.event_type, ae.event_status, ae.summary, ae.payload, ae.evidence_refs, ae.created_at
             FROM audit_events ae
             LEFT JOIN claims c ON c.id = ae.claim_id
             WHERE ($2::text IS NULL OR ae.event_type = $2)
               AND ($3::text IS NULL OR ae.actor_id = $3)
               AND ($4::text IS NULL OR ae.run_id = $4)
               AND (
                 $5::text IS NULL
                 OR ae.payload ->> 'claim_id' = $5
                 OR c.external_claim_id = $5
                 OR ae.claim_id::text = $5
               )
               AND ($6::text IS NULL OR ae.payload ->> 'policy_id' = $6)
               AND ($7::text IS NULL OR ae.payload ->> 'version' = $7)
               AND ($8::text IS NULL OR ae.payload ->> 'review_mode' = $8)
               AND ($9::text IS NULL OR ae.payload ->> 'rule_id' = $9)
               AND ($10::text IS NULL OR ae.payload ->> 'rule_version' = $10)
               AND ($11::text IS NULL OR ae.payload ->> 'model_key' = $11)
               AND ($12::text IS NULL OR ae.payload ->> 'model_version' = $12)
               AND (
                 $13::text IS NULL
                 OR (
                   $13 = 'governance'
                   AND ae.event_type = ANY($14::text[])
                 )
               )
               AND ($15::text IS NULL OR ae.payload ->> 'sample_id' = $15)
               AND ($16::text IS NULL OR ae.payload ->> 'agent_run_id' = $16)
               AND ($17::text IS NULL OR ae.payload ->> 'dataset_id' = $17)
               AND ($18::text IS NULL OR ae.payload ->> 'feature_set_id' = $18)
               AND ($19::text IS NULL OR ae.payload ->> 'model_dataset_id' = $19)
               AND ($20::text IS NULL OR ae.payload ->> 'evaluation_run_id' = $20)
               AND ($21::bool IS NULL OR $21 = false OR ae.payload ? 'canonical_claim_context_trace')
               AND ($22::text IS NULL OR ae.payload ->> 'customer_scope_id' = $22)
             ORDER BY ae.created_at DESC, ae.audit_id DESC
             LIMIT $1",
    )
    .bind(filter.limit as i64)
    .bind(filter.event_type.as_deref())
    .bind(filter.actor_id.as_deref())
    .bind(filter.run_id.as_deref())
    .bind(filter.claim_id.as_deref())
    .bind(filter.routing_policy_id.as_deref())
    .bind(filter.routing_policy_version.as_deref())
    .bind(filter.review_mode.as_deref())
    .bind(filter.rule_id.as_deref())
    .bind(filter.rule_version.as_deref())
    .bind(filter.model_key.as_deref())
    .bind(filter.model_version.as_deref())
    .bind(filter.event_group.as_deref())
    .bind(GOVERNANCE_AUDIT_EVENT_TYPES)
    .bind(filter.sample_id.as_deref())
    .bind(filter.agent_run_id.as_deref())
    .bind(filter.dataset_id.as_deref())
    .bind(filter.feature_set_id.as_deref())
    .bind(filter.model_dataset_id.as_deref())
    .bind(filter.evaluation_run_id.as_deref())
    .bind(filter.has_canonical_trace)
    .bind(filter.customer_scope_id.as_deref())
    .fetch_all(&repository.pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(
                audit_id,
                run_id,
                actor_role,
                event_type,
                event_status,
                summary,
                payload,
                evidence_refs,
                created_at,
            )| AuditHistoryEventRecord {
                audit_id,
                run_id,
                actor_role,
                event_type,
                event_status,
                summary,
                payload,
                evidence_refs: json_array_to_strings(evidence_refs),
                created_at: Some(created_at.to_rfc3339()),
            },
        )
        .collect())
}
