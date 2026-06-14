use super::*;

impl InMemoryScoringRepository {
    pub(super) async fn claim_visible_to_scope(
        &self,
        claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> bool {
        let Some(scope) = customer_scope_id else {
            return true;
        };
        scoped_claim_ids_from_audit_events(self.audit_events.lock().await.iter(), scope)
            .contains(claim_id)
    }

    pub(super) async fn in_memory_upsert_claim_context(
        &self,
        context: ClaimContext,
    ) -> anyhow::Result<()> {
        self.claims
            .lock()
            .await
            .insert(context.claim.external_claim_id.clone(), context);
        Ok(())
    }

    pub(super) async fn in_memory_load_claim_context(
        &self,
        external_claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<ClaimContext>> {
        if !self
            .claim_visible_to_scope(external_claim_id, customer_scope_id)
            .await
        {
            return Ok(None);
        }
        Ok(self.claims.lock().await.get(external_claim_id).cloned())
    }

    pub(super) async fn in_memory_member_profile_summary(
        &self,
        member_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<MemberProfileSummaryRecord>> {
        let visible_claim_ids = match customer_scope_id {
            Some(scope) => Some(scoped_claim_ids_from_audit_events(
                self.audit_events.lock().await.iter(),
                scope,
            )),
            None => None,
        };
        let member_claims = self
            .claims
            .lock()
            .await
            .values()
            .filter(|context| context.member.external_member_id == member_id)
            .filter(|context| {
                visible_claim_ids
                    .as_ref()
                    .is_none_or(|claim_ids| claim_ids.contains(&context.claim.external_claim_id))
            })
            .cloned()
            .collect::<Vec<_>>();
        Ok(member_profile_from_contexts(
            member_id,
            &member_claims,
            self.runs.lock().await.as_slice(),
        ))
    }

    pub(super) async fn in_memory_save_scoring_run(
        &self,
        run: PersistedScoringRun,
    ) -> anyhow::Result<()> {
        let context = self.claims.lock().await.get(&run.claim_id).cloned();
        if let Some(lead) = lead_from_scoring_run(&run, context.as_ref()) {
            self.leads.lock().await.insert(lead.lead_id.clone(), lead);
        }
        self.audit_events.lock().await.push(PersistedAuditEvent {
            audit_id: run.audit_id.clone(),
            run_id: run.run_id.clone(),
            claim_id: run.claim_id.clone(),
            source_system: run.source_system.clone(),
            actor_id: run.actor_id.clone(),
            actor_role: "tpa_system".into(),
            event_type: run
                .audit_event
                .get("event_type")
                .and_then(Value::as_str)
                .unwrap_or("scoring.completed")
                .to_string(),
            event_status: run
                .audit_event
                .get("event_status")
                .and_then(Value::as_str)
                .unwrap_or("succeeded")
                .to_string(),
            summary: "FWA scoring completed".into(),
            payload: mask_audit_payload(run.audit_event.clone()),
            evidence_refs: run.evidence_refs.clone(),
        });
        self.runs.lock().await.push(run);
        Ok(())
    }

    pub(super) async fn in_memory_save_inbox_claim_run(
        &self,
        run: PersistedInboxClaimRun,
    ) -> anyhow::Result<()> {
        self.inbox_claim_runs
            .lock()
            .await
            .insert(run.run_id.clone(), run);
        Ok(())
    }

    pub(super) async fn in_memory_get_inbox_claim_run_by_idempotency_key(
        &self,
        idempotency_key: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
        Ok(self
            .inbox_claim_runs
            .lock()
            .await
            .values()
            .find(|run| {
                run.idempotency_key.as_deref() == Some(idempotency_key)
                    && customer_scope_id.is_none_or(|scope| run.customer_scope_id == scope)
            })
            .cloned())
    }

    pub(super) async fn in_memory_get_inbox_claim_run_by_run_id(
        &self,
        run_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
        Ok(self
            .inbox_claim_runs
            .lock()
            .await
            .get(run_id)
            .filter(|run| customer_scope_id.is_none_or(|scope| run.customer_scope_id == scope))
            .cloned())
    }
}
