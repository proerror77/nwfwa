use super::*;

pub(super) async fn save_scoring_run(
    repository: &PostgresScoringRepository,
    run: PersistedScoringRun,
) -> anyhow::Result<()> {
    let mut tx = repository.pool.begin().await?;
    let claim_row: Option<(String,)> =
        sqlx::query_as("SELECT id::text FROM claims WHERE external_claim_id = $1")
            .bind(&run.claim_id)
            .fetch_optional(&mut *tx)
            .await?;

    let claim_uuid = claim_row.map(|row| row.0);
    sqlx::query(
        "INSERT INTO scoring_runs
         (run_id, claim_id, source_system, actor_id, status, risk_score, rag, risk_level, recommended_action, confidence_score, confidence, routing_reason, routing_policy, score_breakdown, completed_at)
         VALUES ($1, $2::uuid, $3, $4, 'succeeded', $5, $6, $7, $8, $9, $10, $11, $12, $13, now())",
    )
    .bind(&run.run_id)
    .bind(claim_uuid.as_deref())
    .bind(&run.source_system)
    .bind(&run.actor_id)
    .bind(run.risk_score as i32)
    .bind(&run.rag)
    .bind(&run.risk_level)
    .bind(&run.recommended_action)
    .bind(run.confidence_score as i32)
    .bind(&run.confidence)
    .bind(&run.routing_reason)
    .bind(&run.routing_policy)
    .bind(&run.score_breakdown)
    .execute(&mut *tx)
    .await?;

    for feature in &run.feature_values {
        let feature_name = feature["name"].as_str().unwrap_or("unknown");
        let feature_version = feature["version"].as_i64().unwrap_or(1) as i32;
        sqlx::query(
            "INSERT INTO feature_values
             (run_id, claim_id, feature_name, feature_version, value_json, evidence_json)
             VALUES ($1, $2::uuid, $3, $4, $5, $6)",
        )
        .bind(&run.run_id)
        .bind(claim_uuid.as_deref())
        .bind(feature_name)
        .bind(feature_version)
        .bind(feature["value"].clone())
        .bind(feature["evidence_refs"].clone())
        .execute(&mut *tx)
        .await?;
    }

    for rule_run in &run.rule_runs {
        let rule_evidence = rule_run
            .get("evidence_refs")
            .filter(|evidence| evidence.is_array())
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        sqlx::query(
            "INSERT INTO rule_runs
             (run_id, rule_id, rule_version_id, matched, score_contribution, alert_code, reason, evidence_json)
             VALUES (
               $1,
               (SELECT id FROM rules WHERE rule_key = $2),
               (
                 SELECT rv.id
                 FROM rule_versions rv
                 JOIN rules r ON r.id = rv.rule_id
                 WHERE r.rule_key = $2 AND rv.version = $3
               ),
               true,
               $4,
               $5,
               $6,
               $7
             )",
        )
        .bind(&run.run_id)
        .bind(rule_run["rule_id"].as_str())
        .bind(rule_run["rule_version"].as_i64().unwrap_or(1) as i32)
        .bind(rule_run["score_contribution"].as_i64().unwrap_or(0) as i32)
        .bind(rule_run["alert_code"].as_str())
        .bind(rule_run["reason"].as_str())
        .bind(rule_evidence)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "INSERT INTO model_scores
         (run_id, model_version_id, model_key, runtime_kind, execution_provider, score, label, explanation_json, latency_ms)
         VALUES (
           $1,
           (
             SELECT id
             FROM model_versions
             WHERE model_key = $2 AND version = $3
           ),
           $2,
           $4,
           $5,
           $6,
           $7,
           $8,
           $9
         )",
    )
    .bind(&run.run_id)
    .bind(run.model_score["model_key"].as_str().unwrap_or("unknown"))
    .bind(run.model_score["model_version"].as_str().unwrap_or("unknown"))
    .bind(run.model_score["runtime_kind"].as_str().unwrap_or("unknown"))
    .bind(run.model_score["execution_provider"].as_str().unwrap_or("cpu"))
    .bind(run.model_score["score"].as_i64().unwrap_or(0) as i32)
    .bind(run.model_score["label"].as_str().unwrap_or("UNKNOWN"))
    .bind(run.model_score["explanations"].clone())
    .bind(run.model_score["latency_ms"].as_i64().unwrap_or(0) as i32)
    .execute(&mut *tx)
    .await?;

    if let Some(mut lead) = lead_from_scoring_run(&run, None) {
        if let Some((member_id, provider_id)) = sqlx::query_as::<_, (String, String)>(
            "SELECT m.external_member_id, pr.external_provider_id
             FROM claims c
             JOIN members m ON m.id = c.member_id
             JOIN providers pr ON pr.id = c.provider_id
             WHERE c.external_claim_id = $1",
        )
        .bind(&run.claim_id)
        .fetch_optional(&mut *tx)
        .await?
        {
            lead.member_id = member_id;
            lead.provider_id = provider_id;
        }
        sqlx::query(
            "INSERT INTO fwa_leads
             (lead_id, run_id, claim_id, member_id, provider_id, source_system, review_mode, scheme_family, lead_source, status, disposition, risk_score, rag, reason, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             ON CONFLICT (lead_id) DO UPDATE
             SET run_id = EXCLUDED.run_id,
                 claim_id = EXCLUDED.claim_id,
                 member_id = EXCLUDED.member_id,
                 provider_id = EXCLUDED.provider_id,
                 source_system = EXCLUDED.source_system,
                 review_mode = EXCLUDED.review_mode,
                 scheme_family = EXCLUDED.scheme_family,
                 lead_source = EXCLUDED.lead_source,
                 status = EXCLUDED.status,
                 disposition = EXCLUDED.disposition,
                 risk_score = EXCLUDED.risk_score,
                 rag = EXCLUDED.rag,
                 reason = EXCLUDED.reason,
                 evidence_refs = EXCLUDED.evidence_refs,
                 updated_at = now()",
        )
        .bind(&lead.lead_id)
        .bind(&lead.run_id)
        .bind(&lead.claim_id)
        .bind(&lead.member_id)
        .bind(&lead.provider_id)
        .bind(&lead.source_system)
        .bind(&lead.review_mode)
        .bind(&lead.scheme_family)
        .bind(&lead.lead_source)
        .bind(&lead.status)
        .bind(&lead.disposition)
        .bind(lead.risk_score as i32)
        .bind(&lead.rag)
        .bind(&lead.reason)
        .bind(serde_json::json!(lead.evidence_refs))
        .execute(&mut *tx)
        .await?;
    }

    insert_audit_event(
        &mut tx,
        &PersistedAuditEvent {
            audit_id: run.audit_id,
            run_id: run.run_id,
            claim_id: run.claim_id,
            source_system: run.source_system,
            actor_id: run.actor_id,
            actor_role: "tpa_system".into(),
            event_type: "scoring.completed".into(),
            event_status: "succeeded".into(),
            summary: "FWA scoring completed".into(),
            payload: run.audit_event,
            evidence_refs: run.evidence_refs,
        },
        claim_uuid.as_deref(),
    )
    .await?;

    tx.commit().await?;
    Ok(())
}

pub(super) async fn save_audit_event(
    repository: &PostgresScoringRepository,
    event: PersistedAuditEvent,
) -> anyhow::Result<()> {
    let mut tx = repository.pool.begin().await?;
    let claim_row: Option<(String,)> =
        sqlx::query_as("SELECT id::text FROM claims WHERE external_claim_id = $1")
            .bind(&event.claim_id)
            .fetch_optional(&mut *tx)
            .await?;
    sqlx::query(
        "INSERT INTO scoring_runs
         (run_id, claim_id, source_system, actor_id, status, completed_at, error_code, error_message)
         VALUES ($1, $2::uuid, $3, $4, $5, now(), $6, $7)
         ON CONFLICT (run_id) DO NOTHING",
    )
    .bind(&event.run_id)
    .bind(claim_row.as_ref().map(|row| row.0.as_str()))
    .bind(&event.source_system)
    .bind(&event.actor_id)
    .bind(&event.event_status)
    .bind(&event.event_type)
    .bind(event.payload["error"].as_str())
    .execute(&mut *tx)
    .await?;
    insert_audit_event(
        &mut tx,
        &event,
        claim_row.as_ref().map(|row| row.0.as_str()),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
