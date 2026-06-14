use super::*;

impl InMemoryScoringRepository {
    pub(super) async fn in_memory_list_leads(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<LeadRecord>> {
        let visible_claim_ids = match customer_scope_id {
            Some(scope) => Some(scoped_claim_ids_from_audit_events(
                self.audit_events.lock().await.iter(),
                scope,
            )),
            None => None,
        };
        let mut leads = self
            .leads
            .lock()
            .await
            .values()
            .filter(|lead| {
                visible_claim_ids
                    .as_ref()
                    .is_none_or(|claim_ids| claim_ids.contains(&lead.claim_id))
            })
            .cloned()
            .collect::<Vec<_>>();
        leads.sort_by(|left, right| left.lead_id.cmp(&right.lead_id));
        Ok(leads)
    }

    pub(super) async fn in_memory_triage_lead(
        &self,
        lead_id: &str,
        input: TriageLeadInput,
    ) -> anyhow::Result<Option<TriageLeadRecord>> {
        let mut leads = self.leads.lock().await;
        let visible_claim_ids = match input.customer_scope_id.as_deref() {
            Some(scope) => Some(scoped_claim_ids_from_audit_events(
                self.audit_events.lock().await.iter(),
                scope,
            )),
            None => None,
        };
        if input.decision == "merge_lead"
            && !merge_target_exists_in_memory(&leads, &input, visible_claim_ids.as_ref())
        {
            return Ok(None);
        }
        let Some(lead) = leads.get_mut(lead_id) else {
            return Ok(None);
        };
        if visible_claim_ids
            .as_ref()
            .is_some_and(|claim_ids| !claim_ids.contains(&lead.claim_id))
        {
            return Ok(None);
        }
        lead.status = triage_status_for_decision(&input.decision).into();
        lead.disposition = triage_disposition_for_decision(&input.decision).into();
        let lead = lead.clone();
        let case = if input.decision == "open_case" {
            let case = case_from_lead(&lead, &input);
            self.cases
                .lock()
                .await
                .insert(case.case_id.clone(), case.clone());
            Some(case)
        } else {
            None
        };
        let audit_id = AuditEventId::new().to_string();
        self.audit_events.lock().await.push(PersistedAuditEvent {
            audit_id: audit_id.clone(),
            run_id: lead.run_id.clone(),
            claim_id: lead.claim_id.clone(),
            source_system: lead.source_system.clone(),
            actor_id: input.assignee.clone(),
            actor_role: "fwa_operator".into(),
            event_type: "lead.triaged".into(),
            event_status: "succeeded".into(),
            summary: format!("Lead triaged: {}", input.decision),
            payload: triage_audit_payload(&lead, &input, case.as_ref()),
            evidence_refs: input
                .evidence_refs
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        });
        Ok(Some(TriageLeadRecord {
            lead,
            case,
            audit_id,
        }))
    }

    pub(super) async fn in_memory_list_cases(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<CaseRecord>> {
        let visible_claim_ids = match customer_scope_id {
            Some(scope) => Some(scoped_claim_ids_from_audit_events(
                self.audit_events.lock().await.iter(),
                scope,
            )),
            None => None,
        };
        let mut cases = self
            .cases
            .lock()
            .await
            .values()
            .filter(|case| {
                visible_claim_ids
                    .as_ref()
                    .is_none_or(|claim_ids| claim_ids.contains(&case.claim_id))
            })
            .cloned()
            .collect::<Vec<_>>();
        cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));
        Ok(cases)
    }

    pub(super) async fn in_memory_update_case_status(
        &self,
        case_id: &str,
        input: UpdateCaseStatusInput,
    ) -> anyhow::Result<Option<UpdateCaseStatusRecord>> {
        let mut cases = self.cases.lock().await;
        let Some(case) = cases.get_mut(case_id) else {
            return Ok(None);
        };
        if !self
            .claim_visible_to_scope(&case.claim_id, input.customer_scope_id.as_deref())
            .await
        {
            return Ok(None);
        }
        let from_status = case.status.clone();
        case.status = input.status.clone();
        if is_terminal_case_status(&case.status) {
            case.time_to_closure_hours = Some(0.0);
        } else {
            case.time_to_closure_hours = None;
        }
        let elapsed_hours = case.time_to_closure_hours.unwrap_or(0.0);
        case.sla_status = case_sla_status(&case.status, case.sla_target_hours, elapsed_hours);
        let case = case.clone();
        drop(cases);
        let audit_run_id = self
            .leads
            .lock()
            .await
            .get(&case.lead_id)
            .map(|lead| lead.run_id.clone())
            .unwrap_or_else(|| format!("case_status_{}", case.case_id));
        let audit_id = AuditEventId::new().to_string();
        self.audit_events.lock().await.push(PersistedAuditEvent {
            audit_id: audit_id.clone(),
            run_id: audit_run_id,
            claim_id: case.claim_id.clone(),
            source_system: case.source_system.clone(),
            actor_id: input.actor_id.clone(),
            actor_role: "fwa_operator".into(),
            event_type: "case.status.updated".into(),
            event_status: "succeeded".into(),
            summary: format!("Case status updated: {} -> {}", from_status, case.status),
            payload: serde_json::json!({
                "claim_id": case.claim_id,
                "case_id": case.case_id,
                "lead_id": case.lead_id,
                "from_status": from_status,
                "to_status": case.status,
                "notes": input.notes,
                "customer_scope_id": input.customer_scope_id
            }),
            evidence_refs: input
                .evidence_refs
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        });
        Ok(Some(UpdateCaseStatusRecord { case, audit_id }))
    }

    pub(super) async fn in_memory_create_audit_sample(
        &self,
        input: CreateAuditSampleInput,
    ) -> anyhow::Result<AuditSampleRecord> {
        let mut sequence = self.audit_sample_sequence.lock().await;
        *sequence += 1;
        let sample_id = format!("sample_{}", *sequence);
        let customer_scope_id = input.customer_scope_id.as_deref();
        let leads = if input.sample_mode == "random_control" {
            let visible_claim_ids = match customer_scope_id {
                Some(scope) => Some(scoped_claim_ids_from_audit_events(
                    self.audit_events.lock().await.iter(),
                    scope,
                )),
                None => None,
            };
            let claims = self.claims.lock().await;
            self.runs
                .lock()
                .await
                .iter()
                .filter(|run| {
                    visible_claim_ids
                        .as_ref()
                        .is_none_or(|claim_ids| claim_ids.contains(&run.claim_id))
                })
                .map(|run| control_lead_from_scoring_run(run, claims.get(&run.claim_id)))
                .collect()
        } else {
            self.in_memory_list_leads(customer_scope_id).await?
        };
        let claims = self.claims.lock().await;
        let strata_contexts = audit_sample_strata_contexts_from_claims(&claims);
        drop(claims);
        let samples = self.audit_samples.lock().await;
        let reviewer_history = reviewer_lead_sample_counts(samples.values(), &input.reviewer);
        drop(samples);
        let sample = build_audit_sample(
            sample_id,
            input,
            leads,
            &strata_contexts,
            &reviewer_history,
            None,
        );
        self.audit_samples
            .lock()
            .await
            .insert(sample.sample_id.clone(), sample.clone());
        Ok(sample)
    }

    pub(super) async fn in_memory_list_audit_samples(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditSampleRecord>> {
        let mut samples = self
            .audit_samples
            .lock()
            .await
            .values()
            .filter(|sample| {
                customer_scope_id.is_none_or(|scope| sample.customer_scope_id == scope)
            })
            .cloned()
            .collect::<Vec<_>>();
        samples.sort_by(|left, right| left.sample_id.cmp(&right.sample_id));
        let reviews = self.list_qa_reviews(customer_scope_id).await?;
        Ok(with_sample_outcome_distributions(samples, &reviews))
    }
}
