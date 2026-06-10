use super::*;

mod audit;
mod cases;
mod claims;
mod dashboard;
mod datasets;
mod evidence;
mod knowledge_agents;
mod models;
mod outcomes;
mod routing;
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
}

#[async_trait]
impl ScoringRepository for InMemoryScoringRepository {
    async fn upsert_claim_context(
        &self,
        context: ClaimContext,
        _raw_payload: Value,
    ) -> anyhow::Result<()> {
        self.in_memory_upsert_claim_context(context).await
    }

    async fn load_claim_context(
        &self,
        external_claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<ClaimContext>> {
        self.in_memory_load_claim_context(external_claim_id, customer_scope_id)
            .await
    }

    async fn member_profile_summary(
        &self,
        member_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<MemberProfileSummaryRecord>> {
        self.in_memory_member_profile_summary(member_id, customer_scope_id)
            .await
    }

    async fn save_scoring_run(&self, run: PersistedScoringRun) -> anyhow::Result<()> {
        self.in_memory_save_scoring_run(run).await
    }

    async fn save_audit_event(&self, event: PersistedAuditEvent) -> anyhow::Result<()> {
        self.in_memory_save_audit_event(event).await
    }

    async fn save_inbox_claim_run(&self, run: PersistedInboxClaimRun) -> anyhow::Result<()> {
        self.in_memory_save_inbox_claim_run(run).await
    }

    async fn get_inbox_claim_run_by_idempotency_key(
        &self,
        idempotency_key: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
        self.in_memory_get_inbox_claim_run_by_idempotency_key(idempotency_key, customer_scope_id)
            .await
    }

    async fn get_inbox_claim_run_by_run_id(
        &self,
        run_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
        self.in_memory_get_inbox_claim_run_by_run_id(run_id, customer_scope_id)
            .await
    }

    async fn active_routing_policy(
        &self,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicy>> {
        self.in_memory_active_routing_policy(review_mode).await
    }

    async fn list_routing_policies(&self) -> anyhow::Result<Vec<RoutingPolicyRecord>> {
        self.in_memory_list_routing_policies().await
    }

    async fn save_routing_policy_candidate(
        &self,
        policy: RoutingPolicy,
        owner: String,
    ) -> anyhow::Result<RoutingPolicyRecord> {
        self.in_memory_save_routing_policy_candidate(policy, owner)
            .await
    }

    async fn get_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        self.in_memory_get_routing_policy(policy_id, version, review_mode)
            .await
    }

    async fn update_routing_policy_status(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
        status: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        self.in_memory_update_routing_policy_status(policy_id, version, review_mode, status)
            .await
    }

    async fn activate_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        self.in_memory_activate_routing_policy(policy_id, version, review_mode)
            .await
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
        self.in_memory_list_knowledge_cases().await
    }

    async fn save_knowledge_case(
        &self,
        record: KnowledgeCaseRecord,
    ) -> anyhow::Result<KnowledgeCaseRecord> {
        self.in_memory_save_knowledge_case(record).await
    }

    async fn search_similar_cases(
        &self,
        query: SimilarCaseQuery,
    ) -> anyhow::Result<Vec<SimilarCaseRecord>> {
        self.in_memory_search_similar_cases(query).await
    }

    async fn save_agent_run(&self, run: PersistedAgentRun) -> anyhow::Result<()> {
        self.in_memory_save_agent_run(run).await
    }

    async fn list_agent_runs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AgentRunLogRecord>> {
        self.in_memory_list_agent_runs(customer_scope_id).await
    }

    async fn save_agent_approval(
        &self,
        approval: AgentApprovalRecord,
    ) -> anyhow::Result<AgentApprovalRecord> {
        self.in_memory_save_agent_approval(approval).await
    }

    async fn register_dataset(&self, input: RegisterDatasetInput) -> anyhow::Result<DatasetRecord> {
        self.in_memory_register_dataset(input).await
    }

    async fn list_datasets(&self) -> anyhow::Result<Vec<DatasetRecord>> {
        self.in_memory_list_datasets().await
    }

    async fn get_dataset(&self, dataset_id: &str) -> anyhow::Result<Option<DatasetRecord>> {
        self.in_memory_get_dataset(dataset_id).await
    }

    async fn add_field_mapping(
        &self,
        dataset_id: &str,
        input: CreateFieldMappingInput,
    ) -> anyhow::Result<Option<FieldMappingRecord>> {
        self.in_memory_add_field_mapping(dataset_id, input).await
    }

    async fn save_investigation_result(
        &self,
        record: InvestigationResultRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        self.in_memory_save_investigation_result(record).await
    }

    async fn save_qa_review(
        &self,
        record: QaReviewRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        self.in_memory_save_qa_review(record).await
    }

    async fn list_qa_feedback_items(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaFeedbackItemRecord>> {
        self.in_memory_list_qa_feedback_items(customer_scope_id)
            .await
    }

    async fn update_qa_feedback_status(
        &self,
        feedback_id: &str,
        input: UpdateQaFeedbackStatusInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<UpdateQaFeedbackStatusRecord>> {
        self.in_memory_update_qa_feedback_status(feedback_id, input, customer_scope_id)
            .await
    }

    async fn list_qa_reviews(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaReviewRecord>> {
        self.in_memory_list_qa_reviews(customer_scope_id).await
    }

    async fn list_outcome_labels(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<OutcomeLabelRecord>> {
        self.in_memory_list_outcome_labels(customer_scope_id).await
    }

    async fn claim_audit_history(
        &self,
        claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        self.in_memory_claim_audit_history(claim_id, customer_scope_id)
            .await
    }

    async fn list_audit_events(
        &self,
        filter: AuditEventListFilter,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        self.in_memory_list_audit_events(filter).await
    }

    async fn list_webhook_events(&self) -> anyhow::Result<Vec<WebhookEventRecord>> {
        self.in_memory_list_webhook_events().await
    }

    async fn save_webhook_delivery_attempt(
        &self,
        input: WebhookDeliveryAttemptInput,
    ) -> anyhow::Result<WebhookDeliveryAttemptRecord> {
        self.in_memory_save_webhook_delivery_attempt(input).await
    }

    async fn register_feature_set(
        &self,
        input: RegisterFeatureSetInput,
    ) -> anyhow::Result<Option<FeatureSetRecord>> {
        self.in_memory_register_feature_set(input).await
    }

    async fn register_model_dataset(
        &self,
        input: RegisterModelDatasetInput,
    ) -> anyhow::Result<Option<ModelDatasetRecord>> {
        self.in_memory_register_model_dataset(input).await
    }

    async fn get_model_dataset_source_dataset(
        &self,
        model_dataset_id: &str,
    ) -> anyhow::Result<Option<DatasetRecord>> {
        self.in_memory_get_model_dataset_source_dataset(model_dataset_id)
            .await
    }

    async fn register_model_evaluation(
        &self,
        input: RegisterModelEvaluationInput,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        self.in_memory_register_model_evaluation(input).await
    }

    async fn get_model_evaluation(
        &self,
        evaluation_run_id: &str,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        self.in_memory_get_model_evaluation(evaluation_run_id).await
    }

    async fn list_model_evaluations(&self) -> anyhow::Result<Vec<ModelEvaluationRecord>> {
        self.in_memory_list_model_evaluations().await
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
