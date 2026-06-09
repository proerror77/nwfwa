use super::*;

pub(super) async fn dashboard_summary(
    repository: &PostgresScoringRepository,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<DashboardSummaryRecord> {
    let suspected: (i64, Option<Decimal>) = sqlx::query_as(
        "SELECT COUNT(*)::bigint, COALESCE(SUM(c.claim_amount), 0)
             FROM scoring_runs sr
             LEFT JOIN claims c ON c.id = sr.claim_id
             WHERE sr.risk_score >= 70
               AND ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))",
    )
    .bind(customer_scope_id)
    .fetch_one(&repository.pool)
    .await?;

    let rag_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT COALESCE(rag, 'UNKNOWN'), COUNT(*)::bigint
             FROM scoring_runs sr
             WHERE rag IS NOT NULL
               AND ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))
             GROUP BY rag
             ORDER BY rag",
    )
    .bind(customer_scope_id)
    .fetch_all(&repository.pool)
    .await?;

    let rule_hits: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::bigint
             FROM rule_runs rr
             JOIN scoring_runs sr ON sr.run_id = rr.run_id
             WHERE rr.matched = true
               AND ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))",
    )
    .bind(customer_scope_id)
    .fetch_one(&repository.pool)
    .await?;

    let model_rows: Vec<(String, i64, Option<Decimal>, Option<i64>)> = sqlx::query_as(
        "SELECT model_key,
                    COUNT(*)::bigint,
                    AVG(score),
                    SUM(CASE WHEN score >= 70 THEN 1 ELSE 0 END)::bigint
             FROM model_scores ms
             JOIN scoring_runs sr ON sr.run_id = ms.run_id
             WHERE ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))
             GROUP BY model_key
             ORDER BY model_key",
    )
    .bind(customer_scope_id)
    .fetch_all(&repository.pool)
    .await?;

    let layer_payloads: Vec<(Value,)> = sqlx::query_as(
        "SELECT payload
             FROM audit_events
             WHERE event_type = 'scoring.completed'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)",
    )
    .bind(customer_scope_id)
    .fetch_all(&repository.pool)
    .await?;
    let audit_coverage_row: (i64, Option<i64>) = sqlx::query_as(
        "SELECT COUNT(*)::bigint,
                    SUM(
                        CASE
                            WHEN jsonb_typeof(payload->'canonical_claim_context_trace') = 'object'
                            THEN 1
                            ELSE 0
                        END
                    )::bigint
             FROM audit_events
             WHERE event_type = 'scoring.completed'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)",
    )
    .bind(customer_scope_id)
    .fetch_one(&repository.pool)
    .await?;
    let audit_coverage = summarize_dashboard_audit_coverage(
        audit_coverage_row.0 as u32,
        audit_coverage_row.1.unwrap_or(0) as u32,
    );
    let mut layer_accumulators = BTreeMap::<String, (String, u32, u32, u32)>::new();
    for (payload,) in layer_payloads {
        for layer in payload
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let layer_id = layer["layer_id"].as_str().unwrap_or("UNKNOWN").to_string();
            let layer_name = layer["name"].as_str().unwrap_or("Unknown").to_string();
            let layer_score = layer["score"].as_u64().unwrap_or(0) as u32;
            let entry = layer_accumulators
                .entry(layer_id)
                .or_insert((layer_name.clone(), 0, 0, 0));
            entry.0 = layer_name;
            entry.1 += 1;
            entry.2 += layer_score;
            if layer_score >= 70 {
                entry.3 += 1;
            }
        }
    }

    let investigation: (i64, i64, Option<Decimal>) = sqlx::query_as(
        "SELECT COUNT(*)::bigint,
                    COALESCE(SUM(CASE WHEN confirmed_fwa THEN 1 ELSE 0 END), 0)::bigint,
                    COALESCE(SUM(saving_amount), 0)
             FROM investigation_results ir
             WHERE ($1::text IS NULL OR EXISTS (
               SELECT 1 FROM audit_events ae
               WHERE ae.event_type = 'investigation.result.received'
                 AND ae.event_status = 'succeeded'
                 AND ae.payload ->> 'investigation_id' = ir.investigation_id
                 AND ae.payload ->> 'customer_scope_id' = $1
             ))",
    )
    .bind(customer_scope_id)
    .fetch_one(&repository.pool)
    .await?;

    let qa_reviews: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::bigint
             FROM qa_reviews qr
             WHERE ($1::text IS NULL OR EXISTS (
               SELECT 1 FROM audit_events ae
               WHERE ae.event_type = 'qa.result.received'
                 AND ae.event_status = 'succeeded'
                 AND ae.payload ->> 'qa_case_id' = qr.qa_case_id
                 AND ae.payload ->> 'customer_scope_id' = $1
             ))",
    )
    .bind(customer_scope_id)
    .fetch_one(&repository.pool)
    .await?;

    let scheme_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT scheme_family, COUNT(*)::bigint
             FROM fwa_leads l
             WHERE ($1::text IS NULL OR EXISTS (
               SELECT 1 FROM audit_events ae
               WHERE ae.run_id = l.run_id
                 AND ae.event_type = 'scoring.completed'
                 AND ae.event_status = 'succeeded'
                 AND ae.payload ->> 'customer_scope_id' = $1
             ))
             GROUP BY scheme_family
             ORDER BY scheme_family",
    )
    .bind(customer_scope_id)
    .fetch_all(&repository.pool)
    .await?;

    let financial_impact_rows: Vec<(bool, Option<String>, Option<Decimal>, Option<String>)> =
        sqlx::query_as(
            "SELECT confirmed_fwa, financial_impact_type, saving_amount, currency
                 FROM investigation_results ir
                 WHERE ($1::text IS NULL OR EXISTS (
                   SELECT 1 FROM audit_events ae
                   WHERE ae.event_type = 'investigation.result.received'
                     AND ae.event_status = 'succeeded'
                     AND ae.payload ->> 'investigation_id' = ir.investigation_id
                     AND ae.payload ->> 'customer_scope_id' = $1
                 ))
                 ORDER BY created_at, investigation_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&repository.pool)
        .await?;
    let financial_impacts = financial_impact_rows
        .into_iter()
        .filter_map(
            |(confirmed_fwa, financial_impact_type, saving_amount, currency)| {
                financial_impact_from_parts(
                    confirmed_fwa,
                    financial_impact_type.as_deref(),
                    saving_amount,
                    currency,
                )
            },
        )
        .collect::<Vec<_>>();

    let saving_attributions: Vec<(
        String,
        String,
        String,
        String,
        Option<Decimal>,
        String,
        i64,
        Vec<String>,
    )> = sqlx::query_as(
        "SELECT source_type,
                        source_id,
                        financial_impact_type,
                        action,
                        COALESCE(SUM(saving_amount), 0),
                        currency,
                        COUNT(DISTINCT claim_id)::bigint,
                        ARRAY_REMOVE(ARRAY_AGG(DISTINCT ref.value ORDER BY ref.value), NULL)
                 FROM saving_attributions s
                 LEFT JOIN LATERAL jsonb_array_elements_text(s.evidence_refs) AS ref(value) ON TRUE
                 WHERE ($1::text IS NULL OR EXISTS (
                   SELECT 1 FROM audit_events ae
                   WHERE ae.event_type = 'investigation.result.received'
                     AND ae.event_status = 'succeeded'
                     AND ae.payload ->> 'investigation_id' = s.investigation_id
                     AND ae.payload ->> 'customer_scope_id' = $1
                 ))
                 GROUP BY source_type, source_id, financial_impact_type, action, currency
                 ORDER BY source_type, source_id, financial_impact_type, action, currency",
    )
    .bind(customer_scope_id)
    .fetch_all(&repository.pool)
    .await?;
    let saving_segments: Vec<(String, String, Option<Decimal>, String, i64, i64)> =
        sqlx::query_as(
            "SELECT segment_type,
                        segment_id,
                        COALESCE(SUM(saving_amount), 0),
                        currency,
                        COUNT(DISTINCT claim_id)::bigint,
                        COUNT(*)::bigint
                 FROM (
                   SELECT 'provider'::text AS segment_type,
                          COALESCE(l.provider_id, 'unknown') AS segment_id,
                          s.saving_amount,
                          s.currency,
                          s.claim_id
                   FROM saving_attributions s
                   LEFT JOIN fwa_leads l ON l.claim_id = s.claim_id
                   WHERE ($1::text IS NULL OR EXISTS (
                     SELECT 1 FROM audit_events ae
                     WHERE ae.event_type = 'investigation.result.received'
                       AND ae.event_status = 'succeeded'
                       AND ae.payload ->> 'investigation_id' = s.investigation_id
                       AND ae.payload ->> 'customer_scope_id' = $1
                   ))
                   UNION ALL
                   SELECT 'scheme'::text AS segment_type,
                          COALESCE(l.scheme_family, 'unknown') AS segment_id,
                          s.saving_amount,
                          s.currency,
                          s.claim_id
                   FROM saving_attributions s
                   LEFT JOIN fwa_leads l ON l.claim_id = s.claim_id
                   WHERE ($1::text IS NULL OR EXISTS (
                     SELECT 1 FROM audit_events ae
                     WHERE ae.event_type = 'investigation.result.received'
                       AND ae.event_status = 'succeeded'
                       AND ae.payload ->> 'investigation_id' = s.investigation_id
                       AND ae.payload ->> 'customer_scope_id' = $1
                   ))
                   UNION ALL
                   SELECT 'campaign'::text AS segment_type,
                          COALESCE(NULLIF(regexp_replace(ref.value, '^campaigns?:', ''), ''), 'unknown') AS segment_id,
                          s.saving_amount,
                          s.currency,
                          s.claim_id
                   FROM saving_attributions s
                   CROSS JOIN LATERAL jsonb_array_elements_text(s.evidence_refs) AS ref(value)
                   WHERE (ref.value LIKE 'campaign:%'
                      OR ref.value LIKE 'campaigns:%')
                     AND ($1::text IS NULL OR EXISTS (
                       SELECT 1 FROM audit_events ae
                       WHERE ae.event_type = 'investigation.result.received'
                         AND ae.event_status = 'succeeded'
                         AND ae.payload ->> 'investigation_id' = s.investigation_id
                         AND ae.payload ->> 'customer_scope_id' = $1
                     ))
                 ) segments
                 GROUP BY segment_type, segment_id, currency
                 ORDER BY segment_type, segment_id, currency",
        )
        .bind(customer_scope_id)
        .fetch_all(&repository.pool)
        .await?;
    let outcome_labels = repository.list_outcome_labels(customer_scope_id).await?;
    let audit_samples = repository.list_audit_samples(customer_scope_id).await?;
    let qa_review_records = repository.list_qa_reviews(customer_scope_id).await?;
    let qa_feedback_items = repository.list_qa_feedback_items(customer_scope_id).await?;
    let agent_runs = repository.list_agent_runs(customer_scope_id).await?;
    let models = repository.list_models().await?;
    let model_evaluations = repository.list_model_evaluations().await?;
    let rules = repository.list_rules().await?;
    let rule_performance = repository.rule_performance().await?;

    Ok(DashboardSummaryRecord {
        suspected_claims: suspected.0 as u32,
        confirmed_fwa: investigation.1 as u32,
        risk_amount: suspected.1.unwrap_or(Decimal::ZERO).to_string(),
        saving_amount: investigation.2.unwrap_or(Decimal::ZERO).to_string(),
        rag_distribution: rag_rows
            .into_iter()
            .map(|(rag, count)| (rag, count as u32))
            .collect(),
        scheme_distribution: scheme_rows
            .into_iter()
            .map(|(scheme_family, count)| (scheme_family, count as u32))
            .collect(),
        rule_hits: rule_hits.0 as u32,
        model_scores: model_rows
            .into_iter()
            .map(|(model_key, scored_runs, average_score, high_risk_count)| {
                (
                    model_key,
                    DashboardModelScoreRecord {
                        scored_runs: scored_runs as u32,
                        average_score: average_score
                            .map(|value| value.to_string().parse().unwrap_or(0.0))
                            .unwrap_or(0.0),
                        high_risk_count: high_risk_count.unwrap_or(0) as u32,
                    },
                )
            })
            .collect(),
        layer_scores: layer_accumulators
            .into_iter()
            .map(
                |(layer_id, (name, scored_runs, score_sum, high_risk_count))| {
                    let average_score = if scored_runs == 0 {
                        0.0
                    } else {
                        score_sum as f64 / scored_runs as f64
                    };
                    (
                        layer_id,
                        DashboardLayerScoreRecord {
                            name,
                            scored_runs,
                            average_score,
                            high_risk_count,
                        },
                    )
                },
            )
            .collect(),
        saving_attributions: saving_attributions
            .into_iter()
            .map(
                |(
                    source_type,
                    source_id,
                    financial_impact_type,
                    action,
                    saving_amount,
                    currency,
                    claim_count,
                    evidence_refs,
                )| {
                    DashboardSavingAttributionRecord {
                        source_type,
                        source_id,
                        financial_impact_type,
                        action,
                        saving_amount: format_decimal_cents(saving_amount.unwrap_or(Decimal::ZERO)),
                        currency,
                        claim_count: claim_count as u32,
                        evidence_refs,
                    }
                },
            )
            .collect(),
        saving_segments: saving_segments
            .into_iter()
            .map(
                |(
                    segment_type,
                    segment_id,
                    saving_amount,
                    currency,
                    claim_count,
                    attribution_count,
                )| {
                    let saving_amount = saving_amount.unwrap_or(Decimal::ZERO);
                    let claim_count = claim_count as u32;
                    DashboardSavingSegmentRecord {
                        segment_type,
                        segment_id,
                        saving_amount: format_decimal_cents(saving_amount),
                        currency,
                        claim_count,
                        attribution_count: attribution_count as u32,
                        roi: segment_roi(saving_amount, claim_count),
                    }
                },
            )
            .collect(),
        value_measurement: summarize_dashboard_value_measurement(
            &financial_impacts,
            rule_hits.0 as u32,
            rule_performance
                .iter()
                .map(|record| record.false_positive_count)
                .sum::<u32>(),
        ),
        audit_coverage,
        label_pool: summarize_dashboard_label_pool(&outcome_labels),
        qa_queue: summarize_dashboard_qa_queue(
            &audit_samples,
            &qa_review_records,
            &qa_feedback_items,
        ),
        case_sla: summarize_dashboard_case_sla(&repository.list_cases(customer_scope_id).await?),
        agent_governance: summarize_dashboard_agent_governance(&agent_runs),
        model_governance: summarize_dashboard_model_governance(&models, &model_evaluations),
        rule_governance: summarize_dashboard_rule_governance(&rules, &rule_performance),
        investigation_results: investigation.0 as u32,
        qa_reviews: qa_reviews.0 as u32,
    })
}
