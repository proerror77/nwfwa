use super::*;

pub(super) async fn load_evidence_requests(
    state: &AppState,
    customer_scope_id: &str,
) -> Result<Vec<EvidenceRequestRecord>, ApiError> {
    let generated_events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 10_000,
            event_type: Some("evidence.request.generated".into()),
            customer_scope_id: Some(customer_scope_id.into()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("EVIDENCE_REQUEST_LIST_FAILED"))?;
    let status_events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 10_000,
            event_type: Some("evidence.request.status_updated".into()),
            customer_scope_id: Some(customer_scope_id.into()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("EVIDENCE_REQUEST_LIST_FAILED"))?;
    let mut requests = generated_events
        .iter()
        .filter_map(evidence_request_from_event)
        .map(|request| (request.request_id.clone(), request))
        .collect::<BTreeMap<_, _>>();
    for event in status_events.iter().rev() {
        let Some(request_id) = event.payload["request_id"].as_str() else {
            continue;
        };
        let Some(request) = requests.get_mut(request_id) else {
            continue;
        };
        if let Some(status) = event.payload["to_status"].as_str() {
            request.status = status.to_string();
        }
        request.updated_at = event.created_at.clone();
        for reference in &event.evidence_refs {
            if !request.evidence_refs.contains(reference) {
                request.evidence_refs.push(reference.clone());
            }
        }
    }
    let mut requests = requests.into_values().collect::<Vec<_>>();
    requests.sort_by(|left, right| left.request_id.cmp(&right.request_id));
    Ok(requests)
}

pub(super) fn validate_generate_evidence_request(
    request: &GenerateEvidenceRequestsRequest,
) -> Result<(), ApiError> {
    validate_optional_notes(
        request.notes.as_deref(),
        "EVIDENCE_REQUEST_NOTES_CONTAIN_PII",
    )
}

pub(super) fn validate_evidence_status_update(
    request: &UpdateEvidenceRequestStatusRequest,
) -> Result<(), ApiError> {
    if !matches!(
        request.status.as_str(),
        "open" | "requested" | "received" | "cancelled"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "UNSUPPORTED_EVIDENCE_REQUEST_STATUS",
            "status must be one of open, requested, received, or cancelled",
        ));
    }
    if request.actor_id.trim().is_empty() || request.notes.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_EVIDENCE_REQUEST_STATUS_UPDATE",
            "actor_id and notes are required",
        ));
    }
    if request.status == "received" && !has_document_evidence_ref(&request.evidence_refs) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "EVIDENCE_REQUEST_DOCUMENT_EVIDENCE_REQUIRED",
            "received evidence requests require at least one evidence_documents reference",
        ));
    }
    validate_optional_notes(Some(&request.notes), "EVIDENCE_REQUEST_NOTES_CONTAIN_PII")
}

pub(super) fn has_document_evidence_ref(evidence_refs: &[String]) -> bool {
    evidence_refs
        .iter()
        .any(|reference| reference.starts_with("evidence_documents:"))
}

pub(super) fn evidence_request_from_scoring_event(
    event: &AuditHistoryEventRecord,
    request: &GenerateEvidenceRequestsRequest,
    actor: &ActorContext,
) -> Option<EvidenceRequestRecord> {
    let clinical = &event.payload["clinical_evidence"];
    let missing_evidence = json_array_to_strings(&clinical["missing_evidence"]);
    if missing_evidence.is_empty() {
        return None;
    }
    let claim_id = event.payload["claim_id"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    if claim_id.is_empty() {
        return None;
    }
    let request_id = format!("evidence_request_{}", event.audit_id);
    let items = missing_evidence
        .iter()
        .enumerate()
        .map(|(index, document_type)| EvidenceRequestItemRecord {
            item_id: format!("{request_id}_item_{}", index + 1),
            document_type: document_type.clone(),
            status: "open".into(),
            reason: evidence_request_reason(document_type).into(),
            blocking: true,
            policy_authority_ref: Some("policy:clinical-evidence:v1".into()),
            exception_check: Some("required_clinical_documents_not_present".into()),
        })
        .collect::<Vec<_>>();
    Some(EvidenceRequestRecord {
        request_id,
        claim_id,
        scoring_audit_id: event.audit_id.clone(),
        status: "open".into(),
        request_reason: "missing_clinical_evidence".into(),
        missing_evidence,
        items,
        reviewer_queue: request
            .reviewer_queue
            .clone()
            .unwrap_or_else(|| "clinical-evidence".into()),
        requested_by: request
            .requested_by
            .clone()
            .unwrap_or_else(|| actor.actor_id.clone()),
        notes: request.notes.clone(),
        evidence_refs: event.evidence_refs.clone(),
        created_at: event.created_at.clone(),
        updated_at: None,
    })
}

pub(super) fn evidence_request_payload(
    customer_scope_id: &str,
    request: &EvidenceRequestRecord,
) -> Value {
    json!({
        "customer_scope_id": customer_scope_id,
        "request_id": request.request_id,
        "claim_id": request.claim_id,
        "scoring_audit_id": request.scoring_audit_id,
        "status": request.status,
        "request_reason": request.request_reason,
        "missing_evidence": request.missing_evidence,
        "items": request.items,
        "reviewer_queue": request.reviewer_queue,
        "requested_by": request.requested_by,
        "notes": request.notes,
    })
}

fn evidence_request_from_event(event: &AuditHistoryEventRecord) -> Option<EvidenceRequestRecord> {
    Some(EvidenceRequestRecord {
        request_id: event.payload["request_id"].as_str()?.to_string(),
        claim_id: event.payload["claim_id"].as_str()?.to_string(),
        scoring_audit_id: event.payload["scoring_audit_id"].as_str()?.to_string(),
        status: event.payload["status"]
            .as_str()
            .unwrap_or("open")
            .to_string(),
        request_reason: event.payload["request_reason"]
            .as_str()
            .unwrap_or("missing_clinical_evidence")
            .to_string(),
        missing_evidence: json_array_to_strings(&event.payload["missing_evidence"]),
        items: event
            .payload
            .get("items")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(evidence_request_item_from_value)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        reviewer_queue: event.payload["reviewer_queue"]
            .as_str()
            .unwrap_or("clinical-evidence")
            .to_string(),
        requested_by: event.payload["requested_by"]
            .as_str()
            .unwrap_or("ops")
            .to_string(),
        notes: event.payload["notes"].as_str().map(str::to_string),
        evidence_refs: event.evidence_refs.clone(),
        created_at: event.created_at.clone(),
        updated_at: None,
    })
}

fn evidence_request_item_from_value(value: &Value) -> Option<EvidenceRequestItemRecord> {
    Some(EvidenceRequestItemRecord {
        item_id: value["item_id"].as_str()?.to_string(),
        document_type: value["document_type"].as_str()?.to_string(),
        status: value["status"].as_str().unwrap_or("open").to_string(),
        reason: value["reason"].as_str().unwrap_or_default().to_string(),
        blocking: value["blocking"].as_bool().unwrap_or(true),
        policy_authority_ref: value["policy_authority_ref"].as_str().map(str::to_string),
        exception_check: value["exception_check"].as_str().map(str::to_string),
    })
}

fn evidence_request_reason(document_type: &str) -> &'static str {
    match document_type {
        "radiology_report" => "radiology evidence is required before clinical necessity review",
        "dental_xray" => "dental X-ray evidence is required before clinical necessity review",
        "medication_order" | "prescription" | "prescription_detail" => {
            "medication detail is required before pharmacy review"
        }
        "operation_record" => "operation record is required before surgical necessity review",
        "clinical_order" | "medical_record" => {
            "source clinical documentation is required before adjudication"
        }
        _ => "supporting evidence is required before label approval",
    }
}
