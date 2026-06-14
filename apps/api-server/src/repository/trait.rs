use super::types::*;
use async_trait::async_trait;
use fwa_core::ClaimContext;
use fwa_rules::Rule;
use fwa_scoring::RoutingPolicy;
use serde_json::Value;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// ClaimsRepository
// ---------------------------------------------------------------------------

#[async_trait]
pub trait ClaimsRepository: Send + Sync {
    async fn upsert_claim_context(
        &self,
        context: ClaimContext,
        raw_payload: Value,
    ) -> anyhow::Result<()>;

    async fn load_claim_context(
        &self,
        external_claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<ClaimContext>>;

    async fn member_profile_summary(
        &self,
        member_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<MemberProfileSummaryRecord>>;

    async fn save_scoring_run(&self, run: PersistedScoringRun) -> anyhow::Result<()>;

    async fn save_audit_event(&self, event: PersistedAuditEvent) -> anyhow::Result<()>;

    async fn save_inbox_claim_run(&self, run: PersistedInboxClaimRun) -> anyhow::Result<()>;

    async fn get_inbox_claim_run_by_idempotency_key(
        &self,
        idempotency_key: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>>;

    async fn get_inbox_claim_run_by_run_id(
        &self,
        run_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>>;
}

// ---------------------------------------------------------------------------
// RoutingRepository
// ---------------------------------------------------------------------------

#[async_trait]
pub trait RoutingRepository: Send + Sync {
    async fn active_routing_policy(
        &self,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicy>>;

    async fn list_routing_policies(&self) -> anyhow::Result<Vec<RoutingPolicyRecord>>;

    async fn save_routing_policy_candidate(
        &self,
        policy: RoutingPolicy,
        owner: String,
    ) -> anyhow::Result<RoutingPolicyRecord>;

    async fn get_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>>;

    async fn update_routing_policy_status(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
        status: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>>;

    async fn activate_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>>;
}

// ---------------------------------------------------------------------------
// RulesRepository
// ---------------------------------------------------------------------------

#[async_trait]
pub trait RulesRepository: Send + Sync {
    async fn list_rules(&self) -> anyhow::Result<Vec<RuleSummaryRecord>>;

    async fn list_active_rules(&self) -> anyhow::Result<Vec<Rule>>;

    async fn get_rule(&self, rule_id: &str) -> anyhow::Result<Option<RuleDetailRecord>>;

    async fn rule_audit_history(
        &self,
        rule_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>>;

    async fn save_rule_candidate(
        &self,
        rule: Rule,
        owner: String,
    ) -> anyhow::Result<RuleDetailRecord>;

    async fn update_rule_status(
        &self,
        rule_id: &str,
        status: &str,
        status_actor_id: Option<&str>,
    ) -> anyhow::Result<Option<RuleSummaryRecord>>;

    async fn list_rule_conditions(&self) -> anyhow::Result<Vec<RuleConditionLibraryRecord>>;

    async fn rule_performance(&self) -> anyhow::Result<Vec<RulePerformanceRecord>>;

    async fn save_rule_backtest(
        &self,
        record: RuleBacktestRecord,
    ) -> anyhow::Result<RuleBacktestRecord>;

    async fn latest_rule_backtest(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleBacktestRecord>>;

    async fn save_rule_shadow_run(
        &self,
        record: RuleShadowRunRecord,
    ) -> anyhow::Result<RuleShadowRunRecord>;

    async fn latest_rule_shadow_run(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleShadowRunRecord>>;

    async fn save_rule_promotion_review(
        &self,
        record: RulePromotionReviewRecord,
    ) -> anyhow::Result<RulePromotionReviewRecord>;

    async fn latest_rule_promotion_review(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RulePromotionReviewRecord>>;
}

// ---------------------------------------------------------------------------
// CasesRepository
// ---------------------------------------------------------------------------

#[async_trait]
pub trait CasesRepository: Send + Sync {
    async fn list_leads(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<LeadRecord>>;

    async fn triage_lead(
        &self,
        lead_id: &str,
        input: TriageLeadInput,
    ) -> anyhow::Result<Option<TriageLeadRecord>>;

    async fn list_cases(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<CaseRecord>>;

    async fn update_case_status(
        &self,
        case_id: &str,
        input: UpdateCaseStatusInput,
    ) -> anyhow::Result<Option<UpdateCaseStatusRecord>>;

    async fn create_audit_sample(
        &self,
        input: CreateAuditSampleInput,
    ) -> anyhow::Result<AuditSampleRecord>;

    async fn list_audit_samples(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditSampleRecord>>;
}

// ---------------------------------------------------------------------------
// ModelsRepository
// ---------------------------------------------------------------------------

#[async_trait]
pub trait ModelsRepository: Send + Sync {
    async fn list_models(&self) -> anyhow::Result<Vec<ModelVersionRecord>>;

    async fn save_model_version(
        &self,
        record: ModelVersionRecord,
    ) -> anyhow::Result<ModelVersionRecord>;

    async fn update_model_status(
        &self,
        model_key: &str,
        model_version: &str,
        status: &str,
    ) -> anyhow::Result<Option<ModelVersionRecord>>;

    async fn model_performance(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Option<ModelPerformanceRecord>>;

    async fn save_model_promotion_review(
        &self,
        record: ModelPromotionReviewRecord,
    ) -> anyhow::Result<ModelPromotionReviewRecord>;

    async fn save_probability_calibration_report(
        &self,
        record: ProbabilityCalibrationReportRecord,
    ) -> anyhow::Result<ProbabilityCalibrationReportRecord>;

    async fn latest_model_promotion_review(
        &self,
        model_key: &str,
        model_version: &str,
    ) -> anyhow::Result<Option<ModelPromotionReviewRecord>>;

    async fn save_model_retraining_job(
        &self,
        record: ModelRetrainingJobRecord,
    ) -> anyhow::Result<ModelRetrainingJobRecord>;

    async fn list_model_retraining_jobs(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Vec<ModelRetrainingJobRecord>>;

    async fn get_model_retraining_job(
        &self,
        job_id: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>>;

    async fn claim_next_model_retraining_job(
        &self,
        model_key: Option<&str>,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>>;

    async fn update_model_retraining_job_status(
        &self,
        job_id: &str,
        status: &str,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>>;

    async fn complete_model_retraining_job(
        &self,
        input: CompleteModelRetrainingJobInput<'_>,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>>;
}

// ---------------------------------------------------------------------------
// DatasetsRepository
// ---------------------------------------------------------------------------

#[async_trait]
pub trait DatasetsRepository: Send + Sync {
    async fn register_dataset(&self, input: RegisterDatasetInput) -> anyhow::Result<DatasetRecord>;

    async fn list_datasets(&self) -> anyhow::Result<Vec<DatasetRecord>>;

    async fn get_dataset(&self, dataset_id: &str) -> anyhow::Result<Option<DatasetRecord>>;

    async fn add_field_mapping(
        &self,
        dataset_id: &str,
        input: CreateFieldMappingInput,
    ) -> anyhow::Result<Option<FieldMappingRecord>>;

    async fn register_feature_set(
        &self,
        input: RegisterFeatureSetInput,
    ) -> anyhow::Result<Option<FeatureSetRecord>>;

    async fn register_model_dataset(
        &self,
        input: RegisterModelDatasetInput,
    ) -> anyhow::Result<Option<ModelDatasetRecord>>;

    async fn get_model_dataset_source_dataset(
        &self,
        model_dataset_id: &str,
    ) -> anyhow::Result<Option<DatasetRecord>>;

    async fn register_model_evaluation(
        &self,
        input: RegisterModelEvaluationInput,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>>;

    async fn get_model_evaluation(
        &self,
        evaluation_run_id: &str,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>>;

    async fn list_model_evaluations(&self) -> anyhow::Result<Vec<ModelEvaluationRecord>>;

    async fn save_scoring_feature_context_materialization(
        &self,
        input: SaveScoringFeatureContextMaterializationInput,
    ) -> anyhow::Result<ScoringFeatureContextMaterializationRecord>;

    async fn get_scoring_feature_context_materialization(
        &self,
        materialization_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<ScoringFeatureContextMaterializationRecord>>;

    async fn save_clinical_compatibility_references(
        &self,
        input: SaveClinicalCompatibilityReferencesInput,
    ) -> anyhow::Result<Vec<ClinicalCompatibilityReferenceRecord>>;

    async fn save_unbundling_comparator_candidates(
        &self,
        input: SaveUnbundlingComparatorCandidatesInput,
    ) -> anyhow::Result<Vec<UnbundlingComparatorCandidateRecord>>;
}

// ---------------------------------------------------------------------------
// EvidenceRepository
// ---------------------------------------------------------------------------

#[async_trait]
pub trait EvidenceRepository: Send + Sync {
    async fn save_evidence_document(
        &self,
        input: CreateEvidenceDocumentInput,
    ) -> anyhow::Result<EvidenceDocumentRecord>;

    async fn list_evidence_documents(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentRecord>>;

    async fn get_evidence_document(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentRecord>>;

    async fn save_evidence_document_chunk(
        &self,
        input: CreateEvidenceDocumentChunkInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentChunkRecord>>;

    async fn list_evidence_document_chunks(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentChunkRecord>>;

    async fn save_evidence_ocr_output(
        &self,
        input: CreateEvidenceOcrOutputInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceOcrOutputRecord>>;

    async fn list_evidence_ocr_outputs(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceOcrOutputRecord>>;

    async fn save_evidence_embedding_job(
        &self,
        input: CreateEvidenceEmbeddingJobInput,
    ) -> anyhow::Result<EvidenceEmbeddingJobRecord>;

    async fn list_evidence_embedding_jobs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceEmbeddingJobRecord>>;

    async fn save_evidence_retrieval_audit_event(
        &self,
        input: CreateEvidenceRetrievalAuditEventInput,
    ) -> anyhow::Result<EvidenceRetrievalAuditEventRecord>;

    async fn list_evidence_retrieval_audit_events(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceRetrievalAuditEventRecord>>;
}

// ---------------------------------------------------------------------------
// OutcomesRepository
// ---------------------------------------------------------------------------

#[async_trait]
pub trait OutcomesRepository: Send + Sync {
    async fn save_investigation_result(
        &self,
        record: InvestigationResultRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord>;

    async fn save_qa_review(
        &self,
        record: QaReviewRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord>;

    async fn list_qa_feedback_items(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaFeedbackItemRecord>>;

    async fn update_qa_feedback_status(
        &self,
        feedback_id: &str,
        input: UpdateQaFeedbackStatusInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<UpdateQaFeedbackStatusRecord>>;

    async fn list_qa_reviews(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaReviewRecord>>;

    async fn list_outcome_labels(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<OutcomeLabelRecord>>;

    async fn claim_audit_history(
        &self,
        claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>>;

    async fn list_audit_events(
        &self,
        filter: AuditEventListFilter,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>>;

    async fn list_webhook_events(&self) -> anyhow::Result<Vec<WebhookEventRecord>>;

    async fn save_webhook_delivery_attempt(
        &self,
        input: WebhookDeliveryAttemptInput,
    ) -> anyhow::Result<WebhookDeliveryAttemptRecord>;
}

// ---------------------------------------------------------------------------
// KnowledgeRepository
// ---------------------------------------------------------------------------

#[async_trait]
pub trait KnowledgeRepository: Send + Sync {
    async fn dashboard_summary(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<DashboardSummaryRecord>;

    async fn provider_risk_summary(&self) -> anyhow::Result<ProviderRiskSummaryRecord>;

    async fn save_provider_sanctions(
        &self,
        input: SaveProviderSanctionsInput,
    ) -> anyhow::Result<Vec<ProviderSanctionRecord>>;

    async fn save_provider_profile_windows(
        &self,
        input: SaveProviderProfileWindowsInput,
    ) -> anyhow::Result<Vec<ProviderProfileWindowRecord>>;

    async fn save_provider_graph_signals(
        &self,
        input: SaveProviderGraphSignalsInput,
    ) -> anyhow::Result<Vec<ProviderGraphSignalRecord>>;

    async fn save_peer_benchmark_groups(
        &self,
        input: SavePeerBenchmarkGroupsInput,
    ) -> anyhow::Result<Vec<PeerBenchmarkGroupRecord>>;

    async fn save_episode_rollups(
        &self,
        input: SaveEpisodeRollupsInput,
    ) -> anyhow::Result<Vec<EpisodeRollupRecord>>;

    async fn list_knowledge_cases(&self) -> anyhow::Result<Vec<KnowledgeCaseRecord>>;

    async fn save_knowledge_case(
        &self,
        record: KnowledgeCaseRecord,
    ) -> anyhow::Result<KnowledgeCaseRecord>;

    async fn search_similar_cases(
        &self,
        query: SimilarCaseQuery,
    ) -> anyhow::Result<Vec<SimilarCaseRecord>>;

    async fn save_agent_registry(
        &self,
        record: AgentRegistryRecord,
    ) -> anyhow::Result<AgentRegistryRecord>;

    async fn active_agent_registry(
        &self,
        agent_kind: &str,
        agent_version: u32,
    ) -> anyhow::Result<Option<AgentRegistryRecord>>;

    async fn save_agent_run(&self, run: PersistedAgentRun) -> anyhow::Result<()>;

    async fn list_agent_runs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AgentRunLogRecord>>;

    async fn cancel_agent_run(&self, agent_run_id: &str) -> anyhow::Result<()>;

    async fn save_agent_approval(
        &self,
        approval: AgentApprovalRecord,
    ) -> anyhow::Result<AgentApprovalRecord>;
}

// ---------------------------------------------------------------------------
// ScoringRepository — composed supertrait
// ---------------------------------------------------------------------------

pub trait ScoringRepository:
    ClaimsRepository
    + RoutingRepository
    + RulesRepository
    + CasesRepository
    + ModelsRepository
    + DatasetsRepository
    + EvidenceRepository
    + OutcomesRepository
    + KnowledgeRepository
    + Send
    + Sync
{
}

impl<T> ScoringRepository for T where
    T: ClaimsRepository
        + RoutingRepository
        + RulesRepository
        + CasesRepository
        + ModelsRepository
        + DatasetsRepository
        + EvidenceRepository
        + OutcomesRepository
        + KnowledgeRepository
        + Send
        + Sync
{
}

pub type SharedRepository = Arc<dyn ScoringRepository>;
