use super::*;

pub(super) async fn save_inbox_claim_run(
    repository: &PostgresScoringRepository,
    run: PersistedInboxClaimRun,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO inbox_claim_runs
         (run_id, audit_id, external_message_id, idempotency_key, external_message_fingerprint,
          raw_payload_checksum, raw_payload_ref, mapping_version, validation_result, scoring_ready,
          claim_id, source_system, customer_scope_id, canonical_claim_context, validation_errors,
          data_quality_signals, evidence_refs)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
         ON CONFLICT (run_id) DO UPDATE
         SET audit_id = EXCLUDED.audit_id,
             external_message_id = EXCLUDED.external_message_id,
             idempotency_key = EXCLUDED.idempotency_key,
             external_message_fingerprint = EXCLUDED.external_message_fingerprint,
             raw_payload_checksum = EXCLUDED.raw_payload_checksum,
             raw_payload_ref = EXCLUDED.raw_payload_ref,
             mapping_version = EXCLUDED.mapping_version,
             validation_result = EXCLUDED.validation_result,
             scoring_ready = EXCLUDED.scoring_ready,
             claim_id = EXCLUDED.claim_id,
             source_system = EXCLUDED.source_system,
             customer_scope_id = EXCLUDED.customer_scope_id,
             canonical_claim_context = EXCLUDED.canonical_claim_context,
             validation_errors = EXCLUDED.validation_errors,
             data_quality_signals = EXCLUDED.data_quality_signals,
             evidence_refs = EXCLUDED.evidence_refs,
             updated_at = now()",
    )
    .bind(&run.run_id)
    .bind(&run.audit_id)
    .bind(&run.external_message_id)
    .bind(&run.idempotency_key)
    .bind(&run.external_message_fingerprint)
    .bind(&run.raw_payload_checksum)
    .bind(&run.raw_payload_ref)
    .bind(&run.mapping_version)
    .bind(&run.validation_result)
    .bind(run.scoring_ready)
    .bind(&run.claim_id)
    .bind(&run.source_system)
    .bind(&run.customer_scope_id)
    .bind(&run.canonical_claim_context)
    .bind(&run.validation_errors)
    .bind(&run.data_quality_signals)
    .bind(&run.evidence_refs)
    .execute(&repository.pool)
    .await?;
    Ok(())
}

pub(super) async fn get_inbox_claim_run_by_idempotency_key(
    repository: &PostgresScoringRepository,
    idempotency_key: &str,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
    let row = sqlx::query(
        "SELECT run_id, audit_id, external_message_id, idempotency_key,
                external_message_fingerprint, raw_payload_checksum, raw_payload_ref,
                mapping_version, validation_result, scoring_ready, claim_id,
                source_system, customer_scope_id, canonical_claim_context,
                validation_errors, data_quality_signals, evidence_refs
         FROM inbox_claim_runs
         WHERE idempotency_key = $1
           AND ($2::text IS NULL OR customer_scope_id = $2)",
    )
    .bind(idempotency_key)
    .bind(customer_scope_id)
    .fetch_optional(&repository.pool)
    .await?;
    Ok(row.map(inbox_claim_run_from_row))
}

pub(super) async fn get_inbox_claim_run_by_run_id(
    repository: &PostgresScoringRepository,
    run_id: &str,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
    let row = sqlx::query(
        "SELECT run_id, audit_id, external_message_id, idempotency_key,
                external_message_fingerprint, raw_payload_checksum, raw_payload_ref,
                mapping_version, validation_result, scoring_ready, claim_id,
                source_system, customer_scope_id, canonical_claim_context,
                validation_errors, data_quality_signals, evidence_refs
         FROM inbox_claim_runs
         WHERE run_id = $1
           AND ($2::text IS NULL OR customer_scope_id = $2)",
    )
    .bind(run_id)
    .bind(customer_scope_id)
    .fetch_optional(&repository.pool)
    .await?;
    Ok(row.map(inbox_claim_run_from_row))
}
