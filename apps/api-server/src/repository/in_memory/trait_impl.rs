use super::*;
use async_trait::async_trait;

// ---------------------------------------------------------------------------
// ClaimsRepository
// ---------------------------------------------------------------------------

#[async_trait]
impl ClaimsRepository for InMemoryScoringRepository {
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
}

// ---------------------------------------------------------------------------
// RoutingRepository
// ---------------------------------------------------------------------------

#[async_trait]
impl RoutingRepository for InMemoryScoringRepository {
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
}

// ---------------------------------------------------------------------------
// RulesRepository
// ---------------------------------------------------------------------------

#[async_trait]
impl RulesRepository for InMemoryScoringRepository {
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
        status_actor_id: Option<&str>,
    ) -> anyhow::Result<Option<RuleSummaryRecord>> {
        self.in_memory_update_rule_status(rule_id, status, status_actor_id)
            .await
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
}

// ---------------------------------------------------------------------------
// CasesRepository
// ---------------------------------------------------------------------------

#[async_trait]
impl CasesRepository for InMemoryScoringRepository {
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
}

// ---------------------------------------------------------------------------
// ModelsRepository
// ---------------------------------------------------------------------------

#[async_trait]
impl ModelsRepository for InMemoryScoringRepository {
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
}

// ---------------------------------------------------------------------------
// DatasetsRepository
// ---------------------------------------------------------------------------

#[async_trait]
impl DatasetsRepository for InMemoryScoringRepository {
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

    async fn save_scoring_feature_context_materialization(
        &self,
        input: SaveScoringFeatureContextMaterializationInput,
    ) -> anyhow::Result<ScoringFeatureContextMaterializationRecord> {
        self.in_memory_save_scoring_feature_context_materialization(input)
            .await
    }

    async fn get_scoring_feature_context_materialization(
        &self,
        materialization_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<ScoringFeatureContextMaterializationRecord>> {
        self.in_memory_get_scoring_feature_context_materialization(
            materialization_id,
            customer_scope_id,
        )
        .await
    }
}

// ---------------------------------------------------------------------------
// EvidenceRepository
// ---------------------------------------------------------------------------

#[async_trait]
impl EvidenceRepository for InMemoryScoringRepository {
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

// ---------------------------------------------------------------------------
// OutcomesRepository
// ---------------------------------------------------------------------------

#[async_trait]
impl OutcomesRepository for InMemoryScoringRepository {
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
}

// ---------------------------------------------------------------------------
// KnowledgeRepository
// ---------------------------------------------------------------------------

#[async_trait]
impl KnowledgeRepository for InMemoryScoringRepository {
    async fn dashboard_summary(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<DashboardSummaryRecord> {
        self.in_memory_dashboard_summary(customer_scope_id).await
    }

    async fn provider_risk_summary(&self) -> anyhow::Result<ProviderRiskSummaryRecord> {
        self.in_memory_provider_risk_summary().await
    }

    async fn save_provider_sanctions(
        &self,
        input: SaveProviderSanctionsInput,
    ) -> anyhow::Result<Vec<ProviderSanctionRecord>> {
        self.in_memory_save_provider_sanctions(input).await
    }

    async fn save_provider_profile_windows(
        &self,
        input: SaveProviderProfileWindowsInput,
    ) -> anyhow::Result<Vec<ProviderProfileWindowRecord>> {
        self.in_memory_save_provider_profile_windows(input).await
    }

    async fn save_provider_graph_signals(
        &self,
        input: SaveProviderGraphSignalsInput,
    ) -> anyhow::Result<Vec<ProviderGraphSignalRecord>> {
        self.in_memory_save_provider_graph_signals(input).await
    }

    async fn save_peer_benchmark_groups(
        &self,
        input: SavePeerBenchmarkGroupsInput,
    ) -> anyhow::Result<Vec<PeerBenchmarkGroupRecord>> {
        self.in_memory_save_peer_benchmark_groups(input).await
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

    async fn save_agent_registry(
        &self,
        record: AgentRegistryRecord,
    ) -> anyhow::Result<AgentRegistryRecord> {
        self.in_memory_save_agent_registry(record).await
    }

    async fn active_agent_registry(
        &self,
        agent_kind: &str,
        agent_version: u32,
    ) -> anyhow::Result<Option<AgentRegistryRecord>> {
        self.in_memory_active_agent_registry(agent_kind, agent_version)
            .await
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

    async fn cancel_agent_run(&self, agent_run_id: &str) -> anyhow::Result<()> {
        self.in_memory_cancel_agent_run(agent_run_id).await
    }

    async fn save_agent_approval(
        &self,
        approval: AgentApprovalRecord,
    ) -> anyhow::Result<AgentApprovalRecord> {
        self.in_memory_save_agent_approval(approval).await
    }
}
