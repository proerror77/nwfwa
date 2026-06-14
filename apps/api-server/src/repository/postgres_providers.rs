use super::*;

pub(super) async fn provider_risk_summary(
    repository: &PostgresScoringRepository,
) -> anyhow::Result<ProviderRiskSummaryRecord> {
    let rows: Vec<(Value,)> = sqlx::query_as(
        "SELECT payload
             FROM audit_events
             WHERE event_type = 'scoring.completed'
               AND event_status = 'succeeded'",
    )
    .fetch_all(&repository.pool)
    .await?;
    Ok(summarize_provider_risk_profiles(
        rows.iter().map(|(payload,)| payload),
    ))
}

pub(super) async fn save_provider_sanctions(
    repository: &PostgresScoringRepository,
    input: SaveProviderSanctionsInput,
) -> anyhow::Result<Vec<ProviderSanctionRecord>> {
    let mut saved = Vec::with_capacity(input.provider_upserts.len());
    let mut tx = repository.pool.begin().await?;
    for upsert in input.provider_upserts {
        sqlx::query(
            "INSERT INTO provider_sanctions
                 (customer_scope_id, sanction_key, list, provider_id, npi, provider_name, sanction_type, effective_date, source_ref, risk_feature, risk_score, source_report_uri, submitted_by, notes)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                 ON CONFLICT (customer_scope_id, sanction_key) DO UPDATE
                 SET list = EXCLUDED.list,
                     provider_id = EXCLUDED.provider_id,
                     npi = EXCLUDED.npi,
                     provider_name = EXCLUDED.provider_name,
                     sanction_type = EXCLUDED.sanction_type,
                     effective_date = EXCLUDED.effective_date,
                     source_ref = EXCLUDED.source_ref,
                     risk_feature = EXCLUDED.risk_feature,
                     risk_score = EXCLUDED.risk_score,
                     source_report_uri = EXCLUDED.source_report_uri,
                     submitted_by = EXCLUDED.submitted_by,
                     notes = EXCLUDED.notes,
                     updated_at = now()",
        )
        .bind(&input.customer_scope_id)
        .bind(&upsert.sanction_key)
        .bind(&upsert.list)
        .bind(&upsert.provider_id)
        .bind(&upsert.npi)
        .bind(&upsert.provider_name)
        .bind(&upsert.sanction_type)
        .bind(&upsert.effective_date)
        .bind(&upsert.source_ref)
        .bind(&upsert.risk_feature)
        .bind(upsert.risk_score as i32)
        .bind(&input.source_report_uri)
        .bind(&input.submitted_by)
        .bind(&input.notes)
        .execute(&mut *tx)
        .await?;
        saved.push(ProviderSanctionRecord {
            customer_scope_id: input.customer_scope_id.clone(),
            sanction_key: upsert.sanction_key,
            list: upsert.list,
            provider_id: upsert.provider_id,
            npi: upsert.npi,
            provider_name: upsert.provider_name,
            sanction_type: upsert.sanction_type,
            effective_date: upsert.effective_date,
            source_ref: upsert.source_ref,
            risk_feature: upsert.risk_feature,
            risk_score: upsert.risk_score,
            source_report_uri: input.source_report_uri.clone(),
            submitted_by: input.submitted_by.clone(),
            notes: input.notes.clone(),
        });
    }
    tx.commit().await?;
    Ok(saved)
}
