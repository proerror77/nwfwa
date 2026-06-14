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
}

fn provider_sanction_key(customer_scope_id: &str, sanction_key: &str) -> String {
    format!("{customer_scope_id}::{sanction_key}")
}
