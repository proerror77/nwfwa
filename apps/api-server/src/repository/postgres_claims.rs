use super::row_types::{ClaimContextRow, ClaimItemRow, IntoClaimContext};
use super::*;

pub(super) async fn upsert_claim_context(
    repository: &PostgresScoringRepository,
    context: ClaimContext,
    raw_payload: Value,
) -> anyhow::Result<()> {
    let mut tx = repository.pool.begin().await?;

    let member_row: (String,) = sqlx::query_as(
        "INSERT INTO members (external_member_id)
         VALUES ($1)
         ON CONFLICT (external_member_id) DO UPDATE SET updated_at = now()
         RETURNING id::text",
    )
    .bind(&context.member.external_member_id)
    .fetch_one(&mut *tx)
    .await?;

    let policy_row: (String,) = sqlx::query_as(
        "INSERT INTO policies
         (external_policy_id, member_id, product_code, coverage_start_date, coverage_end_date, coverage_limit_amount, currency)
         VALUES ($1, $2::uuid, $3, $4, $5, $6, $7)
         ON CONFLICT (external_policy_id) DO UPDATE SET updated_at = now()
         RETURNING id::text",
    )
    .bind(&context.policy.external_policy_id)
    .bind(&member_row.0)
    .bind(&context.policy.product_code)
    .bind(context.policy.coverage_start_date)
    .bind(context.policy.coverage_end_date)
    .bind(context.policy.coverage_limit.amount)
    .bind(&context.policy.coverage_limit.currency)
    .fetch_one(&mut *tx)
    .await?;

    let provider_row: (String,) = sqlx::query_as(
        "INSERT INTO providers (external_provider_id, name, provider_type, region, risk_tier)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (external_provider_id) DO UPDATE SET updated_at = now()
         RETURNING id::text",
    )
    .bind(&context.provider.external_provider_id)
    .bind(&context.provider.name)
    .bind(&context.provider.provider_type)
    .bind(&context.provider.region)
    .bind(format!("{:?}", context.provider.risk_tier))
    .fetch_one(&mut *tx)
    .await?;

    let claim_row: (String,) = sqlx::query_as(
        "INSERT INTO claims
         (external_claim_id, member_id, policy_id, provider_id, claim_type, diagnosis_code, service_date, claim_amount, currency, status, raw_payload)
         VALUES ($1, $2::uuid, $3::uuid, $4::uuid, 'medical', $5, $6, $7, $8, 'submitted', $9)
         ON CONFLICT (external_claim_id) DO UPDATE
         SET updated_at = now(), raw_payload = EXCLUDED.raw_payload, claim_amount = EXCLUDED.claim_amount
         RETURNING id::text",
    )
    .bind(&context.claim.external_claim_id)
    .bind(&member_row.0)
    .bind(&policy_row.0)
    .bind(&provider_row.0)
    .bind(&context.claim.diagnosis_code)
    .bind(context.claim.service_date)
    .bind(context.claim.amount.amount)
    .bind(&context.claim.amount.currency)
    .bind(raw_payload)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM claim_items WHERE claim_id = $1::uuid")
        .bind(&claim_row.0)
        .execute(&mut *tx)
        .await?;

    for item in &context.items {
        sqlx::query(
            "INSERT INTO claim_items
             (claim_id, item_code, item_type, description, quantity, unit_amount, total_amount, currency)
             VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&claim_row.0)
        .bind(&item.item_code)
        .bind(&item.item_type)
        .bind(&item.description)
        .bind(item.quantity as i32)
        .bind(item.unit_amount.amount)
        .bind(item.total_amount.amount)
        .bind(&item.total_amount.currency)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub(super) async fn load_claim_context(
    repository: &PostgresScoringRepository,
    external_claim_id: &str,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Option<ClaimContext>> {
    let row: Option<ClaimContextRow> = sqlx::query_as(
        "SELECT c.external_claim_id,
                c.diagnosis_code,
                c.service_date,
                c.claim_amount,
                c.currency AS claim_currency,
                m.external_member_id,
                m.dob,
                m.gender,
                p.external_policy_id,
                p.product_code,
                p.coverage_start_date,
                p.coverage_end_date,
                p.coverage_limit_amount,
                p.currency AS policy_currency,
                pr.external_provider_id,
                pr.name AS provider_name,
                pr.provider_type,
                pr.region AS provider_region,
                pr.risk_tier AS provider_risk_tier
         FROM claims c
         JOIN members m ON m.id = c.member_id
         JOIN policies p ON p.id = c.policy_id
         JOIN providers pr ON pr.id = c.provider_id
         WHERE c.external_claim_id = $1
           AND (
             $2::text IS NULL OR EXISTS (
               SELECT 1
               FROM audit_events ae
               WHERE ae.claim_id = c.id
                 AND ae.payload ->> 'customer_scope_id' = $2
             )
           )",
    )
    .bind(external_claim_id)
    .bind(customer_scope_id)
    .fetch_optional(&repository.pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let item_rows: Vec<ClaimItemRow> = sqlx::query_as(
        "SELECT ci.item_code, ci.item_type, ci.description, ci.quantity, ci.unit_amount, ci.total_amount, ci.currency
         FROM claim_items ci
         JOIN claims c ON c.id = ci.claim_id
         WHERE c.external_claim_id = $1
         ORDER BY ci.created_at, ci.item_code",
    )
    .bind(external_claim_id)
    .fetch_all(&repository.pool)
    .await?;

    Ok(Some(row.into_context(item_rows)))
}

pub(super) async fn member_profile_summary(
    repository: &PostgresScoringRepository,
    member_id: &str,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Option<MemberProfileSummaryRecord>> {
    let member_exists: Option<(String,)> = sqlx::query_as(
        "SELECT m.external_member_id
         FROM members m
         WHERE m.external_member_id = $1
           AND (
             $2::text IS NULL OR EXISTS (
               SELECT 1
               FROM claims c
               JOIN audit_events ae ON ae.claim_id = c.id
               WHERE c.member_id = m.id
                 AND ae.payload ->> 'customer_scope_id' = $2
             )
           )",
    )
    .bind(member_id)
    .bind(customer_scope_id)
    .fetch_optional(&repository.pool)
    .await?;
    if member_exists.is_none() {
        return Ok(None);
    }

    let row: (i64, i64, Option<Decimal>, Option<String>) = sqlx::query_as(
        "SELECT COUNT(c.id)::bigint,
                COUNT(DISTINCT p.id)::bigint,
                SUM(c.claim_amount),
                MIN(c.currency)
         FROM members m
         LEFT JOIN claims c ON c.member_id = m.id
         LEFT JOIN policies p ON p.id = c.policy_id
         WHERE m.external_member_id = $1
           AND (
             $2::text IS NULL OR EXISTS (
               SELECT 1
               FROM audit_events ae
               WHERE ae.claim_id = c.id
                 AND ae.payload ->> 'customer_scope_id' = $2
             )
           )",
    )
    .bind(member_id)
    .bind(customer_scope_id)
    .fetch_one(&repository.pool)
    .await?;
    let latest_claim: Option<(String,)> = sqlx::query_as(
        "SELECT c.external_claim_id
         FROM claims c
         JOIN members m ON m.id = c.member_id
         WHERE m.external_member_id = $1
           AND (
             $2::text IS NULL OR EXISTS (
               SELECT 1
               FROM audit_events ae
               WHERE ae.claim_id = c.id
                 AND ae.payload ->> 'customer_scope_id' = $2
             )
           )
         ORDER BY c.service_date DESC, c.external_claim_id DESC
         LIMIT 1",
    )
    .bind(member_id)
    .bind(customer_scope_id)
    .fetch_optional(&repository.pool)
    .await?;
    let high_risk: (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT c.id)::bigint
         FROM members m
         JOIN claims c ON c.member_id = m.id
         JOIN scoring_runs sr ON sr.claim_id = c.id
         WHERE m.external_member_id = $1
           AND (
             $2::text IS NULL OR EXISTS (
               SELECT 1
               FROM audit_events ae
               WHERE ae.claim_id = c.id
                 AND ae.payload ->> 'customer_scope_id' = $2
             )
           )
           AND sr.risk_score >= 70",
    )
    .bind(member_id)
    .bind(customer_scope_id)
    .fetch_one(&repository.pool)
    .await?;

    Ok(Some(member_profile_summary_record(
        MemberProfileSummaryInput {
            member_id: member_id.into(),
            claim_count: row.0 as u32,
            policy_count: row.1 as u32,
            total_claim_amount: row.2.unwrap_or(Decimal::ZERO),
            currency: row.3.unwrap_or_else(|| "UNKNOWN".into()),
            high_risk_claim_count: high_risk.0 as u32,
            latest_claim_id: latest_claim.map(|claim| claim.0),
            evidence_refs: BTreeSet::from([format!("members:{member_id}")]),
        },
    )))
}
