use super::*;

impl InMemoryScoringRepository {
    pub(super) async fn in_memory_claim_audit_history(
        &self,
        claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        let mut events = self
            .audit_events
            .lock()
            .await
            .iter()
            .filter(|event| event.claim_id == claim_id)
            .filter(|event| {
                customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&event.payload, scope)
                })
            })
            .map(|event| AuditHistoryEventRecord {
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
            .collect::<Vec<_>>();

        events.extend(
            self.pilot_audit_events
                .lock()
                .await
                .iter()
                .filter(|(event_claim_id, event)| {
                    event_claim_id == claim_id
                        && customer_scope_id.is_none_or(|scope| {
                            audit_event_payload_matches_customer_scope(&event.payload, scope)
                        })
                })
                .map(|(_, event)| event.clone()),
        );
        Ok(events)
    }

    pub(super) async fn in_memory_list_audit_events(
        &self,
        filter: AuditEventListFilter,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        let mut events = self
            .audit_events
            .lock()
            .await
            .iter()
            .filter(|event| persisted_audit_event_matches_filter(event, &filter))
            .map(|event| AuditHistoryEventRecord {
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
            .collect::<Vec<_>>();
        events.extend(
            self.pilot_audit_events
                .lock()
                .await
                .iter()
                .filter(|(claim_id, event)| {
                    pilot_audit_event_matches_filter(claim_id, event, &filter)
                })
                .map(|(_, event)| event.clone()),
        );
        events.reverse();
        events.truncate(filter.limit as usize);
        Ok(events)
    }

    pub(super) async fn in_memory_list_webhook_events(
        &self,
    ) -> anyhow::Result<Vec<WebhookEventRecord>> {
        let mut events = self
            .audit_events
            .lock()
            .await
            .iter()
            .filter_map(|event| {
                let audit_event = AuditHistoryEventRecord {
                    audit_id: event.audit_id.clone(),
                    run_id: event.run_id.clone(),
                    actor_role: event.actor_role.clone(),
                    event_type: event.event_type.clone(),
                    event_status: event.event_status.clone(),
                    summary: event.summary.clone(),
                    payload: event.payload.clone(),
                    evidence_refs: evidence_values_to_strings(&event.evidence_refs),
                    created_at: None,
                };
                webhook_event_from_audit(Some(event.claim_id.as_str()), &audit_event)
            })
            .collect::<Vec<_>>();

        events.extend(
            self.pilot_audit_events
                .lock()
                .await
                .iter()
                .filter_map(|(claim_id, event)| webhook_event_from_audit(Some(claim_id), event)),
        );
        let attempts = self
            .webhook_delivery_attempts
            .lock()
            .await
            .values()
            .flat_map(|records| records.iter().cloned())
            .collect::<Vec<_>>();
        apply_webhook_delivery_state(&mut events, &attempts);
        sort_webhook_events(&mut events);
        Ok(events)
    }

    pub(super) async fn in_memory_save_webhook_delivery_attempt(
        &self,
        input: WebhookDeliveryAttemptInput,
    ) -> anyhow::Result<WebhookDeliveryAttemptRecord> {
        let attempted_at = chrono::Utc::now();
        let mut attempts = self.webhook_delivery_attempts.lock().await;
        let event_attempts = attempts.entry(input.event_id.clone()).or_default();
        let attempt_number = event_attempts.len() as u32 + 1;
        let record = WebhookDeliveryAttemptRecord {
            event_id: input.event_id,
            attempt_number,
            delivery_status: input.delivery_status.clone(),
            response_status_code: input.response_status_code,
            error_message: input.error_message,
            next_attempt_at: next_webhook_attempt_at(
                &input.delivery_status,
                attempt_number,
                attempted_at,
            )
            .map(|timestamp| timestamp.to_rfc3339()),
            attempted_at: Some(attempted_at.to_rfc3339()),
        };
        event_attempts.push(record.clone());
        Ok(record)
    }
}
