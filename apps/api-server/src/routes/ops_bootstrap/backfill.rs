use super::*;

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
