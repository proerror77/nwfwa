use super::*;

#[derive(Debug, Default)]
pub struct InMemoryScoringRepository {
    claims: Mutex<HashMap<String, ClaimContext>>,
    inbox_claim_runs: Mutex<HashMap<String, PersistedInboxClaimRun>>,
    runs: Mutex<Vec<PersistedScoringRun>>,
    audit_events: Mutex<Vec<PersistedAuditEvent>>,
    agent_runs: Mutex<Vec<PersistedAgentRun>>,
    leads: Mutex<HashMap<String, LeadRecord>>,
    cases: Mutex<HashMap<String, CaseRecord>>,
    audit_samples: Mutex<HashMap<String, AuditSampleRecord>>,
    audit_sample_sequence: Mutex<u64>,
    candidate_rules: Mutex<HashMap<String, RuleDetailRecord>>,
    rule_statuses: Mutex<HashMap<String, String>>,
    rule_backtests: Mutex<Vec<RuleBacktestRecord>>,
    rule_shadow_runs: Mutex<Vec<RuleShadowRunRecord>>,
    rule_promotion_reviews: Mutex<Vec<RulePromotionReviewRecord>>,
    knowledge_cases: Mutex<HashMap<String, KnowledgeCaseRecord>>,
    datasets: Mutex<HashMap<String, DatasetRecord>>,
    dataset_sequence: Mutex<u64>,
    mapping_sequence: Mutex<u64>,
    pilot_audit_events: Mutex<Vec<(String, AuditHistoryEventRecord)>>,
    feature_sets: Mutex<HashMap<String, FeatureSetRecord>>,
    feature_set_sequence: Mutex<u64>,
    model_datasets: Mutex<HashMap<String, ModelDatasetRecord>>,
    model_dataset_sequence: Mutex<u64>,
    model_versions: Mutex<HashMap<String, ModelVersionRecord>>,
    model_evaluations: Mutex<HashMap<String, ModelEvaluationRecord>>,
    model_promotion_reviews: Mutex<Vec<ModelPromotionReviewRecord>>,
    model_retraining_jobs: Mutex<HashMap<String, ModelRetrainingJobRecord>>,
    model_retraining_job_sequence: Mutex<u64>,
    model_statuses: Mutex<HashMap<String, String>>,
    routing_policies: Mutex<Vec<RoutingPolicyRecord>>,
    webhook_delivery_attempts: Mutex<HashMap<String, Vec<WebhookDeliveryAttemptRecord>>>,
    saving_attributions: Mutex<Vec<SavingAttributionRecord>>,
    evidence_documents: Mutex<HashMap<String, EvidenceDocumentRecord>>,
    evidence_document_chunks: Mutex<HashMap<String, EvidenceDocumentChunkRecord>>,
    evidence_ocr_outputs: Mutex<HashMap<String, EvidenceOcrOutputRecord>>,
    evidence_embedding_jobs: Mutex<HashMap<String, EvidenceEmbeddingJobRecord>>,
    evidence_retrieval_audit_events: Mutex<HashMap<String, EvidenceRetrievalAuditEventRecord>>,
}

async fn upsert_pilot_audit_event(
    events: &Mutex<Vec<(String, AuditHistoryEventRecord)>>,
    claim_id: String,
    event: AuditHistoryEventRecord,
) {
    let mut events = events.lock().await;
    if let Some((stored_claim_id, stored_event)) = events
        .iter_mut()
        .find(|(_, stored_event)| stored_event.audit_id == event.audit_id)
    {
        *stored_claim_id = claim_id;
        *stored_event = event;
    } else {
        events.push((claim_id, event));
    }
}

impl InMemoryScoringRepository {
    pub fn shared() -> SharedRepository {
        Arc::new(Self::default())
    }

    pub fn shared_with_routing_policies(policies: Vec<RoutingPolicy>) -> SharedRepository {
        Arc::new(Self {
            routing_policies: Mutex::new(
                policies
                    .into_iter()
                    .map(|policy| routing_policy_record(policy, "active", "system", None, None))
                    .collect(),
            ),
            ..Self::default()
        })
    }

    async fn claim_visible_to_scope(
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
}

#[async_trait]
impl ScoringRepository for InMemoryScoringRepository {
    async fn upsert_claim_context(
        &self,
        context: ClaimContext,
        _raw_payload: Value,
    ) -> anyhow::Result<()> {
        self.claims
            .lock()
            .await
            .insert(context.claim.external_claim_id.clone(), context);
        Ok(())
    }

    async fn load_claim_context(
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

    async fn member_profile_summary(
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

    async fn save_scoring_run(&self, run: PersistedScoringRun) -> anyhow::Result<()> {
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

    async fn save_audit_event(&self, event: PersistedAuditEvent) -> anyhow::Result<()> {
        let event = PersistedAuditEvent {
            payload: mask_audit_payload(event.payload),
            ..event
        };
        let mut audit_events = self.audit_events.lock().await;
        if let Some(existing) = audit_events
            .iter_mut()
            .find(|existing| existing.audit_id == event.audit_id)
        {
            *existing = event;
        } else {
            audit_events.push(event);
        }
        Ok(())
    }

    async fn save_inbox_claim_run(&self, run: PersistedInboxClaimRun) -> anyhow::Result<()> {
        self.inbox_claim_runs
            .lock()
            .await
            .insert(run.run_id.clone(), run);
        Ok(())
    }

    async fn get_inbox_claim_run_by_idempotency_key(
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

    async fn get_inbox_claim_run_by_run_id(
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

    async fn active_routing_policy(
        &self,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicy>> {
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        Ok(policies
            .iter()
            .filter(|policy| policy.status == "active")
            .filter(|policy| routing_policy_review_mode_applies(&policy.review_mode, review_mode))
            .max_by_key(|policy| (policy.review_mode == review_mode, policy.version))
            .map(routing_policy_from_record))
    }

    async fn list_routing_policies(&self) -> anyhow::Result<Vec<RoutingPolicyRecord>> {
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        Ok(policies.clone())
    }

    async fn save_routing_policy_candidate(
        &self,
        policy: RoutingPolicy,
        owner: String,
    ) -> anyhow::Result<RoutingPolicyRecord> {
        let record = routing_policy_record(policy, "draft", &owner, None, None);
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        policies.retain(|existing| {
            !(existing.policy_id == record.policy_id
                && existing.version == record.version
                && existing.review_mode == record.review_mode)
        });
        policies.push(record.clone());
        Ok(record)
    }

    async fn get_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        Ok(policies
            .iter()
            .find(|policy| {
                policy.policy_id == policy_id
                    && policy.version == version
                    && policy.review_mode == review_mode
            })
            .cloned())
    }

    async fn update_routing_policy_status(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
        status: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        let Some(policy) = policies.iter_mut().find(|policy| {
            policy.policy_id == policy_id
                && policy.version == version
                && policy.review_mode == review_mode
        }) else {
            return Ok(None);
        };
        policy.status = status.into();
        Ok(Some(policy.clone()))
    }

    async fn activate_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        if !policies.iter().any(|policy| {
            policy.policy_id == policy_id
                && policy.version == version
                && policy.review_mode == review_mode
        }) {
            return Ok(None);
        }
        for policy in policies
            .iter_mut()
            .filter(|policy| policy.review_mode == review_mode && policy.status == "active")
        {
            policy.status = "approved".into();
        }
        let policy = policies
            .iter_mut()
            .find(|policy| {
                policy.policy_id == policy_id
                    && policy.version == version
                    && policy.review_mode == review_mode
            })
            .expect("routing policy existence checked before activation");
        policy.status = "active".into();
        Ok(Some(policy.clone()))
    }

    async fn list_rules(&self) -> anyhow::Result<Vec<RuleSummaryRecord>> {
        let statuses = self.rule_statuses.lock().await;
        let backtests = self.rule_backtests.lock().await.clone();
        let mut details = default_rule_details();
        details.extend(self.candidate_rules.lock().await.values().cloned());
        let mut rules = details
            .into_iter()
            .map(|mut detail| {
                apply_rule_status(&mut detail, &statuses);
                if let Some(backtest) = latest_rule_backtest_for(
                    &backtests,
                    &detail.summary.rule_id,
                    detail.summary.latest_version,
                ) {
                    apply_rule_backtest_metadata(&mut detail.summary, Some(backtest));
                }
                detail.summary
            })
            .collect::<Vec<_>>();
        rules.sort_by(|left, right| left.rule_id.cmp(&right.rule_id));
        Ok(rules)
    }

    async fn list_active_rules(&self) -> anyhow::Result<Vec<Rule>> {
        let statuses = self.rule_statuses.lock().await;
        let mut details = default_rule_details();
        details.extend(self.candidate_rules.lock().await.values().cloned());
        details
            .into_iter()
            .filter_map(|mut detail| {
                apply_rule_status(&mut detail, &statuses);
                (detail.summary.status == "active").then_some(detail)
            })
            .map(runtime_rule_from_detail)
            .collect()
    }

    async fn get_rule(&self, rule_id: &str) -> anyhow::Result<Option<RuleDetailRecord>> {
        let statuses = self.rule_statuses.lock().await;
        let backtests = self.rule_backtests.lock().await.clone();
        let mut details = default_rule_details();
        details.extend(self.candidate_rules.lock().await.values().cloned());
        let audit_events = self.rule_audit_history(rule_id).await?;
        Ok(details
            .into_iter()
            .find(|detail| detail.summary.rule_id == rule_id)
            .map(|mut detail| {
                apply_rule_status(&mut detail, &statuses);
                if let Some(backtest) = latest_rule_backtest_for(
                    &backtests,
                    &detail.summary.rule_id,
                    detail.summary.latest_version,
                ) {
                    apply_rule_backtest_metadata(&mut detail.summary, Some(backtest));
                }
                detail.audit_events = audit_events;
                detail
            }))
    }

    async fn rule_audit_history(
        &self,
        rule_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        Ok(self
            .audit_events
            .lock()
            .await
            .iter()
            .filter(|event| event.payload["rule_id"].as_str() == Some(rule_id))
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
            .collect())
    }

    async fn save_rule_candidate(
        &self,
        rule: Rule,
        owner: String,
    ) -> anyhow::Result<RuleDetailRecord> {
        let detail = rule_detail_from_rule(rule, "draft", owner);
        self.candidate_rules
            .lock()
            .await
            .insert(detail.summary.rule_id.clone(), detail.clone());
        Ok(detail)
    }

    async fn update_rule_status(
        &self,
        rule_id: &str,
        status: &str,
    ) -> anyhow::Result<Option<RuleSummaryRecord>> {
        if self.get_rule(rule_id).await?.is_none() {
            return Ok(None);
        }
        self.rule_statuses
            .lock()
            .await
            .insert(rule_id.to_string(), status.to_string());
        Ok(self.get_rule(rule_id).await?.map(|detail| detail.summary))
    }

    async fn list_rule_conditions(&self) -> anyhow::Result<Vec<RuleConditionLibraryRecord>> {
        let statuses = self.rule_statuses.lock().await;
        let mut details = default_rule_details();
        details.extend(self.candidate_rules.lock().await.values().cloned());
        let mut records = Vec::new();
        for mut detail in details {
            apply_rule_status(&mut detail, &statuses);
            records.extend(rule_condition_records_from_detail(&detail)?);
        }
        records.sort_by(|left, right| left.condition_key.cmp(&right.condition_key));
        Ok(records)
    }

    async fn rule_performance(&self) -> anyhow::Result<Vec<RulePerformanceRecord>> {
        let rules = self.list_rules().await?;
        let runs = self.runs.lock().await;
        let pilot_events = self.pilot_audit_events.lock().await;
        let mut outcomes = HashMap::new();
        for (_, event) in pilot_events.iter() {
            if event.event_type != "investigation.result.received" {
                continue;
            }
            let Some(claim_id) = event.payload["claim_id"].as_str() else {
                continue;
            };
            outcomes.insert(
                claim_id.to_string(),
                InvestigationOutcome {
                    confirmed_fwa: event.payload["confirmed_fwa"].as_bool().unwrap_or(false),
                    saving_amount: decimal_from_json(&event.payload["saving_amount"]),
                },
            );
        }

        let mut accumulators = rule_accumulators_from_rules(&rules);
        for run in runs.iter() {
            for rule_run in &run.rule_runs {
                let Some(rule_id) = rule_run["rule_id"].as_str() else {
                    continue;
                };
                let Some(accumulator) = accumulators.get_mut(rule_id) else {
                    continue;
                };
                accumulator.trigger_count += 1;
                accumulator.triggered_claim_ids.insert(run.claim_id.clone());
            }
        }

        Ok(rule_performance_records(
            accumulators,
            &outcomes,
            runs.len() as u32,
        ))
    }

    async fn save_rule_backtest(
        &self,
        mut record: RuleBacktestRecord,
    ) -> anyhow::Result<RuleBacktestRecord> {
        record.created_at = Some(chrono::Utc::now().to_rfc3339());
        self.rule_backtests.lock().await.push(record.clone());
        Ok(record)
    }

    async fn latest_rule_backtest(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleBacktestRecord>> {
        Ok(self
            .rule_backtests
            .lock()
            .await
            .iter()
            .rev()
            .find(|record| record.rule_id == rule_id && record.rule_version == rule_version)
            .cloned())
    }

    async fn save_rule_shadow_run(
        &self,
        mut record: RuleShadowRunRecord,
    ) -> anyhow::Result<RuleShadowRunRecord> {
        record.created_at = Some(chrono::Utc::now().to_rfc3339());
        self.rule_shadow_runs.lock().await.push(record.clone());
        Ok(record)
    }

    async fn latest_rule_shadow_run(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleShadowRunRecord>> {
        Ok(self
            .rule_shadow_runs
            .lock()
            .await
            .iter()
            .rev()
            .find(|record| record.rule_id == rule_id && record.rule_version == rule_version)
            .cloned())
    }

    async fn save_rule_promotion_review(
        &self,
        mut record: RulePromotionReviewRecord,
    ) -> anyhow::Result<RulePromotionReviewRecord> {
        record.created_at = Some(chrono::Utc::now().to_rfc3339());
        self.rule_promotion_reviews
            .lock()
            .await
            .push(record.clone());
        Ok(record)
    }

    async fn latest_rule_promotion_review(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RulePromotionReviewRecord>> {
        Ok(self
            .rule_promotion_reviews
            .lock()
            .await
            .iter()
            .rev()
            .find(|review| review.rule_id == rule_id && review.rule_version == rule_version)
            .cloned())
    }

    async fn list_leads(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<LeadRecord>> {
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

    async fn triage_lead(
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

    async fn list_cases(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<CaseRecord>> {
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

    async fn update_case_status(
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

    async fn create_audit_sample(
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
            self.list_leads(customer_scope_id).await?
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

    async fn list_audit_samples(
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

    async fn list_models(&self) -> anyhow::Result<Vec<ModelVersionRecord>> {
        let statuses = self.model_statuses.lock().await;
        let mut models = default_model_versions();
        models.extend(self.model_versions.lock().await.values().cloned());
        models.sort_by(|left, right| {
            left.model_key
                .cmp(&right.model_key)
                .then_with(|| right.version.cmp(&left.version))
        });
        Ok(models
            .into_iter()
            .map(|mut model| {
                if let Some(status) =
                    statuses.get(&model_version_key(&model.model_key, &model.version))
                {
                    model.status = status.clone();
                }
                model
            })
            .collect())
    }

    async fn save_model_version(
        &self,
        record: ModelVersionRecord,
    ) -> anyhow::Result<ModelVersionRecord> {
        self.model_versions.lock().await.insert(
            model_version_key(&record.model_key, &record.version),
            record.clone(),
        );
        Ok(record)
    }

    async fn update_model_status(
        &self,
        model_key: &str,
        model_version: &str,
        status: &str,
    ) -> anyhow::Result<Option<ModelVersionRecord>> {
        let mut models = self.list_models().await?;
        let Some(model) = models
            .iter_mut()
            .find(|model| model.model_key == model_key && model.version == model_version)
        else {
            return Ok(None);
        };
        model.status = status.to_string();
        self.model_statuses.lock().await.insert(
            model_version_key(model_key, model_version),
            status.to_string(),
        );
        Ok(Some(model.clone()))
    }

    async fn model_performance(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Option<ModelPerformanceRecord>> {
        if default_model_versions()
            .iter()
            .any(|model| model.model_key == model_key)
        {
            let evaluations = self.model_evaluations.lock().await;
            let drift = evaluations
                .values()
                .filter(|evaluation| evaluation.model_key == model_key)
                .max_by(|left, right| left.evaluation_run_id.cmp(&right.evaluation_run_id))
                .map(|evaluation| drift_summary(&evaluation.metrics_json))
                .unwrap_or_else(|| drift_summary(&Value::Null));
            Ok(Some(model_performance_with_drift(
                empty_model_performance(model_key),
                drift,
            )))
        } else {
            Ok(None)
        }
    }

    async fn save_model_promotion_review(
        &self,
        mut record: ModelPromotionReviewRecord,
    ) -> anyhow::Result<ModelPromotionReviewRecord> {
        record.created_at = Some(chrono::Utc::now().to_rfc3339());
        self.model_promotion_reviews
            .lock()
            .await
            .push(record.clone());
        Ok(record)
    }

    async fn latest_model_promotion_review(
        &self,
        model_key: &str,
        model_version: &str,
    ) -> anyhow::Result<Option<ModelPromotionReviewRecord>> {
        Ok(self
            .model_promotion_reviews
            .lock()
            .await
            .iter()
            .rev()
            .find(|review| review.model_key == model_key && review.model_version == model_version)
            .cloned())
    }

    async fn save_model_retraining_job(
        &self,
        mut record: ModelRetrainingJobRecord,
    ) -> anyhow::Result<ModelRetrainingJobRecord> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut sequence = self.model_retraining_job_sequence.lock().await;
        *sequence += 1;
        record.job_id = format!("model_retraining_job_{}", *sequence);
        record.created_at = Some(now.clone());
        record.updated_at = Some(now);
        self.model_retraining_jobs
            .lock()
            .await
            .insert(record.job_id.clone(), record.clone());
        Ok(record)
    }

    async fn list_model_retraining_jobs(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Vec<ModelRetrainingJobRecord>> {
        let mut jobs = self
            .model_retraining_jobs
            .lock()
            .await
            .values()
            .filter(|job| job.model_key == model_key)
            .cloned()
            .collect::<Vec<_>>();
        jobs.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(jobs)
    }

    async fn get_model_retraining_job(
        &self,
        job_id: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        Ok(self.model_retraining_jobs.lock().await.get(job_id).cloned())
    }

    async fn claim_next_model_retraining_job(
        &self,
        model_key: Option<&str>,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let mut jobs = self.model_retraining_jobs.lock().await;
        let next_job_id = jobs
            .values()
            .filter(|job| job.status == "queued")
            .filter(|job| model_key.map(|key| job.model_key == key).unwrap_or(true))
            .min_by(|left, right| left.created_at.cmp(&right.created_at))
            .map(|job| job.job_id.clone());
        let Some(job_id) = next_job_id else {
            return Ok(None);
        };
        let Some(job) = jobs.get_mut(&job_id) else {
            return Ok(None);
        };
        job.status = "running".into();
        job.updated_by = actor.to_string();
        job.status_note = status_note.to_string();
        job.updated_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(Some(job.clone()))
    }

    async fn update_model_retraining_job_status(
        &self,
        job_id: &str,
        status: &str,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let mut jobs = self.model_retraining_jobs.lock().await;
        let Some(job) = jobs.get_mut(job_id) else {
            return Ok(None);
        };
        job.status = status.to_string();
        job.updated_by = actor.to_string();
        job.status_note = status_note.to_string();
        job.updated_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(Some(job.clone()))
    }

    async fn complete_model_retraining_job(
        &self,
        input: CompleteModelRetrainingJobInput<'_>,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let mut jobs = self.model_retraining_jobs.lock().await;
        let Some(job) = jobs.get_mut(input.job_id) else {
            return Ok(None);
        };
        job.status = "completed".into();
        job.updated_by = input.actor.to_string();
        job.status_note = input.status_note.to_string();
        job.candidate_model_version = Some(input.candidate_model_version.to_string());
        job.candidate_artifact_uri = Some(input.candidate_artifact_uri.to_string());
        job.candidate_endpoint_url = input.candidate_endpoint_url.map(ToString::to_string);
        job.validation_report_uri = Some(input.validation_report_uri.to_string());
        job.output_evaluation_id = Some(input.output_evaluation_id.to_string());
        job.updated_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(Some(job.clone()))
    }

    async fn dashboard_summary(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<DashboardSummaryRecord> {
        let runs = self.runs.lock().await;
        let claims = self.claims.lock().await;
        let pilot_events = self.pilot_audit_events.lock().await;
        let saving_attribution_records = self.saving_attributions.lock().await.clone();

        let mut risk_amount = Decimal::ZERO;
        let mut rag_distribution = BTreeMap::new();
        let mut model_accumulators = BTreeMap::<String, (u32, u32, u32)>::new();
        let mut layer_accumulators = BTreeMap::<String, (String, u32, u32, u32)>::new();
        let mut rule_hits = 0_u32;

        let scoped_runs = runs
            .iter()
            .filter(|run| {
                customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&run.audit_event, scope)
                })
            })
            .collect::<Vec<_>>();
        let scoped_pilot_events = pilot_events
            .iter()
            .filter(|(_, event)| {
                customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&event.payload, scope)
                })
            })
            .collect::<Vec<_>>();
        let scoped_claim_ids = scoped_runs
            .iter()
            .map(|run| run.claim_id.clone())
            .chain(
                scoped_pilot_events
                    .iter()
                    .map(|(claim_id, _)| claim_id.clone()),
            )
            .collect::<BTreeSet<_>>();
        let scoped_saving_attributions = saving_attribution_records
            .iter()
            .filter(|attribution| {
                customer_scope_id.is_none() || scoped_claim_ids.contains(&attribution.claim_id)
            })
            .cloned()
            .collect::<Vec<_>>();

        for run in scoped_runs.iter() {
            if run.risk_score >= 70 {
                if let Some(context) = claims.get(&run.claim_id) {
                    risk_amount += context.claim.amount.amount;
                }
            }
            *rag_distribution.entry(run.rag.clone()).or_insert(0) += 1;
            rule_hits += run.rule_runs.len() as u32;

            let model_key = run.model_score["model_key"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let score = run.model_score["score"].as_u64().unwrap_or(0) as u32;
            let entry = model_accumulators.entry(model_key).or_insert((0, 0, 0));
            entry.0 += 1;
            entry.1 += score;
            if score >= 70 {
                entry.2 += 1;
            }

            for layer in run
                .audit_event
                .get("layers")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                let layer_id = layer["layer_id"].as_str().unwrap_or("UNKNOWN").to_string();
                let layer_name = layer["name"].as_str().unwrap_or("Unknown").to_string();
                let layer_score = layer["score"].as_u64().unwrap_or(0) as u32;
                let entry =
                    layer_accumulators
                        .entry(layer_id)
                        .or_insert((layer_name.clone(), 0, 0, 0));
                entry.0 = layer_name;
                entry.1 += 1;
                entry.2 += layer_score;
                if layer_score >= 70 {
                    entry.3 += 1;
                }
            }
        }

        let mut saving_amount = Decimal::ZERO;
        let mut confirmed_fwa = 0_u32;
        let mut investigation_results = 0_u32;
        let mut qa_reviews = 0_u32;
        let mut outcome_labels = Vec::new();
        let mut financial_impacts = Vec::new();
        let feedback_statuses = latest_qa_feedback_statuses(
            &scoped_pilot_events
                .iter()
                .map(|(claim_id, event)| ((*claim_id).clone(), (*event).clone()))
                .collect::<Vec<_>>(),
        );

        for (_, event) in scoped_pilot_events.iter() {
            match event.event_type.as_str() {
                "investigation.result.received" => {
                    investigation_results += 1;
                    if let Ok(record) =
                        serde_json::from_value::<InvestigationResultRecord>(event.payload.clone())
                    {
                        outcome_labels.extend(labels_from_investigation_result(record.clone()));
                        if let Some(impact) = financial_impact_from_investigation(&record) {
                            financial_impacts.push(impact);
                        }
                    }
                    if event.payload["confirmed_fwa"].as_bool().unwrap_or(false) {
                        confirmed_fwa += 1;
                    }
                    if let Some(value) = event.payload["saving_amount"].as_str() {
                        saving_amount += value.parse::<Decimal>().unwrap_or(Decimal::ZERO);
                    }
                }
                "qa.result.received" => {
                    qa_reviews += 1;
                    if let Ok(record) =
                        serde_json::from_value::<QaReviewRecord>(event.payload.clone())
                    {
                        let feedback_id = qa_feedback_id(&record.qa_case_id);
                        let feedback_status = feedback_statuses
                            .get(&feedback_id)
                            .map(|update| update.status.as_str())
                            .unwrap_or("open");
                        outcome_labels.push(label_from_qa_review(record, feedback_status));
                    }
                }
                "medical.review.recorded" => {
                    outcome_labels.extend(labels_from_medical_review_event(event));
                }
                _ => {}
            }
        }
        let runtime_events = self.audit_events.lock().await;
        let scoring_audit_runs = runtime_events
            .iter()
            .filter(|event| {
                event.event_type == "scoring.completed"
                    && event.event_status == "succeeded"
                    && customer_scope_id.is_none_or(|scope| {
                        audit_event_payload_matches_customer_scope(&event.payload, scope)
                    })
            })
            .count() as u32;
        let canonical_trace_audit_runs = runtime_events
            .iter()
            .filter(|event| {
                event.event_type == "scoring.completed"
                    && event.event_status == "succeeded"
                    && customer_scope_id.is_none_or(|scope| {
                        audit_event_payload_matches_customer_scope(&event.payload, scope)
                    })
                    && event
                        .payload
                        .get("canonical_claim_context_trace")
                        .and_then(Value::as_object)
                        .is_some()
            })
            .count() as u32;
        let audit_coverage =
            summarize_dashboard_audit_coverage(scoring_audit_runs, canonical_trace_audit_runs);
        for event in runtime_events.iter().filter(|event| {
            event.event_type == "medical.review.recorded"
                && customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&event.payload, scope)
                })
        }) {
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
            outcome_labels.extend(labels_from_medical_review_event(&audit_event));
        }
        let suspected_claims = scoped_runs
            .iter()
            .filter(|run| run.risk_score >= 70)
            .count() as u32;
        let saving_attributions = summarize_saving_attributions(&scoped_saving_attributions);
        drop(runtime_events);
        drop(pilot_events);
        drop(claims);
        drop(runs);

        let audit_samples = self.list_audit_samples(customer_scope_id).await?;
        let qa_review_records = self.list_qa_reviews(customer_scope_id).await?;
        let qa_feedback_items = self.list_qa_feedback_items(customer_scope_id).await?;
        let cases = self.list_cases(customer_scope_id).await?;
        let agent_runs = self.list_agent_runs(customer_scope_id).await?;
        let models = self.list_models().await?;
        let model_evaluations = self.list_model_evaluations().await?;
        let rules = self.list_rules().await?;
        let rule_performance = self.rule_performance().await?;
        let leads = self.list_leads(customer_scope_id).await?;
        let scheme_distribution = leads
            .iter()
            .fold(BTreeMap::new(), |mut distribution, lead| {
                *distribution.entry(lead.scheme_family.clone()).or_insert(0) += 1;
                distribution
            });
        let saving_segments = summarize_saving_segments(&scoped_saving_attributions, &leads);
        let false_positive_count = rule_performance
            .iter()
            .map(|record| record.false_positive_count)
            .sum::<u32>();
        let value_measurement = summarize_dashboard_value_measurement(
            &financial_impacts,
            rule_hits,
            false_positive_count,
        );

        Ok(DashboardSummaryRecord {
            suspected_claims,
            confirmed_fwa,
            risk_amount: risk_amount.to_string(),
            saving_amount: saving_amount.to_string(),
            rag_distribution,
            scheme_distribution,
            rule_hits,
            model_scores: model_accumulators
                .into_iter()
                .map(|(model_key, (scored_runs, score_sum, high_risk_count))| {
                    let average_score = if scored_runs == 0 {
                        0.0
                    } else {
                        score_sum as f64 / scored_runs as f64
                    };
                    (
                        model_key,
                        DashboardModelScoreRecord {
                            scored_runs,
                            average_score,
                            high_risk_count,
                        },
                    )
                })
                .collect(),
            layer_scores: layer_accumulators
                .into_iter()
                .map(
                    |(layer_id, (name, scored_runs, score_sum, high_risk_count))| {
                        let average_score = if scored_runs == 0 {
                            0.0
                        } else {
                            score_sum as f64 / scored_runs as f64
                        };
                        (
                            layer_id,
                            DashboardLayerScoreRecord {
                                name,
                                scored_runs,
                                average_score,
                                high_risk_count,
                            },
                        )
                    },
                )
                .collect(),
            saving_attributions,
            saving_segments,
            value_measurement,
            audit_coverage,
            label_pool: summarize_dashboard_label_pool(&outcome_labels),
            qa_queue: summarize_dashboard_qa_queue(
                &audit_samples,
                &qa_review_records,
                &qa_feedback_items,
            ),
            case_sla: summarize_dashboard_case_sla(&cases),
            agent_governance: summarize_dashboard_agent_governance(&agent_runs),
            model_governance: summarize_dashboard_model_governance(&models, &model_evaluations),
            rule_governance: summarize_dashboard_rule_governance(&rules, &rule_performance),
            investigation_results,
            qa_reviews,
        })
    }

    async fn provider_risk_summary(&self) -> anyhow::Result<ProviderRiskSummaryRecord> {
        let runs = self.runs.lock().await;
        Ok(summarize_provider_risk_profiles(
            runs.iter().map(|run| &run.audit_event),
        ))
    }

    async fn list_knowledge_cases(&self) -> anyhow::Result<Vec<KnowledgeCaseRecord>> {
        let mut cases = default_knowledge_cases()
            .into_iter()
            .map(|case| (case.case_id.clone(), case))
            .collect::<HashMap<_, _>>();
        cases.extend(self.knowledge_cases.lock().await.clone());
        let mut cases = cases.into_values().collect::<Vec<_>>();
        cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));
        Ok(cases)
    }

    async fn save_knowledge_case(
        &self,
        record: KnowledgeCaseRecord,
    ) -> anyhow::Result<KnowledgeCaseRecord> {
        self.knowledge_cases
            .lock()
            .await
            .insert(record.case_id.clone(), record.clone());
        Ok(record)
    }

    async fn search_similar_cases(
        &self,
        query: SimilarCaseQuery,
    ) -> anyhow::Result<Vec<SimilarCaseRecord>> {
        Ok(search_cases(self.list_knowledge_cases().await?, &query))
    }

    async fn save_agent_run(&self, run: PersistedAgentRun) -> anyhow::Result<()> {
        self.agent_runs.lock().await.push(run);
        Ok(())
    }

    async fn list_agent_runs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AgentRunLogRecord>> {
        let scoped_claim_ids = match customer_scope_id {
            Some(scope) => Some(scoped_claim_ids_from_audit_events(
                self.audit_events.lock().await.iter(),
                scope,
            )),
            None => None,
        };
        let mut runs = self
            .agent_runs
            .lock()
            .await
            .iter()
            .filter(|run| {
                scoped_claim_ids
                    .as_ref()
                    .is_none_or(|claim_ids| claim_ids.contains(&run.claim_id))
            })
            .map(agent_run_log_from_persisted)
            .collect::<Vec<_>>();
        runs.sort_by(|left, right| left.agent_run_id.cmp(&right.agent_run_id));
        Ok(runs)
    }

    async fn save_agent_approval(
        &self,
        approval: AgentApprovalRecord,
    ) -> anyhow::Result<AgentApprovalRecord> {
        let mut runs = self.agent_runs.lock().await;
        let Some(run) = runs
            .iter_mut()
            .find(|run| run.agent_run_id == approval.agent_run_id)
        else {
            anyhow::bail!("agent run not found: {}", approval.agent_run_id);
        };
        if let Some(existing) = run
            .approvals
            .iter_mut()
            .find(|existing| existing.approval_id == approval.approval_id)
        {
            *existing = approval.clone();
        } else {
            run.approvals.push(approval.clone());
        }
        Ok(approval)
    }

    async fn register_dataset(&self, input: RegisterDatasetInput) -> anyhow::Result<DatasetRecord> {
        let mut sequence = self.dataset_sequence.lock().await;
        *sequence += 1;
        let dataset_id = format!("dataset_{}", *sequence);
        let record = DatasetRecord {
            dataset_id: dataset_id.clone(),
            source_key: input.source_key,
            display_name: input.display_name,
            business_domain: input.business_domain,
            dataset_key: input.dataset_key,
            dataset_version: input.dataset_version,
            sample_grain: input.sample_grain,
            label_column: input.label_column,
            entity_keys: input.entity_keys,
            manifest_uri: input.manifest_uri,
            schema_uri: input.schema_uri,
            profile_uri: input.profile_uri,
            storage_format: input.storage_format,
            schema_hash: input.schema_hash,
            row_count: input.row_count,
            status: input.status,
            splits: input.splits,
            fields: input.fields,
            mappings: vec![],
        };
        self.datasets
            .lock()
            .await
            .insert(dataset_id, record.clone());
        Ok(record)
    }

    async fn list_datasets(&self) -> anyhow::Result<Vec<DatasetRecord>> {
        let mut datasets = self
            .datasets
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        datasets.sort_by(|left, right| left.dataset_key.cmp(&right.dataset_key));
        Ok(datasets)
    }

    async fn get_dataset(&self, dataset_id: &str) -> anyhow::Result<Option<DatasetRecord>> {
        Ok(self.datasets.lock().await.get(dataset_id).cloned())
    }

    async fn add_field_mapping(
        &self,
        dataset_id: &str,
        input: CreateFieldMappingInput,
    ) -> anyhow::Result<Option<FieldMappingRecord>> {
        let mut datasets = self.datasets.lock().await;
        let Some(dataset) = datasets.get_mut(dataset_id) else {
            return Ok(None);
        };
        let mut sequence = self.mapping_sequence.lock().await;
        *sequence += 1;
        let mapping = FieldMappingRecord {
            mapping_id: format!("mapping_{}", *sequence),
            dataset_id: dataset_id.to_string(),
            external_field: input.external_field,
            canonical_target: input.canonical_target,
            feature_name: input.feature_name,
            transform_kind: input.transform_kind,
            transform_json: input.transform_json,
            status: input.status,
        };
        dataset.mappings.push(mapping.clone());
        Ok(Some(mapping))
    }

    async fn save_investigation_result(
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

    async fn save_qa_review(
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

    async fn list_qa_feedback_items(
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

    async fn update_qa_feedback_status(
        &self,
        feedback_id: &str,
        input: UpdateQaFeedbackStatusInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<UpdateQaFeedbackStatusRecord>> {
        let Some(mut item) = self
            .list_qa_feedback_items(customer_scope_id)
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

    async fn list_qa_reviews(
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

    async fn list_outcome_labels(
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
            self.list_cases(customer_scope_id)
                .await?
                .into_iter()
                .flat_map(labels_from_case_status),
        );
        sort_outcome_labels(&mut labels);
        Ok(labels)
    }

    async fn claim_audit_history(
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

    async fn list_audit_events(
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

    async fn list_webhook_events(&self) -> anyhow::Result<Vec<WebhookEventRecord>> {
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

    async fn save_webhook_delivery_attempt(
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

    async fn register_feature_set(
        &self,
        input: RegisterFeatureSetInput,
    ) -> anyhow::Result<Option<FeatureSetRecord>> {
        if self.get_dataset(&input.dataset_id).await?.is_none() {
            return Ok(None);
        }
        let mut sequence = self.feature_set_sequence.lock().await;
        *sequence += 1;
        let feature_set_id = format!("feature_set_{}", *sequence);
        let record = FeatureSetRecord {
            feature_set_id: feature_set_id.clone(),
            business_domain: input.business_domain,
            feature_set_key: input.feature_set_key,
            version: input.version,
            dataset_id: input.dataset_id,
            features_uri: input.features_uri,
            feature_list_json: input.feature_list_json,
            row_count: input.row_count,
            label_column: input.label_column,
            status: input.status,
        };
        self.feature_sets
            .lock()
            .await
            .insert(feature_set_id, record.clone());
        Ok(Some(record))
    }

    async fn register_model_dataset(
        &self,
        input: RegisterModelDatasetInput,
    ) -> anyhow::Result<Option<ModelDatasetRecord>> {
        if !self
            .feature_sets
            .lock()
            .await
            .contains_key(&input.feature_set_id)
        {
            return Ok(None);
        }
        let mut sequence = self.model_dataset_sequence.lock().await;
        *sequence += 1;
        let model_dataset_id = format!("model_dataset_{}", *sequence);
        let record = ModelDatasetRecord {
            model_dataset_id: model_dataset_id.clone(),
            business_domain: input.business_domain,
            task_type: input.task_type,
            label_name: input.label_name,
            feature_set_id: input.feature_set_id,
            train_uri: input.train_uri,
            validation_uri: input.validation_uri,
            test_uri: input.test_uri,
            row_counts_json: input.row_counts_json,
            label_distribution_json: input.label_distribution_json,
            status: input.status,
        };
        self.model_datasets
            .lock()
            .await
            .insert(model_dataset_id, record.clone());
        Ok(Some(record))
    }

    async fn get_model_dataset_source_dataset(
        &self,
        model_dataset_id: &str,
    ) -> anyhow::Result<Option<DatasetRecord>> {
        let model_dataset = self
            .model_datasets
            .lock()
            .await
            .get(model_dataset_id)
            .cloned();
        let Some(model_dataset) = model_dataset else {
            return Ok(None);
        };
        let feature_set = self
            .feature_sets
            .lock()
            .await
            .get(&model_dataset.feature_set_id)
            .cloned();
        let Some(feature_set) = feature_set else {
            return Ok(None);
        };
        self.get_dataset(&feature_set.dataset_id).await
    }

    async fn register_model_evaluation(
        &self,
        input: RegisterModelEvaluationInput,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        if !self
            .model_datasets
            .lock()
            .await
            .contains_key(&input.model_dataset_id)
        {
            return Ok(None);
        }
        let record = ModelEvaluationRecord {
            evaluation_run_id: input.evaluation_run_id,
            model_key: input.model_key,
            model_version: input.model_version,
            model_dataset_id: input.model_dataset_id,
            scheme_family: input.scheme_family,
            auc: input.auc,
            ks: input.ks,
            precision: input.precision,
            recall: input.recall,
            f1: input.f1,
            accuracy: input.accuracy,
            threshold: input.threshold,
            confusion_matrix_json: input.confusion_matrix_json,
            feature_importance_uri: input.feature_importance_uri,
            permutation_importance_uri: input.permutation_importance_uri,
            metrics_json: input.metrics_json,
        };
        self.model_evaluations
            .lock()
            .await
            .insert(record.evaluation_run_id.clone(), record.clone());
        Ok(Some(record))
    }

    async fn get_model_evaluation(
        &self,
        evaluation_run_id: &str,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        Ok(self
            .model_evaluations
            .lock()
            .await
            .get(evaluation_run_id)
            .cloned())
    }

    async fn list_model_evaluations(&self) -> anyhow::Result<Vec<ModelEvaluationRecord>> {
        let mut evaluations = self
            .model_evaluations
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        evaluations.sort_by(|left, right| left.evaluation_run_id.cmp(&right.evaluation_run_id));
        Ok(evaluations)
    }

    async fn save_evidence_document(
        &self,
        input: CreateEvidenceDocumentInput,
    ) -> anyhow::Result<EvidenceDocumentRecord> {
        let record = EvidenceDocumentRecord {
            document_id: input.document_id,
            customer_scope_id: input.customer_scope_id,
            source_system: input.source_system,
            source_record_ref: input.source_record_ref,
            claim_id: input.claim_id,
            external_document_id: input.external_document_id,
            document_type: input.document_type,
            storage_uri: input.storage_uri,
            content_checksum: input.content_checksum,
            ingestion_status: input.ingestion_status,
            redaction_status: input.redaction_status,
            retention_policy_id: input.retention_policy_id,
            evidence_refs: input.evidence_refs,
            metadata_json: input.metadata_json,
            created_at: None,
            updated_at: None,
        };
        self.evidence_documents
            .lock()
            .await
            .insert(record.document_id.clone(), record.clone());
        Ok(record)
    }

    async fn list_evidence_documents(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentRecord>> {
        let mut records = self
            .evidence_documents
            .lock()
            .await
            .values()
            .filter(|record| {
                customer_scope_id.is_none_or(|scope| record.customer_scope_id == scope)
            })
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.document_id.cmp(&right.document_id));
        Ok(records)
    }

    async fn get_evidence_document(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentRecord>> {
        Ok(self
            .evidence_documents
            .lock()
            .await
            .get(document_id)
            .filter(|record| {
                customer_scope_id.is_none_or(|scope| record.customer_scope_id == scope)
            })
            .cloned())
    }

    async fn save_evidence_document_chunk(
        &self,
        input: CreateEvidenceDocumentChunkInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentChunkRecord>> {
        if self
            .get_evidence_document(&input.document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let record = EvidenceDocumentChunkRecord {
            chunk_id: input.chunk_id,
            document_id: input.document_id,
            chunk_index: input.chunk_index,
            chunking_version: input.chunking_version,
            redaction_status: input.redaction_status,
            text_checksum: input.text_checksum,
            token_count: input.token_count,
            storage_uri: input.storage_uri,
            source_offsets_json: input.source_offsets_json,
            evidence_refs: input.evidence_refs,
            created_at: None,
        };
        self.evidence_document_chunks
            .lock()
            .await
            .insert(record.chunk_id.clone(), record.clone());
        Ok(Some(record))
    }

    async fn list_evidence_document_chunks(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentChunkRecord>> {
        if self
            .get_evidence_document(document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(Vec::new());
        }
        let mut records = self
            .evidence_document_chunks
            .lock()
            .await
            .values()
            .filter(|record| record.document_id == document_id)
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by_key(|record| record.chunk_index);
        Ok(records)
    }

    async fn save_evidence_ocr_output(
        &self,
        input: CreateEvidenceOcrOutputInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceOcrOutputRecord>> {
        if self
            .get_evidence_document(&input.document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let record = EvidenceOcrOutputRecord {
            ocr_output_id: input.ocr_output_id,
            document_id: input.document_id,
            ocr_engine: input.ocr_engine,
            ocr_engine_version: input.ocr_engine_version,
            output_uri: input.output_uri,
            output_checksum: input.output_checksum,
            confidence_score: input.confidence_score,
            quality_status: input.quality_status,
            evidence_refs: input.evidence_refs,
            created_at: None,
        };
        self.evidence_ocr_outputs
            .lock()
            .await
            .insert(record.ocr_output_id.clone(), record.clone());
        Ok(Some(record))
    }

    async fn list_evidence_ocr_outputs(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceOcrOutputRecord>> {
        if self
            .get_evidence_document(document_id, customer_scope_id)
            .await?
            .is_none()
        {
            return Ok(Vec::new());
        }
        let mut records = self
            .evidence_ocr_outputs
            .lock()
            .await
            .values()
            .filter(|record| record.document_id == document_id)
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.ocr_output_id.cmp(&right.ocr_output_id));
        Ok(records)
    }

    async fn save_evidence_embedding_job(
        &self,
        input: CreateEvidenceEmbeddingJobInput,
    ) -> anyhow::Result<EvidenceEmbeddingJobRecord> {
        let record = EvidenceEmbeddingJobRecord {
            embedding_job_id: input.embedding_job_id,
            customer_scope_id: input.customer_scope_id,
            target_kind: input.target_kind,
            target_ref: input.target_ref,
            embedding_model: input.embedding_model,
            embedding_model_version: input.embedding_model_version,
            chunking_version: input.chunking_version,
            redaction_status: input.redaction_status,
            vector_store_kind: input.vector_store_kind,
            vector_store_ref: input.vector_store_ref,
            embedding_checksum: input.embedding_checksum,
            status: input.status,
            evidence_refs: input.evidence_refs,
            created_at: None,
            completed_at: None,
        };
        self.evidence_embedding_jobs
            .lock()
            .await
            .insert(record.embedding_job_id.clone(), record.clone());
        Ok(record)
    }

    async fn list_evidence_embedding_jobs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceEmbeddingJobRecord>> {
        let mut records = self
            .evidence_embedding_jobs
            .lock()
            .await
            .values()
            .filter(|record| {
                customer_scope_id.is_none_or(|scope| record.customer_scope_id == scope)
            })
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.embedding_job_id.cmp(&right.embedding_job_id));
        Ok(records)
    }

    async fn save_evidence_retrieval_audit_event(
        &self,
        input: CreateEvidenceRetrievalAuditEventInput,
    ) -> anyhow::Result<EvidenceRetrievalAuditEventRecord> {
        let record = EvidenceRetrievalAuditEventRecord {
            retrieval_id: input.retrieval_id,
            customer_scope_id: input.customer_scope_id,
            actor_id: input.actor_id,
            actor_role: input.actor_role,
            query_kind: input.query_kind,
            query_checksum: input.query_checksum,
            retrieval_method: input.retrieval_method,
            embedding_model_version: input.embedding_model_version,
            top_k: input.top_k,
            source_refs: input.source_refs,
            result_refs: input.result_refs,
            redaction_status: input.redaction_status,
            evidence_refs: input.evidence_refs,
            created_at: None,
        };
        self.evidence_retrieval_audit_events
            .lock()
            .await
            .insert(record.retrieval_id.clone(), record.clone());
        Ok(record)
    }

    async fn list_evidence_retrieval_audit_events(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceRetrievalAuditEventRecord>> {
        let mut records = self
            .evidence_retrieval_audit_events
            .lock()
            .await
            .values()
            .filter(|record| {
                customer_scope_id.is_none_or(|scope| record.customer_scope_id == scope)
            })
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.retrieval_id.cmp(&right.retrieval_id));
        Ok(records)
    }
}
