use super::*;

pub async fn create_historical_backfill(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Json(request): Json<CreateHistoricalBackfillRequest>,
) -> Result<Json<HistoricalBackfillResponse>, ApiError> {
    validate_backfill_request(&request)?;
    let leads = state
        .repository
        .list_leads(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("BACKFILL_LEAD_LIST_FAILED"))?;
    let limit = request.limit.unwrap_or(25).clamp(1, 200) as usize;
    let selected = leads
        .into_iter()
        .take(limit)
        .map(backfill_lead_from_lead)
        .collect::<Vec<_>>();
    let job_id = request
        .job_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("backfill_{}", AuditEventId::new()));
    let audit_id = AuditEventId::new().to_string();
    let evidence_refs = backfill_evidence_refs(&request, &selected);
    let job = HistoricalBackfillJobRecord {
        job_id: job_id.clone(),
        status: "completed".into(),
        dataset_refs: request.dataset_refs.clone(),
        rule_refs: request.rule_refs.clone(),
        candidate_count: selected.len() as u32,
        leads: selected,
        reviewer: request.reviewer.clone(),
        notes: request.notes.clone(),
        evidence_refs: evidence_refs.clone(),
        created_at: None,
    };
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id,
            run_id: ScoringRunId::new().to_string(),
            claim_id: first_claim_id(&job.leads),
            source_system: "ops-studio".into(),
            actor_id: request
                .reviewer
                .clone()
                .unwrap_or_else(|| actor.actor_id.clone()),
            actor_role: actor.actor_role.clone(),
            event_type: "historical_backfill.created".into(),
            event_status: "succeeded".into(),
            summary: format!("Historical backfill completed: {job_id}"),
            payload: json!({
                "customer_scope_id": actor.customer_scope_id,
                "job_id": job.job_id,
                "status": job.status,
                "dataset_refs": job.dataset_refs,
                "rule_refs": job.rule_refs,
                "candidate_count": job.candidate_count,
                "leads": job.leads,
                "reviewer": job.reviewer,
                "notes": job.notes,
            }),
            evidence_refs: evidence_refs
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        })
        .await
        .map_err(internal_error("BACKFILL_SAVE_FAILED"))?;
    Ok(Json(HistoricalBackfillResponse { job }))
}

pub async fn list_historical_backfills(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<HistoricalBackfillListResponse>, ApiError> {
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 200,
            event_type: Some("historical_backfill.created".into()),
            customer_scope_id: Some(actor.customer_scope_id),
            ..Default::default()
        })
        .await
        .map_err(internal_error("BACKFILL_LIST_FAILED"))?;
    Ok(Json(HistoricalBackfillListResponse {
        jobs: events.iter().filter_map(backfill_job_from_event).collect(),
    }))
}

pub async fn list_historical_backfill_leads(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(job_id): Path<String>,
) -> Result<Json<HistoricalBackfillLeadResponse>, ApiError> {
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 500,
            event_type: Some("historical_backfill.created".into()),
            customer_scope_id: Some(actor.customer_scope_id),
            ..Default::default()
        })
        .await
        .map_err(internal_error("BACKFILL_LEADS_FAILED"))?;
    let job = events
        .iter()
        .filter_map(backfill_job_from_event)
        .find(|job| job.job_id == job_id)
        .ok_or_else(not_found(
            "BACKFILL_JOB_NOT_FOUND",
            "backfill job not found",
        ))?;
    Ok(Json(HistoricalBackfillLeadResponse {
        job_id,
        leads: job.leads,
    }))
}

pub(super) fn validate_backfill_request(
    request: &CreateHistoricalBackfillRequest,
) -> Result<(), ApiError> {
    validate_optional_notes(request.notes.as_deref(), "BACKFILL_NOTES_CONTAIN_PII")?;
    if request
        .dataset_refs
        .iter()
        .chain(request.rule_refs.iter())
        .any(|value| value.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_BACKFILL_REFS",
            "dataset_refs and rule_refs cannot contain blank values",
        ));
    }
    Ok(())
}

pub(super) fn backfill_lead_from_lead(lead: LeadRecord) -> HistoricalBackfillLeadRecord {
    HistoricalBackfillLeadRecord {
        lead_id: lead.lead_id,
        claim_id: lead.claim_id,
        scheme_family: lead.scheme_family,
        risk_score: lead.risk_score,
        rag: lead.rag,
        status: lead.status,
        reason: lead.reason,
        evidence_refs: lead.evidence_refs,
    }
}

pub(super) fn first_claim_id(leads: &[HistoricalBackfillLeadRecord]) -> String {
    leads
        .first()
        .map(|lead| lead.claim_id.clone())
        .unwrap_or_default()
}

pub(super) fn backfill_evidence_refs(
    request: &CreateHistoricalBackfillRequest,
    leads: &[HistoricalBackfillLeadRecord],
) -> Vec<String> {
    let mut refs = request
        .dataset_refs
        .iter()
        .map(|value| format!("datasets:{value}"))
        .chain(
            request
                .rule_refs
                .iter()
                .map(|value| format!("rules:{value}")),
        )
        .collect::<Vec<_>>();
    refs.extend(
        leads
            .iter()
            .map(|lead| format!("fwa_leads:{}", lead.lead_id)),
    );
    refs
}

pub(super) fn backfill_job_from_event(
    event: &AuditHistoryEventRecord,
) -> Option<HistoricalBackfillJobRecord> {
    Some(HistoricalBackfillJobRecord {
        job_id: event.payload["job_id"].as_str()?.to_string(),
        status: event.payload["status"]
            .as_str()
            .unwrap_or("completed")
            .to_string(),
        dataset_refs: json_array_to_strings(&event.payload["dataset_refs"]),
        rule_refs: json_array_to_strings(&event.payload["rule_refs"]),
        candidate_count: event.payload["candidate_count"].as_u64().unwrap_or(0) as u32,
        leads: event
            .payload
            .get("leads")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(backfill_lead_from_value)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        reviewer: event.payload["reviewer"].as_str().map(str::to_string),
        notes: event.payload["notes"].as_str().map(str::to_string),
        evidence_refs: event.evidence_refs.clone(),
        created_at: event.created_at.clone(),
    })
}

fn backfill_lead_from_value(value: &Value) -> Option<HistoricalBackfillLeadRecord> {
    Some(HistoricalBackfillLeadRecord {
        lead_id: value["lead_id"].as_str()?.to_string(),
        claim_id: value["claim_id"].as_str()?.to_string(),
        scheme_family: value["scheme_family"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        risk_score: value["risk_score"].as_u64().unwrap_or(0).min(100) as u8,
        rag: value["rag"].as_str().unwrap_or_default().to_string(),
        status: value["status"].as_str().unwrap_or_default().to_string(),
        reason: value["reason"].as_str().unwrap_or_default().to_string(),
        evidence_refs: json_array_to_strings(&value["evidence_refs"]),
    })
}
