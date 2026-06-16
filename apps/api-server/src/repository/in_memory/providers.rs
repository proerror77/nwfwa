use super::*;

impl InMemoryScoringRepository {
    pub(super) async fn in_memory_save_provider_sanctions(
        &self,
        input: SaveProviderSanctionsInput,
    ) -> anyhow::Result<Vec<ProviderSanctionRecord>> {
        let mut records = self.provider_sanctions.lock().await;
        let mut saved = Vec::with_capacity(input.provider_upserts.len());
        for upsert in input.provider_upserts {
            let record = ProviderSanctionRecord {
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
            };
            records.insert(
                provider_sanction_key(&record.customer_scope_id, &record.sanction_key),
                record.clone(),
            );
            saved.push(record);
        }
        Ok(saved)
    }

    pub(super) async fn in_memory_provider_sanctions_for_provider(
        &self,
        provider_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<ProviderSanctionRecord>> {
        let mut records = self
            .provider_sanctions
            .lock()
            .await
            .values()
            .filter(|record| record.provider_id.as_deref() == Some(provider_id))
            .filter(|record| {
                customer_scope_id
                    .map(|scope| record.customer_scope_id == scope)
                    .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            left.list
                .cmp(&right.list)
                .then_with(|| left.sanction_key.cmp(&right.sanction_key))
        });
        Ok(records)
    }

    pub(super) async fn in_memory_save_provider_profile_windows(
        &self,
        input: SaveProviderProfileWindowsInput,
    ) -> anyhow::Result<Vec<ProviderProfileWindowRecord>> {
        let mut records = self.provider_profile_windows.lock().await;
        let mut saved = Vec::with_capacity(input.provider_profiles.len());
        for profile in input.provider_profiles {
            let record = ProviderProfileWindowRecord {
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
            };
            records.insert(
                provider_profile_window_key(
                    &record.customer_scope_id,
                    &record.provider_id,
                    &record.as_of_date,
                ),
                record.clone(),
            );
            saved.push(record);
        }
        Ok(saved)
    }

    pub(super) async fn in_memory_latest_provider_profile_windows_for_provider(
        &self,
        provider_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<ProviderProfileWindowRecord>> {
        let records = self.provider_profile_windows.lock().await;
        let mut candidates = records
            .values()
            .filter(|record| record.provider_id == provider_id)
            .filter(|record| {
                customer_scope_id
                    .map(|scope| record.customer_scope_id == scope)
                    .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            right
                .as_of_date
                .cmp(&left.as_of_date)
                .then_with(|| right.source_report_uri.cmp(&left.source_report_uri))
        });
        Ok(candidates.into_iter().next())
    }

    pub(super) async fn in_memory_save_provider_graph_signals(
        &self,
        input: SaveProviderGraphSignalsInput,
    ) -> anyhow::Result<Vec<ProviderGraphSignalRecord>> {
        let mut records = self.provider_graph_signals.lock().await;
        let mut saved = Vec::with_capacity(input.provider_relationships.len());
        for relationship in input.provider_relationships {
            let record = ProviderGraphSignalRecord {
                customer_scope_id: input.customer_scope_id.clone(),
                provider_id: relationship.provider_id,
                as_of_date: input.as_of_date.clone(),
                high_risk_neighbor_ratio: relationship.high_risk_neighbor_ratio,
                provider_patient_overlap_score: relationship.provider_patient_overlap_score,
                referral_concentration_score: relationship.referral_concentration_score,
                billing_ring_membership: relationship.billing_ring_membership,
                temporal_co_billing_frequency_7d: relationship.temporal_co_billing_frequency_7d,
                referral_concentration_entropy: relationship.referral_concentration_entropy,
                shared_member_provider_count: relationship.shared_member_provider_count,
                connected_confirmed_fwa_count: relationship.connected_confirmed_fwa_count,
                network_component_risk_score: relationship.network_component_risk_score,
                evidence_refs: relationship.evidence_refs,
                source_report_uri: input.source_report_uri.clone(),
                submitted_by: input.submitted_by.clone(),
                notes: input.notes.clone(),
            };
            records.insert(
                provider_signal_key(
                    &record.customer_scope_id,
                    &record.provider_id,
                    &record.as_of_date,
                ),
                record.clone(),
            );
            saved.push(record);
        }
        Ok(saved)
    }

    pub(super) async fn in_memory_latest_provider_graph_signal_for_provider(
        &self,
        provider_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<ProviderGraphSignalRecord>> {
        let records = self.provider_graph_signals.lock().await;
        let mut candidates = records
            .values()
            .filter(|record| record.provider_id == provider_id)
            .filter(|record| {
                customer_scope_id
                    .map(|scope| record.customer_scope_id == scope)
                    .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            right
                .as_of_date
                .cmp(&left.as_of_date)
                .then_with(|| right.source_report_uri.cmp(&left.source_report_uri))
        });
        Ok(candidates.into_iter().next())
    }

    pub(super) async fn in_memory_save_peer_benchmark_groups(
        &self,
        input: SavePeerBenchmarkGroupsInput,
    ) -> anyhow::Result<Vec<PeerBenchmarkGroupRecord>> {
        let mut records = self.peer_benchmark_groups.lock().await;
        let mut saved = Vec::with_capacity(input.peer_groups.len());
        for group in input.peer_groups {
            let record = PeerBenchmarkGroupRecord {
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
            };
            records.insert(
                peer_benchmark_group_key(
                    &record.customer_scope_id,
                    &record.peer_group_key,
                    &record.benchmark_month,
                ),
                record.clone(),
            );
            saved.push(record);
        }
        Ok(saved)
    }

    pub(super) async fn in_memory_latest_peer_benchmark_group(
        &self,
        specialty: &str,
        region: &str,
        service_segment: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PeerBenchmarkGroupRecord>> {
        let specialty = specialty.trim();
        let region = region.trim();
        let service_segment = service_segment.trim();
        let record = self
            .peer_benchmark_groups
            .lock()
            .await
            .values()
            .filter(|record| {
                customer_scope_id
                    .map(|scope| record.customer_scope_id == scope)
                    .unwrap_or(true)
            })
            .filter(|record| {
                record.specialty == specialty
                    && record.region == region
                    && record.service_segment == service_segment
            })
            .max_by(|left, right| left.benchmark_month.cmp(&right.benchmark_month))
            .cloned();
        Ok(record)
    }

    pub(super) async fn in_memory_save_episode_rollups(
        &self,
        input: SaveEpisodeRollupsInput,
    ) -> anyhow::Result<Vec<EpisodeRollupRecord>> {
        let mut records = self.episode_rollups.lock().await;
        let mut saved = Vec::with_capacity(input.episodes.len());
        for episode in input.episodes {
            let record = EpisodeRollupRecord {
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
            };
            records.insert(
                episode_rollup_key(
                    &record.customer_scope_id,
                    &record.episode_key,
                    &record.as_of_date,
                ),
                record.clone(),
            );
            saved.push(record);
        }
        Ok(saved)
    }

    pub(super) async fn in_memory_latest_episode_rollup_for_member_provider(
        &self,
        member_id: &str,
        provider_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EpisodeRollupRecord>> {
        let member_id = member_id.trim();
        let provider_id = provider_id.trim();
        let record = self
            .episode_rollups
            .lock()
            .await
            .values()
            .filter(|record| {
                customer_scope_id
                    .map(|scope| record.customer_scope_id == scope)
                    .unwrap_or(true)
            })
            .filter(|record| record.member_id == member_id && record.provider_id == provider_id)
            .max_by(|left, right| left.as_of_date.cmp(&right.as_of_date))
            .cloned();
        Ok(record)
    }
}

fn provider_sanction_key(customer_scope_id: &str, sanction_key: &str) -> String {
    format!("{}\x00{}", customer_scope_id, sanction_key)
}

fn provider_profile_window_key(
    customer_scope_id: &str,
    provider_id: &str,
    as_of_date: &str,
) -> String {
    format!("{}\x00{}\x00{}", customer_scope_id, provider_id, as_of_date)
}

fn provider_signal_key(customer_scope_id: &str, provider_id: &str, as_of_date: &str) -> String {
    format!("{}\x00{}\x00{}", customer_scope_id, provider_id, as_of_date)
}

fn peer_benchmark_group_key(
    customer_scope_id: &str,
    peer_group_key: &str,
    benchmark_month: &str,
) -> String {
    format!(
        "{}\x00{}\x00{}",
        customer_scope_id, peer_group_key, benchmark_month
    )
}

fn episode_rollup_key(customer_scope_id: &str, episode_key: &str, as_of_date: &str) -> String {
    format!("{}\x00{}\x00{}", customer_scope_id, episode_key, as_of_date)
}
