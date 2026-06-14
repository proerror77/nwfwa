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

pub(super) async fn latest_provider_profile_windows_for_provider(
    repository: &PostgresScoringRepository,
    provider_id: &str,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Option<ProviderProfileWindowRecord>> {
    let row: Option<(
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        Value,
        Value,
        String,
        String,
        String,
    )> = sqlx::query_as(
        "SELECT customer_scope_id, provider_id, specialty, network_status, as_of_date, windows, evidence_refs, source_report_uri, submitted_by, notes
             FROM provider_profile_windows
             WHERE provider_id = $1
               AND ($2::text IS NULL OR customer_scope_id = $2)
             ORDER BY as_of_date DESC, updated_at DESC
             LIMIT 1",
    )
    .bind(provider_id)
    .bind(customer_scope_id)
    .fetch_optional(&repository.pool)
    .await?;

    Ok(row.map(
        |(
            customer_scope_id,
            provider_id,
            specialty,
            network_status,
            as_of_date,
            windows,
            evidence_refs,
            source_report_uri,
            submitted_by,
            notes,
        )| ProviderProfileWindowRecord {
            customer_scope_id,
            provider_id,
            specialty,
            network_status,
            as_of_date,
            windows: serde_json::from_value(windows).unwrap_or_default(),
            evidence_refs: serde_json::from_value(evidence_refs).unwrap_or_default(),
            source_report_uri,
            submitted_by,
            notes,
        },
    ))
}

pub(super) async fn save_provider_graph_signals(
    repository: &PostgresScoringRepository,
    input: SaveProviderGraphSignalsInput,
) -> anyhow::Result<Vec<ProviderGraphSignalRecord>> {
    let mut saved = Vec::with_capacity(input.provider_relationships.len());
    let mut tx = repository.pool.begin().await?;
    for relationship in input.provider_relationships {
        let evidence_refs = serde_json::Value::Array(
            relationship
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        );
        sqlx::query(
            "INSERT INTO provider_graph_signals
                 (customer_scope_id, provider_id, as_of_date, billing_ring_membership, temporal_co_billing_frequency_7d, referral_concentration_entropy, shared_member_provider_count, evidence_refs, source_report_uri, submitted_by, notes)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                 ON CONFLICT (customer_scope_id, provider_id, as_of_date) DO UPDATE
                 SET billing_ring_membership = EXCLUDED.billing_ring_membership,
                     temporal_co_billing_frequency_7d = EXCLUDED.temporal_co_billing_frequency_7d,
                     referral_concentration_entropy = EXCLUDED.referral_concentration_entropy,
                     shared_member_provider_count = EXCLUDED.shared_member_provider_count,
                     evidence_refs = EXCLUDED.evidence_refs,
                     source_report_uri = EXCLUDED.source_report_uri,
                     submitted_by = EXCLUDED.submitted_by,
                     notes = EXCLUDED.notes,
                     updated_at = now()",
        )
        .bind(&input.customer_scope_id)
        .bind(&relationship.provider_id)
        .bind(&input.as_of_date)
        .bind(relationship.billing_ring_membership)
        .bind(relationship.temporal_co_billing_frequency_7d)
        .bind(relationship.referral_concentration_entropy)
        .bind(relationship.shared_member_provider_count as i32)
        .bind(&evidence_refs)
        .bind(&input.source_report_uri)
        .bind(&input.submitted_by)
        .bind(&input.notes)
        .execute(&mut *tx)
        .await?;
        saved.push(ProviderGraphSignalRecord {
            customer_scope_id: input.customer_scope_id.clone(),
            provider_id: relationship.provider_id,
            as_of_date: input.as_of_date.clone(),
            billing_ring_membership: relationship.billing_ring_membership,
            temporal_co_billing_frequency_7d: relationship.temporal_co_billing_frequency_7d,
            referral_concentration_entropy: relationship.referral_concentration_entropy,
            shared_member_provider_count: relationship.shared_member_provider_count,
            evidence_refs: relationship.evidence_refs,
            source_report_uri: input.source_report_uri.clone(),
            submitted_by: input.submitted_by.clone(),
            notes: input.notes.clone(),
        });
    }
    tx.commit().await?;
    Ok(saved)
}

pub(super) async fn save_peer_benchmark_groups(
    repository: &PostgresScoringRepository,
    input: SavePeerBenchmarkGroupsInput,
) -> anyhow::Result<Vec<PeerBenchmarkGroupRecord>> {
    let mut saved = Vec::with_capacity(input.peer_groups.len());
    let mut tx = repository.pool.begin().await?;
    for group in input.peer_groups {
        let evidence_refs = serde_json::Value::Array(
            group
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        );
        sqlx::query(
            "INSERT INTO peer_benchmark_groups
                 (customer_scope_id, peer_group_key, specialty, region, service_segment, benchmark_month, claim_count, p25, p50, p75, p90, p99, evidence_refs, source_report_uri, submitted_by, notes)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
                 ON CONFLICT (customer_scope_id, peer_group_key, benchmark_month) DO UPDATE
                 SET specialty = EXCLUDED.specialty,
                     region = EXCLUDED.region,
                     service_segment = EXCLUDED.service_segment,
                     claim_count = EXCLUDED.claim_count,
                     p25 = EXCLUDED.p25,
                     p50 = EXCLUDED.p50,
                     p75 = EXCLUDED.p75,
                     p90 = EXCLUDED.p90,
                     p99 = EXCLUDED.p99,
                     evidence_refs = EXCLUDED.evidence_refs,
                     source_report_uri = EXCLUDED.source_report_uri,
                     submitted_by = EXCLUDED.submitted_by,
                     notes = EXCLUDED.notes,
                     updated_at = now()",
        )
        .bind(&input.customer_scope_id)
        .bind(&group.peer_group_key)
        .bind(&group.specialty)
        .bind(&group.region)
        .bind(&group.service_segment)
        .bind(&input.benchmark_month)
        .bind(group.claim_count as i32)
        .bind(group.p25)
        .bind(group.p50)
        .bind(group.p75)
        .bind(group.p90)
        .bind(group.p99)
        .bind(&evidence_refs)
        .bind(&input.source_report_uri)
        .bind(&input.submitted_by)
        .bind(&input.notes)
        .execute(&mut *tx)
        .await?;
        saved.push(PeerBenchmarkGroupRecord {
            customer_scope_id: input.customer_scope_id.clone(),
            peer_group_key: group.peer_group_key,
            specialty: group.specialty,
            region: group.region,
            service_segment: group.service_segment,
            benchmark_month: input.benchmark_month.clone(),
            claim_count: group.claim_count,
            p25: group.p25,
            p50: group.p50,
            p75: group.p75,
            p90: group.p90,
            p99: group.p99,
            evidence_refs: group.evidence_refs,
            source_report_uri: input.source_report_uri.clone(),
            submitted_by: input.submitted_by.clone(),
            notes: input.notes.clone(),
        });
    }
    tx.commit().await?;
    Ok(saved)
}

pub(super) async fn save_episode_rollups(
    repository: &PostgresScoringRepository,
    input: SaveEpisodeRollupsInput,
) -> anyhow::Result<Vec<EpisodeRollupRecord>> {
    let mut saved = Vec::with_capacity(input.episodes.len());
    let mut tx = repository.pool.begin().await?;
    for episode in input.episodes {
        let windows = serde_json::Value::Array(episode.windows.clone());
        let evidence_refs = serde_json::Value::Array(
            episode
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        );
        sqlx::query(
            "INSERT INTO episode_rollups
                 (customer_scope_id, episode_key, member_id, provider_id, as_of_date, windows, evidence_refs, source_report_uri, submitted_by, notes)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                 ON CONFLICT (customer_scope_id, episode_key, as_of_date) DO UPDATE
                 SET member_id = EXCLUDED.member_id,
                     provider_id = EXCLUDED.provider_id,
                     windows = EXCLUDED.windows,
                     evidence_refs = EXCLUDED.evidence_refs,
                     source_report_uri = EXCLUDED.source_report_uri,
                     submitted_by = EXCLUDED.submitted_by,
                     notes = EXCLUDED.notes,
                     updated_at = now()",
        )
        .bind(&input.customer_scope_id)
        .bind(&episode.episode_key)
        .bind(&episode.member_id)
        .bind(&episode.provider_id)
        .bind(&input.as_of_date)
        .bind(&windows)
        .bind(&evidence_refs)
        .bind(&input.source_report_uri)
        .bind(&input.submitted_by)
        .bind(&input.notes)
        .execute(&mut *tx)
        .await?;
        saved.push(EpisodeRollupRecord {
            customer_scope_id: input.customer_scope_id.clone(),
            episode_key: episode.episode_key,
            member_id: episode.member_id,
            provider_id: episode.provider_id,
            as_of_date: input.as_of_date.clone(),
            windows: episode.windows,
            evidence_refs: episode.evidence_refs,
            source_report_uri: input.source_report_uri.clone(),
            submitted_by: input.submitted_by.clone(),
            notes: input.notes.clone(),
        });
    }
    tx.commit().await?;
    Ok(saved)
}
