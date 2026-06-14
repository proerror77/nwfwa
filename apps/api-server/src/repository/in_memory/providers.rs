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
                billing_ring_membership: relationship.billing_ring_membership,
                temporal_co_billing_frequency_7d: relationship.temporal_co_billing_frequency_7d,
                referral_concentration_entropy: relationship.referral_concentration_entropy,
                shared_member_provider_count: relationship.shared_member_provider_count,
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
}

fn provider_sanction_key(customer_scope_id: &str, sanction_key: &str) -> String {
    format!("{customer_scope_id}::{sanction_key}")
}

fn provider_profile_window_key(
    customer_scope_id: &str,
    provider_id: &str,
    as_of_date: &str,
) -> String {
    format!("{customer_scope_id}::{provider_id}::{as_of_date}")
}

fn provider_signal_key(customer_scope_id: &str, provider_id: &str, as_of_date: &str) -> String {
    format!("{customer_scope_id}::{provider_id}::{as_of_date}")
}
