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
mod trait_impl;

#[derive(Debug, Default)]
pub struct InMemoryScoringRepository {
    claims: Mutex<HashMap<String, ClaimContext>>,
    inbox_claim_runs: Mutex<HashMap<String, PersistedInboxClaimRun>>,
    runs: Mutex<Vec<PersistedScoringRun>>,
    audit_events: Mutex<Vec<PersistedAuditEvent>>,
    agent_runs: Mutex<Vec<PersistedAgentRun>>,
    agent_registry: Mutex<HashMap<String, AgentRegistryRecord>>,
    agent_investigations: Mutex<HashMap<String, AgentInvestigationRecord>>,
    agent_audit_events: Mutex<Vec<AgentAuditEventRecord>>,
    leads: Mutex<HashMap<String, LeadRecord>>,
    cases: Mutex<HashMap<String, CaseRecord>>,
    audit_samples: Mutex<HashMap<String, AuditSampleRecord>>,
    audit_sample_sequence: Mutex<u64>,
    candidate_rules: Mutex<HashMap<String, RuleDetailRecord>>,
    rule_statuses: Mutex<HashMap<String, String>>,
    rule_submitters: Mutex<HashMap<String, String>>,
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

    #[tokio::test]
    async fn in_memory_agent_run_appends_structured_audit_event() {
        let repository = InMemoryScoringRepository::default();

        repository
            .save_agent_run(PersistedAgentRun {
                agent_run_id: "agent_01HX".into(),
                claim_id: "CLM-0287".into(),
                status: "succeeded".into(),
                decision_boundary: "assistive_only".into(),
                output_json: serde_json::json!({
                    "findings": [{"finding": "peer outlier"}],
                    "evidence_sufficiency": "sufficient"
                }),
                evidence_refs: vec![Value::String("agent_run:agent_01HX".into())],
                steps: vec![],
                context_snapshots: vec![],
                policy_checks: vec![],
                tool_calls: vec![],
                tool_results: vec![],
                approvals: vec![],
            })
            .await
            .unwrap();

        let audit_events = repository.agent_audit_events.lock().await;
        assert_eq!(audit_events.len(), 1);
        assert!(audit_events[0]
            .investigation_id
            .starts_with("investigation:"));
        assert_eq!(audit_events[0].decision_boundary, "assistive_only");
        assert_eq!(audit_events[0].findings_count, 1);
        assert!(audit_events[0].input_digest.starts_with("sha256:"));
        assert!(!audit_events[0].payload.to_string().contains("CLM-0287"));
        drop(audit_events);

        let registry = repository.agent_registry.lock().await;
        assert!(registry.contains_key(DEFAULT_AGENT_IDENTITY_ID));
        drop(registry);

        let investigations = repository.agent_investigations.lock().await;
        assert_eq!(investigations.len(), 1);
        assert_eq!(
            investigations.values().next().unwrap().orchestrator_version,
            DEFAULT_ORCHESTRATOR_VERSION
        );
    }
}
