use super::*;

pub(super) async fn save_rule_backtest(
    repository: &PostgresScoringRepository,
    record: RuleBacktestRecord,
) -> anyhow::Result<RuleBacktestRecord> {
    let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
        "INSERT INTO rule_backtest_runs
             (rule_id, rule_version, sample_count, matched_count, reviewed_count,
              confirmed_fwa_count, false_positive_count, precision_value, recall_value,
              lift, false_positive_rate, estimated_saving, promotion_recommendation,
              blockers, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             RETURNING created_at",
    )
    .bind(&record.rule_id)
    .bind(record.rule_version as i32)
    .bind(record.sample_count as i32)
    .bind(record.matched_count as i32)
    .bind(record.reviewed_count as i32)
    .bind(record.confirmed_fwa_count as i32)
    .bind(record.false_positive_count as i32)
    .bind(record.precision)
    .bind(record.recall)
    .bind(record.lift)
    .bind(record.false_positive_rate)
    .bind(&record.estimated_saving)
    .bind(&record.promotion_recommendation)
    .bind(serde_json::json!(record.blockers))
    .bind(serde_json::json!(record.evidence_refs))
    .fetch_one(&repository.pool)
    .await?;
    Ok(RuleBacktestRecord {
        created_at: Some(row.0.to_rfc3339()),
        ..record
    })
}

pub(super) async fn latest_rule_backtest(
    repository: &PostgresScoringRepository,
    rule_id: &str,
    rule_version: u32,
) -> anyhow::Result<Option<RuleBacktestRecord>> {
    let row: Option<(
        String,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        f64,
        f64,
        f64,
        f64,
        String,
        String,
        Value,
        Value,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "SELECT rule_id, rule_version, sample_count, matched_count, reviewed_count,
                    confirmed_fwa_count, false_positive_count, precision_value, recall_value,
                    lift, false_positive_rate, estimated_saving, promotion_recommendation,
                    blockers, evidence_refs, created_at
             FROM rule_backtest_runs
             WHERE rule_id = $1 AND rule_version = $2
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
    )
    .bind(rule_id)
    .bind(rule_version as i32)
    .fetch_optional(&repository.pool)
    .await?;

    Ok(row.map(
        |(
            rule_id,
            rule_version,
            sample_count,
            matched_count,
            reviewed_count,
            confirmed_fwa_count,
            false_positive_count,
            precision,
            recall,
            lift,
            false_positive_rate,
            estimated_saving,
            promotion_recommendation,
            blockers,
            evidence_refs,
            created_at,
        )| RuleBacktestRecord {
            rule_id,
            rule_version: rule_version as u32,
            sample_count: sample_count.max(0) as u32,
            matched_count: matched_count.max(0) as u32,
            reviewed_count: reviewed_count.max(0) as u32,
            confirmed_fwa_count: confirmed_fwa_count.max(0) as u32,
            false_positive_count: false_positive_count.max(0) as u32,
            precision,
            recall,
            lift,
            false_positive_rate,
            estimated_saving,
            promotion_recommendation,
            blockers: json_array_to_strings(blockers),
            evidence_refs: json_array_to_strings(evidence_refs),
            created_at: Some(created_at.to_rfc3339()),
        },
    ))
}

pub(super) async fn save_rule_shadow_run(
    repository: &PostgresScoringRepository,
    record: RuleShadowRunRecord,
) -> anyhow::Result<RuleShadowRunRecord> {
    let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
        "INSERT INTO rule_shadow_runs
             (rule_id, rule_version, report_uri, decision, reviewer, notes,
              reviewed_count, matched_count, false_positive_count, false_positive_rate,
              blockers, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             RETURNING created_at",
    )
    .bind(&record.rule_id)
    .bind(record.rule_version as i32)
    .bind(&record.report_uri)
    .bind(&record.decision)
    .bind(&record.reviewer)
    .bind(&record.notes)
    .bind(record.reviewed_count as i32)
    .bind(record.matched_count as i32)
    .bind(record.false_positive_count as i32)
    .bind(record.false_positive_rate)
    .bind(serde_json::json!(record.blockers.clone()))
    .bind(serde_json::json!(record.evidence_refs.clone()))
    .fetch_one(&repository.pool)
    .await?;
    Ok(RuleShadowRunRecord {
        created_at: Some(row.0.to_rfc3339()),
        ..record
    })
}

pub(super) async fn latest_rule_shadow_run(
    repository: &PostgresScoringRepository,
    rule_id: &str,
    rule_version: u32,
) -> anyhow::Result<Option<RuleShadowRunRecord>> {
    let row: Option<(
        String,
        i32,
        String,
        String,
        String,
        String,
        i32,
        i32,
        i32,
        f64,
        Value,
        Value,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "SELECT rule_id, rule_version, report_uri, decision, reviewer, notes,
                    reviewed_count, matched_count, false_positive_count, false_positive_rate,
                    blockers, evidence_refs, created_at
             FROM rule_shadow_runs
             WHERE rule_id = $1 AND rule_version = $2
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
    )
    .bind(rule_id)
    .bind(rule_version as i32)
    .fetch_optional(&repository.pool)
    .await?;

    Ok(row.map(
        |(
            rule_id,
            rule_version,
            report_uri,
            decision,
            reviewer,
            notes,
            reviewed_count,
            matched_count,
            false_positive_count,
            false_positive_rate,
            blockers,
            evidence_refs,
            created_at,
        )| RuleShadowRunRecord {
            rule_id,
            rule_version: rule_version as u32,
            report_uri,
            decision,
            reviewer,
            notes,
            reviewed_count: reviewed_count.max(0) as u32,
            matched_count: matched_count.max(0) as u32,
            false_positive_count: false_positive_count.max(0) as u32,
            false_positive_rate,
            blockers: json_array_to_strings(blockers),
            evidence_refs: json_array_to_strings(evidence_refs),
            created_at: Some(created_at.to_rfc3339()),
        },
    ))
}

pub(super) async fn save_rule_promotion_review(
    repository: &PostgresScoringRepository,
    record: RulePromotionReviewRecord,
) -> anyhow::Result<RulePromotionReviewRecord> {
    let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
        "INSERT INTO rule_promotion_reviews
             (rule_id, rule_version, decision, reviewer, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING created_at",
    )
    .bind(&record.rule_id)
    .bind(record.rule_version as i32)
    .bind(&record.decision)
    .bind(&record.reviewer)
    .bind(&record.notes)
    .bind(serde_json::json!(record.evidence_refs.clone()))
    .fetch_one(&repository.pool)
    .await?;
    Ok(RulePromotionReviewRecord {
        created_at: Some(row.0.to_rfc3339()),
        ..record
    })
}

pub(super) async fn latest_rule_promotion_review(
    repository: &PostgresScoringRepository,
    rule_id: &str,
    rule_version: u32,
) -> anyhow::Result<Option<RulePromotionReviewRecord>> {
    let row: Option<(
        String,
        i32,
        String,
        String,
        String,
        serde_json::Value,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "SELECT rule_id, rule_version, decision, reviewer, notes, evidence_refs, created_at
                 FROM rule_promotion_reviews
                 WHERE rule_id = $1 AND rule_version = $2
                 ORDER BY created_at DESC
                 LIMIT 1",
    )
    .bind(rule_id)
    .bind(rule_version as i32)
    .fetch_optional(&repository.pool)
    .await?;
    Ok(row.map(
        |(rule_id, rule_version, decision, reviewer, notes, evidence_refs, created_at)| {
            RulePromotionReviewRecord {
                rule_id,
                rule_version: rule_version as u32,
                decision,
                reviewer,
                notes,
                evidence_refs: json_array_to_strings(evidence_refs),
                created_at: Some(created_at.to_rfc3339()),
            }
        },
    ))
}
