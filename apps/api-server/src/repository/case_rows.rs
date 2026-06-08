use super::row_types::{CaseRow, LeadRow};
use super::{
    case_sla_status, hours_between, is_terminal_case_status, json_array_to_strings,
    sla_target_hours_for_priority, AuditSampleStrataContext, CaseRecord, LeadRecord,
};
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashMap;

pub(super) async fn load_leads(
    pool: &PgPool,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<LeadRecord>> {
    let rows: Vec<LeadRow> = sqlx::query_as(
        "SELECT lead_id, run_id, claim_id, member_id, provider_id, source_system, COALESCE(review_mode, 'pre_payment'), scheme_family, lead_source, status, disposition, risk_score, rag, reason, evidence_refs
         FROM fwa_leads
         WHERE (
           $1::text IS NULL OR EXISTS (
             SELECT 1
             FROM audit_events ae
             JOIN claims scoped_claim ON scoped_claim.id = ae.claim_id
             WHERE scoped_claim.external_claim_id = fwa_leads.claim_id
               AND ae.payload ->> 'customer_scope_id' = $1
           )
         )
         ORDER BY created_at, lead_id",
    )
    .bind(customer_scope_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(lead_from_row).collect())
}

pub(super) async fn load_control_audit_population(
    pool: &PgPool,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<LeadRecord>> {
    let rows: Vec<LeadRow> = sqlx::query_as(
        "SELECT 'control_lead_' || c.external_claim_id,
                sr.run_id,
                c.external_claim_id,
                COALESCE(m.external_member_id, ''),
                COALESCE(pr.external_provider_id, ''),
                sr.source_system,
                COALESCE(scoring_event.payload->>'review_mode', 'pre_payment'),
                CASE
                  WHEN sr.risk_score >= 70 THEN 'high_risk_claim'
                  ELSE 'control_baseline'
                END,
                'random_control_scoring_run',
                'new',
                'pending_control_review',
                sr.risk_score,
                COALESCE(sr.rag, 'GREEN'),
                'Random control baseline sample: ' || COALESCE(sr.routing_reason, 'scored claim'),
                jsonb_build_array('scoring_runs:' || sr.run_id)
         FROM scoring_runs sr
         JOIN claims c ON c.id = sr.claim_id
         LEFT JOIN members m ON m.id = c.member_id
         LEFT JOIN providers pr ON pr.id = c.provider_id
         LEFT JOIN LATERAL (
           SELECT payload
           FROM audit_events ae
           WHERE ae.run_id = sr.run_id
             AND ae.event_type = 'scoring.completed'
           ORDER BY ae.created_at DESC
           LIMIT 1
         ) scoring_event ON TRUE
         WHERE sr.status = 'succeeded'
           AND sr.risk_score IS NOT NULL
           AND ($1::text IS NULL OR scoring_event.payload->>'customer_scope_id' = $1)
         ORDER BY sr.completed_at, sr.run_id",
    )
    .bind(customer_scope_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(lead_from_row).collect())
}

pub(super) async fn load_audit_sample_strata_contexts(
    pool: &PgPool,
) -> anyhow::Result<HashMap<String, AuditSampleStrataContext>> {
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT c.external_claim_id,
                COALESCE(pr.provider_type, 'unknown'),
                COALESCE(pr.region, 'unknown'),
                COALESCE(p.product_code, 'unknown')
         FROM claims c
         LEFT JOIN providers pr ON pr.id = c.provider_id
         LEFT JOIN policies p ON p.id = c.policy_id",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(claim_id, provider_type, provider_region, policy_type)| {
            (
                claim_id,
                AuditSampleStrataContext {
                    provider_type,
                    provider_region,
                    policy_type,
                },
            )
        })
        .collect())
}

pub(super) async fn load_lead_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    lead_id: &str,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Option<LeadRecord>> {
    let row: Option<LeadRow> = sqlx::query_as(
        "SELECT lead_id, run_id, claim_id, member_id, provider_id, source_system, COALESCE(review_mode, 'pre_payment'), scheme_family, lead_source, status, disposition, risk_score, rag, reason, evidence_refs
         FROM fwa_leads
         WHERE lead_id = $1
           AND (
             $2::text IS NULL OR EXISTS (
               SELECT 1
               FROM audit_events ae
               JOIN claims scoped_claim ON scoped_claim.id = ae.claim_id
               WHERE scoped_claim.external_claim_id = fwa_leads.claim_id
                 AND ae.payload ->> 'customer_scope_id' = $2
             )
           )",
    )
    .bind(lead_id)
    .bind(customer_scope_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(lead_from_row))
}

fn lead_from_row(row: LeadRow) -> LeadRecord {
    let (
        lead_id,
        run_id,
        claim_id,
        member_id,
        provider_id,
        source_system,
        review_mode,
        scheme_family,
        lead_source,
        status,
        disposition,
        risk_score,
        rag,
        reason,
        evidence_refs,
    ) = row;
    LeadRecord {
        lead_id,
        run_id,
        claim_id,
        member_id,
        provider_id,
        source_system,
        review_mode,
        scheme_family,
        lead_source,
        status,
        disposition,
        risk_score: risk_score.clamp(0, 100) as u8,
        rag,
        reason,
        evidence_refs: json_array_to_strings(evidence_refs),
    }
}

pub(super) async fn load_cases(
    pool: &PgPool,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<CaseRecord>> {
    let rows: Vec<CaseRow> = sqlx::query_as(
        "SELECT c.case_id, c.lead_id, c.claim_id, c.member_id, c.provider_id, c.source_system, COALESCE(c.review_mode, l.review_mode, 'pre_payment') AS review_mode, c.scheme_family, c.lead_source, c.status, c.assignee, c.reviewer, c.priority, c.routing_reason, c.evidence_package_json, c.final_outcome, c.reviewer_notes, c.investigation_result_id, l.created_at AS lead_created_at, c.created_at AS case_created_at, c.updated_at AS case_updated_at
         FROM investigation_cases c
         JOIN fwa_leads l ON l.lead_id = c.lead_id
         WHERE (
           $1::text IS NULL OR EXISTS (
             SELECT 1
             FROM audit_events ae
             JOIN claims scoped_claim ON scoped_claim.id = ae.claim_id
             WHERE scoped_claim.external_claim_id = c.claim_id
               AND ae.payload ->> 'customer_scope_id' = $1
           )
         )
         ORDER BY c.created_at, c.case_id",
    )
    .bind(customer_scope_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(case_from_row).collect())
}

pub(super) async fn load_case_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    case_id: &str,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Option<CaseRecord>> {
    let row: Option<CaseRow> = sqlx::query_as(
        "SELECT c.case_id, c.lead_id, c.claim_id, c.member_id, c.provider_id, c.source_system, COALESCE(c.review_mode, l.review_mode, 'pre_payment') AS review_mode, c.scheme_family, c.lead_source, c.status, c.assignee, c.reviewer, c.priority, c.routing_reason, c.evidence_package_json, c.final_outcome, c.reviewer_notes, c.investigation_result_id, l.created_at AS lead_created_at, c.created_at AS case_created_at, c.updated_at AS case_updated_at
         FROM investigation_cases c
         JOIN fwa_leads l ON l.lead_id = c.lead_id
         WHERE c.case_id = $1
           AND (
             $2::text IS NULL OR EXISTS (
               SELECT 1
               FROM audit_events ae
               JOIN claims scoped_claim ON scoped_claim.id = ae.claim_id
               WHERE scoped_claim.external_claim_id = c.claim_id
                 AND ae.payload ->> 'customer_scope_id' = $2
             )
           )",
    )
    .bind(case_id)
    .bind(customer_scope_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(case_from_row))
}

fn case_from_row(row: CaseRow) -> CaseRecord {
    let sla_target_hours = sla_target_hours_for_priority(&row.priority);
    let time_to_closure_hours = is_terminal_case_status(&row.status)
        .then(|| hours_between(row.case_created_at, row.case_updated_at));
    let elapsed_hours = time_to_closure_hours
        .unwrap_or_else(|| hours_between(row.case_created_at, chrono::Utc::now()));
    let sla_status = case_sla_status(&row.status, sla_target_hours, elapsed_hours);
    let time_to_triage_hours = hours_between(row.lead_created_at, row.case_created_at);
    CaseRecord {
        case_id: row.case_id,
        lead_id: row.lead_id,
        claim_id: row.claim_id,
        member_id: row.member_id,
        provider_id: row.provider_id,
        source_system: row.source_system,
        review_mode: row.review_mode,
        scheme_family: row.scheme_family,
        lead_source: row.lead_source,
        status: row.status,
        assignee: row.assignee,
        reviewer: row.reviewer,
        priority: row.priority,
        routing_reason: row.routing_reason,
        evidence_package: row.evidence_package_json,
        sla_target_hours,
        sla_status,
        time_to_triage_hours,
        time_to_closure_hours,
        final_outcome: row.final_outcome,
        reviewer_notes: row.reviewer_notes,
        investigation_result_id: row.investigation_result_id,
    }
}
