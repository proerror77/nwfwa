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

pub(super) async fn save_provider_profile_windows(
    repository: &PostgresScoringRepository,
    input: SaveProviderProfileWindowsInput,
) -> anyhow::Result<Vec<ProviderProfileWindowRecord>> {
    let mut saved = Vec::with_capacity(input.provider_profiles.len());
    let mut tx = repository.pool.begin().await?;
    for profile in input.provider_profiles {
        let windows = serde_json::Value::Array(profile.windows.clone());
        let evidence_refs = serde_json::Value::Array(
            profile
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        );
        sqlx::query(
            "INSERT INTO provider_profile_windows
                 (customer_scope_id, provider_id, specialty, network_status, as_of_date, windows, evidence_refs, source_report_uri, submitted_by, notes)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                 ON CONFLICT (customer_scope_id, provider_id, as_of_date) DO UPDATE
                 SET specialty = EXCLUDED.specialty,
                     network_status = EXCLUDED.network_status,
                     windows = EXCLUDED.windows,
                     evidence_refs = EXCLUDED.evidence_refs,
                     source_report_uri = EXCLUDED.source_report_uri,
                     submitted_by = EXCLUDED.submitted_by,
                     notes = EXCLUDED.notes,
                     updated_at = now()",
        )
        .bind(&input.customer_scope_id)
        .bind(&profile.provider_id)
        .bind(&profile.specialty)
        .bind(&profile.network_status)
        .bind(&input.as_of_date)
        .bind(&windows)
        .bind(&evidence_refs)
        .bind(&input.source_report_uri)
        .bind(&input.submitted_by)
        .bind(&input.notes)
        .execute(&mut *tx)
        .await?;
        saved.push(ProviderProfileWindowRecord {
            customer_scope_id: input.customer_scope_id.clone(),
            provider_id: profile.provider_id,
            specialty: profile.specialty,
            network_status: profile.network_status,
            as_of_date: input.as_of_date.clone(),
            windows: profile.windows,
            evidence_refs: profile.evidence_refs,
            source_report_uri: input.source_report_uri.clone(),
            submitted_by: input.submitted_by.clone(),
            notes: input.notes.clone(),
        });
    }
    tx.commit().await?;
    Ok(saved)
}
