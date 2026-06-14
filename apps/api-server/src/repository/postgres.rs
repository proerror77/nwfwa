use super::*;
use std::time::Duration;

const DEFAULT_DB_MAX_CONNECTIONS: u32 = 20;
const DEFAULT_DB_ACQUIRE_TIMEOUT_SECONDS: u64 = 5;

#[derive(Debug, Clone)]
pub struct PostgresScoringRepository {
    pub(super) pool: PgPool,
}

fn configured_db_max_connections() -> u32 {
    positive_u32_env("FWA_DB_MAX_CONNECTIONS", DEFAULT_DB_MAX_CONNECTIONS)
}

fn configured_db_acquire_timeout_seconds() -> u64 {
    positive_u64_env(
        "FWA_DB_ACQUIRE_TIMEOUT_SECONDS",
        DEFAULT_DB_ACQUIRE_TIMEOUT_SECONDS,
    )
}

fn positive_u32_env(name: &str, default: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn positive_u64_env(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

impl PostgresScoringRepository {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(configured_db_max_connections())
            .acquire_timeout(Duration::from_secs(configured_db_acquire_timeout_seconds()))
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_u32_env_accepts_positive_values() {
        let name = "NWFWA_TEST_POSITIVE_U32_ENV";
        std::env::set_var(name, "32");

        assert_eq!(positive_u32_env(name, 20), 32);

        std::env::remove_var(name);
    }

    #[test]
    fn positive_env_helpers_fall_back_for_invalid_values() {
        let u32_name = "NWFWA_TEST_INVALID_U32_ENV";
        let u64_name = "NWFWA_TEST_INVALID_U64_ENV";
        std::env::set_var(u32_name, "0");
        std::env::set_var(u64_name, "not-a-number");

        assert_eq!(positive_u32_env(u32_name, 20), 20);
        assert_eq!(positive_u64_env(u64_name, 5), 5);

        std::env::remove_var(u32_name);
        std::env::remove_var(u64_name);
    }
}

#[async_trait]
impl ClaimsRepository for PostgresScoringRepository {
    async fn upsert_claim_context(
        &self,
        context: ClaimContext,
        raw_payload: Value,
    ) -> anyhow::Result<()> {
        postgres_claims::upsert_claim_context(self, context, raw_payload).await
    }

    async fn load_claim_context(
        &self,
        external_claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<ClaimContext>> {
        postgres_claims::load_claim_context(self, external_claim_id, customer_scope_id).await
    }

    async fn member_profile_summary(
        &self,
        member_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<MemberProfileSummaryRecord>> {
        postgres_claims::member_profile_summary(self, member_id, customer_scope_id).await
    }

    async fn save_scoring_run(&self, run: PersistedScoringRun) -> anyhow::Result<()> {
        postgres_scoring::save_scoring_run(self, run).await
    }

    async fn save_audit_event(&self, event: PersistedAuditEvent) -> anyhow::Result<()> {
        postgres_scoring::save_audit_event(self, event).await
    }

    async fn save_inbox_claim_run(&self, run: PersistedInboxClaimRun) -> anyhow::Result<()> {
        postgres_inbox::save_inbox_claim_run(self, run).await
    }

    async fn get_inbox_claim_run_by_idempotency_key(
        &self,
        idempotency_key: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
        postgres_inbox::get_inbox_claim_run_by_idempotency_key(
            self,
            idempotency_key,
            customer_scope_id,
        )
        .await
    }

    async fn get_inbox_claim_run_by_run_id(
        &self,
        run_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
        postgres_inbox::get_inbox_claim_run_by_run_id(self, run_id, customer_scope_id).await
    }
}

#[async_trait]
impl RoutingRepository for PostgresScoringRepository {
    async fn active_routing_policy(
        &self,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicy>> {
        postgres_routing_policies::active_routing_policy(self, review_mode).await
    }

    async fn list_routing_policies(&self) -> anyhow::Result<Vec<RoutingPolicyRecord>> {
        postgres_routing_policies::list_routing_policies(self).await
    }

    async fn save_routing_policy_candidate(
        &self,
        policy: RoutingPolicy,
        owner: String,
    ) -> anyhow::Result<RoutingPolicyRecord> {
        postgres_routing_policies::save_routing_policy_candidate(self, policy, owner).await
    }

    async fn get_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        postgres_routing_policies::get_routing_policy(self, policy_id, version, review_mode).await
    }

    async fn update_routing_policy_status(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
        status: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        postgres_routing_policies::update_routing_policy_status(
            self,
            policy_id,
            version,
            review_mode,
            status,
        )
        .await
    }

    async fn activate_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        postgres_routing_policies::activate_routing_policy(self, policy_id, version, review_mode)
            .await
    }
}

#[async_trait]
impl RulesRepository for PostgresScoringRepository {
    async fn list_rules(&self) -> anyhow::Result<Vec<RuleSummaryRecord>> {
        postgres_rules::list_rules(self).await
    }

    async fn list_active_rules(&self) -> anyhow::Result<Vec<Rule>> {
        postgres_rules::list_active_rules(self).await
    }

    async fn get_rule(&self, rule_id: &str) -> anyhow::Result<Option<RuleDetailRecord>> {
        postgres_rules::get_rule(self, rule_id).await
    }

    async fn rule_audit_history(
        &self,
        rule_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        postgres_rules::rule_audit_history(self, rule_id).await
    }

    async fn save_rule_candidate(
        &self,
        rule: Rule,
        owner: String,
    ) -> anyhow::Result<RuleDetailRecord> {
        postgres_rules::save_rule_candidate(self, rule, owner).await
    }

    async fn update_rule_status(
        &self,
        rule_id: &str,
        status: &str,
        status_actor_id: Option<&str>,
    ) -> anyhow::Result<Option<RuleSummaryRecord>> {
        postgres_rules::update_rule_status(self, rule_id, status, status_actor_id).await
    }

    async fn list_rule_conditions(&self) -> anyhow::Result<Vec<RuleConditionLibraryRecord>> {
        postgres_rules::list_rule_conditions(self).await
    }

    async fn rule_performance(&self) -> anyhow::Result<Vec<RulePerformanceRecord>> {
        postgres_rules::rule_performance(self).await
    }

    async fn save_rule_backtest(
        &self,
        record: RuleBacktestRecord,
    ) -> anyhow::Result<RuleBacktestRecord> {
        postgres_rule_reviews::save_rule_backtest(self, record).await
    }

    async fn latest_rule_backtest(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleBacktestRecord>> {
        postgres_rule_reviews::latest_rule_backtest(self, rule_id, rule_version).await
    }

    async fn save_rule_shadow_run(
        &self,
        record: RuleShadowRunRecord,
    ) -> anyhow::Result<RuleShadowRunRecord> {
        postgres_rule_reviews::save_rule_shadow_run(self, record).await
    }

    async fn latest_rule_shadow_run(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleShadowRunRecord>> {
        postgres_rule_reviews::latest_rule_shadow_run(self, rule_id, rule_version).await
    }

    async fn save_rule_promotion_review(
        &self,
        record: RulePromotionReviewRecord,
    ) -> anyhow::Result<RulePromotionReviewRecord> {
        postgres_rule_reviews::save_rule_promotion_review(self, record).await
    }

    async fn latest_rule_promotion_review(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RulePromotionReviewRecord>> {
        postgres_rule_reviews::latest_rule_promotion_review(self, rule_id, rule_version).await
    }
}

#[async_trait]
impl CasesRepository for PostgresScoringRepository {
    async fn list_leads(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<LeadRecord>> {
        postgres_cases::list_leads(self, customer_scope_id).await
    }

    async fn triage_lead(
        &self,
        lead_id: &str,
        input: TriageLeadInput,
    ) -> anyhow::Result<Option<TriageLeadRecord>> {
        postgres_cases::triage_lead(self, lead_id, input).await
    }

    async fn list_cases(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<CaseRecord>> {
        postgres_cases::list_cases(self, customer_scope_id).await
    }

    async fn update_case_status(
        &self,
        case_id: &str,
        input: UpdateCaseStatusInput,
    ) -> anyhow::Result<Option<UpdateCaseStatusRecord>> {
        postgres_cases::update_case_status(self, case_id, input).await
    }

    async fn create_audit_sample(
        &self,
        input: CreateAuditSampleInput,
    ) -> anyhow::Result<AuditSampleRecord> {
        postgres_audit_samples::create_audit_sample(self, input).await
    }

    async fn list_audit_samples(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditSampleRecord>> {
        postgres_audit_samples::list_audit_samples(self, customer_scope_id).await
    }
}

#[async_trait]
impl ModelsRepository for PostgresScoringRepository {
    async fn list_models(&self) -> anyhow::Result<Vec<ModelVersionRecord>> {
        postgres_models::list_models(self).await
    }

    async fn save_model_version(
        &self,
        record: ModelVersionRecord,
    ) -> anyhow::Result<ModelVersionRecord> {
        postgres_models::save_model_version(self, record).await
    }

    async fn update_model_status(
        &self,
        model_key: &str,
        model_version: &str,
        status: &str,
    ) -> anyhow::Result<Option<ModelVersionRecord>> {
        postgres_models::update_model_status(self, model_key, model_version, status).await
    }

    async fn model_performance(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Option<ModelPerformanceRecord>> {
        postgres_models::model_performance(self, model_key).await
    }

    async fn save_model_promotion_review(
        &self,
        record: ModelPromotionReviewRecord,
    ) -> anyhow::Result<ModelPromotionReviewRecord> {
        postgres_models::save_model_promotion_review(self, record).await
    }

    async fn save_probability_calibration_report(
        &self,
        record: ProbabilityCalibrationReportRecord,
    ) -> anyhow::Result<ProbabilityCalibrationReportRecord> {
        postgres_models::save_probability_calibration_report(self, record).await
    }

    async fn latest_model_promotion_review(
        &self,
        model_key: &str,
        model_version: &str,
    ) -> anyhow::Result<Option<ModelPromotionReviewRecord>> {
        postgres_models::latest_model_promotion_review(self, model_key, model_version).await
    }

    async fn save_model_retraining_job(
        &self,
        record: ModelRetrainingJobRecord,
    ) -> anyhow::Result<ModelRetrainingJobRecord> {
        postgres_models::save_model_retraining_job(self, record).await
    }

    async fn list_model_retraining_jobs(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Vec<ModelRetrainingJobRecord>> {
        postgres_models::list_model_retraining_jobs(self, model_key).await
    }

    async fn get_model_retraining_job(
        &self,
        job_id: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        postgres_models::get_model_retraining_job(self, job_id).await
    }

    async fn claim_next_model_retraining_job(
        &self,
        model_key: Option<&str>,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        postgres_models::claim_next_model_retraining_job(self, model_key, actor, status_note).await
    }

    async fn update_model_retraining_job_status(
        &self,
        job_id: &str,
        status: &str,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        postgres_models::update_model_retraining_job_status(
            self,
            job_id,
            status,
            actor,
            status_note,
        )
        .await
    }

    async fn complete_model_retraining_job(
        &self,
        input: CompleteModelRetrainingJobInput<'_>,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        postgres_models::complete_model_retraining_job(self, input).await
    }
}

#[async_trait]
impl DatasetsRepository for PostgresScoringRepository {
    async fn register_dataset(&self, input: RegisterDatasetInput) -> anyhow::Result<DatasetRecord> {
        postgres_datasets::register_dataset(self, input).await
    }

    async fn list_datasets(&self) -> anyhow::Result<Vec<DatasetRecord>> {
        postgres_datasets::list_datasets(self).await
    }

    async fn get_dataset(&self, dataset_id: &str) -> anyhow::Result<Option<DatasetRecord>> {
        postgres_datasets::get_dataset(self, dataset_id).await
    }

    async fn add_field_mapping(
        &self,
        dataset_id: &str,
        input: CreateFieldMappingInput,
    ) -> anyhow::Result<Option<FieldMappingRecord>> {
        postgres_datasets::add_field_mapping(self, dataset_id, input).await
    }

    async fn register_feature_set(
        &self,
        input: RegisterFeatureSetInput,
    ) -> anyhow::Result<Option<FeatureSetRecord>> {
        postgres_datasets::register_feature_set(self, input).await
    }

    async fn register_model_dataset(
        &self,
        input: RegisterModelDatasetInput,
    ) -> anyhow::Result<Option<ModelDatasetRecord>> {
        postgres_datasets::register_model_dataset(self, input).await
    }

    async fn get_model_dataset_source_dataset(
        &self,
        model_dataset_id: &str,
    ) -> anyhow::Result<Option<DatasetRecord>> {
        postgres_datasets::get_model_dataset_source_dataset(self, model_dataset_id).await
    }

    async fn register_model_evaluation(
        &self,
        input: RegisterModelEvaluationInput,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        postgres_datasets::register_model_evaluation(self, input).await
    }

    async fn get_model_evaluation(
        &self,
        evaluation_run_id: &str,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        postgres_datasets::get_model_evaluation(self, evaluation_run_id).await
    }

    async fn list_model_evaluations(&self) -> anyhow::Result<Vec<ModelEvaluationRecord>> {
        postgres_datasets::list_model_evaluations(self).await
    }

    async fn save_scoring_feature_context_materialization(
        &self,
        input: SaveScoringFeatureContextMaterializationInput,
    ) -> anyhow::Result<ScoringFeatureContextMaterializationRecord> {
        postgres_datasets::save_scoring_feature_context_materialization(self, input).await
    }

    async fn get_scoring_feature_context_materialization(
        &self,
        materialization_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<ScoringFeatureContextMaterializationRecord>> {
        postgres_datasets::get_scoring_feature_context_materialization(
            self,
            materialization_id,
            customer_scope_id,
        )
        .await
    }

    async fn save_clinical_compatibility_references(
        &self,
        input: SaveClinicalCompatibilityReferencesInput,
    ) -> anyhow::Result<Vec<ClinicalCompatibilityReferenceRecord>> {
        postgres_datasets::save_clinical_compatibility_references(self, input).await
    }

    async fn save_unbundling_comparator_candidates(
        &self,
        input: SaveUnbundlingComparatorCandidatesInput,
    ) -> anyhow::Result<Vec<UnbundlingComparatorCandidateRecord>> {
        postgres_datasets::save_unbundling_comparator_candidates(self, input).await
    }
}

#[async_trait]
impl EvidenceRepository for PostgresScoringRepository {
    async fn save_evidence_document(
        &self,
        input: CreateEvidenceDocumentInput,
    ) -> anyhow::Result<EvidenceDocumentRecord> {
        postgres_evidence::save_evidence_document(self, input).await
    }

    async fn list_evidence_documents(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentRecord>> {
        postgres_evidence::list_evidence_documents(self, customer_scope_id).await
    }

    async fn get_evidence_document(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentRecord>> {
        postgres_evidence::get_evidence_document(self, document_id, customer_scope_id).await
    }

    async fn save_evidence_document_chunk(
        &self,
        input: CreateEvidenceDocumentChunkInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentChunkRecord>> {
        postgres_evidence::save_evidence_document_chunk(self, input, customer_scope_id).await
    }

    async fn list_evidence_document_chunks(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentChunkRecord>> {
        postgres_evidence::list_evidence_document_chunks(self, document_id, customer_scope_id).await
    }

    async fn save_evidence_ocr_output(
        &self,
        input: CreateEvidenceOcrOutputInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceOcrOutputRecord>> {
        postgres_evidence::save_evidence_ocr_output(self, input, customer_scope_id).await
    }

    async fn list_evidence_ocr_outputs(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceOcrOutputRecord>> {
        postgres_evidence::list_evidence_ocr_outputs(self, document_id, customer_scope_id).await
    }

    async fn save_evidence_embedding_job(
        &self,
        input: CreateEvidenceEmbeddingJobInput,
    ) -> anyhow::Result<EvidenceEmbeddingJobRecord> {
        postgres_evidence::save_evidence_embedding_job(self, input).await
    }

    async fn list_evidence_embedding_jobs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceEmbeddingJobRecord>> {
        postgres_evidence::list_evidence_embedding_jobs(self, customer_scope_id).await
    }

    async fn save_evidence_retrieval_audit_event(
        &self,
        input: CreateEvidenceRetrievalAuditEventInput,
    ) -> anyhow::Result<EvidenceRetrievalAuditEventRecord> {
        postgres_evidence::save_evidence_retrieval_audit_event(self, input).await
    }

    async fn list_evidence_retrieval_audit_events(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceRetrievalAuditEventRecord>> {
        postgres_evidence::list_evidence_retrieval_audit_events(self, customer_scope_id).await
    }
}

#[async_trait]
impl OutcomesRepository for PostgresScoringRepository {
    async fn save_investigation_result(
        &self,
        record: InvestigationResultRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        postgres_outcomes::save_investigation_result(self, record).await
    }

    async fn save_qa_review(
        &self,
        record: QaReviewRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        postgres_qa::save_qa_review(self, record).await
    }

    async fn list_qa_feedback_items(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaFeedbackItemRecord>> {
        postgres_qa::list_qa_feedback_items(self, customer_scope_id).await
    }

    async fn update_qa_feedback_status(
        &self,
        feedback_id: &str,
        input: UpdateQaFeedbackStatusInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<UpdateQaFeedbackStatusRecord>> {
        postgres_qa::update_qa_feedback_status(self, feedback_id, input, customer_scope_id).await
    }

    async fn list_qa_reviews(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaReviewRecord>> {
        postgres_qa::list_qa_reviews(self, customer_scope_id).await
    }

    async fn list_outcome_labels(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<OutcomeLabelRecord>> {
        postgres_outcomes::list_outcome_labels(self, customer_scope_id).await
    }

    async fn claim_audit_history(
        &self,
        claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        postgres_audit::claim_audit_history(self, claim_id, customer_scope_id).await
    }

    async fn list_audit_events(
        &self,
        filter: AuditEventListFilter,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        postgres_audit::list_audit_events(self, filter).await
    }

    async fn list_webhook_events(&self) -> anyhow::Result<Vec<WebhookEventRecord>> {
        postgres_webhooks::list_webhook_events(self).await
    }

    async fn save_webhook_delivery_attempt(
        &self,
        input: WebhookDeliveryAttemptInput,
    ) -> anyhow::Result<WebhookDeliveryAttemptRecord> {
        postgres_webhooks::save_webhook_delivery_attempt(self, input).await
    }
}

#[async_trait]
impl KnowledgeRepository for PostgresScoringRepository {
    async fn dashboard_summary(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<DashboardSummaryRecord> {
        postgres_dashboard::dashboard_summary(self, customer_scope_id).await
    }

    async fn provider_risk_summary(&self) -> anyhow::Result<ProviderRiskSummaryRecord> {
        postgres_providers::provider_risk_summary(self).await
    }

    async fn save_provider_sanctions(
        &self,
        input: SaveProviderSanctionsInput,
    ) -> anyhow::Result<Vec<ProviderSanctionRecord>> {
        postgres_providers::save_provider_sanctions(self, input).await
    }

    async fn save_provider_profile_windows(
        &self,
        input: SaveProviderProfileWindowsInput,
    ) -> anyhow::Result<Vec<ProviderProfileWindowRecord>> {
        postgres_providers::save_provider_profile_windows(self, input).await
    }

    async fn save_provider_graph_signals(
        &self,
        input: SaveProviderGraphSignalsInput,
    ) -> anyhow::Result<Vec<ProviderGraphSignalRecord>> {
        postgres_providers::save_provider_graph_signals(self, input).await
    }

    async fn save_peer_benchmark_groups(
        &self,
        input: SavePeerBenchmarkGroupsInput,
    ) -> anyhow::Result<Vec<PeerBenchmarkGroupRecord>> {
        postgres_providers::save_peer_benchmark_groups(self, input).await
    }

    async fn save_episode_rollups(
        &self,
        input: SaveEpisodeRollupsInput,
    ) -> anyhow::Result<Vec<EpisodeRollupRecord>> {
        postgres_providers::save_episode_rollups(self, input).await
    }

    async fn list_knowledge_cases(&self) -> anyhow::Result<Vec<KnowledgeCaseRecord>> {
        postgres_knowledge::list_knowledge_cases(self).await
    }

    async fn save_knowledge_case(
        &self,
        record: KnowledgeCaseRecord,
    ) -> anyhow::Result<KnowledgeCaseRecord> {
        postgres_knowledge::save_knowledge_case(self, record).await
    }

    async fn search_similar_cases(
        &self,
        query: SimilarCaseQuery,
    ) -> anyhow::Result<Vec<SimilarCaseRecord>> {
        postgres_knowledge::search_similar_cases(self, query).await
    }

    async fn save_agent_registry(
        &self,
        record: AgentRegistryRecord,
    ) -> anyhow::Result<AgentRegistryRecord> {
        postgres_agents::save_agent_registry(self, record).await
    }

    async fn active_agent_registry(
        &self,
        agent_kind: &str,
        agent_version: u32,
    ) -> anyhow::Result<Option<AgentRegistryRecord>> {
        postgres_agents::active_agent_registry(self, agent_kind, agent_version).await
    }

    async fn save_agent_run(&self, run: PersistedAgentRun) -> anyhow::Result<()> {
        postgres_agents::save_agent_run(self, run).await
    }

    async fn list_agent_runs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AgentRunLogRecord>> {
        postgres_agents::list_agent_runs(self, customer_scope_id).await
    }

    async fn cancel_agent_run(&self, agent_run_id: &str) -> anyhow::Result<()> {
        postgres_agents::cancel_agent_run(self, agent_run_id).await
    }

    async fn save_agent_approval(
        &self,
        approval: AgentApprovalRecord,
    ) -> anyhow::Result<AgentApprovalRecord> {
        postgres_agents::save_agent_approval(self, approval).await
    }
}
