use super::*;

mod cases;
mod dashboard;
mod evidence;
mod models;
mod rules;

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
        self.in_memory_list_rules().await
    }

    async fn list_active_rules(&self) -> anyhow::Result<Vec<Rule>> {
        self.in_memory_list_active_rules().await
    }

    async fn get_rule(&self, rule_id: &str) -> anyhow::Result<Option<RuleDetailRecord>> {
        self.in_memory_get_rule(rule_id).await
    }

    async fn rule_audit_history(
        &self,
        rule_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        self.in_memory_rule_audit_history(rule_id).await
    }

    async fn save_rule_candidate(
        &self,
        rule: Rule,
        owner: String,
    ) -> anyhow::Result<RuleDetailRecord> {
        self.in_memory_save_rule_candidate(rule, owner).await
    }

    async fn update_rule_status(
        &self,
        rule_id: &str,
        status: &str,
    ) -> anyhow::Result<Option<RuleSummaryRecord>> {
        self.in_memory_update_rule_status(rule_id, status).await
    }

    async fn list_rule_conditions(&self) -> anyhow::Result<Vec<RuleConditionLibraryRecord>> {
        self.in_memory_list_rule_conditions().await
    }

    async fn rule_performance(&self) -> anyhow::Result<Vec<RulePerformanceRecord>> {
        self.in_memory_rule_performance().await
    }

    async fn save_rule_backtest(
        &self,
        record: RuleBacktestRecord,
    ) -> anyhow::Result<RuleBacktestRecord> {
        self.in_memory_save_rule_backtest(record).await
    }

    async fn latest_rule_backtest(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleBacktestRecord>> {
        self.in_memory_latest_rule_backtest(rule_id, rule_version)
            .await
    }

    async fn save_rule_shadow_run(
        &self,
        record: RuleShadowRunRecord,
    ) -> anyhow::Result<RuleShadowRunRecord> {
        self.in_memory_save_rule_shadow_run(record).await
    }

    async fn latest_rule_shadow_run(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleShadowRunRecord>> {
        self.in_memory_latest_rule_shadow_run(rule_id, rule_version)
            .await
    }

    async fn save_rule_promotion_review(
        &self,
        record: RulePromotionReviewRecord,
    ) -> anyhow::Result<RulePromotionReviewRecord> {
        self.in_memory_save_rule_promotion_review(record).await
    }

    async fn latest_rule_promotion_review(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RulePromotionReviewRecord>> {
        self.in_memory_latest_rule_promotion_review(rule_id, rule_version)
            .await
    }

    async fn list_leads(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<LeadRecord>> {
        self.in_memory_list_leads(customer_scope_id).await
    }

    async fn triage_lead(
        &self,
        lead_id: &str,
        input: TriageLeadInput,
    ) -> anyhow::Result<Option<TriageLeadRecord>> {
        self.in_memory_triage_lead(lead_id, input).await
    }

    async fn list_cases(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<CaseRecord>> {
        self.in_memory_list_cases(customer_scope_id).await
    }

    async fn update_case_status(
        &self,
        case_id: &str,
        input: UpdateCaseStatusInput,
    ) -> anyhow::Result<Option<UpdateCaseStatusRecord>> {
        self.in_memory_update_case_status(case_id, input).await
    }

    async fn create_audit_sample(
        &self,
        input: CreateAuditSampleInput,
    ) -> anyhow::Result<AuditSampleRecord> {
        self.in_memory_create_audit_sample(input).await
    }

    async fn list_audit_samples(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditSampleRecord>> {
        self.in_memory_list_audit_samples(customer_scope_id).await
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelVersionRecord>> {
        self.in_memory_list_models().await
    }

    async fn save_model_version(
        &self,
        record: ModelVersionRecord,
    ) -> anyhow::Result<ModelVersionRecord> {
        self.in_memory_save_model_version(record).await
    }

    async fn update_model_status(
        &self,
        model_key: &str,
        model_version: &str,
        status: &str,
    ) -> anyhow::Result<Option<ModelVersionRecord>> {
        self.in_memory_update_model_status(model_key, model_version, status)
            .await
    }

    async fn model_performance(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Option<ModelPerformanceRecord>> {
        self.in_memory_model_performance(model_key).await
    }

    async fn save_model_promotion_review(
        &self,
        record: ModelPromotionReviewRecord,
    ) -> anyhow::Result<ModelPromotionReviewRecord> {
        self.in_memory_save_model_promotion_review(record).await
    }

    async fn latest_model_promotion_review(
        &self,
        model_key: &str,
        model_version: &str,
    ) -> anyhow::Result<Option<ModelPromotionReviewRecord>> {
        self.in_memory_latest_model_promotion_review(model_key, model_version)
            .await
    }

    async fn save_model_retraining_job(
        &self,
        record: ModelRetrainingJobRecord,
    ) -> anyhow::Result<ModelRetrainingJobRecord> {
        self.in_memory_save_model_retraining_job(record).await
    }

    async fn list_model_retraining_jobs(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Vec<ModelRetrainingJobRecord>> {
        self.in_memory_list_model_retraining_jobs(model_key).await
    }

    async fn get_model_retraining_job(
        &self,
        job_id: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        self.in_memory_get_model_retraining_job(job_id).await
    }

    async fn claim_next_model_retraining_job(
        &self,
        model_key: Option<&str>,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        self.in_memory_claim_next_model_retraining_job(model_key, actor, status_note)
            .await
    }

    async fn update_model_retraining_job_status(
        &self,
        job_id: &str,
        status: &str,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        self.in_memory_update_model_retraining_job_status(job_id, status, actor, status_note)
            .await
    }

    async fn complete_model_retraining_job(
        &self,
        input: CompleteModelRetrainingJobInput<'_>,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        self.in_memory_complete_model_retraining_job(input).await
    }

    async fn dashboard_summary(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<DashboardSummaryRecord> {
        self.in_memory_dashboard_summary(customer_scope_id).await
    }

    async fn provider_risk_summary(&self) -> anyhow::Result<ProviderRiskSummaryRecord> {
        self.in_memory_provider_risk_summary().await
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
        self.in_memory_save_evidence_document(input).await
    }

    async fn list_evidence_documents(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentRecord>> {
        self.in_memory_list_evidence_documents(customer_scope_id)
            .await
    }

    async fn get_evidence_document(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentRecord>> {
        self.in_memory_get_evidence_document(document_id, customer_scope_id)
            .await
    }

    async fn save_evidence_document_chunk(
        &self,
        input: CreateEvidenceDocumentChunkInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentChunkRecord>> {
        self.in_memory_save_evidence_document_chunk(input, customer_scope_id)
            .await
    }

    async fn list_evidence_document_chunks(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentChunkRecord>> {
        self.in_memory_list_evidence_document_chunks(document_id, customer_scope_id)
            .await
    }

    async fn save_evidence_ocr_output(
        &self,
        input: CreateEvidenceOcrOutputInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceOcrOutputRecord>> {
        self.in_memory_save_evidence_ocr_output(input, customer_scope_id)
            .await
    }

    async fn list_evidence_ocr_outputs(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceOcrOutputRecord>> {
        self.in_memory_list_evidence_ocr_outputs(document_id, customer_scope_id)
            .await
    }

    async fn save_evidence_embedding_job(
        &self,
        input: CreateEvidenceEmbeddingJobInput,
    ) -> anyhow::Result<EvidenceEmbeddingJobRecord> {
        self.in_memory_save_evidence_embedding_job(input).await
    }

    async fn list_evidence_embedding_jobs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceEmbeddingJobRecord>> {
        self.in_memory_list_evidence_embedding_jobs(customer_scope_id)
            .await
    }

    async fn save_evidence_retrieval_audit_event(
        &self,
        input: CreateEvidenceRetrievalAuditEventInput,
    ) -> anyhow::Result<EvidenceRetrievalAuditEventRecord> {
        self.in_memory_save_evidence_retrieval_audit_event(input)
            .await
    }

    async fn list_evidence_retrieval_audit_events(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceRetrievalAuditEventRecord>> {
        self.in_memory_list_evidence_retrieval_audit_events(customer_scope_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_audit_events_mask_pii_payload_fields() {
        let repository = InMemoryScoringRepository::default();

        repository
            .save_audit_event(PersistedAuditEvent {
                audit_id: "audit-1".into(),
                run_id: "run-1".into(),
                claim_id: "claim-1".into(),
                source_system: "tpa-demo".into(),
                actor_id: "actor-1".into(),
                actor_role: "tpa_system".into(),
                event_type: "scoring.completed".into(),
                event_status: "succeeded".into(),
                summary: "summary".into(),
                payload: serde_json::json!({
                    "external_member_id": "MBR-12345",
                    "dob": "1988-03-12",
                    "gender": "F",
                    "risk_score": 72
                }),
                evidence_refs: vec![],
            })
            .await
            .unwrap();

        let audit_events = repository.audit_events.lock().await;
        let payload = &audit_events[0].payload;
        assert_ne!(payload["external_member_id"], "MBR-12345");
        assert_eq!(payload["dob"], "1988-XX-XX");
        assert_eq!(payload["gender"], "MASKED");
        assert_eq!(payload["risk_score"], 72);
    }
}
