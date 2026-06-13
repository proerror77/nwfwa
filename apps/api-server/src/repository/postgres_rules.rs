use super::*;

pub(super) async fn list_rules(
    repository: &PostgresScoringRepository,
) -> anyhow::Result<Vec<RuleSummaryRecord>> {
    ensure_default_rules_seeded(&repository.pool).await?;
    let rows: Vec<(
        String,
        String,
        String,
        String,
        Option<String>,
        i32,
        Value,
        i32,
        String,
    )> = sqlx::query_as(
        "SELECT r.rule_key, r.name, r.status, r.owner, r.submitted_by_actor_id, rv.version, rv.dsl, rv.score, rv.recommended_action
             FROM rules r
             JOIN LATERAL (
               SELECT version, dsl, score, recommended_action
               FROM rule_versions
               WHERE rule_id = r.id
               ORDER BY version DESC
               LIMIT 1
             ) rv ON true
             ORDER BY r.rule_key",
    )
    .fetch_all(&repository.pool)
    .await?;

    let summaries = rows
        .into_iter()
        .map(
            |(
                rule_id,
                name,
                status,
                owner,
                submitted_by_actor_id,
                version,
                dsl,
                score,
                recommended_action,
            )| {
                let action = dsl.get("action").cloned().unwrap_or(Value::Null);
                let review_mode = review_mode_from_dsl(&dsl);
                let scheme_family = scheme_family_from_dsl(&dsl);
                RuleSummaryRecord {
                    rule_id: rule_id.clone(),
                    name,
                    active_version: if status == "active" {
                        Some(version as u32)
                    } else {
                        None
                    },
                    latest_version: version as u32,
                    review_mode: review_mode.clone(),
                    scheme_family: scheme_family.clone(),
                    status,
                    owner,
                    submitted_by_actor_id,
                    score: score as u8,
                    alert_code: action["alert_code"]
                        .as_str()
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    recommended_action: parse_recommended_action(&recommended_action),
                    applicability_scope: rule_applicability_scope(&review_mode, &scheme_family),
                    backtest_result: default_rule_backtest_summary(),
                    estimated_saving: "0.00".into(),
                    false_positive_history: default_rule_false_positive_history(),
                    evidence_refs: rule_governance_evidence_refs(&rule_id, version as u32),
                }
            },
        )
        .collect::<Vec<_>>();

    // Batch-fetch the latest backtest for all rules in a single query instead of N individual
    // queries. We group by (rule_id, rule_version) and pick the row with the highest created_at
    // (ties broken by id DESC to match the per-rule query in postgres_rule_reviews).
    let backtest_rows: Vec<(
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
        "SELECT rb.rule_id, rb.rule_version, rb.sample_count, rb.matched_count,
                rb.reviewed_count, rb.confirmed_fwa_count, rb.false_positive_count,
                rb.precision_value, rb.recall_value, rb.lift, rb.false_positive_rate,
                rb.estimated_saving, rb.promotion_recommendation,
                rb.blockers, rb.evidence_refs, rb.created_at
         FROM rule_backtest_runs rb
         INNER JOIN (
             SELECT rule_id, rule_version, MAX(created_at) AS max_created
             FROM rule_backtest_runs
             GROUP BY rule_id, rule_version
         ) latest ON rb.rule_id = latest.rule_id
                 AND rb.rule_version = latest.rule_version
                 AND rb.created_at = latest.max_created",
    )
    .fetch_all(&repository.pool)
    .await?;

    // Build a HashMap keyed by (rule_id, rule_version) for O(1) lookup.
    let mut backtest_map: std::collections::HashMap<(String, u32), RuleBacktestRecord> =
        std::collections::HashMap::new();
    for (
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
    ) in backtest_rows
    {
        let key = (rule_id.clone(), rule_version.max(0) as u32);
        backtest_map.insert(
            key,
            RuleBacktestRecord {
                rule_id,
                rule_version: rule_version.max(0) as u32,
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
        );
    }

    let mut summaries = summaries;
    for summary in &mut summaries {
        let key = (summary.rule_id.clone(), summary.latest_version);
        let latest_backtest = backtest_map.get(&key);
        apply_rule_backtest_metadata(summary, latest_backtest);
    }

    Ok(summaries)
}

pub(super) async fn list_active_rules(
    repository: &PostgresScoringRepository,
) -> anyhow::Result<Vec<Rule>> {
    ensure_default_rules_seeded(&repository.pool).await?;
    let rows: Vec<(String, String, i32, Value)> = sqlx::query_as(
        "SELECT r.rule_key, r.name, rv.version, rv.dsl
             FROM rules r
             JOIN LATERAL (
               SELECT version, dsl
               FROM rule_versions
               WHERE rule_id = r.id
               ORDER BY version DESC
               LIMIT 1
             ) rv ON true
             WHERE r.status = 'active'
             ORDER BY r.rule_key",
    )
    .fetch_all(&repository.pool)
    .await?;

    rows.into_iter()
        .map(|(rule_id, name, version, dsl)| {
            runtime_rule_from_parts(rule_id, name, version as u32, dsl)
        })
        .collect()
}

pub(super) async fn get_rule(
    repository: &PostgresScoringRepository,
    rule_id: &str,
) -> anyhow::Result<Option<RuleDetailRecord>> {
    ensure_default_rules_seeded(&repository.pool).await?;

    // Direct single-rule query — avoids fetching all rules and filtering in memory.
    let summary_row: Option<(
        String,
        String,
        String,
        String,
        Option<String>,
        i32,
        Value,
        i32,
        String,
    )> =
        sqlx::query_as(
            "SELECT r.rule_key, r.name, r.status, r.owner, r.submitted_by_actor_id, rv.version, rv.dsl, rv.score, rv.recommended_action
             FROM rules r
             JOIN LATERAL (
               SELECT version, dsl, score, recommended_action
               FROM rule_versions
               WHERE rule_id = r.id
               ORDER BY version DESC
               LIMIT 1
             ) rv ON true
             WHERE r.rule_key = $1",
        )
        .bind(rule_id)
        .fetch_optional(&repository.pool)
        .await?;

    let Some((
        rid,
        name,
        status,
        owner,
        submitted_by_actor_id,
        version,
        dsl,
        score,
        recommended_action,
    )) = summary_row
    else {
        return Ok(None);
    };

    let action = dsl.get("action").cloned().unwrap_or(Value::Null);
    let review_mode = review_mode_from_dsl(&dsl);
    let scheme_family = scheme_family_from_dsl(&dsl);
    let mut summary = RuleSummaryRecord {
        rule_id: rid.clone(),
        name,
        active_version: if status == "active" {
            Some(version as u32)
        } else {
            None
        },
        latest_version: version as u32,
        review_mode: review_mode.clone(),
        scheme_family: scheme_family.clone(),
        status,
        owner,
        submitted_by_actor_id,
        score: score as u8,
        alert_code: action["alert_code"]
            .as_str()
            .unwrap_or("UNKNOWN")
            .to_string(),
        recommended_action: parse_recommended_action(&recommended_action),
        applicability_scope: rule_applicability_scope(&review_mode, &scheme_family),
        backtest_result: default_rule_backtest_summary(),
        estimated_saving: "0.00".into(),
        false_positive_history: default_rule_false_positive_history(),
        evidence_refs: rule_governance_evidence_refs(&rid, version as u32),
    };

    let latest_backtest = postgres_rule_reviews::latest_rule_backtest(
        repository,
        &summary.rule_id,
        summary.latest_version,
    )
    .await?;
    apply_rule_backtest_metadata(&mut summary, latest_backtest.as_ref());

    let rows: Vec<(i32, Value, i32, String)> = sqlx::query_as(
        "SELECT rv.version, rv.dsl, rv.score, rv.recommended_action
             FROM rule_versions rv
             JOIN rules r ON r.id = rv.rule_id
             WHERE r.rule_key = $1
             ORDER BY rv.version DESC",
    )
    .bind(rule_id)
    .fetch_all(&repository.pool)
    .await?;

    let versions = rows
        .into_iter()
        .map(|(version, dsl, score, recommended_action)| {
            let action = dsl.get("action").cloned().unwrap_or(Value::Null);
            RuleVersionRecord {
                version: version as u32,
                status: summary.status.clone(),
                review_mode: review_mode_from_dsl(&dsl),
                scheme_family: scheme_family_from_dsl(&dsl),
                dsl,
                score: score as u8,
                alert_code: action["alert_code"]
                    .as_str()
                    .unwrap_or("UNKNOWN")
                    .to_string(),
                recommended_action: parse_recommended_action(&recommended_action),
                reason: action["reason"].as_str().unwrap_or("").to_string(),
            }
        })
        .collect();

    let audit_events = rule_audit_history(repository, rule_id).await?;

    Ok(Some(RuleDetailRecord {
        summary,
        versions,
        audit_events,
    }))
}

pub(super) async fn rule_audit_history(
    repository: &PostgresScoringRepository,
    rule_id: &str,
) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
    let rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        Value,
        Value,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "SELECT audit_id, run_id, actor_role, event_type, event_status, summary, payload, evidence_refs, created_at
                 FROM audit_events
                 WHERE payload ->> 'rule_id' = $1
                 ORDER BY created_at, audit_id",
    )
    .bind(rule_id)
    .fetch_all(&repository.pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(
                audit_id,
                run_id,
                actor_role,
                event_type,
                event_status,
                summary,
                payload,
                evidence_refs,
                created_at,
            )| AuditHistoryEventRecord {
                audit_id,
                run_id,
                actor_role,
                event_type,
                event_status,
                summary,
                payload,
                evidence_refs: json_array_to_strings(evidence_refs),
                created_at: Some(created_at.to_rfc3339()),
            },
        )
        .collect())
}

pub(super) async fn save_rule_candidate(
    repository: &PostgresScoringRepository,
    rule: Rule,
    owner: String,
) -> anyhow::Result<RuleDetailRecord> {
    ensure_default_rules_seeded(&repository.pool).await?;
    ensure_rule_condition_library_table(&repository.pool).await?;
    let detail = rule_detail_from_rule(rule, "draft", owner);
    let mut tx = repository.pool.begin().await?;
    let row: (String,) = sqlx::query_as(
        "INSERT INTO rules (rule_key, name, status, owner)
             VALUES ($1, $2, 'draft', $3)
             ON CONFLICT (rule_key) DO UPDATE
             SET name = EXCLUDED.name,
                 status = 'draft',
                 owner = EXCLUDED.owner,
                 updated_at = now()
             RETURNING id::text",
    )
    .bind(&detail.summary.rule_id)
    .bind(&detail.summary.name)
    .bind(&detail.summary.owner)
    .fetch_one(&mut *tx)
    .await?;

    let version = &detail.versions[0];
    sqlx::query(
        "INSERT INTO rule_versions
             (rule_id, version, dsl, score, recommended_action, created_by)
             VALUES ($1::uuid, $2, $3, $4, $5, $6)
             ON CONFLICT (rule_id, version) DO UPDATE
             SET dsl = EXCLUDED.dsl,
                 score = EXCLUDED.score,
                 recommended_action = EXCLUDED.recommended_action",
    )
    .bind(&row.0)
    .bind(version.version as i32)
    .bind(&version.dsl)
    .bind(version.score as i32)
    .bind(format!("{:?}", version.recommended_action))
    .bind(&detail.summary.owner)
    .execute(&mut *tx)
    .await?;

    upsert_rule_conditions_tx(&mut tx, &row.0, &detail).await?;

    tx.commit().await?;
    Ok(detail)
}

pub(super) async fn update_rule_status(
    repository: &PostgresScoringRepository,
    rule_id: &str,
    status: &str,
    status_actor_id: Option<&str>,
) -> anyhow::Result<Option<RuleSummaryRecord>> {
    ensure_default_rules_seeded(&repository.pool).await?;
    ensure_rule_condition_library_table(&repository.pool).await?;
    let result = sqlx::query(
        "UPDATE rules
             SET status = $1,
                 submitted_by_actor_id = CASE
                     WHEN $1 = 'submitted' THEN $3
                     ELSE submitted_by_actor_id
                 END,
                 updated_at = now()
             WHERE rule_key = $2",
    )
    .bind(status)
    .bind(rule_id)
    .bind(status_actor_id)
    .execute(&repository.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    sqlx::query(
        "UPDATE rule_condition_library
             SET status = $1, updated_at = now()
             WHERE source_rule_key = $2",
    )
    .bind(rule_condition_status(status))
    .bind(rule_id)
    .execute(&repository.pool)
    .await?;
    Ok(list_rules(repository)
        .await?
        .into_iter()
        .find(|rule| rule.rule_id == rule_id))
}

pub(super) async fn list_rule_conditions(
    repository: &PostgresScoringRepository,
) -> anyhow::Result<Vec<RuleConditionLibraryRecord>> {
    ensure_default_rules_seeded(&repository.pool).await?;
    ensure_rule_condition_library_table(&repository.pool).await?;
    let rows: Vec<(
        String,
        String,
        i32,
        i32,
        String,
        String,
        Value,
        String,
        String,
        String,
        String,
        Value,
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "SELECT condition_key, source_rule_key, source_rule_version, condition_index,
                    field_name, operator, value, review_mode, scheme_family, status, owner,
                    evidence_refs, created_at, updated_at
             FROM rule_condition_library
             ORDER BY source_rule_key, source_rule_version, condition_index",
    )
    .fetch_all(&repository.pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(
                condition_key,
                source_rule_key,
                source_rule_version,
                condition_index,
                field,
                operator,
                value,
                review_mode,
                scheme_family,
                status,
                owner,
                evidence_refs,
                created_at,
                updated_at,
            )| RuleConditionLibraryRecord {
                condition_key,
                source_rule_key,
                source_rule_version: source_rule_version.max(0) as u32,
                condition_index: condition_index.max(0) as u32,
                field,
                operator,
                value,
                review_mode,
                scheme_family,
                status,
                owner,
                evidence_refs: json_array_to_strings(evidence_refs),
                created_at: Some(created_at.to_rfc3339()),
                updated_at: Some(updated_at.to_rfc3339()),
            },
        )
        .collect())
}

pub(super) async fn rule_performance(
    repository: &PostgresScoringRepository,
) -> anyhow::Result<Vec<RulePerformanceRecord>> {
    ensure_default_rules_seeded(&repository.pool).await?;
    let rules = list_rules(repository).await?;
    let total_runs: (i64,) =
        sqlx::query_as("SELECT COUNT(*)::bigint FROM scoring_runs WHERE status = 'succeeded'")
            .fetch_one(&repository.pool)
            .await?;

    let rule_run_rows: Vec<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT r.rule_key, rr.alert_code, c.external_claim_id
             FROM rule_runs rr
             JOIN scoring_runs sr ON sr.run_id = rr.run_id
             LEFT JOIN rules r ON r.id = rr.rule_id
             LEFT JOIN claims c ON c.id = sr.claim_id
             WHERE rr.matched = true",
    )
    .fetch_all(&repository.pool)
    .await?;

    let outcome_rows: Vec<(String, bool, Option<Decimal>)> = sqlx::query_as(
        "SELECT claim_id, confirmed_fwa, saving_amount
             FROM investigation_results",
    )
    .fetch_all(&repository.pool)
    .await?;
    let outcomes = outcome_rows
        .into_iter()
        .map(|(claim_id, confirmed_fwa, saving_amount)| {
            (
                claim_id,
                InvestigationOutcome {
                    confirmed_fwa,
                    saving_amount: saving_amount.unwrap_or(Decimal::ZERO),
                },
            )
        })
        .collect::<HashMap<_, _>>();

    let alert_to_rule = rules
        .iter()
        .map(|rule| (rule.alert_code.clone(), rule.rule_id.clone()))
        .collect::<HashMap<_, _>>();
    let mut accumulators = rule_accumulators_from_rules(&rules);
    for (rule_id, alert_code, claim_id) in rule_run_rows {
        let rule_id = rule_id.or_else(|| {
            alert_code
                .as_ref()
                .and_then(|alert_code| alert_to_rule.get(alert_code).cloned())
        });
        let (Some(rule_id), Some(claim_id)) = (rule_id, claim_id) else {
            continue;
        };
        let Some(accumulator) = accumulators.get_mut(&rule_id) else {
            continue;
        };
        accumulator.trigger_count += 1;
        accumulator.triggered_claim_ids.insert(claim_id);
    }

    Ok(rule_performance_records(
        accumulators,
        &outcomes,
        total_runs.0.max(0) as u32,
    ))
}
