use super::*;

pub async fn label_bootstrap_queue(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<LabelBootstrapQueueResponse>, ApiError> {
    Ok(Json(LabelBootstrapQueueResponse {
        items: load_label_bootstrap_items(&state, &actor.customer_scope_id).await?,
    }))
}

pub async fn review_label_bootstrap_item(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(item_id): Path<String>,
    Json(request): Json<ReviewLabelBootstrapItemRequest>,
) -> Result<Json<LabelBootstrapReviewResponse>, ApiError> {
    validate_label_review(&request)?;
    let mut item = load_label_bootstrap_items(&state, &actor.customer_scope_id)
        .await?
        .into_iter()
        .find(|item| item.item_id == item_id)
        .ok_or_else(not_found(
            "LABEL_BOOTSTRAP_ITEM_NOT_FOUND",
            "label bootstrap item not found",
        ))?;
    validate_label_training_approval(&item, &request)?;
    let audit_id = AuditEventId::new().to_string();
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: audit_id.clone(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: item.claim_id.clone(),
            source_system: "ops-studio".into(),
            actor_id: request.reviewer.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "label.bootstrap.reviewed".into(),
            event_status: "succeeded".into(),
            summary: format!("Label bootstrap item reviewed: {item_id}"),
            payload: json!({
                "customer_scope_id": actor.customer_scope_id,
                "item_id": item_id,
                "claim_id": item.claim_id,
                "source_type": item.source_type,
                "source_id": item.source_id,
                "reviewer": request.reviewer,
                "label_name": request.label_name,
                "label_value": request.label_value,
                "governance_status": request.governance_status,
                "feedback_target": request.feedback_target,
                "notes": request.notes,
            }),
            evidence_refs: request
                .evidence_refs
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        })
        .await
        .map_err(internal_error("LABEL_BOOTSTRAP_REVIEW_SAVE_FAILED"))?;
    item.review_status = "reviewed".into();
    item.review_audit_id = Some(audit_id.clone());
    item.reviewer = Some(request.reviewer);
    item.suggested_label_name = request.label_name;
    item.suggested_label_value = request.label_value;
    item.governance_status = request.governance_status;
    item.feedback_target = request.feedback_target;
    item.training_eligible = item.governance_status == "approved_for_training";
    item.evidence_refs = request.evidence_refs;
    Ok(Json(LabelBootstrapReviewResponse { item, audit_id }))
}

async fn load_label_bootstrap_items(
    state: &AppState,
    customer_scope_id: &str,
) -> Result<Vec<LabelBootstrapItemRecord>, ApiError> {
    let requests = evidence::load_evidence_requests(state, customer_scope_id).await?;
    let reviews = load_label_bootstrap_reviews(state, customer_scope_id).await?;
    let mut items = requests
        .into_iter()
        .map(label_item_from_evidence_request)
        .collect::<Vec<_>>();
    for item in &mut items {
        if let Some(review) = reviews.get(&item.item_id) {
            apply_label_review(item, review);
        }
    }
    items.sort_by(|left, right| left.item_id.cmp(&right.item_id));
    Ok(items)
}

async fn load_label_bootstrap_reviews(
    state: &AppState,
    customer_scope_id: &str,
) -> Result<BTreeMap<String, AuditHistoryEventRecord>, ApiError> {
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 10_000,
            event_type: Some("label.bootstrap.reviewed".into()),
            customer_scope_id: Some(customer_scope_id.into()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("LABEL_BOOTSTRAP_REVIEW_LIST_FAILED"))?;
    let mut reviews = BTreeMap::new();
    for event in events.into_iter().rev() {
        if let Some(item_id) = event.payload["item_id"].as_str() {
            reviews.insert(item_id.to_string(), event);
        }
    }
    Ok(reviews)
}

fn validate_label_review(request: &ReviewLabelBootstrapItemRequest) -> Result<(), ApiError> {
    if request.reviewer.trim().is_empty()
        || request.label_name.trim().is_empty()
        || request.label_value.trim().is_empty()
        || request.feedback_target.trim().is_empty()
        || request.notes.trim().is_empty()
        || request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_LABEL_BOOTSTRAP_REVIEW",
            "reviewer, label fields, feedback_target, notes, and evidence_refs are required",
        ));
    }
    if !matches!(
        request.governance_status.as_str(),
        "needs_review" | "approved_for_training" | "rejected_for_training"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "UNSUPPORTED_LABEL_GOVERNANCE_STATUS",
            "governance_status must be needs_review, approved_for_training, or rejected_for_training",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.notes.as_str())
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_LABEL_BOOTSTRAP_REVIEW",
            "label bootstrap review notes and evidence_refs must not contain PII",
        ));
    }
    validate_label_review_production_evidence_refs(&request.evidence_refs)
}

fn validate_label_review_production_evidence_refs(
    evidence_refs: &[String],
) -> Result<(), ApiError> {
    if evidence_refs.iter().any(|reference| {
        let reference = reference.trim();
        reference.contains("local://")
            || reference.contains("file://")
            || reference.contains('{')
            || reference.contains('}')
    }) {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_LABEL_BOOTSTRAP_REVIEW_EVIDENCE",
            "label bootstrap review evidence_refs must not use local dry-run or placeholder evidence",
        ))
    } else {
        Ok(())
    }
}

fn validate_label_training_approval(
    item: &LabelBootstrapItemRecord,
    request: &ReviewLabelBootstrapItemRequest,
) -> Result<(), ApiError> {
    if request.governance_status != "approved_for_training" {
        return Ok(());
    }
    if item.suggested_label_name == "insufficient_evidence" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "LABEL_BOOTSTRAP_EVIDENCE_NOT_RECEIVED",
            "evidence must be received before a bootstrap label can be approved for training",
        ));
    }
    if !evidence::has_document_evidence_ref(&request.evidence_refs) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "LABEL_BOOTSTRAP_DOCUMENT_EVIDENCE_REQUIRED",
            "approved training labels require at least one evidence_documents reference",
        ));
    }
    Ok(())
}

fn label_item_from_evidence_request(request: EvidenceRequestRecord) -> LabelBootstrapItemRecord {
    let received = request.status == "received";
    LabelBootstrapItemRecord {
        item_id: format!("label_bootstrap_{}", request.request_id),
        claim_id: request.claim_id,
        source_type: "evidence_request".into(),
        source_id: request.request_id,
        suggested_label_name: if received {
            "clinical_evidence_sufficient".into()
        } else {
            "insufficient_evidence".into()
        },
        suggested_label_value: "true".into(),
        governance_status: "needs_review".into(),
        training_eligible: false,
        review_status: "open".into(),
        review_audit_id: None,
        reviewer: None,
        feedback_target: "workflow".into(),
        evidence_refs: request.evidence_refs,
        created_at: request.created_at,
    }
}

fn apply_label_review(item: &mut LabelBootstrapItemRecord, event: &AuditHistoryEventRecord) {
    item.review_status = "reviewed".into();
    item.review_audit_id = Some(event.audit_id.clone());
    item.reviewer = event.payload["reviewer"].as_str().map(str::to_string);
    item.suggested_label_name = event.payload["label_name"]
        .as_str()
        .unwrap_or(&item.suggested_label_name)
        .to_string();
    item.suggested_label_value = event.payload["label_value"]
        .as_str()
        .unwrap_or(&item.suggested_label_value)
        .to_string();
    item.governance_status = event.payload["governance_status"]
        .as_str()
        .unwrap_or(&item.governance_status)
        .to_string();
    item.feedback_target = event.payload["feedback_target"]
        .as_str()
        .unwrap_or(&item.feedback_target)
        .to_string();
    item.training_eligible = item.governance_status == "approved_for_training";
    if !event.evidence_refs.is_empty() {
        item.evidence_refs = event.evidence_refs.clone();
    }
}
