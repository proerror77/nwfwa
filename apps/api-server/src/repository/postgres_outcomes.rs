use super::*;

pub(super) async fn save_investigation_result(
    repository: &PostgresScoringRepository,
    record: InvestigationResultRecord,
) -> anyhow::Result<AuditHistoryEventRecord> {
    let saving_attributions = derive_saving_attributions(&record);
    let mut tx = repository.pool.begin().await?;
    let previous_case_id: Option<String> =
        sqlx::query_scalar("SELECT case_id FROM investigation_results WHERE investigation_id = $1")
            .bind(&record.investigation_id)
            .fetch_optional(&mut *tx)
            .await?;
    sqlx::query(
        "INSERT INTO investigation_results
         (investigation_id, case_id, claim_id, outcome, confirmed_fwa, financial_impact_type, saving_amount, currency, notes, evidence_refs)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         ON CONFLICT (investigation_id) DO UPDATE
         SET case_id = EXCLUDED.case_id,
             claim_id = EXCLUDED.claim_id,
             outcome = EXCLUDED.outcome,
             confirmed_fwa = EXCLUDED.confirmed_fwa,
             financial_impact_type = EXCLUDED.financial_impact_type,
             saving_amount = EXCLUDED.saving_amount,
             currency = EXCLUDED.currency,
             notes = EXCLUDED.notes,
             evidence_refs = EXCLUDED.evidence_refs",
    )
    .bind(&record.investigation_id)
    .bind(&record.case_id)
    .bind(&record.claim_id)
    .bind(&record.outcome)
    .bind(record.confirmed_fwa)
    .bind(normalize_financial_impact_type(
        record.financial_impact_type.as_deref(),
    ))
    .bind(record.saving_amount)
    .bind(&record.currency)
    .bind(&record.notes)
    .bind(serde_json::json!(record.evidence_refs))
    .execute(&mut *tx)
    .await?;

    if previous_case_id.as_deref() != record.case_id.as_deref() {
        if let Some(case_id) = previous_case_id.as_deref() {
            sqlx::query(
                "UPDATE investigation_cases
                 SET final_outcome = NULL,
                     reviewer_notes = NULL,
                     investigation_result_id = NULL,
                     updated_at = now()
                 WHERE case_id = $1
                   AND investigation_result_id = $2",
            )
            .bind(case_id)
            .bind(&record.investigation_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    if let Some(case_id) = record.case_id.as_deref() {
        let update = sqlx::query(
            "UPDATE investigation_cases
             SET final_outcome = $1,
                 reviewer_notes = $2,
                 investigation_result_id = $3,
                 updated_at = now()
             WHERE case_id = $4
               AND claim_id = $5",
        )
        .bind(&record.outcome)
        .bind(&record.notes)
        .bind(&record.investigation_id)
        .bind(case_id)
        .bind(&record.claim_id)
        .execute(&mut *tx)
        .await?;
        if update.rows_affected() == 0 {
            anyhow::bail!("case not found for investigation result: {case_id}");
        }
    }

    sqlx::query("DELETE FROM saving_attributions WHERE investigation_id = $1")
        .bind(&record.investigation_id)
        .execute(&mut *tx)
        .await?;
    for attribution in saving_attributions {
        sqlx::query(
            "INSERT INTO saving_attributions
             (attribution_id, claim_id, investigation_id, source_type, source_id, financial_impact_type, action, saving_amount, currency, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(&attribution.attribution_id)
        .bind(&attribution.claim_id)
        .bind(&attribution.investigation_id)
        .bind(&attribution.source_type)
        .bind(&attribution.source_id)
        .bind(&attribution.financial_impact_type)
        .bind(&attribution.action)
        .bind(attribution.saving_amount)
        .bind(&attribution.currency)
        .bind(serde_json::json!(attribution.evidence_refs))
        .execute(&mut *tx)
        .await?;
    }

    let event = AuditHistoryEventRecord {
        audit_id: format!("audit_investigation_{}", record.investigation_id),
        run_id: format!("pilot_investigation_{}", record.investigation_id),
        actor_role: record
            .actor_role
            .clone()
            .unwrap_or_else(|| "tpa_system".into()),
        event_type: "investigation.result.received".into(),
        event_status: "succeeded".into(),
        summary: format!("Investigation result received: {}", record.outcome),
        payload: serde_json::to_value(&record)?,
        evidence_refs: record.evidence_refs.clone(),
        created_at: None,
    };
    insert_pilot_audit_event(&mut tx, &record.claim_id, &event).await?;
    tx.commit().await?;
    Ok(event)
}

pub(super) async fn save_qa_review(
    repository: &PostgresScoringRepository,
    mut record: QaReviewRecord,
) -> anyhow::Result<AuditHistoryEventRecord> {
    record.feedback_target = canonical_feedback_target(&record.feedback_target).into();
    let mut tx = repository.pool.begin().await?;
    sqlx::query(
        "INSERT INTO qa_reviews
         (qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, feedback_status, notes, evidence_refs)
         VALUES ($1, $2, $3, $4, $5, 'open', $6, $7)
         ON CONFLICT (qa_case_id) DO UPDATE
         SET qa_conclusion = EXCLUDED.qa_conclusion,
             issue_type = EXCLUDED.issue_type,
             feedback_target = EXCLUDED.feedback_target,
             feedback_status = EXCLUDED.feedback_status,
             notes = EXCLUDED.notes,
             evidence_refs = EXCLUDED.evidence_refs",
    )
    .bind(&record.qa_case_id)
    .bind(&record.claim_id)
    .bind(&record.qa_conclusion)
    .bind(&record.issue_type)
    .bind(&record.feedback_target)
    .bind(&record.notes)
    .bind(serde_json::json!(record.evidence_refs))
    .execute(&mut *tx)
    .await?;

    let event = AuditHistoryEventRecord {
        audit_id: format!("audit_qa_{}", record.qa_case_id),
        run_id: format!("pilot_qa_{}", record.qa_case_id),
        actor_role: record
            .actor_role
            .clone()
            .unwrap_or_else(|| "tpa_system".into()),
        event_type: "qa.result.received".into(),
        event_status: "succeeded".into(),
        summary: format!("QA result received: {}", record.qa_conclusion),
        payload: serde_json::to_value(&record)?,
        evidence_refs: record.evidence_refs.clone(),
        created_at: None,
    };
    insert_pilot_audit_event(&mut tx, &record.claim_id, &event).await?;
    tx.commit().await?;
    Ok(event)
}

pub(super) async fn list_qa_feedback_items(
    repository: &PostgresScoringRepository,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<QaFeedbackItemRecord>> {
    let allowed_qa_case_ids = if let Some(scope) = customer_scope_id {
        Some(
            repository
                .list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("qa.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| event.payload["qa_case_id"].as_str().map(str::to_string))
                .collect::<BTreeSet<_>>(),
        )
    } else {
        None
    };
    let mut status_events = repository
        .list_audit_events(AuditEventListFilter {
            limit: 10_000,
            event_type: Some("qa.feedback.status.updated".into()),
            customer_scope_id: customer_scope_id.map(str::to_string),
            ..Default::default()
        })
        .await?;
    status_events.reverse();
    let feedback_statuses = latest_qa_feedback_statuses(
        &status_events
            .into_iter()
            .map(|event| {
                (
                    event.payload["claim_id"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    event,
                )
            })
            .collect::<Vec<_>>(),
    );
    let rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Value,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "SELECT qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, feedback_status, notes, evidence_refs, created_at
         FROM qa_reviews
         WHERE qa_conclusion <> 'pass'
         ORDER BY created_at, qa_case_id",
    )
    .fetch_all(&repository.pool)
    .await?;
    let mut items = rows
        .into_iter()
        .filter(|(qa_case_id, _, _, _, _, _, _, _, _)| {
            allowed_qa_case_ids
                .as_ref()
                .is_none_or(|ids| ids.contains(qa_case_id))
        })
        .map(
            |(
                qa_case_id,
                claim_id,
                qa_conclusion,
                issue_type,
                feedback_target,
                feedback_status,
                notes,
                evidence_refs,
                created_at,
            )| {
                let feedback_id = qa_feedback_id(&qa_case_id);
                let status_update = feedback_statuses.get(&feedback_id);
                qa_review_to_feedback_item(
                    QaReviewRecord {
                        qa_case_id,
                        claim_id,
                        qa_conclusion,
                        issue_type,
                        feedback_target,
                        notes,
                        evidence_refs: json_array_to_strings(evidence_refs),
                        customer_scope_id: None,
                        actor_id: None,
                        actor_role: None,
                    },
                    Some(created_at.to_rfc3339()),
                    &feedback_status,
                    status_update,
                )
            },
        )
        .collect::<Vec<_>>();
    sort_qa_feedback_items(&mut items);
    Ok(items)
}

pub(super) async fn update_qa_feedback_status(
    repository: &PostgresScoringRepository,
    feedback_id: &str,
    input: UpdateQaFeedbackStatusInput,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Option<UpdateQaFeedbackStatusRecord>> {
    let Some(qa_case_id) = qa_case_id_from_feedback_id(feedback_id) else {
        return Ok(None);
    };
    if let Some(scope) = customer_scope_id {
        let is_in_scope = repository
            .list_audit_events(AuditEventListFilter {
                limit: 1,
                event_type: Some("qa.result.received".into()),
                qa_case_id: Some(qa_case_id.into()),
                customer_scope_id: Some(scope.into()),
                ..Default::default()
            })
            .await?
            .into_iter()
            .next()
            .is_some();
        if !is_in_scope {
            return Ok(None);
        }
    }
    let mut tx = repository.pool.begin().await?;
    let row: Option<(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Value,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "WITH existing AS (
             SELECT qa_case_id, feedback_status AS from_status
             FROM qa_reviews
             WHERE qa_case_id = $1 AND qa_conclusion <> 'pass'
         ),
         updated AS (
             UPDATE qa_reviews
             SET feedback_status = $2
             FROM existing
             WHERE qa_reviews.qa_case_id = existing.qa_case_id
             RETURNING existing.from_status,
                       qa_reviews.qa_case_id,
                       qa_reviews.claim_id,
                       qa_reviews.qa_conclusion,
                       qa_reviews.issue_type,
                       qa_reviews.feedback_target,
                       qa_reviews.feedback_status,
                       qa_reviews.notes,
                       qa_reviews.evidence_refs,
                       qa_reviews.created_at
         )
         SELECT * FROM updated",
    )
    .bind(qa_case_id)
    .bind(&input.status)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((
        from_status,
        qa_case_id,
        claim_id,
        qa_conclusion,
        issue_type,
        feedback_target,
        feedback_status,
        notes,
        evidence_refs,
        created_at,
    )) = row
    else {
        return Ok(None);
    };
    let audit_id = AuditEventId::new().to_string();
    let item = qa_review_to_feedback_item(
        QaReviewRecord {
            qa_case_id,
            claim_id: claim_id.clone(),
            qa_conclusion,
            issue_type,
            feedback_target,
            notes,
            evidence_refs: json_array_to_strings(evidence_refs),
            customer_scope_id: None,
            actor_id: None,
            actor_role: None,
        },
        Some(created_at.to_rfc3339()),
        &feedback_status,
        Some(&QaFeedbackStatusUpdate {
            status: feedback_status.clone(),
            actor_id: Some(input.actor_id.clone()),
            audit_id: audit_id.clone(),
            updated_at: None,
            evidence_refs: input.evidence_refs.clone(),
        }),
    );
    insert_pilot_audit_event(
        &mut tx,
        &claim_id,
        &AuditHistoryEventRecord {
            audit_id: audit_id.clone(),
            run_id: format!("qa_feedback_status_{}", item.feedback_id),
            actor_role: "fwa_operator".into(),
            event_type: "qa.feedback.status.updated".into(),
            event_status: "succeeded".into(),
            summary: format!("QA feedback status updated: {}", item.status),
            payload: serde_json::json!({
                "feedback_id": item.feedback_id,
                "qa_case_id": item.qa_case_id,
                "claim_id": item.claim_id,
                "feedback_target": item.feedback_target,
                "from_status": from_status,
                "to_status": item.status,
                "actor_id": input.actor_id,
                "notes": input.notes,
                "customer_scope_id": input.customer_scope_id
            }),
            evidence_refs: input.evidence_refs,
            created_at: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Some(UpdateQaFeedbackStatusRecord { item, audit_id }))
}

pub(super) async fn list_qa_reviews(
    repository: &PostgresScoringRepository,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<QaReviewRecord>> {
    let allowed_qa_case_ids = if let Some(scope) = customer_scope_id {
        Some(
            repository
                .list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("qa.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| event.payload["qa_case_id"].as_str().map(str::to_string))
                .collect::<BTreeSet<_>>(),
        )
    } else {
        None
    };
    let rows: Vec<(String, String, String, String, String, String, Value)> = sqlx::query_as(
        "SELECT qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, notes, evidence_refs
         FROM qa_reviews
         ORDER BY qa_case_id",
    )
    .fetch_all(&repository.pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter(|(qa_case_id, _, _, _, _, _, _)| {
            allowed_qa_case_ids
                .as_ref()
                .is_none_or(|ids| ids.contains(qa_case_id))
        })
        .map(
            |(
                qa_case_id,
                claim_id,
                qa_conclusion,
                issue_type,
                feedback_target,
                notes,
                evidence_refs,
            )| {
                let feedback_target = canonical_feedback_target(&feedback_target).into();
                QaReviewRecord {
                    qa_case_id,
                    claim_id,
                    qa_conclusion,
                    issue_type,
                    feedback_target,
                    notes,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    customer_scope_id: None,
                    actor_id: None,
                    actor_role: None,
                }
            },
        )
        .collect())
}

pub(super) async fn list_outcome_labels(
    repository: &PostgresScoringRepository,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<OutcomeLabelRecord>> {
    let allowed_investigation_ids = if let Some(scope) = customer_scope_id {
        Some(
            repository
                .list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("investigation.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| {
                    event.payload["investigation_id"]
                        .as_str()
                        .map(str::to_string)
                })
                .collect::<BTreeSet<_>>(),
        )
    } else {
        None
    };
    let allowed_qa_case_ids = if let Some(scope) = customer_scope_id {
        Some(
            repository
                .list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("qa.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| event.payload["qa_case_id"].as_str().map(str::to_string))
                .collect::<BTreeSet<_>>(),
        )
    } else {
        None
    };
    let investigation_rows: Vec<(
        String,
        String,
        String,
        bool,
        Option<String>,
        Option<Decimal>,
        Option<String>,
        String,
        Value,
    )> = sqlx::query_as(
        "SELECT investigation_id, claim_id, outcome, confirmed_fwa, financial_impact_type, saving_amount, currency, notes, evidence_refs
         FROM investigation_results
         ORDER BY created_at, investigation_id",
    )
    .fetch_all(&repository.pool)
    .await?;
    let qa_rows: Vec<(String, String, String, String, String, String, String, Value)> =
        sqlx::query_as(
            "SELECT qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, feedback_status, notes, evidence_refs
             FROM qa_reviews
             ORDER BY created_at, qa_case_id",
        )
        .fetch_all(&repository.pool)
        .await?;
    let medical_review_rows: Vec<(String, String, Value, Value)> = sqlx::query_as(
        "SELECT audit_id, actor_role, payload, evidence_refs
         FROM audit_events
         WHERE event_type = 'medical.review.recorded'
           AND event_status = 'succeeded'
           AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)
         ORDER BY created_at, audit_id",
    )
    .bind(customer_scope_id)
    .fetch_all(&repository.pool)
    .await?;
    let lead_triage_rows: Vec<(String, String, String, Value, Value)> = sqlx::query_as(
        "SELECT audit_id, run_id, actor_role, payload, evidence_refs
         FROM audit_events
         WHERE event_type = 'lead.triaged'
           AND event_status = 'succeeded'
           AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)
         ORDER BY created_at, audit_id",
    )
    .bind(customer_scope_id)
    .fetch_all(&repository.pool)
    .await?;
    let label_bootstrap_rows: Vec<(String, String, String, Value, Value)> = sqlx::query_as(
        "SELECT audit_id, run_id, actor_role, payload, evidence_refs
         FROM audit_events
         WHERE event_type = 'label.bootstrap.reviewed'
           AND event_status = 'succeeded'
           AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)
         ORDER BY created_at, audit_id",
    )
    .bind(customer_scope_id)
    .fetch_all(&repository.pool)
    .await?;

    let mut labels = investigation_rows
        .into_iter()
        .filter(|(investigation_id, _, _, _, _, _, _, _, _)| {
            allowed_investigation_ids
                .as_ref()
                .is_none_or(|ids| ids.contains(investigation_id))
        })
        .flat_map(
            |(
                investigation_id,
                claim_id,
                outcome,
                confirmed_fwa,
                financial_impact_type,
                saving_amount,
                currency,
                notes,
                evidence_refs,
            )| {
                labels_from_investigation_result(InvestigationResultRecord {
                    investigation_id,
                    case_id: None,
                    claim_id,
                    outcome,
                    confirmed_fwa,
                    financial_impact_type,
                    saving_amount,
                    currency,
                    notes,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    customer_scope_id: None,
                    actor_id: None,
                    actor_role: None,
                })
            },
        )
        .chain(
            qa_rows
                .into_iter()
                .filter(|(qa_case_id, _, _, _, _, _, _, _)| {
                    allowed_qa_case_ids
                        .as_ref()
                        .is_none_or(|ids| ids.contains(qa_case_id))
                })
                .map(
                    |(
                        qa_case_id,
                        claim_id,
                        qa_conclusion,
                        issue_type,
                        feedback_target,
                        feedback_status,
                        notes,
                        evidence_refs,
                    )| {
                        label_from_qa_review(
                            QaReviewRecord {
                                qa_case_id,
                                claim_id,
                                qa_conclusion,
                                issue_type,
                                feedback_target,
                                notes,
                                evidence_refs: json_array_to_strings(evidence_refs),
                                customer_scope_id: None,
                                actor_id: None,
                                actor_role: None,
                            },
                            &feedback_status,
                        )
                    },
                ),
        )
        .chain(medical_review_rows.into_iter().flat_map(
            |(audit_id, actor_role, payload, evidence_refs)| {
                labels_from_medical_review_event(&AuditHistoryEventRecord {
                    audit_id,
                    run_id: String::new(),
                    actor_role,
                    event_type: "medical.review.recorded".into(),
                    event_status: "succeeded".into(),
                    summary: String::new(),
                    payload,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: None,
                })
            },
        ))
        .chain(label_bootstrap_rows.into_iter().filter_map(
            |(audit_id, run_id, actor_role, payload, evidence_refs)| {
                label_from_bootstrap_review_event(&AuditHistoryEventRecord {
                    audit_id,
                    run_id,
                    actor_role,
                    event_type: "label.bootstrap.reviewed".into(),
                    event_status: "succeeded".into(),
                    summary: String::new(),
                    payload,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: None,
                })
            },
        ))
        .collect::<Vec<_>>();
    labels.extend(labels_from_lead_triage_events(
        lead_triage_rows.into_iter().map(
            |(audit_id, run_id, actor_role, payload, evidence_refs)| AuditHistoryEventRecord {
                audit_id,
                run_id,
                actor_role,
                event_type: "lead.triaged".into(),
                event_status: "succeeded".into(),
                summary: String::new(),
                payload,
                evidence_refs: json_array_to_strings(evidence_refs),
                created_at: None,
            },
        ),
    ));
    labels.extend(
        repository
            .list_cases(None)
            .await?
            .into_iter()
            .flat_map(labels_from_case_status),
    );
    sort_outcome_labels(&mut labels);
    Ok(labels)
}
