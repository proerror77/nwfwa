use super::*;

impl InMemoryScoringRepository {
    pub(super) async fn in_memory_save_investigation_result(
        &self,
        record: InvestigationResultRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        let saving_attributions = derive_saving_attributions(&record);
        let audit_id = format!("audit_investigation_{}", record.investigation_id);
        let previous_case_id = {
            let events = self.pilot_audit_events.lock().await;
            events
                .iter()
                .find(|(_, event)| event.audit_id == audit_id)
                .and_then(|(_, event)| event.payload["case_id"].as_str())
                .map(str::to_string)
        };
        let mut cases = self.cases.lock().await;
        if previous_case_id.as_deref() != record.case_id.as_deref() {
            if let Some(case_id) = previous_case_id.as_deref() {
                if let Some(case) = cases.get_mut(case_id) {
                    if case.investigation_result_id.as_deref()
                        == Some(record.investigation_id.as_str())
                    {
                        case.final_outcome = None;
                        case.reviewer_notes = None;
                        case.investigation_result_id = None;
                    }
                }
            }
        }
        if let Some(case_id) = record.case_id.as_deref() {
            if let Some(case) = cases.get_mut(case_id) {
                case.final_outcome = Some(record.outcome.clone());
                case.reviewer_notes = Some(record.notes.clone());
                case.investigation_result_id = Some(record.investigation_id.clone());
            } else {
                anyhow::bail!("case not found for investigation result: {case_id}");
            }
        }
        drop(cases);
        let event = AuditHistoryEventRecord {
            audit_id,
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
        upsert_pilot_audit_event(
            &self.pilot_audit_events,
            record.claim_id.clone(),
            event.clone(),
        )
        .await;
        let mut stored_attributions = self.saving_attributions.lock().await;
        stored_attributions
            .retain(|attribution| attribution.investigation_id != record.investigation_id);
        stored_attributions.extend(saving_attributions);
        Ok(event)
    }

    pub(super) async fn in_memory_save_qa_review(
        &self,
        mut record: QaReviewRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        record.feedback_target = canonical_feedback_target(&record.feedback_target).into();
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
        upsert_pilot_audit_event(&self.pilot_audit_events, record.claim_id, event.clone()).await;
        Ok(event)
    }

    pub(super) async fn in_memory_list_qa_feedback_items(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaFeedbackItemRecord>> {
        let events = self.pilot_audit_events.lock().await.clone();
        let scoped_events = events
            .into_iter()
            .filter(|(_, event)| {
                customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&event.payload, scope)
                })
            })
            .collect::<Vec<_>>();
        let feedback_statuses = latest_qa_feedback_statuses(&scoped_events);
        let mut items = scoped_events
            .iter()
            .filter_map(|(_, event)| {
                (event.event_type == "qa.result.received")
                    .then(|| serde_json::from_value::<QaReviewRecord>(event.payload.clone()).ok())
                    .flatten()
            })
            .filter(|review| review.qa_conclusion != "pass")
            .map(|review| {
                let feedback_id = qa_feedback_id(&review.qa_case_id);
                let status_update = feedback_statuses.get(&feedback_id);
                let status = status_update
                    .map(|update| update.status.as_str())
                    .unwrap_or("open");
                qa_review_to_feedback_item(review, None, status, status_update)
            })
            .collect::<Vec<_>>();
        sort_qa_feedback_items(&mut items);
        Ok(items)
    }

    pub(super) async fn in_memory_update_qa_feedback_status(
        &self,
        feedback_id: &str,
        input: UpdateQaFeedbackStatusInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<UpdateQaFeedbackStatusRecord>> {
        let Some(mut item) = self
            .in_memory_list_qa_feedback_items(customer_scope_id)
            .await?
            .into_iter()
            .find(|item| item.feedback_id == feedback_id)
        else {
            return Ok(None);
        };
        let from_status = item.status.clone();
        item.status = input.status.clone();
        let audit_id = AuditEventId::new().to_string();
        item.status_updated_by = Some(input.actor_id.clone());
        item.status_audit_id = Some(audit_id.clone());
        item.status_updated_at = None;
        item.status_evidence_refs = input.evidence_refs.clone();
        let event = AuditHistoryEventRecord {
            audit_id: audit_id.clone(),
            run_id: format!("qa_feedback_status_{}", item.feedback_id),
            actor_role: "fwa_operator".into(),
            event_type: "qa.feedback.status.updated".into(),
            event_status: "succeeded".into(),
            summary: format!(
                "QA feedback status updated: {} -> {}",
                from_status, item.status
            ),
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
        };
        self.pilot_audit_events
            .lock()
            .await
            .push((item.claim_id.clone(), event));
        Ok(Some(UpdateQaFeedbackStatusRecord { item, audit_id }))
    }

    pub(super) async fn in_memory_list_qa_reviews(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaReviewRecord>> {
        let mut reviews = self
            .pilot_audit_events
            .lock()
            .await
            .iter()
            .filter(|(_, event)| {
                customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&event.payload, scope)
                })
            })
            .filter_map(|(_, event)| {
                (event.event_type == "qa.result.received")
                    .then(|| serde_json::from_value::<QaReviewRecord>(event.payload.clone()).ok())
                    .flatten()
            })
            .map(|mut review| {
                review.feedback_target = canonical_feedback_target(&review.feedback_target).into();
                review
            })
            .collect::<Vec<_>>();
        reviews.sort_by(|left, right| left.qa_case_id.cmp(&right.qa_case_id));
        Ok(reviews)
    }

    pub(super) async fn in_memory_list_outcome_labels(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<OutcomeLabelRecord>> {
        let events = self.pilot_audit_events.lock().await.clone();
        let scoped_events = events
            .into_iter()
            .filter(|(_, event)| {
                customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&event.payload, scope)
                })
            })
            .collect::<Vec<_>>();
        let feedback_statuses = latest_qa_feedback_statuses(&scoped_events);
        let mut labels = scoped_events
            .iter()
            .filter_map(|(_, event)| match event.event_type.as_str() {
                "investigation.result.received" => {
                    serde_json::from_value::<InvestigationResultRecord>(event.payload.clone())
                        .ok()
                        .map(labels_from_investigation_result)
                }
                "qa.result.received" => {
                    serde_json::from_value::<QaReviewRecord>(event.payload.clone())
                        .ok()
                        .map(|review| {
                            let feedback_id = qa_feedback_id(&review.qa_case_id);
                            let feedback_status = feedback_statuses
                                .get(&feedback_id)
                                .map(|update| update.status.as_str())
                                .unwrap_or("open");
                            vec![label_from_qa_review(review, feedback_status)]
                        })
                }
                "medical.review.recorded" => Some(labels_from_medical_review_event(event)),
                _ => None,
            })
            .flatten()
            .collect::<Vec<_>>();
        labels.extend(
            self.audit_events
                .lock()
                .await
                .iter()
                .filter(|event| {
                    event.event_type == "medical.review.recorded"
                        && customer_scope_id.is_none_or(|scope| {
                            audit_event_payload_matches_customer_scope(&event.payload, scope)
                        })
                })
                .flat_map(|event| {
                    labels_from_medical_review_event(&AuditHistoryEventRecord {
                        audit_id: event.audit_id.clone(),
                        run_id: event.run_id.clone(),
                        actor_role: event.actor_role.clone(),
                        event_type: event.event_type.clone(),
                        event_status: event.event_status.clone(),
                        summary: event.summary.clone(),
                        payload: event.payload.clone(),
                        evidence_refs: evidence_values_to_strings(&event.evidence_refs),
                        created_at: None,
                    })
                }),
        );
        labels.extend(
            self.audit_events
                .lock()
                .await
                .iter()
                .filter(|event| {
                    event.event_type == "label.bootstrap.reviewed"
                        && event.event_status == "succeeded"
                        && customer_scope_id.is_none_or(|scope| {
                            audit_event_payload_matches_customer_scope(&event.payload, scope)
                        })
                })
                .filter_map(|event| {
                    label_from_bootstrap_review_event(&audit_history_from_persisted(event))
                }),
        );
        let lead_triage_events = self
            .audit_events
            .lock()
            .await
            .iter()
            .filter(|event| {
                event.event_type == "lead.triaged"
                    && event.event_status == "succeeded"
                    && customer_scope_id.is_none_or(|scope| {
                        audit_event_payload_matches_customer_scope(&event.payload, scope)
                    })
            })
            .map(audit_history_from_persisted)
            .collect::<Vec<_>>();
        labels.extend(labels_from_lead_triage_events(lead_triage_events));
        labels.extend(
            self.in_memory_list_cases(customer_scope_id)
                .await?
                .into_iter()
                .flat_map(labels_from_case_status),
        );
        sort_outcome_labels(&mut labels);
        Ok(labels)
    }
}
