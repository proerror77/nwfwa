use super::*;

pub async fn generate_evidence_requests(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Json(request): Json<GenerateEvidenceRequestsRequest>,
) -> Result<Json<EvidenceRequestGenerateResponse>, ApiError> {
    let actor = require_permission(principal, "ops:bootstrap:write")?;
    validate_generate_evidence_request(&request)?;
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: request.limit.unwrap_or(50).clamp(1, 200),
            event_type: Some("scoring.completed".into()),
            claim_id: request.claim_id.clone(),
            customer_scope_id: Some(actor.customer_scope_id.clone()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("EVIDENCE_REQUEST_SOURCE_LOOKUP_FAILED"))?;
    let mut generated = Vec::new();
    for event in events
        .iter()
        .filter(|event| {
            request
                .scoring_audit_id
                .as_deref()
                .is_none_or(|id| event.audit_id == id)
        })
        .filter_map(|event| evidence_request_from_scoring_event(event, &request, &actor))
    {
        let audit_id = AuditEventId::new().to_string();
        state
            .repository
            .save_audit_event(PersistedAuditEvent {
                audit_id,
                run_id: ScoringRunId::new().to_string(),
                claim_id: event.claim_id.clone(),
                source_system: "ops-studio".into(),
                actor_id: event.requested_by.clone(),
                actor_role: actor.actor_role.clone(),
                event_type: "evidence.request.generated".into(),
                event_status: "succeeded".into(),
                summary: format!("Evidence request generated: {}", event.request_id),
                payload: evidence_request_payload(&actor.customer_scope_id, &event),
                evidence_refs: event
                    .evidence_refs
                    .iter()
                    .map(|value| Value::String(value.clone()))
                    .collect(),
            })
            .await
            .map_err(internal_error("EVIDENCE_REQUEST_SAVE_FAILED"))?;
        generated.push(event);
    }
    Ok(Json(EvidenceRequestGenerateResponse {
        requests: generated,
    }))
}

pub async fn list_evidence_requests(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<EvidenceRequestListResponse>, ApiError> {
    Ok(Json(EvidenceRequestListResponse {
        requests: load_evidence_requests(&state, &actor.customer_scope_id).await?,
    }))
}

pub async fn update_evidence_request_status(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(request_id): Path<String>,
    Json(request): Json<UpdateEvidenceRequestStatusRequest>,
) -> Result<Json<EvidenceRequestRecord>, ApiError> {
    let actor = require_permission(principal, "ops:bootstrap:write")?;
    validate_evidence_status_update(&request)?;
    let current = load_evidence_requests(&state, &actor.customer_scope_id)
        .await?
        .into_iter()
        .find(|record| record.request_id == request_id)
        .ok_or_else(not_found(
            "EVIDENCE_REQUEST_NOT_FOUND",
            "evidence request not found",
        ))?;
    let audit_id = AuditEventId::new().to_string();
    let mut evidence_refs = current.evidence_refs.clone();
    for reference in &request.evidence_refs {
        if !evidence_refs.contains(reference) {
            evidence_refs.push(reference.clone());
        }
    }
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id,
            run_id: ScoringRunId::new().to_string(),
            claim_id: current.claim_id.clone(),
            source_system: "ops-studio".into(),
            actor_id: request.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "evidence.request.status_updated".into(),
            event_status: "succeeded".into(),
            summary: format!("Evidence request status updated: {request_id}"),
            payload: json!({
                "customer_scope_id": actor.customer_scope_id,
                "request_id": request_id,
                "claim_id": current.claim_id,
                "scoring_audit_id": current.scoring_audit_id,
                "from_status": current.status,
                "to_status": request.status,
                "actor_id": request.actor_id,
                "notes": request.notes,
            }),
            evidence_refs: evidence_refs
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        })
        .await
        .map_err(internal_error("EVIDENCE_REQUEST_STATUS_SAVE_FAILED"))?;
    load_evidence_requests(&state, &actor.customer_scope_id)
        .await?
        .into_iter()
        .find(|record| record.request_id == request_id)
        .map(Json)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "EVIDENCE_REQUEST_RELOAD_FAILED",
                "evidence request update saved but could not be reloaded",
            )
        })
}

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
    if request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_EVIDENCE_REQUEST_STATUS_EVIDENCE",
            "evidence request status evidence_refs must not be blank",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.notes.as_str())
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_EVIDENCE_REQUEST_STATUS",
            "evidence request status notes and evidence_refs must not contain PII",
        ));
    }
    validate_evidence_request_status_production_evidence_refs(&request.evidence_refs)?;
    if request.status == "received" && !has_document_evidence_ref(&request.evidence_refs) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "EVIDENCE_REQUEST_DOCUMENT_EVIDENCE_REQUIRED",
            "received evidence requests require at least one evidence_documents reference",
        ));
    }
    Ok(())
}

fn validate_evidence_request_status_production_evidence_refs(
    evidence_refs: &[String],
) -> Result<(), ApiError> {
    if evidence_refs.iter().any(|reference| {
        let reference = reference.trim();
        let normalized = reference.to_ascii_lowercase();
        normalized.contains("local://")
            || normalized.contains("file://")
            || normalized.contains("://localhost")
            || normalized.contains("://127.")
            || normalized.contains("://0.0.0.0")
            || normalized.contains("://[::1]")
            || reference.contains('{')
            || reference.contains('}')
    }) {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_EVIDENCE_REQUEST_STATUS_EVIDENCE",
            "evidence request status evidence_refs must not use local dry-run or placeholder evidence",
        ))
    } else {
        Ok(())
    }
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
