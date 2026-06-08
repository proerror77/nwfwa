use crate::pii::mask_audit_payload;
use async_trait::async_trait;
use fwa_core::{canonical_scheme_family, AuditEventId, ClaimContext};
use fwa_rules::Rule;
use fwa_scoring::RoutingPolicy;
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use tokio::sync::Mutex;

mod agent_helpers;
mod audit_helpers;
mod audit_sample_helpers;
mod case_rows;
mod dashboard_helpers;
mod dataset_rows;
mod evidence_rows;
mod knowledge_helpers;
mod member_profile_helpers;
mod model_helpers;
mod outcome_helpers;
mod provider_helpers;
mod row_types;
mod rule_helpers;
mod rule_performance_helpers;
mod saving_helpers;
mod r#trait;
mod triage_helpers;
mod types;
mod webhook_helpers;

use self::agent_helpers::agent_run_log_from_persisted;
use self::audit_helpers::{
    audit_event_payload_matches_customer_scope, audit_history_from_persisted,
    evidence_values_to_strings, persisted_audit_event_matches_filter,
    pilot_audit_event_matches_filter, scoped_claim_ids_from_audit_events,
};
use self::audit_sample_helpers::{
    audit_sample_strata_contexts_from_claims, build_audit_sample, reviewer_lead_sample_counts,
    with_sample_outcome_distributions,
};
use self::case_rows::{
    load_audit_sample_strata_contexts, load_case_in_tx, load_cases, load_control_audit_population,
    load_lead_in_tx, load_leads,
};
use self::dashboard_helpers::{
    decimal_to_f64, summarize_dashboard_agent_governance, summarize_dashboard_audit_coverage,
    summarize_dashboard_case_sla, summarize_dashboard_label_pool,
    summarize_dashboard_model_governance, summarize_dashboard_qa_queue,
    summarize_dashboard_rule_governance, summarize_dashboard_value_measurement,
};
use self::dataset_rows::load_dataset_record;
use self::evidence_rows::{
    evidence_document_chunk_from_row, evidence_document_from_row, evidence_embedding_job_from_row,
    evidence_ocr_output_from_row, evidence_retrieval_audit_event_from_row,
};
use self::knowledge_helpers::{
    default_knowledge_cases, ensure_default_knowledge_cases_seeded, search_cases,
};
use self::member_profile_helpers::{member_profile_from_contexts, member_profile_summary_record};
use self::model_helpers::{
    default_model_versions, drift_summary, empty_model_performance, ensure_default_models_seeded,
    model_performance_with_drift, model_retraining_job_from_pg_row, model_version_key,
};
use self::outcome_helpers::{
    financial_impact_from_investigation, financial_impact_from_parts,
    label_from_bootstrap_review_event, label_from_qa_review, labels_from_case_status,
    labels_from_investigation_result, labels_from_lead_triage_events,
    labels_from_medical_review_event, latest_qa_feedback_statuses, normalize_financial_impact_type,
    qa_case_id_from_feedback_id, qa_feedback_id, qa_review_to_feedback_item, sort_outcome_labels,
    sort_qa_feedback_items, FinancialImpactRecord,
};
use self::provider_helpers::summarize_provider_risk_profiles;
pub use self::r#trait::{ScoringRepository, SharedRepository};
use self::row_types::{
    inbox_claim_run_from_row, AgentApprovalRow, AgentPolicyCheckRow, ClaimContextRow, ClaimItemRow,
    IntoClaimContext,
};
pub use self::rule_helpers::default_runtime_rules;
use self::rule_helpers::{
    apply_rule_backtest_metadata, apply_rule_status, default_routing_policies,
    default_rule_backtest_summary, default_rule_details, default_rule_false_positive_history,
    ensure_default_rules_seeded, ensure_rule_condition_library_table, latest_rule_backtest_for,
    normalize_review_mode, parse_recommended_action, review_mode_from_dsl,
    routing_policy_from_record, routing_policy_record, routing_policy_record_from_row,
    routing_policy_review_mode_applies, rule_applicability_scope,
    rule_condition_records_from_detail, rule_condition_status, rule_detail_from_rule,
    rule_governance_evidence_refs, runtime_rule_from_detail, runtime_rule_from_parts,
    seed_default_routing_policy_records, upsert_rule_conditions_tx,
};
use self::rule_performance_helpers::{
    decimal_from_json, ratio, rule_accumulators_from_rules, rule_performance_records,
    InvestigationOutcome, RULE_REVIEW_COST_AMOUNT,
};
use self::saving_helpers::{
    derive_saving_attributions, format_decimal_cents, segment_roi, summarize_saving_attributions,
    summarize_saving_segments,
};
use self::triage_helpers::{
    case_from_lead, case_sla_status, control_lead_from_scoring_run, hours_between,
    is_terminal_case_status, lead_from_scoring_run, merge_target_exists_in_memory,
    merge_target_lead_in_tx, scheme_family_from_alert_code, scheme_family_from_dsl,
    sla_target_hours_for_priority, triage_audit_payload, triage_disposition_for_decision,
    triage_status_for_decision,
};
pub use self::types::*;
use self::types::{
    AuditSampleStrataContext, MemberProfileSummaryInput, QaFeedbackStatusUpdate,
    SavingAttributionRecord,
};
use self::webhook_helpers::{
    apply_webhook_delivery_state, next_webhook_attempt_at, sort_webhook_events,
    webhook_event_from_audit,
};

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

#[derive(Debug, Clone)]
pub struct PostgresScoringRepository {
    pool: PgPool,
}

impl PostgresScoringRepository {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let max_connections = std::env::var("FWA_DB_MAX_CONNECTIONS")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(5);
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    async fn load_agent_tool_calls(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<Vec<AgentToolCallRecord>> {
        let rows: Vec<(String, String, String, Value, Value)> = sqlx::query_as(
            "SELECT tool_call_id, tool_name, status, input_json, evidence_refs
             FROM tool_calls
             WHERE agent_run_id = $1
             ORDER BY created_at, id",
        )
        .bind(agent_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(tool_call_id, tool_name, status, input_json, evidence_refs)| {
                    AgentToolCallRecord {
                        tool_call_id,
                        tool_name,
                        status,
                        input_json,
                        evidence_refs: json_array_to_strings(evidence_refs),
                    }
                },
            )
            .collect())
    }

    async fn load_agent_context_snapshots(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<Vec<AgentContextSnapshotRecord>> {
        let rows: Vec<(String, String, Value, Value, String)> = sqlx::query_as(
            "SELECT snapshot_id, redaction_status, context_json, source_refs, checksum
             FROM agent_context_snapshots
             WHERE agent_run_id = $1
             ORDER BY created_at, id",
        )
        .bind(agent_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(snapshot_id, redaction_status, context_json, source_refs, checksum)| {
                    AgentContextSnapshotRecord {
                        snapshot_id,
                        redaction_status,
                        context_json,
                        source_refs: json_array_to_strings(source_refs),
                        checksum,
                    }
                },
            )
            .collect())
    }

    async fn load_agent_policy_checks(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<Vec<AgentPolicyCheckRecord>> {
        let rows: Vec<AgentPolicyCheckRow> = sqlx::query_as(
            "SELECT policy_check_id, tool_call_id, tool_name, policy_name, decision, reason, evidence_refs, created_at
             FROM agent_policy_checks
             WHERE agent_run_id = $1
             ORDER BY created_at, id",
        )
        .bind(agent_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| AgentPolicyCheckRecord {
                policy_check_id: row.policy_check_id,
                agent_run_id: agent_run_id.to_string(),
                tool_call_id: row.tool_call_id,
                tool_name: row.tool_name,
                policy_name: row.policy_name,
                decision: row.decision,
                reason: row.reason,
                evidence_refs: json_array_to_strings(row.evidence_refs),
                created_at: Some(row.created_at.to_rfc3339()),
            })
            .collect())
    }

    async fn load_agent_tool_results(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<Vec<AgentToolResultRecord>> {
        let rows: Vec<(String, String, String, String, Value, Value)> = sqlx::query_as(
            "SELECT tool_result_id, tool_call_id, tool_name, status, output_json, evidence_refs
             FROM tool_results
             WHERE agent_run_id = $1
             ORDER BY created_at, id",
        )
        .bind(agent_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(tool_result_id, tool_call_id, tool_name, status, output_json, evidence_refs)| {
                    AgentToolResultRecord {
                        tool_result_id,
                        tool_call_id,
                        tool_name,
                        status,
                        output_json,
                        evidence_refs: json_array_to_strings(evidence_refs),
                    }
                },
            )
            .collect())
    }

    async fn load_agent_approvals(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<Vec<AgentApprovalRecord>> {
        let rows: Vec<AgentApprovalRow> = sqlx::query_as(
            "SELECT approval_id, proposed_action, decision, approver, reason, evidence_refs, created_at
             FROM agent_approvals
             WHERE agent_run_id = $1
             ORDER BY created_at, id",
        )
        .bind(agent_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| AgentApprovalRecord {
                approval_id: row.approval_id,
                agent_run_id: agent_run_id.to_string(),
                proposed_action: row.proposed_action,
                decision: row.decision,
                approver: row.approver,
                reason: row.reason,
                evidence_refs: json_array_to_strings(row.evidence_refs),
                created_at: Some(row.created_at.to_rfc3339()),
            })
            .collect())
    }
}

#[async_trait]
impl ScoringRepository for PostgresScoringRepository {
    async fn upsert_claim_context(
        &self,
        context: ClaimContext,
        raw_payload: Value,
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;

        let member_row: (String,) = sqlx::query_as(
            "INSERT INTO members (external_member_id)
             VALUES ($1)
             ON CONFLICT (external_member_id) DO UPDATE SET updated_at = now()
             RETURNING id::text",
        )
        .bind(&context.member.external_member_id)
        .fetch_one(&mut *tx)
        .await?;

        let policy_row: (String,) = sqlx::query_as(
            "INSERT INTO policies
             (external_policy_id, member_id, product_code, coverage_start_date, coverage_end_date, coverage_limit_amount, currency)
             VALUES ($1, $2::uuid, $3, $4, $5, $6, $7)
             ON CONFLICT (external_policy_id) DO UPDATE SET updated_at = now()
             RETURNING id::text",
        )
        .bind(&context.policy.external_policy_id)
        .bind(&member_row.0)
        .bind(&context.policy.product_code)
        .bind(context.policy.coverage_start_date)
        .bind(context.policy.coverage_end_date)
        .bind(context.policy.coverage_limit.amount)
        .bind(&context.policy.coverage_limit.currency)
        .fetch_one(&mut *tx)
        .await?;

        let provider_row: (String,) = sqlx::query_as(
            "INSERT INTO providers (external_provider_id, name, provider_type, region, risk_tier)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (external_provider_id) DO UPDATE SET updated_at = now()
             RETURNING id::text",
        )
        .bind(&context.provider.external_provider_id)
        .bind(&context.provider.name)
        .bind(&context.provider.provider_type)
        .bind(&context.provider.region)
        .bind(format!("{:?}", context.provider.risk_tier))
        .fetch_one(&mut *tx)
        .await?;

        let claim_row: (String,) = sqlx::query_as(
            "INSERT INTO claims
             (external_claim_id, member_id, policy_id, provider_id, claim_type, diagnosis_code, service_date, claim_amount, currency, status, raw_payload)
             VALUES ($1, $2::uuid, $3::uuid, $4::uuid, 'medical', $5, $6, $7, $8, 'submitted', $9)
             ON CONFLICT (external_claim_id) DO UPDATE
             SET updated_at = now(), raw_payload = EXCLUDED.raw_payload, claim_amount = EXCLUDED.claim_amount
             RETURNING id::text",
        )
        .bind(&context.claim.external_claim_id)
        .bind(&member_row.0)
        .bind(&policy_row.0)
        .bind(&provider_row.0)
        .bind(&context.claim.diagnosis_code)
        .bind(context.claim.service_date)
        .bind(context.claim.amount.amount)
        .bind(&context.claim.amount.currency)
        .bind(raw_payload)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM claim_items WHERE claim_id = $1::uuid")
            .bind(&claim_row.0)
            .execute(&mut *tx)
            .await?;

        for item in &context.items {
            sqlx::query(
                "INSERT INTO claim_items
                 (claim_id, item_code, item_type, description, quantity, unit_amount, total_amount, currency)
                 VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(&claim_row.0)
            .bind(&item.item_code)
            .bind(&item.item_type)
            .bind(&item.description)
            .bind(item.quantity as i32)
            .bind(item.unit_amount.amount)
            .bind(item.total_amount.amount)
            .bind(&item.total_amount.currency)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn load_claim_context(
        &self,
        external_claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<ClaimContext>> {
        let row: Option<ClaimContextRow> = sqlx::query_as(
            "SELECT c.external_claim_id,
                    c.diagnosis_code,
                    c.service_date,
                    c.claim_amount,
                    c.currency AS claim_currency,
                    m.external_member_id,
                    m.dob,
                    m.gender,
                    p.external_policy_id,
                    p.product_code,
                    p.coverage_start_date,
                    p.coverage_end_date,
                    p.coverage_limit_amount,
                    p.currency AS policy_currency,
                    pr.external_provider_id,
                    pr.name AS provider_name,
                    pr.provider_type,
                    pr.region AS provider_region,
                    pr.risk_tier AS provider_risk_tier
             FROM claims c
             JOIN members m ON m.id = c.member_id
             JOIN policies p ON p.id = c.policy_id
             JOIN providers pr ON pr.id = c.provider_id
             WHERE c.external_claim_id = $1
               AND (
                 $2::text IS NULL OR EXISTS (
                   SELECT 1
                   FROM audit_events ae
                   WHERE ae.claim_id = c.id
                     AND ae.payload ->> 'customer_scope_id' = $2
                 )
               )",
        )
        .bind(external_claim_id)
        .bind(customer_scope_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let item_rows: Vec<ClaimItemRow> = sqlx::query_as(
            "SELECT ci.item_code, ci.item_type, ci.description, ci.quantity, ci.unit_amount, ci.total_amount, ci.currency
             FROM claim_items ci
             JOIN claims c ON c.id = ci.claim_id
             WHERE c.external_claim_id = $1
             ORDER BY ci.created_at, ci.item_code",
        )
        .bind(external_claim_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(Some(row.into_context(item_rows)))
    }

    async fn member_profile_summary(
        &self,
        member_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<MemberProfileSummaryRecord>> {
        let member_exists: Option<(String,)> = sqlx::query_as(
            "SELECT m.external_member_id
             FROM members m
             WHERE m.external_member_id = $1
               AND (
                 $2::text IS NULL OR EXISTS (
                   SELECT 1
                   FROM claims c
                   JOIN audit_events ae ON ae.claim_id = c.id
                   WHERE c.member_id = m.id
                     AND ae.payload ->> 'customer_scope_id' = $2
                 )
               )",
        )
        .bind(member_id)
        .bind(customer_scope_id)
        .fetch_optional(&self.pool)
        .await?;
        if member_exists.is_none() {
            return Ok(None);
        }

        let row: (i64, i64, Option<Decimal>, Option<String>) = sqlx::query_as(
            "SELECT COUNT(c.id)::bigint,
                    COUNT(DISTINCT p.id)::bigint,
                    SUM(c.claim_amount),
                    MIN(c.currency)
             FROM members m
             LEFT JOIN claims c ON c.member_id = m.id
             LEFT JOIN policies p ON p.id = c.policy_id
             WHERE m.external_member_id = $1
               AND (
                 $2::text IS NULL OR EXISTS (
                   SELECT 1
                   FROM audit_events ae
                   WHERE ae.claim_id = c.id
                     AND ae.payload ->> 'customer_scope_id' = $2
                 )
               )",
        )
        .bind(member_id)
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;
        let latest_claim: Option<(String,)> = sqlx::query_as(
            "SELECT c.external_claim_id
             FROM claims c
             JOIN members m ON m.id = c.member_id
             WHERE m.external_member_id = $1
               AND (
                 $2::text IS NULL OR EXISTS (
                   SELECT 1
                   FROM audit_events ae
                   WHERE ae.claim_id = c.id
                     AND ae.payload ->> 'customer_scope_id' = $2
                 )
               )
             ORDER BY c.service_date DESC, c.external_claim_id DESC
             LIMIT 1",
        )
        .bind(member_id)
        .bind(customer_scope_id)
        .fetch_optional(&self.pool)
        .await?;
        let high_risk: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT c.id)::bigint
             FROM members m
             JOIN claims c ON c.member_id = m.id
             JOIN scoring_runs sr ON sr.claim_id = c.id
             WHERE m.external_member_id = $1
               AND (
                 $2::text IS NULL OR EXISTS (
                   SELECT 1
                   FROM audit_events ae
                   WHERE ae.claim_id = c.id
                     AND ae.payload ->> 'customer_scope_id' = $2
                 )
               )
               AND sr.risk_score >= 70",
        )
        .bind(member_id)
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Some(member_profile_summary_record(
            MemberProfileSummaryInput {
                member_id: member_id.into(),
                claim_count: row.0 as u32,
                policy_count: row.1 as u32,
                total_claim_amount: row.2.unwrap_or(Decimal::ZERO),
                currency: row.3.unwrap_or_else(|| "UNKNOWN".into()),
                high_risk_claim_count: high_risk.0 as u32,
                latest_claim_id: latest_claim.map(|claim| claim.0),
                evidence_refs: BTreeSet::from([format!("members:{member_id}")]),
            },
        )))
    }

    async fn save_scoring_run(&self, run: PersistedScoringRun) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        let claim_row: Option<(String,)> =
            sqlx::query_as("SELECT id::text FROM claims WHERE external_claim_id = $1")
                .bind(&run.claim_id)
                .fetch_optional(&mut *tx)
                .await?;

        let claim_uuid = claim_row.map(|row| row.0);
        sqlx::query(
            "INSERT INTO scoring_runs
             (run_id, claim_id, source_system, actor_id, status, risk_score, rag, risk_level, recommended_action, confidence_score, confidence, routing_reason, routing_policy, score_breakdown, completed_at)
             VALUES ($1, $2::uuid, $3, $4, 'succeeded', $5, $6, $7, $8, $9, $10, $11, $12, $13, now())",
        )
        .bind(&run.run_id)
        .bind(claim_uuid.as_deref())
        .bind(&run.source_system)
        .bind(&run.actor_id)
        .bind(run.risk_score as i32)
        .bind(&run.rag)
        .bind(&run.risk_level)
        .bind(&run.recommended_action)
        .bind(run.confidence_score as i32)
        .bind(&run.confidence)
        .bind(&run.routing_reason)
        .bind(&run.routing_policy)
        .bind(&run.score_breakdown)
        .execute(&mut *tx)
        .await?;

        for feature in &run.feature_values {
            let feature_name = feature["name"].as_str().unwrap_or("unknown");
            let feature_version = feature["version"].as_i64().unwrap_or(1) as i32;
            sqlx::query(
                "INSERT INTO feature_values
                 (run_id, claim_id, feature_name, feature_version, value_json, evidence_json)
                 VALUES ($1, $2::uuid, $3, $4, $5, $6)",
            )
            .bind(&run.run_id)
            .bind(claim_uuid.as_deref())
            .bind(feature_name)
            .bind(feature_version)
            .bind(feature["value"].clone())
            .bind(feature["evidence_refs"].clone())
            .execute(&mut *tx)
            .await?;
        }

        for rule_run in &run.rule_runs {
            let rule_evidence = rule_run
                .get("evidence_refs")
                .filter(|evidence| evidence.is_array())
                .cloned()
                .unwrap_or_else(|| serde_json::json!([]));
            sqlx::query(
                "INSERT INTO rule_runs
                 (run_id, rule_id, rule_version_id, matched, score_contribution, alert_code, reason, evidence_json)
                 VALUES (
                   $1,
                   (SELECT id FROM rules WHERE rule_key = $2),
                   (
                     SELECT rv.id
                     FROM rule_versions rv
                     JOIN rules r ON r.id = rv.rule_id
                     WHERE r.rule_key = $2 AND rv.version = $3
                   ),
                   true,
                   $4,
                   $5,
                   $6,
                   $7
                 )",
            )
            .bind(&run.run_id)
            .bind(rule_run["rule_id"].as_str())
            .bind(rule_run["rule_version"].as_i64().unwrap_or(1) as i32)
            .bind(rule_run["score_contribution"].as_i64().unwrap_or(0) as i32)
            .bind(rule_run["alert_code"].as_str())
            .bind(rule_run["reason"].as_str())
            .bind(rule_evidence)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            "INSERT INTO model_scores
             (run_id, model_version_id, model_key, runtime_kind, execution_provider, score, label, explanation_json, latency_ms)
             VALUES (
               $1,
               (
                 SELECT id
                 FROM model_versions
                 WHERE model_key = $2 AND version = $3
               ),
               $2,
               $4,
               $5,
               $6,
               $7,
               $8,
               $9
             )",
        )
        .bind(&run.run_id)
        .bind(run.model_score["model_key"].as_str().unwrap_or("unknown"))
        .bind(run.model_score["model_version"].as_str().unwrap_or("unknown"))
        .bind(run.model_score["runtime_kind"].as_str().unwrap_or("unknown"))
        .bind(run.model_score["execution_provider"].as_str().unwrap_or("cpu"))
        .bind(run.model_score["score"].as_i64().unwrap_or(0) as i32)
        .bind(run.model_score["label"].as_str().unwrap_or("UNKNOWN"))
        .bind(run.model_score["explanations"].clone())
        .bind(run.model_score["latency_ms"].as_i64().unwrap_or(0) as i32)
        .execute(&mut *tx)
        .await?;

        if let Some(mut lead) = lead_from_scoring_run(&run, None) {
            if let Some((member_id, provider_id)) = sqlx::query_as::<_, (String, String)>(
                "SELECT m.external_member_id, pr.external_provider_id
                 FROM claims c
                 JOIN members m ON m.id = c.member_id
                 JOIN providers pr ON pr.id = c.provider_id
                 WHERE c.external_claim_id = $1",
            )
            .bind(&run.claim_id)
            .fetch_optional(&mut *tx)
            .await?
            {
                lead.member_id = member_id;
                lead.provider_id = provider_id;
            }
            sqlx::query(
                "INSERT INTO fwa_leads
                 (lead_id, run_id, claim_id, member_id, provider_id, source_system, review_mode, scheme_family, lead_source, status, disposition, risk_score, rag, reason, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                 ON CONFLICT (lead_id) DO UPDATE
                 SET run_id = EXCLUDED.run_id,
                     claim_id = EXCLUDED.claim_id,
                     member_id = EXCLUDED.member_id,
                     provider_id = EXCLUDED.provider_id,
                     source_system = EXCLUDED.source_system,
                     review_mode = EXCLUDED.review_mode,
                     scheme_family = EXCLUDED.scheme_family,
                     lead_source = EXCLUDED.lead_source,
                     status = EXCLUDED.status,
                     disposition = EXCLUDED.disposition,
                     risk_score = EXCLUDED.risk_score,
                     rag = EXCLUDED.rag,
                     reason = EXCLUDED.reason,
                     evidence_refs = EXCLUDED.evidence_refs,
                     updated_at = now()",
            )
            .bind(&lead.lead_id)
            .bind(&lead.run_id)
            .bind(&lead.claim_id)
            .bind(&lead.member_id)
            .bind(&lead.provider_id)
            .bind(&lead.source_system)
            .bind(&lead.review_mode)
            .bind(&lead.scheme_family)
            .bind(&lead.lead_source)
            .bind(&lead.status)
            .bind(&lead.disposition)
            .bind(lead.risk_score as i32)
            .bind(&lead.rag)
            .bind(&lead.reason)
            .bind(serde_json::json!(lead.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }

        insert_audit_event(
            &mut tx,
            &PersistedAuditEvent {
                audit_id: run.audit_id,
                run_id: run.run_id,
                claim_id: run.claim_id,
                source_system: run.source_system,
                actor_id: run.actor_id,
                actor_role: "tpa_system".into(),
                event_type: "scoring.completed".into(),
                event_status: "succeeded".into(),
                summary: "FWA scoring completed".into(),
                payload: run.audit_event,
                evidence_refs: run.evidence_refs,
            },
            claim_uuid.as_deref(),
        )
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn save_audit_event(&self, event: PersistedAuditEvent) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        let claim_row: Option<(String,)> =
            sqlx::query_as("SELECT id::text FROM claims WHERE external_claim_id = $1")
                .bind(&event.claim_id)
                .fetch_optional(&mut *tx)
                .await?;
        sqlx::query(
            "INSERT INTO scoring_runs
             (run_id, claim_id, source_system, actor_id, status, completed_at, error_code, error_message)
             VALUES ($1, $2::uuid, $3, $4, $5, now(), $6, $7)
             ON CONFLICT (run_id) DO NOTHING",
        )
        .bind(&event.run_id)
        .bind(claim_row.as_ref().map(|row| row.0.as_str()))
        .bind(&event.source_system)
        .bind(&event.actor_id)
        .bind(&event.event_status)
        .bind(&event.event_type)
        .bind(event.payload["error"].as_str())
        .execute(&mut *tx)
        .await?;
        insert_audit_event(
            &mut tx,
            &event,
            claim_row.as_ref().map(|row| row.0.as_str()),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn save_inbox_claim_run(&self, run: PersistedInboxClaimRun) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO inbox_claim_runs
             (run_id, audit_id, external_message_id, idempotency_key, external_message_fingerprint,
              raw_payload_checksum, raw_payload_ref, mapping_version, validation_result, scoring_ready,
              claim_id, source_system, customer_scope_id, canonical_claim_context, validation_errors,
              data_quality_signals, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
             ON CONFLICT (run_id) DO UPDATE
             SET audit_id = EXCLUDED.audit_id,
                 external_message_id = EXCLUDED.external_message_id,
                 idempotency_key = EXCLUDED.idempotency_key,
                 external_message_fingerprint = EXCLUDED.external_message_fingerprint,
                 raw_payload_checksum = EXCLUDED.raw_payload_checksum,
                 raw_payload_ref = EXCLUDED.raw_payload_ref,
                 mapping_version = EXCLUDED.mapping_version,
                 validation_result = EXCLUDED.validation_result,
                 scoring_ready = EXCLUDED.scoring_ready,
                 claim_id = EXCLUDED.claim_id,
                 source_system = EXCLUDED.source_system,
                 customer_scope_id = EXCLUDED.customer_scope_id,
                 canonical_claim_context = EXCLUDED.canonical_claim_context,
                 validation_errors = EXCLUDED.validation_errors,
                 data_quality_signals = EXCLUDED.data_quality_signals,
                 evidence_refs = EXCLUDED.evidence_refs,
                 updated_at = now()",
        )
        .bind(&run.run_id)
        .bind(&run.audit_id)
        .bind(&run.external_message_id)
        .bind(&run.idempotency_key)
        .bind(&run.external_message_fingerprint)
        .bind(&run.raw_payload_checksum)
        .bind(&run.raw_payload_ref)
        .bind(&run.mapping_version)
        .bind(&run.validation_result)
        .bind(run.scoring_ready)
        .bind(&run.claim_id)
        .bind(&run.source_system)
        .bind(&run.customer_scope_id)
        .bind(&run.canonical_claim_context)
        .bind(&run.validation_errors)
        .bind(&run.data_quality_signals)
        .bind(&run.evidence_refs)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_inbox_claim_run_by_idempotency_key(
        &self,
        idempotency_key: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
        let row = sqlx::query(
            "SELECT run_id, audit_id, external_message_id, idempotency_key,
                    external_message_fingerprint, raw_payload_checksum, raw_payload_ref,
                    mapping_version, validation_result, scoring_ready, claim_id,
                    source_system, customer_scope_id, canonical_claim_context,
                    validation_errors, data_quality_signals, evidence_refs
             FROM inbox_claim_runs
             WHERE idempotency_key = $1
               AND ($2::text IS NULL OR customer_scope_id = $2)",
        )
        .bind(idempotency_key)
        .bind(customer_scope_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(inbox_claim_run_from_row))
    }

    async fn get_inbox_claim_run_by_run_id(
        &self,
        run_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
        let row = sqlx::query(
            "SELECT run_id, audit_id, external_message_id, idempotency_key,
                    external_message_fingerprint, raw_payload_checksum, raw_payload_ref,
                    mapping_version, validation_result, scoring_ready, claim_id,
                    source_system, customer_scope_id, canonical_claim_context,
                    validation_errors, data_quality_signals, evidence_refs
             FROM inbox_claim_runs
             WHERE run_id = $1
               AND ($2::text IS NULL OR customer_scope_id = $2)",
        )
        .bind(run_id)
        .bind(customer_scope_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(inbox_claim_run_from_row))
    }

    async fn active_routing_policy(
        &self,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicy>> {
        ensure_default_routing_policies_seeded(&self.pool).await?;
        let row: Option<(Value,)> = sqlx::query_as(
            "SELECT policy_json
             FROM routing_policies
             WHERE status = 'active'
               AND review_mode IN ($1, 'both')
             ORDER BY CASE WHEN review_mode = $1 THEN 0 ELSE 1 END, version DESC
             LIMIT 1",
        )
        .bind(review_mode)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| serde_json::from_value(row.0))
            .transpose()
            .map_err(Into::into)
    }

    async fn list_routing_policies(&self) -> anyhow::Result<Vec<RoutingPolicyRecord>> {
        ensure_default_routing_policies_seeded(&self.pool).await?;
        let rows: Vec<(Value, String, String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT policy_json, status, owner, activated_at::text, created_at::text
             FROM routing_policies
             ORDER BY policy_key, review_mode, version DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(routing_policy_record_from_row)
            .collect()
    }

    async fn save_routing_policy_candidate(
        &self,
        policy: RoutingPolicy,
        owner: String,
    ) -> anyhow::Result<RoutingPolicyRecord> {
        ensure_default_routing_policies_seeded(&self.pool).await?;
        sqlx::query(
            "INSERT INTO routing_policies
             (policy_key, version, review_mode, status, owner, policy_json)
             VALUES ($1, $2, $3, 'draft', $4, $5)
             ON CONFLICT (policy_key, version, review_mode) DO UPDATE
             SET status = 'draft',
                 owner = EXCLUDED.owner,
                 policy_json = EXCLUDED.policy_json",
        )
        .bind(&policy.policy_id)
        .bind(policy.version as i32)
        .bind(&policy.review_mode)
        .bind(&owner)
        .bind(serde_json::to_value(&policy)?)
        .execute(&self.pool)
        .await?;

        Ok(routing_policy_record(policy, "draft", &owner, None, None))
    }

    async fn get_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        ensure_default_routing_policies_seeded(&self.pool).await?;
        let row: Option<(Value, String, String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT policy_json, status, owner, activated_at::text, created_at::text
                 FROM routing_policies
                 WHERE policy_key = $1 AND version = $2 AND review_mode = $3",
        )
        .bind(policy_id)
        .bind(version as i32)
        .bind(review_mode)
        .fetch_optional(&self.pool)
        .await?;

        row.map(routing_policy_record_from_row).transpose()
    }

    async fn update_routing_policy_status(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
        status: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        ensure_default_routing_policies_seeded(&self.pool).await?;
        let row: Option<(Value, String, String, Option<String>, Option<String>)> = sqlx::query_as(
            "UPDATE routing_policies
                 SET status = $4
                 WHERE policy_key = $1 AND version = $2 AND review_mode = $3
                 RETURNING policy_json, status, owner, activated_at::text, created_at::text",
        )
        .bind(policy_id)
        .bind(version as i32)
        .bind(review_mode)
        .bind(status)
        .fetch_optional(&self.pool)
        .await?;

        row.map(routing_policy_record_from_row).transpose()
    }

    async fn activate_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        ensure_default_routing_policies_seeded(&self.pool).await?;
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE routing_policies
             SET status = 'approved'
             WHERE review_mode = $1
               AND status = 'active'
               AND NOT (policy_key = $2 AND version = $3)",
        )
        .bind(review_mode)
        .bind(policy_id)
        .bind(version as i32)
        .execute(&mut *tx)
        .await?;

        let row: Option<(Value, String, String, Option<String>, Option<String>)> = sqlx::query_as(
            "UPDATE routing_policies
                 SET status = 'active', activated_at = now()
                 WHERE policy_key = $1 AND version = $2 AND review_mode = $3
                 RETURNING policy_json, status, owner, activated_at::text, created_at::text",
        )
        .bind(policy_id)
        .bind(version as i32)
        .bind(review_mode)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;

        row.map(routing_policy_record_from_row).transpose()
    }

    async fn list_rules(&self) -> anyhow::Result<Vec<RuleSummaryRecord>> {
        ensure_default_rules_seeded(&self.pool).await?;
        let rows: Vec<(String, String, String, String, i32, Value, i32, String)> = sqlx::query_as(
            "SELECT r.rule_key, r.name, r.status, r.owner, rv.version, rv.dsl, rv.score, rv.recommended_action
             FROM rules r
             JOIN LATERAL (
               SELECT version, dsl, score, recommended_action
               FROM rule_versions
               WHERE rule_id = r.id
               ORDER BY version DESC
               LIMIT 1
             ) rv ON true
             ORDER BY r.rule_key",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut summaries = rows
            .into_iter()
            .map(
                |(rule_id, name, status, owner, version, dsl, score, recommended_action)| {
                    let action = dsl.get("action").cloned().unwrap_or(Value::Null);
                    let review_mode = review_mode_from_dsl(&dsl);
                    let scheme_family = scheme_family_from_dsl(&dsl);
                    RuleSummaryRecord {
                        rule_id: rule_id.clone(),
                        name,
                        active_version: if status == "active" {
                            Some(version as u32)
                        } else {
                            None
                        },
                        latest_version: version as u32,
                        review_mode: review_mode.clone(),
                        scheme_family: scheme_family.clone(),
                        status,
                        owner,
                        score: score as u8,
                        alert_code: action["alert_code"]
                            .as_str()
                            .unwrap_or("UNKNOWN")
                            .to_string(),
                        recommended_action: parse_recommended_action(&recommended_action),
                        applicability_scope: rule_applicability_scope(&review_mode, &scheme_family),
                        backtest_result: default_rule_backtest_summary(),
                        estimated_saving: "0.00".into(),
                        false_positive_history: default_rule_false_positive_history(),
                        evidence_refs: rule_governance_evidence_refs(&rule_id, version as u32),
                    }
                },
            )
            .collect::<Vec<_>>();

        for summary in &mut summaries {
            let latest_backtest = self
                .latest_rule_backtest(&summary.rule_id, summary.latest_version)
                .await?;
            apply_rule_backtest_metadata(summary, latest_backtest.as_ref());
        }

        Ok(summaries)
    }

    async fn list_active_rules(&self) -> anyhow::Result<Vec<Rule>> {
        ensure_default_rules_seeded(&self.pool).await?;
        let rows: Vec<(String, String, i32, Value)> = sqlx::query_as(
            "SELECT r.rule_key, r.name, rv.version, rv.dsl
             FROM rules r
             JOIN LATERAL (
               SELECT version, dsl
               FROM rule_versions
               WHERE rule_id = r.id
               ORDER BY version DESC
               LIMIT 1
             ) rv ON true
             WHERE r.status = 'active'
             ORDER BY r.rule_key",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|(rule_id, name, version, dsl)| {
                runtime_rule_from_parts(rule_id, name, version as u32, dsl)
            })
            .collect()
    }

    async fn get_rule(&self, rule_id: &str) -> anyhow::Result<Option<RuleDetailRecord>> {
        ensure_default_rules_seeded(&self.pool).await?;
        let summary = self
            .list_rules()
            .await?
            .into_iter()
            .find(|rule| rule.rule_id == rule_id);
        let Some(summary) = summary else {
            return Ok(None);
        };

        let rows: Vec<(i32, Value, i32, String)> = sqlx::query_as(
            "SELECT rv.version, rv.dsl, rv.score, rv.recommended_action
             FROM rule_versions rv
             JOIN rules r ON r.id = rv.rule_id
             WHERE r.rule_key = $1
             ORDER BY rv.version DESC",
        )
        .bind(rule_id)
        .fetch_all(&self.pool)
        .await?;

        let versions = rows
            .into_iter()
            .map(|(version, dsl, score, recommended_action)| {
                let action = dsl.get("action").cloned().unwrap_or(Value::Null);
                RuleVersionRecord {
                    version: version as u32,
                    status: summary.status.clone(),
                    review_mode: review_mode_from_dsl(&dsl),
                    scheme_family: scheme_family_from_dsl(&dsl),
                    dsl,
                    score: score as u8,
                    alert_code: action["alert_code"]
                        .as_str()
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    recommended_action: parse_recommended_action(&recommended_action),
                    reason: action["reason"].as_str().unwrap_or("").to_string(),
                }
            })
            .collect();

        let audit_events = self.rule_audit_history(rule_id).await?;

        Ok(Some(RuleDetailRecord {
            summary,
            versions,
            audit_events,
        }))
    }

    async fn rule_audit_history(
        &self,
        rule_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        let rows: Vec<(String, String, String, String, String, String, Value, Value, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as(
                "SELECT audit_id, run_id, actor_role, event_type, event_status, summary, payload, evidence_refs, created_at
                 FROM audit_events
                 WHERE payload ->> 'rule_id' = $1
                 ORDER BY created_at, audit_id",
            )
            .bind(rule_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs,
                    created_at,
                )| AuditHistoryEventRecord {
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: Some(created_at.to_rfc3339()),
                },
            )
            .collect())
    }

    async fn save_rule_candidate(
        &self,
        rule: Rule,
        owner: String,
    ) -> anyhow::Result<RuleDetailRecord> {
        ensure_default_rules_seeded(&self.pool).await?;
        ensure_rule_condition_library_table(&self.pool).await?;
        let detail = rule_detail_from_rule(rule, "draft", owner);
        let mut tx = self.pool.begin().await?;
        let row: (String,) = sqlx::query_as(
            "INSERT INTO rules (rule_key, name, status, owner)
             VALUES ($1, $2, 'draft', $3)
             ON CONFLICT (rule_key) DO UPDATE
             SET name = EXCLUDED.name,
                 status = 'draft',
                 owner = EXCLUDED.owner,
                 updated_at = now()
             RETURNING id::text",
        )
        .bind(&detail.summary.rule_id)
        .bind(&detail.summary.name)
        .bind(&detail.summary.owner)
        .fetch_one(&mut *tx)
        .await?;

        let version = &detail.versions[0];
        sqlx::query(
            "INSERT INTO rule_versions
             (rule_id, version, dsl, score, recommended_action, created_by)
             VALUES ($1::uuid, $2, $3, $4, $5, $6)
             ON CONFLICT (rule_id, version) DO UPDATE
             SET dsl = EXCLUDED.dsl,
                 score = EXCLUDED.score,
                 recommended_action = EXCLUDED.recommended_action",
        )
        .bind(&row.0)
        .bind(version.version as i32)
        .bind(&version.dsl)
        .bind(version.score as i32)
        .bind(format!("{:?}", version.recommended_action))
        .bind(&detail.summary.owner)
        .execute(&mut *tx)
        .await?;

        upsert_rule_conditions_tx(&mut tx, &row.0, &detail).await?;

        tx.commit().await?;
        Ok(detail)
    }

    async fn update_rule_status(
        &self,
        rule_id: &str,
        status: &str,
    ) -> anyhow::Result<Option<RuleSummaryRecord>> {
        ensure_default_rules_seeded(&self.pool).await?;
        ensure_rule_condition_library_table(&self.pool).await?;
        let result =
            sqlx::query("UPDATE rules SET status = $1, updated_at = now() WHERE rule_key = $2")
                .bind(status)
                .bind(rule_id)
                .execute(&self.pool)
                .await?;
        if result.rows_affected() == 0 {
            return Ok(None);
        }
        sqlx::query(
            "UPDATE rule_condition_library
             SET status = $1, updated_at = now()
             WHERE source_rule_key = $2",
        )
        .bind(rule_condition_status(status))
        .bind(rule_id)
        .execute(&self.pool)
        .await?;
        Ok(self
            .list_rules()
            .await?
            .into_iter()
            .find(|rule| rule.rule_id == rule_id))
    }

    async fn list_rule_conditions(&self) -> anyhow::Result<Vec<RuleConditionLibraryRecord>> {
        ensure_default_rules_seeded(&self.pool).await?;
        ensure_rule_condition_library_table(&self.pool).await?;
        let rows: Vec<(
            String,
            String,
            i32,
            i32,
            String,
            String,
            Value,
            String,
            String,
            String,
            String,
            Value,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT condition_key, source_rule_key, source_rule_version, condition_index,
                    field_name, operator, value, review_mode, scheme_family, status, owner,
                    evidence_refs, created_at, updated_at
             FROM rule_condition_library
             ORDER BY source_rule_key, source_rule_version, condition_index",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    condition_key,
                    source_rule_key,
                    source_rule_version,
                    condition_index,
                    field,
                    operator,
                    value,
                    review_mode,
                    scheme_family,
                    status,
                    owner,
                    evidence_refs,
                    created_at,
                    updated_at,
                )| RuleConditionLibraryRecord {
                    condition_key,
                    source_rule_key,
                    source_rule_version: source_rule_version.max(0) as u32,
                    condition_index: condition_index.max(0) as u32,
                    field,
                    operator,
                    value,
                    review_mode,
                    scheme_family,
                    status,
                    owner,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: Some(created_at.to_rfc3339()),
                    updated_at: Some(updated_at.to_rfc3339()),
                },
            )
            .collect())
    }

    async fn rule_performance(&self) -> anyhow::Result<Vec<RulePerformanceRecord>> {
        ensure_default_rules_seeded(&self.pool).await?;
        let rules = self.list_rules().await?;
        let total_runs: (i64,) =
            sqlx::query_as("SELECT COUNT(*)::bigint FROM scoring_runs WHERE status = 'succeeded'")
                .fetch_one(&self.pool)
                .await?;

        let rule_run_rows: Vec<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT r.rule_key, rr.alert_code, c.external_claim_id
             FROM rule_runs rr
             JOIN scoring_runs sr ON sr.run_id = rr.run_id
             LEFT JOIN rules r ON r.id = rr.rule_id
             LEFT JOIN claims c ON c.id = sr.claim_id
             WHERE rr.matched = true",
        )
        .fetch_all(&self.pool)
        .await?;

        let outcome_rows: Vec<(String, bool, Option<Decimal>)> = sqlx::query_as(
            "SELECT claim_id, confirmed_fwa, saving_amount
             FROM investigation_results",
        )
        .fetch_all(&self.pool)
        .await?;
        let outcomes = outcome_rows
            .into_iter()
            .map(|(claim_id, confirmed_fwa, saving_amount)| {
                (
                    claim_id,
                    InvestigationOutcome {
                        confirmed_fwa,
                        saving_amount: saving_amount.unwrap_or(Decimal::ZERO),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        let alert_to_rule = rules
            .iter()
            .map(|rule| (rule.alert_code.clone(), rule.rule_id.clone()))
            .collect::<HashMap<_, _>>();
        let mut accumulators = rule_accumulators_from_rules(&rules);
        for (rule_id, alert_code, claim_id) in rule_run_rows {
            let rule_id = rule_id.or_else(|| {
                alert_code
                    .as_ref()
                    .and_then(|alert_code| alert_to_rule.get(alert_code).cloned())
            });
            let (Some(rule_id), Some(claim_id)) = (rule_id, claim_id) else {
                continue;
            };
            let Some(accumulator) = accumulators.get_mut(&rule_id) else {
                continue;
            };
            accumulator.trigger_count += 1;
            accumulator.triggered_claim_ids.insert(claim_id);
        }

        Ok(rule_performance_records(
            accumulators,
            &outcomes,
            total_runs.0.max(0) as u32,
        ))
    }

    async fn save_rule_backtest(
        &self,
        record: RuleBacktestRecord,
    ) -> anyhow::Result<RuleBacktestRecord> {
        let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
            "INSERT INTO rule_backtest_runs
             (rule_id, rule_version, sample_count, matched_count, reviewed_count,
              confirmed_fwa_count, false_positive_count, precision_value, recall_value,
              lift, false_positive_rate, estimated_saving, promotion_recommendation,
              blockers, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             RETURNING created_at",
        )
        .bind(&record.rule_id)
        .bind(record.rule_version as i32)
        .bind(record.sample_count as i32)
        .bind(record.matched_count as i32)
        .bind(record.reviewed_count as i32)
        .bind(record.confirmed_fwa_count as i32)
        .bind(record.false_positive_count as i32)
        .bind(record.precision)
        .bind(record.recall)
        .bind(record.lift)
        .bind(record.false_positive_rate)
        .bind(&record.estimated_saving)
        .bind(&record.promotion_recommendation)
        .bind(serde_json::json!(record.blockers))
        .bind(serde_json::json!(record.evidence_refs))
        .fetch_one(&self.pool)
        .await?;
        Ok(RuleBacktestRecord {
            created_at: Some(row.0.to_rfc3339()),
            ..record
        })
    }

    async fn latest_rule_backtest(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleBacktestRecord>> {
        let row: Option<(
            String,
            i32,
            i32,
            i32,
            i32,
            i32,
            i32,
            f64,
            f64,
            f64,
            f64,
            String,
            String,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT rule_id, rule_version, sample_count, matched_count, reviewed_count,
                    confirmed_fwa_count, false_positive_count, precision_value, recall_value,
                    lift, false_positive_rate, estimated_saving, promotion_recommendation,
                    blockers, evidence_refs, created_at
             FROM rule_backtest_runs
             WHERE rule_id = $1 AND rule_version = $2
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
        )
        .bind(rule_id)
        .bind(rule_version as i32)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                rule_id,
                rule_version,
                sample_count,
                matched_count,
                reviewed_count,
                confirmed_fwa_count,
                false_positive_count,
                precision,
                recall,
                lift,
                false_positive_rate,
                estimated_saving,
                promotion_recommendation,
                blockers,
                evidence_refs,
                created_at,
            )| RuleBacktestRecord {
                rule_id,
                rule_version: rule_version as u32,
                sample_count: sample_count.max(0) as u32,
                matched_count: matched_count.max(0) as u32,
                reviewed_count: reviewed_count.max(0) as u32,
                confirmed_fwa_count: confirmed_fwa_count.max(0) as u32,
                false_positive_count: false_positive_count.max(0) as u32,
                precision,
                recall,
                lift,
                false_positive_rate,
                estimated_saving,
                promotion_recommendation,
                blockers: json_array_to_strings(blockers),
                evidence_refs: json_array_to_strings(evidence_refs),
                created_at: Some(created_at.to_rfc3339()),
            },
        ))
    }

    async fn save_rule_shadow_run(
        &self,
        record: RuleShadowRunRecord,
    ) -> anyhow::Result<RuleShadowRunRecord> {
        let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
            "INSERT INTO rule_shadow_runs
             (rule_id, rule_version, report_uri, decision, reviewer, notes,
              reviewed_count, matched_count, false_positive_count, false_positive_rate,
              blockers, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             RETURNING created_at",
        )
        .bind(&record.rule_id)
        .bind(record.rule_version as i32)
        .bind(&record.report_uri)
        .bind(&record.decision)
        .bind(&record.reviewer)
        .bind(&record.notes)
        .bind(record.reviewed_count as i32)
        .bind(record.matched_count as i32)
        .bind(record.false_positive_count as i32)
        .bind(record.false_positive_rate)
        .bind(serde_json::json!(record.blockers.clone()))
        .bind(serde_json::json!(record.evidence_refs.clone()))
        .fetch_one(&self.pool)
        .await?;
        Ok(RuleShadowRunRecord {
            created_at: Some(row.0.to_rfc3339()),
            ..record
        })
    }

    async fn latest_rule_shadow_run(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleShadowRunRecord>> {
        let row: Option<(
            String,
            i32,
            String,
            String,
            String,
            String,
            i32,
            i32,
            i32,
            f64,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT rule_id, rule_version, report_uri, decision, reviewer, notes,
                    reviewed_count, matched_count, false_positive_count, false_positive_rate,
                    blockers, evidence_refs, created_at
             FROM rule_shadow_runs
             WHERE rule_id = $1 AND rule_version = $2
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
        )
        .bind(rule_id)
        .bind(rule_version as i32)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                rule_id,
                rule_version,
                report_uri,
                decision,
                reviewer,
                notes,
                reviewed_count,
                matched_count,
                false_positive_count,
                false_positive_rate,
                blockers,
                evidence_refs,
                created_at,
            )| RuleShadowRunRecord {
                rule_id,
                rule_version: rule_version as u32,
                report_uri,
                decision,
                reviewer,
                notes,
                reviewed_count: reviewed_count.max(0) as u32,
                matched_count: matched_count.max(0) as u32,
                false_positive_count: false_positive_count.max(0) as u32,
                false_positive_rate,
                blockers: json_array_to_strings(blockers),
                evidence_refs: json_array_to_strings(evidence_refs),
                created_at: Some(created_at.to_rfc3339()),
            },
        ))
    }

    async fn save_rule_promotion_review(
        &self,
        record: RulePromotionReviewRecord,
    ) -> anyhow::Result<RulePromotionReviewRecord> {
        let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
            "INSERT INTO rule_promotion_reviews
             (rule_id, rule_version, decision, reviewer, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING created_at",
        )
        .bind(&record.rule_id)
        .bind(record.rule_version as i32)
        .bind(&record.decision)
        .bind(&record.reviewer)
        .bind(&record.notes)
        .bind(serde_json::json!(record.evidence_refs.clone()))
        .fetch_one(&self.pool)
        .await?;
        Ok(RulePromotionReviewRecord {
            created_at: Some(row.0.to_rfc3339()),
            ..record
        })
    }

    async fn latest_rule_promotion_review(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RulePromotionReviewRecord>> {
        let row: Option<(
            String,
            i32,
            String,
            String,
            String,
            serde_json::Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT rule_id, rule_version, decision, reviewer, notes, evidence_refs, created_at
                 FROM rule_promotion_reviews
                 WHERE rule_id = $1 AND rule_version = $2
                 ORDER BY created_at DESC
                 LIMIT 1",
        )
        .bind(rule_id)
        .bind(rule_version as i32)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(rule_id, rule_version, decision, reviewer, notes, evidence_refs, created_at)| {
                RulePromotionReviewRecord {
                    rule_id,
                    rule_version: rule_version as u32,
                    decision,
                    reviewer,
                    notes,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: Some(created_at.to_rfc3339()),
                }
            },
        ))
    }

    async fn list_leads(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<LeadRecord>> {
        load_leads(&self.pool, customer_scope_id).await
    }

    async fn triage_lead(
        &self,
        lead_id: &str,
        input: TriageLeadInput,
    ) -> anyhow::Result<Option<TriageLeadRecord>> {
        let mut tx = self.pool.begin().await?;
        let lead = load_lead_in_tx(&mut tx, lead_id, input.customer_scope_id.as_deref()).await?;
        let Some(mut lead) = lead else {
            return Ok(None);
        };
        if input.decision == "merge_lead"
            && merge_target_lead_in_tx(&mut tx, &input).await?.is_none()
        {
            return Ok(None);
        }
        lead.status = triage_status_for_decision(&input.decision).into();
        lead.disposition = triage_disposition_for_decision(&input.decision).into();
        let case = (input.decision == "open_case").then(|| case_from_lead(&lead, &input));
        sqlx::query(
            "UPDATE fwa_leads
             SET status = $2, disposition = $3, updated_at = now()
             WHERE lead_id = $1",
        )
        .bind(&lead.lead_id)
        .bind(&lead.status)
        .bind(&lead.disposition)
        .execute(&mut *tx)
        .await?;
        if let Some(case) = &case {
            sqlx::query(
                "INSERT INTO investigation_cases
                 (case_id, lead_id, claim_id, member_id, provider_id, source_system, review_mode, scheme_family, lead_source, status, assignee, reviewer, priority, routing_reason, evidence_package_json)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                 ON CONFLICT (case_id) DO UPDATE
                 SET status = EXCLUDED.status,
                     review_mode = EXCLUDED.review_mode,
                     assignee = EXCLUDED.assignee,
                     reviewer = EXCLUDED.reviewer,
                     priority = EXCLUDED.priority,
                     routing_reason = EXCLUDED.routing_reason,
                     evidence_package_json = EXCLUDED.evidence_package_json,
                     updated_at = now()",
            )
            .bind(&case.case_id)
            .bind(&case.lead_id)
            .bind(&case.claim_id)
            .bind(&case.member_id)
            .bind(&case.provider_id)
            .bind(&case.source_system)
            .bind(&case.review_mode)
            .bind(&case.scheme_family)
            .bind(&case.lead_source)
            .bind(&case.status)
            .bind(&case.assignee)
            .bind(&case.reviewer)
            .bind(&case.priority)
            .bind(&case.routing_reason)
            .bind(&case.evidence_package)
            .execute(&mut *tx)
            .await?;
        }

        let audit_id = AuditEventId::new().to_string();
        insert_audit_event(
            &mut tx,
            &PersistedAuditEvent {
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
            },
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(Some(TriageLeadRecord {
            lead,
            case,
            audit_id,
        }))
    }

    async fn list_cases(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<CaseRecord>> {
        load_cases(&self.pool, customer_scope_id).await
    }

    async fn update_case_status(
        &self,
        case_id: &str,
        input: UpdateCaseStatusInput,
    ) -> anyhow::Result<Option<UpdateCaseStatusRecord>> {
        let mut tx = self.pool.begin().await?;
        let case = load_case_in_tx(&mut tx, case_id, input.customer_scope_id.as_deref()).await?;
        let Some(mut case) = case else {
            return Ok(None);
        };
        let audit_run_id =
            load_lead_in_tx(&mut tx, &case.lead_id, input.customer_scope_id.as_deref())
                .await?
                .map(|lead| lead.run_id)
                .unwrap_or_else(|| format!("case_status_{}", case.case_id));
        let from_status = case.status.clone();
        case.status = input.status.clone();
        sqlx::query(
            "UPDATE investigation_cases
             SET status = $2, updated_at = now()
             WHERE case_id = $1",
        )
        .bind(&case.case_id)
        .bind(&case.status)
        .execute(&mut *tx)
        .await?;
        let case = load_case_in_tx(&mut tx, case_id, input.customer_scope_id.as_deref())
            .await?
            .expect("case should exist after status update");

        let audit_id = AuditEventId::new().to_string();
        insert_audit_event(
            &mut tx,
            &PersistedAuditEvent {
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
            },
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(Some(UpdateCaseStatusRecord { case, audit_id }))
    }

    async fn create_audit_sample(
        &self,
        input: CreateAuditSampleInput,
    ) -> anyhow::Result<AuditSampleRecord> {
        let sample_id = format!("sample_{}", AuditEventId::new());
        let customer_scope_filter = input.customer_scope_id.clone();
        let customer_scope_id = customer_scope_filter.as_deref();
        let leads = if input.sample_mode == "random_control" {
            load_control_audit_population(&self.pool, customer_scope_id).await?
        } else {
            self.list_leads(customer_scope_id).await?
        };
        let strata_contexts = load_audit_sample_strata_contexts(&self.pool).await?;
        let existing_samples = self.list_audit_samples(customer_scope_id).await?;
        let reviewer_history =
            reviewer_lead_sample_counts(existing_samples.iter(), &input.reviewer);
        let sample = build_audit_sample(
            sample_id,
            input,
            leads,
            &strata_contexts,
            &reviewer_history,
            None,
        );
        sqlx::query(
            "INSERT INTO audit_samples
             (sample_id, customer_scope_id, sample_mode, population_definition, inclusion_criteria_json, deterministic_seed, selection_method, sample_size, reviewer, assignment_queue, selected_leads_json, outcome_distribution_json)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(&sample.sample_id)
        .bind(&sample.customer_scope_id)
        .bind(&sample.sample_mode)
        .bind(&sample.population_definition)
        .bind(&sample.inclusion_criteria)
        .bind(&sample.deterministic_seed)
        .bind(&sample.selection_method)
        .bind(sample.sample_size as i32)
        .bind(&sample.reviewer)
        .bind(&sample.assignment_queue)
        .bind(serde_json::to_value(&sample.selected_leads)?)
        .bind(&sample.outcome_distribution)
        .execute(&self.pool)
        .await?;
        self.list_audit_samples(customer_scope_id)
            .await?
            .into_iter()
            .find(|record| record.sample_id == sample.sample_id)
            .ok_or_else(|| anyhow::anyhow!("created audit sample was not found"))
    }

    async fn list_audit_samples(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditSampleRecord>> {
        let rows: Vec<(
            String,
            String,
            String,
            String,
            Value,
            Option<String>,
            String,
            i32,
            String,
            String,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT sample_id, customer_scope_id, sample_mode, population_definition, inclusion_criteria_json, deterministic_seed, selection_method, sample_size, reviewer, assignment_queue, selected_leads_json, outcome_distribution_json, created_at
             FROM audit_samples
             WHERE ($1::text IS NULL OR customer_scope_id = $1)
             ORDER BY created_at, sample_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        let samples = rows
            .into_iter()
            .map(
                |(
                    sample_id,
                    customer_scope_id,
                    sample_mode,
                    population_definition,
                    inclusion_criteria,
                    deterministic_seed,
                    selection_method,
                    sample_size,
                    reviewer,
                    assignment_queue,
                    selected_leads,
                    outcome_distribution,
                    created_at,
                )| AuditSampleRecord {
                    sample_id,
                    customer_scope_id,
                    sample_mode,
                    population_definition,
                    inclusion_criteria,
                    deterministic_seed,
                    selection_method,
                    sample_size: sample_size.max(0) as usize,
                    reviewer,
                    assignment_queue,
                    selected_leads: serde_json::from_value(selected_leads).unwrap_or_default(),
                    outcome_distribution,
                    created_at: Some(created_at.to_rfc3339()),
                },
            )
            .collect::<Vec<_>>();
        let reviews = self.list_qa_reviews(customer_scope_id).await?;
        Ok(with_sample_outcome_distributions(samples, &reviews))
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelVersionRecord>> {
        ensure_default_models_seeded(&self.pool).await?;
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT model_key, version, model_type, runtime_kind, execution_provider, status, COALESCE(metrics ->> 'review_mode', 'both'), artifact_uri, endpoint_url
             FROM model_versions
             ORDER BY model_key, version DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    model_key,
                    version,
                    model_type,
                    runtime_kind,
                    execution_provider,
                    status,
                    review_mode,
                    artifact_uri,
                    endpoint_url,
                )| ModelVersionRecord {
                    model_key,
                    version,
                    model_type,
                    runtime_kind,
                    execution_provider,
                    status,
                    review_mode: normalize_review_mode(&review_mode),
                    artifact_uri,
                    endpoint_url,
                },
            )
            .collect())
    }

    async fn save_model_version(
        &self,
        record: ModelVersionRecord,
    ) -> anyhow::Result<ModelVersionRecord> {
        sqlx::query(
            "INSERT INTO model_versions
             (model_key, version, model_type, runtime_kind, artifact_uri, endpoint_url, execution_provider, status, metrics, activated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, CASE WHEN $8 = 'active' THEN now() ELSE NULL END)
             ON CONFLICT (model_key, version) DO UPDATE
             SET model_type = EXCLUDED.model_type,
                 runtime_kind = EXCLUDED.runtime_kind,
                 artifact_uri = EXCLUDED.artifact_uri,
                 endpoint_url = EXCLUDED.endpoint_url,
                 execution_provider = EXCLUDED.execution_provider,
                 status = EXCLUDED.status,
                 metrics = model_versions.metrics || EXCLUDED.metrics,
                 activated_at = CASE WHEN EXCLUDED.status = 'active' THEN now() ELSE model_versions.activated_at END",
        )
        .bind(&record.model_key)
        .bind(&record.version)
        .bind(&record.model_type)
        .bind(&record.runtime_kind)
        .bind(&record.artifact_uri)
        .bind(&record.endpoint_url)
        .bind(&record.execution_provider)
        .bind(&record.status)
        .bind(serde_json::json!({ "review_mode": record.review_mode }))
        .execute(&self.pool)
        .await?;
        Ok(record)
    }

    async fn update_model_status(
        &self,
        model_key: &str,
        model_version: &str,
        status: &str,
    ) -> anyhow::Result<Option<ModelVersionRecord>> {
        ensure_default_models_seeded(&self.pool).await?;
        let row: Option<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "UPDATE model_versions
             SET status = $3,
                 activated_at = CASE WHEN $3 = 'active' THEN now() ELSE NULL END
             WHERE model_key = $1 AND version = $2
             RETURNING model_key, version, model_type, runtime_kind, execution_provider, status, COALESCE(metrics ->> 'review_mode', 'both'), artifact_uri, endpoint_url",
        )
        .bind(model_key)
        .bind(model_version)
        .bind(status)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(
                model_key,
                version,
                model_type,
                runtime_kind,
                execution_provider,
                status,
                review_mode,
                artifact_uri,
                endpoint_url,
            )| ModelVersionRecord {
                model_key,
                version,
                model_type,
                runtime_kind,
                execution_provider,
                status,
                review_mode: normalize_review_mode(&review_mode),
                artifact_uri,
                endpoint_url,
            },
        ))
    }

    async fn model_performance(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Option<ModelPerformanceRecord>> {
        ensure_default_models_seeded(&self.pool).await?;
        let known = self
            .list_models()
            .await?
            .into_iter()
            .any(|model| model.model_key == model_key);
        if !known {
            return Ok(None);
        }

        let row: (
            i64,
            Option<Decimal>,
            Option<i64>,
            Option<chrono::DateTime<chrono::Utc>>,
        ) = sqlx::query_as(
            "SELECT
                   COUNT(*)::bigint,
                   AVG(score),
                   SUM(CASE WHEN score >= 70 THEN 1 ELSE 0 END)::bigint,
                   MAX(created_at)
                 FROM model_scores
                 WHERE model_key = $1",
        )
        .bind(model_key)
        .fetch_one(&self.pool)
        .await?;
        let drift_metrics: Option<(Value,)> = sqlx::query_as(
            "SELECT metrics_json
             FROM model_evaluation_runs
             WHERE model_key = $1
             ORDER BY created_at DESC, evaluation_run_id DESC
             LIMIT 1",
        )
        .bind(model_key)
        .fetch_optional(&self.pool)
        .await?;
        let drift = drift_summary(
            drift_metrics
                .as_ref()
                .map(|row| &row.0)
                .unwrap_or(&Value::Null),
        );

        let scored_runs = row.0 as u32;
        if scored_runs == 0 {
            return Ok(Some(model_performance_with_drift(
                empty_model_performance(model_key),
                drift,
            )));
        }

        Ok(Some(model_performance_with_drift(
            ModelPerformanceRecord {
                model_key: model_key.to_string(),
                data_status: "ready".into(),
                scored_runs,
                average_score: row
                    .1
                    .map(|value| value.to_string().parse().unwrap_or(0.0))
                    .unwrap_or(0.0),
                high_risk_count: row.2.unwrap_or(0) as u32,
                score_psi: None,
                drift_status: "not_available".into(),
                latest_scored_at: row.3.map(|timestamp| timestamp.to_rfc3339()),
            },
            drift,
        )))
    }

    async fn save_model_promotion_review(
        &self,
        record: ModelPromotionReviewRecord,
    ) -> anyhow::Result<ModelPromotionReviewRecord> {
        let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
            "INSERT INTO model_promotion_reviews
             (model_key, model_version, decision, reviewer, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING created_at",
        )
        .bind(&record.model_key)
        .bind(&record.model_version)
        .bind(&record.decision)
        .bind(&record.reviewer)
        .bind(&record.notes)
        .bind(serde_json::json!(record.evidence_refs.clone()))
        .fetch_one(&self.pool)
        .await?;
        Ok(ModelPromotionReviewRecord {
            created_at: Some(row.0.to_rfc3339()),
            ..record
        })
    }

    async fn latest_model_promotion_review(
        &self,
        model_key: &str,
        model_version: &str,
    ) -> anyhow::Result<Option<ModelPromotionReviewRecord>> {
        let row: Option<(
            String,
            String,
            String,
            String,
            String,
            serde_json::Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT model_key, model_version, decision, reviewer, notes, evidence_refs, created_at
                 FROM model_promotion_reviews
                 WHERE model_key = $1 AND model_version = $2
                 ORDER BY created_at DESC
                 LIMIT 1",
        )
        .bind(model_key)
        .bind(model_version)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(model_key, model_version, decision, reviewer, notes, evidence_refs, created_at)| {
                ModelPromotionReviewRecord {
                    model_key,
                    model_version,
                    decision,
                    reviewer,
                    notes,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: Some(created_at.to_rfc3339()),
                }
            },
        ))
    }

    async fn save_model_retraining_job(
        &self,
        record: ModelRetrainingJobRecord,
    ) -> anyhow::Result<ModelRetrainingJobRecord> {
        let row: (
            String,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        ) = sqlx::query_as(
            "INSERT INTO model_retraining_jobs
                 (model_key, model_version, status, requested_by, request_notes, status_note,
                  updated_by, readiness_recommendation, latest_evaluation_id, source_dataset_id,
                  source_data_quality_score, source_data_quality_status, trigger_summary_json,
                  blocker_summary_json)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                 RETURNING id::text, created_at, updated_at",
        )
        .bind(&record.model_key)
        .bind(&record.model_version)
        .bind(&record.status)
        .bind(&record.requested_by)
        .bind(&record.request_notes)
        .bind(&record.status_note)
        .bind(&record.updated_by)
        .bind(&record.readiness_recommendation)
        .bind(&record.latest_evaluation_id)
        .bind(&record.source_dataset_id)
        .bind(record.source_data_quality_score)
        .bind(&record.source_data_quality_status)
        .bind(serde_json::json!(record.trigger_summary))
        .bind(serde_json::json!(record.blocker_summary))
        .fetch_one(&self.pool)
        .await?;
        Ok(ModelRetrainingJobRecord {
            job_id: row.0,
            created_at: Some(row.1.to_rfc3339()),
            updated_at: Some(row.2.to_rfc3339()),
            ..record
        })
    }

    async fn list_model_retraining_jobs(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Vec<ModelRetrainingJobRecord>> {
        let rows = sqlx::query(
            "SELECT id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                    status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                    source_dataset_id, source_data_quality_score, source_data_quality_status,
                    trigger_summary_json, blocker_summary_json, candidate_model_version,
                    candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                    output_evaluation_id, created_at, updated_at
             FROM model_retraining_jobs
             WHERE model_key = $1
             ORDER BY created_at DESC",
        )
        .bind(model_key)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(model_retraining_job_from_pg_row)
            .collect())
    }

    async fn get_model_retraining_job(
        &self,
        job_id: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let row = sqlx::query(
            "SELECT id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                    status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                    source_dataset_id, source_data_quality_score, source_data_quality_status,
                    trigger_summary_json, blocker_summary_json, candidate_model_version,
                    candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                    output_evaluation_id, created_at, updated_at
             FROM model_retraining_jobs
             WHERE id = $1::uuid",
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(model_retraining_job_from_pg_row))
    }

    async fn claim_next_model_retraining_job(
        &self,
        model_key: Option<&str>,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let row = sqlx::query(
            "WITH next_job AS (
                 SELECT id
                 FROM model_retraining_jobs
                 WHERE status = 'queued'
                   AND ($3::text IS NULL OR model_key = $3)
                 ORDER BY created_at ASC
                 LIMIT 1
                 FOR UPDATE SKIP LOCKED
             )
             UPDATE model_retraining_jobs
             SET status = 'running', updated_by = $1, status_note = $2, updated_at = now()
             WHERE id = (SELECT id FROM next_job)
             RETURNING id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                       status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                       source_dataset_id, source_data_quality_score, source_data_quality_status,
                       trigger_summary_json, blocker_summary_json, candidate_model_version,
                       candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                       output_evaluation_id, created_at, updated_at",
        )
        .bind(actor)
        .bind(status_note)
        .bind(model_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(model_retraining_job_from_pg_row))
    }

    async fn update_model_retraining_job_status(
        &self,
        job_id: &str,
        status: &str,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let row = sqlx::query(
            "UPDATE model_retraining_jobs
             SET status = $2, updated_by = $3, status_note = $4, updated_at = now()
             WHERE id = $1::uuid
             RETURNING id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                       status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                       source_dataset_id, source_data_quality_score, source_data_quality_status,
                       trigger_summary_json, blocker_summary_json, candidate_model_version,
                       candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                       output_evaluation_id, created_at, updated_at",
        )
        .bind(job_id)
        .bind(status)
        .bind(actor)
        .bind(status_note)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(model_retraining_job_from_pg_row))
    }

    async fn complete_model_retraining_job(
        &self,
        input: CompleteModelRetrainingJobInput<'_>,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let row = sqlx::query(
            "UPDATE model_retraining_jobs
             SET status = 'completed',
                 updated_by = $2,
                 status_note = $3,
                 candidate_model_version = $4,
                 candidate_artifact_uri = $5,
                 candidate_endpoint_url = $6,
                 validation_report_uri = $7,
                 output_evaluation_id = $8,
                 updated_at = now()
             WHERE id = $1::uuid
             RETURNING id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                       status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                       source_dataset_id, source_data_quality_score, source_data_quality_status,
                       trigger_summary_json, blocker_summary_json, candidate_model_version,
                       candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                       output_evaluation_id, created_at, updated_at",
        )
        .bind(input.job_id)
        .bind(input.actor)
        .bind(input.status_note)
        .bind(input.candidate_model_version)
        .bind(input.candidate_artifact_uri)
        .bind(input.candidate_endpoint_url)
        .bind(input.validation_report_uri)
        .bind(input.output_evaluation_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(model_retraining_job_from_pg_row))
    }

    async fn dashboard_summary(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<DashboardSummaryRecord> {
        let suspected: (i64, Option<Decimal>) = sqlx::query_as(
            "SELECT COUNT(*)::bigint, COALESCE(SUM(c.claim_amount), 0)
             FROM scoring_runs sr
             LEFT JOIN claims c ON c.id = sr.claim_id
             WHERE sr.risk_score >= 70
               AND ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))",
        )
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;

        let rag_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT COALESCE(rag, 'UNKNOWN'), COUNT(*)::bigint
             FROM scoring_runs sr
             WHERE rag IS NOT NULL
               AND ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))
             GROUP BY rag
             ORDER BY rag",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;

        let rule_hits: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint
             FROM rule_runs rr
             JOIN scoring_runs sr ON sr.run_id = rr.run_id
             WHERE rr.matched = true
               AND ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))",
        )
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;

        let model_rows: Vec<(String, i64, Option<Decimal>, Option<i64>)> = sqlx::query_as(
            "SELECT model_key,
                    COUNT(*)::bigint,
                    AVG(score),
                    SUM(CASE WHEN score >= 70 THEN 1 ELSE 0 END)::bigint
             FROM model_scores ms
             JOIN scoring_runs sr ON sr.run_id = ms.run_id
             WHERE ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))
             GROUP BY model_key
             ORDER BY model_key",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;

        let layer_payloads: Vec<(Value,)> = sqlx::query_as(
            "SELECT payload
             FROM audit_events
             WHERE event_type = 'scoring.completed'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        let audit_coverage_row: (i64, Option<i64>) = sqlx::query_as(
            "SELECT COUNT(*)::bigint,
                    SUM(
                        CASE
                            WHEN jsonb_typeof(payload->'canonical_claim_context_trace') = 'object'
                            THEN 1
                            ELSE 0
                        END
                    )::bigint
             FROM audit_events
             WHERE event_type = 'scoring.completed'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)",
        )
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;
        let audit_coverage = summarize_dashboard_audit_coverage(
            audit_coverage_row.0 as u32,
            audit_coverage_row.1.unwrap_or(0) as u32,
        );
        let mut layer_accumulators = BTreeMap::<String, (String, u32, u32, u32)>::new();
        for (payload,) in layer_payloads {
            for layer in payload
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

        let investigation: (i64, i64, Option<Decimal>) = sqlx::query_as(
            "SELECT COUNT(*)::bigint,
                    COALESCE(SUM(CASE WHEN confirmed_fwa THEN 1 ELSE 0 END), 0)::bigint,
                    COALESCE(SUM(saving_amount), 0)
             FROM investigation_results ir
             WHERE ($1::text IS NULL OR EXISTS (
               SELECT 1 FROM audit_events ae
               WHERE ae.event_type = 'investigation.result.received'
                 AND ae.event_status = 'succeeded'
                 AND ae.payload ->> 'investigation_id' = ir.investigation_id
                 AND ae.payload ->> 'customer_scope_id' = $1
             ))",
        )
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;

        let qa_reviews: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint
             FROM qa_reviews qr
             WHERE ($1::text IS NULL OR EXISTS (
               SELECT 1 FROM audit_events ae
               WHERE ae.event_type = 'qa.result.received'
                 AND ae.event_status = 'succeeded'
                 AND ae.payload ->> 'qa_case_id' = qr.qa_case_id
                 AND ae.payload ->> 'customer_scope_id' = $1
             ))",
        )
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;

        let scheme_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT scheme_family, COUNT(*)::bigint
             FROM fwa_leads l
             WHERE ($1::text IS NULL OR EXISTS (
               SELECT 1 FROM audit_events ae
               WHERE ae.run_id = l.run_id
                 AND ae.event_type = 'scoring.completed'
                 AND ae.event_status = 'succeeded'
                 AND ae.payload ->> 'customer_scope_id' = $1
             ))
             GROUP BY scheme_family
             ORDER BY scheme_family",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;

        let financial_impact_rows: Vec<(bool, Option<String>, Option<Decimal>, Option<String>)> =
            sqlx::query_as(
                "SELECT confirmed_fwa, financial_impact_type, saving_amount, currency
                 FROM investigation_results ir
                 WHERE ($1::text IS NULL OR EXISTS (
                   SELECT 1 FROM audit_events ae
                   WHERE ae.event_type = 'investigation.result.received'
                     AND ae.event_status = 'succeeded'
                     AND ae.payload ->> 'investigation_id' = ir.investigation_id
                     AND ae.payload ->> 'customer_scope_id' = $1
                 ))
                 ORDER BY created_at, investigation_id",
            )
            .bind(customer_scope_id)
            .fetch_all(&self.pool)
            .await?;
        let financial_impacts = financial_impact_rows
            .into_iter()
            .filter_map(
                |(confirmed_fwa, financial_impact_type, saving_amount, currency)| {
                    financial_impact_from_parts(
                        confirmed_fwa,
                        financial_impact_type.as_deref(),
                        saving_amount,
                        currency,
                    )
                },
            )
            .collect::<Vec<_>>();

        let saving_attributions: Vec<(
            String,
            String,
            String,
            String,
            Option<Decimal>,
            String,
            i64,
            Vec<String>,
        )> = sqlx::query_as(
            "SELECT source_type,
                        source_id,
                        financial_impact_type,
                        action,
                        COALESCE(SUM(saving_amount), 0),
                        currency,
                        COUNT(DISTINCT claim_id)::bigint,
                        ARRAY_REMOVE(ARRAY_AGG(DISTINCT ref.value ORDER BY ref.value), NULL)
                 FROM saving_attributions s
                 LEFT JOIN LATERAL jsonb_array_elements_text(s.evidence_refs) AS ref(value) ON TRUE
                 WHERE ($1::text IS NULL OR EXISTS (
                   SELECT 1 FROM audit_events ae
                   WHERE ae.event_type = 'investigation.result.received'
                     AND ae.event_status = 'succeeded'
                     AND ae.payload ->> 'investigation_id' = s.investigation_id
                     AND ae.payload ->> 'customer_scope_id' = $1
                 ))
                 GROUP BY source_type, source_id, financial_impact_type, action, currency
                 ORDER BY source_type, source_id, financial_impact_type, action, currency",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        let saving_segments: Vec<(String, String, Option<Decimal>, String, i64, i64)> =
            sqlx::query_as(
                "SELECT segment_type,
                        segment_id,
                        COALESCE(SUM(saving_amount), 0),
                        currency,
                        COUNT(DISTINCT claim_id)::bigint,
                        COUNT(*)::bigint
                 FROM (
                   SELECT 'provider'::text AS segment_type,
                          COALESCE(l.provider_id, 'unknown') AS segment_id,
                          s.saving_amount,
                          s.currency,
                          s.claim_id
                   FROM saving_attributions s
                   LEFT JOIN fwa_leads l ON l.claim_id = s.claim_id
                   WHERE ($1::text IS NULL OR EXISTS (
                     SELECT 1 FROM audit_events ae
                     WHERE ae.event_type = 'investigation.result.received'
                       AND ae.event_status = 'succeeded'
                       AND ae.payload ->> 'investigation_id' = s.investigation_id
                       AND ae.payload ->> 'customer_scope_id' = $1
                   ))
                   UNION ALL
                   SELECT 'scheme'::text AS segment_type,
                          COALESCE(l.scheme_family, 'unknown') AS segment_id,
                          s.saving_amount,
                          s.currency,
                          s.claim_id
                   FROM saving_attributions s
                   LEFT JOIN fwa_leads l ON l.claim_id = s.claim_id
                   WHERE ($1::text IS NULL OR EXISTS (
                     SELECT 1 FROM audit_events ae
                     WHERE ae.event_type = 'investigation.result.received'
                       AND ae.event_status = 'succeeded'
                       AND ae.payload ->> 'investigation_id' = s.investigation_id
                       AND ae.payload ->> 'customer_scope_id' = $1
                   ))
                   UNION ALL
                   SELECT 'campaign'::text AS segment_type,
                          COALESCE(NULLIF(regexp_replace(ref.value, '^campaigns?:', ''), ''), 'unknown') AS segment_id,
                          s.saving_amount,
                          s.currency,
                          s.claim_id
                   FROM saving_attributions s
                   CROSS JOIN LATERAL jsonb_array_elements_text(s.evidence_refs) AS ref(value)
                   WHERE (ref.value LIKE 'campaign:%'
                      OR ref.value LIKE 'campaigns:%')
                     AND ($1::text IS NULL OR EXISTS (
                       SELECT 1 FROM audit_events ae
                       WHERE ae.event_type = 'investigation.result.received'
                         AND ae.event_status = 'succeeded'
                         AND ae.payload ->> 'investigation_id' = s.investigation_id
                         AND ae.payload ->> 'customer_scope_id' = $1
                     ))
                 ) segments
                 GROUP BY segment_type, segment_id, currency
                 ORDER BY segment_type, segment_id, currency",
            )
            .bind(customer_scope_id)
            .fetch_all(&self.pool)
            .await?;
        let outcome_labels = self.list_outcome_labels(customer_scope_id).await?;
        let audit_samples = self.list_audit_samples(customer_scope_id).await?;
        let qa_review_records = self.list_qa_reviews(customer_scope_id).await?;
        let qa_feedback_items = self.list_qa_feedback_items(customer_scope_id).await?;
        let agent_runs = self.list_agent_runs(customer_scope_id).await?;
        let models = self.list_models().await?;
        let model_evaluations = self.list_model_evaluations().await?;
        let rules = self.list_rules().await?;
        let rule_performance = self.rule_performance().await?;

        Ok(DashboardSummaryRecord {
            suspected_claims: suspected.0 as u32,
            confirmed_fwa: investigation.1 as u32,
            risk_amount: suspected.1.unwrap_or(Decimal::ZERO).to_string(),
            saving_amount: investigation.2.unwrap_or(Decimal::ZERO).to_string(),
            rag_distribution: rag_rows
                .into_iter()
                .map(|(rag, count)| (rag, count as u32))
                .collect(),
            scheme_distribution: scheme_rows
                .into_iter()
                .map(|(scheme_family, count)| (scheme_family, count as u32))
                .collect(),
            rule_hits: rule_hits.0 as u32,
            model_scores: model_rows
                .into_iter()
                .map(|(model_key, scored_runs, average_score, high_risk_count)| {
                    (
                        model_key,
                        DashboardModelScoreRecord {
                            scored_runs: scored_runs as u32,
                            average_score: average_score
                                .map(|value| value.to_string().parse().unwrap_or(0.0))
                                .unwrap_or(0.0),
                            high_risk_count: high_risk_count.unwrap_or(0) as u32,
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
            saving_attributions: saving_attributions
                .into_iter()
                .map(
                    |(
                        source_type,
                        source_id,
                        financial_impact_type,
                        action,
                        saving_amount,
                        currency,
                        claim_count,
                        evidence_refs,
                    )| {
                        DashboardSavingAttributionRecord {
                            source_type,
                            source_id,
                            financial_impact_type,
                            action,
                            saving_amount: format_decimal_cents(
                                saving_amount.unwrap_or(Decimal::ZERO),
                            ),
                            currency,
                            claim_count: claim_count as u32,
                            evidence_refs,
                        }
                    },
                )
                .collect(),
            saving_segments: saving_segments
                .into_iter()
                .map(
                    |(
                        segment_type,
                        segment_id,
                        saving_amount,
                        currency,
                        claim_count,
                        attribution_count,
                    )| {
                        let saving_amount = saving_amount.unwrap_or(Decimal::ZERO);
                        let claim_count = claim_count as u32;
                        DashboardSavingSegmentRecord {
                            segment_type,
                            segment_id,
                            saving_amount: format_decimal_cents(saving_amount),
                            currency,
                            claim_count,
                            attribution_count: attribution_count as u32,
                            roi: segment_roi(saving_amount, claim_count),
                        }
                    },
                )
                .collect(),
            value_measurement: summarize_dashboard_value_measurement(
                &financial_impacts,
                rule_hits.0 as u32,
                rule_performance
                    .iter()
                    .map(|record| record.false_positive_count)
                    .sum::<u32>(),
            ),
            audit_coverage,
            label_pool: summarize_dashboard_label_pool(&outcome_labels),
            qa_queue: summarize_dashboard_qa_queue(
                &audit_samples,
                &qa_review_records,
                &qa_feedback_items,
            ),
            case_sla: summarize_dashboard_case_sla(&self.list_cases(customer_scope_id).await?),
            agent_governance: summarize_dashboard_agent_governance(&agent_runs),
            model_governance: summarize_dashboard_model_governance(&models, &model_evaluations),
            rule_governance: summarize_dashboard_rule_governance(&rules, &rule_performance),
            investigation_results: investigation.0 as u32,
            qa_reviews: qa_reviews.0 as u32,
        })
    }

    async fn provider_risk_summary(&self) -> anyhow::Result<ProviderRiskSummaryRecord> {
        let rows: Vec<(Value,)> = sqlx::query_as(
            "SELECT payload
             FROM audit_events
             WHERE event_type = 'scoring.completed'
               AND event_status = 'succeeded'",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(summarize_provider_risk_profiles(
            rows.iter().map(|(payload,)| payload),
        ))
    }

    async fn list_knowledge_cases(&self) -> anyhow::Result<Vec<KnowledgeCaseRecord>> {
        ensure_default_knowledge_cases_seeded(&self.pool).await?;
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            Value,
        )> = sqlx::query_as(
            "SELECT case_id, title, fwa_type, scheme_family, diagnosis_code, provider_region, provider_type, summary, outcome, tags, evidence_refs
             FROM knowledge_cases
             ORDER BY case_id",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    case_id,
                    title,
                    fwa_type,
                    scheme_family,
                    diagnosis_code,
                    provider_region,
                    provider_type,
                    summary,
                    outcome,
                    tags,
                    evidence_refs,
                )| KnowledgeCaseRecord {
                    case_id,
                    title,
                    fwa_type,
                    scheme_family,
                    diagnosis_code,
                    provider_region,
                    provider_type,
                    summary,
                    outcome,
                    tags: json_array_to_strings(tags),
                    evidence_refs: json_array_to_strings(evidence_refs),
                },
            )
            .collect())
    }

    async fn save_knowledge_case(
        &self,
        record: KnowledgeCaseRecord,
    ) -> anyhow::Result<KnowledgeCaseRecord> {
        sqlx::query(
            "INSERT INTO knowledge_cases
             (case_id, title, fwa_type, scheme_family, diagnosis_code, provider_region, provider_type, summary, outcome, tags, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (case_id) DO UPDATE
             SET title = EXCLUDED.title,
                 fwa_type = EXCLUDED.fwa_type,
                 scheme_family = EXCLUDED.scheme_family,
                 diagnosis_code = EXCLUDED.diagnosis_code,
                 provider_region = EXCLUDED.provider_region,
                 provider_type = EXCLUDED.provider_type,
                 summary = EXCLUDED.summary,
                 outcome = EXCLUDED.outcome,
                 tags = EXCLUDED.tags,
                 evidence_refs = EXCLUDED.evidence_refs,
                 updated_at = now()",
        )
        .bind(&record.case_id)
        .bind(&record.title)
        .bind(&record.fwa_type)
        .bind(&record.scheme_family)
        .bind(&record.diagnosis_code)
        .bind(&record.provider_region)
        .bind(&record.provider_type)
        .bind(&record.summary)
        .bind(&record.outcome)
        .bind(serde_json::json!(record.tags))
        .bind(serde_json::json!(record.evidence_refs))
        .execute(&self.pool)
        .await?;
        Ok(record)
    }

    async fn search_similar_cases(
        &self,
        query: SimilarCaseQuery,
    ) -> anyhow::Result<Vec<SimilarCaseRecord>> {
        let cases = self.list_knowledge_cases().await?;
        Ok(search_cases(cases, &query))
    }

    async fn save_agent_run(&self, run: PersistedAgentRun) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO agent_runs
             (agent_run_id, claim_id, status, decision_boundary, output_json, evidence_refs, completed_at)
             VALUES ($1, $2, $3, $4, $5, $6, now())
             ON CONFLICT (agent_run_id) DO UPDATE
             SET status = EXCLUDED.status,
                 decision_boundary = EXCLUDED.decision_boundary,
                 output_json = EXCLUDED.output_json,
                 evidence_refs = EXCLUDED.evidence_refs,
                 completed_at = EXCLUDED.completed_at",
        )
        .bind(&run.agent_run_id)
        .bind(&run.claim_id)
        .bind(&run.status)
        .bind(&run.decision_boundary)
        .bind(&run.output_json)
        .bind(Value::Array(run.evidence_refs.clone()))
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM agent_steps WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM agent_context_snapshots WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM tool_results WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM agent_policy_checks WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM tool_calls WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM agent_approvals WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;

        for step in &run.steps {
            sqlx::query(
                "INSERT INTO agent_steps
                 (agent_run_id, step_name, status, output_json, evidence_refs)
                 VALUES ($1, $2, 'succeeded', $3, $4)",
            )
            .bind(&run.agent_run_id)
            .bind(step["step_name"].as_str().unwrap_or("investigate"))
            .bind(step)
            .bind(step["evidence_refs"].clone())
            .execute(&mut *tx)
            .await?;
        }
        for snapshot in &run.context_snapshots {
            sqlx::query(
                "INSERT INTO agent_context_snapshots
                 (snapshot_id, agent_run_id, redaction_status, context_json, source_refs, checksum)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&snapshot.snapshot_id)
            .bind(&run.agent_run_id)
            .bind(&snapshot.redaction_status)
            .bind(&snapshot.context_json)
            .bind(string_values(&snapshot.source_refs))
            .bind(&snapshot.checksum)
            .execute(&mut *tx)
            .await?;
        }
        for call in &run.tool_calls {
            sqlx::query(
                "INSERT INTO tool_calls
                 (tool_call_id, agent_run_id, tool_name, status, input_json, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&call.tool_call_id)
            .bind(&run.agent_run_id)
            .bind(&call.tool_name)
            .bind(&call.status)
            .bind(&call.input_json)
            .bind(string_values(&call.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }
        for check in &run.policy_checks {
            sqlx::query(
                "INSERT INTO agent_policy_checks
                 (policy_check_id, agent_run_id, tool_call_id, tool_name, policy_name, decision, reason, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(&check.policy_check_id)
            .bind(&run.agent_run_id)
            .bind(&check.tool_call_id)
            .bind(&check.tool_name)
            .bind(&check.policy_name)
            .bind(&check.decision)
            .bind(&check.reason)
            .bind(string_values(&check.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }
        for result in &run.tool_results {
            sqlx::query(
                "INSERT INTO tool_results
                 (tool_result_id, tool_call_id, agent_run_id, tool_name, status, output_json, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&result.tool_result_id)
            .bind(&result.tool_call_id)
            .bind(&run.agent_run_id)
            .bind(&result.tool_name)
            .bind(&result.status)
            .bind(&result.output_json)
            .bind(string_values(&result.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }
        for approval in &run.approvals {
            sqlx::query(
                "INSERT INTO agent_approvals
                 (approval_id, agent_run_id, proposed_action, decision, approver, reason, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&approval.approval_id)
            .bind(&run.agent_run_id)
            .bind(&approval.proposed_action)
            .bind(&approval.decision)
            .bind(&approval.approver)
            .bind(&approval.reason)
            .bind(string_values(&approval.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn list_agent_runs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AgentRunLogRecord>> {
        let rows: Vec<(
            String,
            String,
            String,
            String,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
            Option<chrono::DateTime<chrono::Utc>>,
        )> = sqlx::query_as(
            "SELECT agent_run_id, claim_id, status, decision_boundary, output_json, evidence_refs, created_at, completed_at
             FROM agent_runs ar
             WHERE (
               $1::text IS NULL OR EXISTS (
                 SELECT 1
                 FROM audit_events ae
                 LEFT JOIN claims c ON c.id = ae.claim_id
                 WHERE ae.payload ->> 'customer_scope_id' = $1
                   AND (
                     ae.payload ->> 'claim_id' = ar.claim_id
                     OR c.external_claim_id = ar.claim_id
                     OR ae.claim_id::text = ar.claim_id
                   )
               )
             )
             ORDER BY created_at DESC, agent_run_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;

        let mut runs = Vec::with_capacity(rows.len());
        for (
            agent_run_id,
            claim_id,
            status,
            decision_boundary,
            output_json,
            evidence_refs,
            created_at,
            completed_at,
        ) in rows
        {
            let steps: Vec<(Value,)> = sqlx::query_as(
                "SELECT output_json
                 FROM agent_steps
                 WHERE agent_run_id = $1
                 ORDER BY created_at, id",
            )
            .bind(&agent_run_id)
            .fetch_all(&self.pool)
            .await?;
            let context_snapshots = self.load_agent_context_snapshots(&agent_run_id).await?;
            let policy_checks = self.load_agent_policy_checks(&agent_run_id).await?;
            let tool_calls = self.load_agent_tool_calls(&agent_run_id).await?;
            let tool_results = self.load_agent_tool_results(&agent_run_id).await?;
            let approvals = self.load_agent_approvals(&agent_run_id).await?;
            runs.push(AgentRunLogRecord {
                agent_run_id,
                claim_id,
                status,
                decision_boundary,
                output_json,
                evidence_refs: json_array_to_strings(evidence_refs),
                steps: steps.into_iter().map(|row| row.0).collect(),
                context_snapshots,
                policy_checks,
                tool_calls,
                tool_results,
                approvals,
                created_at: Some(created_at.to_rfc3339()),
                completed_at: completed_at.map(|value| value.to_rfc3339()),
            });
        }

        Ok(runs)
    }

    async fn save_agent_approval(
        &self,
        approval: AgentApprovalRecord,
    ) -> anyhow::Result<AgentApprovalRecord> {
        sqlx::query(
            "INSERT INTO agent_approvals
             (approval_id, agent_run_id, proposed_action, decision, approver, reason, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (approval_id) DO UPDATE
             SET decision = EXCLUDED.decision,
                 approver = EXCLUDED.approver,
                 reason = EXCLUDED.reason,
                 evidence_refs = EXCLUDED.evidence_refs",
        )
        .bind(&approval.approval_id)
        .bind(&approval.agent_run_id)
        .bind(&approval.proposed_action)
        .bind(&approval.decision)
        .bind(&approval.approver)
        .bind(&approval.reason)
        .bind(string_values(&approval.evidence_refs))
        .execute(&self.pool)
        .await?;
        Ok(approval)
    }

    async fn register_dataset(&self, input: RegisterDatasetInput) -> anyhow::Result<DatasetRecord> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO external_data_sources
             (source_key, display_name, business_domain, owner, description, status)
             VALUES ($1, $2, $3, $4, $5, 'active')
             ON CONFLICT (source_key) DO UPDATE
             SET display_name = EXCLUDED.display_name,
                 business_domain = EXCLUDED.business_domain,
                 owner = EXCLUDED.owner,
                 description = EXCLUDED.description,
                 updated_at = now()",
        )
        .bind(&input.source_key)
        .bind(&input.display_name)
        .bind(&input.business_domain)
        .bind(&input.owner)
        .bind(&input.description)
        .execute(&mut *tx)
        .await?;

        let dataset_row: (String,) = sqlx::query_as(
            "INSERT INTO external_dataset_versions
             (source_key, dataset_key, dataset_version, sample_grain, label_column, entity_keys, manifest_uri, schema_uri, profile_uri, storage_format, schema_hash, row_count, status)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (dataset_key, dataset_version) DO UPDATE
             SET manifest_uri = EXCLUDED.manifest_uri,
                 schema_uri = EXCLUDED.schema_uri,
                 profile_uri = EXCLUDED.profile_uri,
                 schema_hash = EXCLUDED.schema_hash,
                 row_count = EXCLUDED.row_count,
                 status = EXCLUDED.status
             RETURNING id::text",
        )
        .bind(&input.source_key)
        .bind(&input.dataset_key)
        .bind(&input.dataset_version)
        .bind(&input.sample_grain)
        .bind(&input.label_column)
        .bind(serde_json::json!(input.entity_keys))
        .bind(&input.manifest_uri)
        .bind(&input.schema_uri)
        .bind(&input.profile_uri)
        .bind(&input.storage_format)
        .bind(&input.schema_hash)
        .bind(input.row_count as i64)
        .bind(&input.status)
        .fetch_one(&mut *tx)
        .await?;

        for split in &input.splits {
            sqlx::query(
                "INSERT INTO external_dataset_splits
                 (dataset_id, split_name, data_uri, row_count, positive_count, negative_count, label_distribution_json)
                 VALUES ($1::uuid, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (dataset_id, split_name) DO UPDATE
                 SET data_uri = EXCLUDED.data_uri,
                     row_count = EXCLUDED.row_count,
                     positive_count = EXCLUDED.positive_count,
                     negative_count = EXCLUDED.negative_count,
                     label_distribution_json = EXCLUDED.label_distribution_json",
            )
            .bind(&dataset_row.0)
            .bind(&split.split_name)
            .bind(&split.data_uri)
            .bind(split.row_count as i64)
            .bind(split.positive_count.map(|value| value as i64))
            .bind(split.negative_count.map(|value| value as i64))
            .bind(&split.label_distribution_json)
            .execute(&mut *tx)
            .await?;
        }

        for field in &input.fields {
            sqlx::query(
                "INSERT INTO external_schema_fields
                 (dataset_id, field_name, logical_type, nullable, semantic_role, description, profile_json)
                 VALUES ($1::uuid, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (dataset_id, field_name) DO UPDATE
                 SET logical_type = EXCLUDED.logical_type,
                     nullable = EXCLUDED.nullable,
                     semantic_role = EXCLUDED.semantic_role,
                     description = EXCLUDED.description,
                     profile_json = EXCLUDED.profile_json",
            )
            .bind(&dataset_row.0)
            .bind(&field.field_name)
            .bind(&field.logical_type)
            .bind(field.nullable)
            .bind(&field.semantic_role)
            .bind(&field.description)
            .bind(&field.profile_json)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        load_dataset_record(&self.pool, &dataset_row.0)
            .await?
            .ok_or_else(|| anyhow::anyhow!("registered dataset was not found"))
    }

    async fn list_datasets(&self) -> anyhow::Result<Vec<DatasetRecord>> {
        let ids: Vec<(String,)> = sqlx::query_as(
            "SELECT id::text FROM external_dataset_versions ORDER BY dataset_key, dataset_version",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut datasets = Vec::new();
        for (id,) in ids {
            if let Some(dataset) = load_dataset_record(&self.pool, &id).await? {
                datasets.push(dataset);
            }
        }
        Ok(datasets)
    }

    async fn get_dataset(&self, dataset_id: &str) -> anyhow::Result<Option<DatasetRecord>> {
        load_dataset_record(&self.pool, dataset_id).await
    }

    async fn add_field_mapping(
        &self,
        dataset_id: &str,
        input: CreateFieldMappingInput,
    ) -> anyhow::Result<Option<FieldMappingRecord>> {
        if load_dataset_record(&self.pool, dataset_id).await?.is_none() {
            return Ok(None);
        }

        let row: (String,) = sqlx::query_as(
            "INSERT INTO external_field_mappings
             (dataset_id, external_field, canonical_target, feature_name, transform_kind, transform_json, status)
             VALUES ($1::uuid, $2, $3, $4, $5, $6, $7)
             RETURNING id::text",
        )
        .bind(dataset_id)
        .bind(&input.external_field)
        .bind(&input.canonical_target)
        .bind(&input.feature_name)
        .bind(&input.transform_kind)
        .bind(&input.transform_json)
        .bind(&input.status)
        .fetch_one(&self.pool)
        .await?;

        Ok(Some(FieldMappingRecord {
            mapping_id: row.0,
            dataset_id: dataset_id.to_string(),
            external_field: input.external_field,
            canonical_target: input.canonical_target,
            feature_name: input.feature_name,
            transform_kind: input.transform_kind,
            transform_json: input.transform_json,
            status: input.status,
        }))
    }

    async fn save_investigation_result(
        &self,
        record: InvestigationResultRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        let saving_attributions = derive_saving_attributions(&record);
        let mut tx = self.pool.begin().await?;
        let previous_case_id: Option<String> = sqlx::query_scalar(
            "SELECT case_id FROM investigation_results WHERE investigation_id = $1",
        )
        .bind(&record.investigation_id)
        .fetch_optional(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO investigation_results
             (investigation_id, case_id, claim_id, outcome, confirmed_fwa, financial_impact_type, saving_amount, currency, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (investigation_id) DO UPDATE
             SET case_id = EXCLUDED.case_id,
                 claim_id = EXCLUDED.claim_id,
                 outcome = EXCLUDED.outcome,
                 confirmed_fwa = EXCLUDED.confirmed_fwa,
                 financial_impact_type = EXCLUDED.financial_impact_type,
                 saving_amount = EXCLUDED.saving_amount,
                 currency = EXCLUDED.currency,
                 notes = EXCLUDED.notes,
                 evidence_refs = EXCLUDED.evidence_refs",
        )
        .bind(&record.investigation_id)
        .bind(&record.case_id)
        .bind(&record.claim_id)
        .bind(&record.outcome)
        .bind(record.confirmed_fwa)
        .bind(normalize_financial_impact_type(
            record.financial_impact_type.as_deref(),
        ))
        .bind(record.saving_amount)
        .bind(&record.currency)
        .bind(&record.notes)
        .bind(serde_json::json!(record.evidence_refs))
        .execute(&mut *tx)
        .await?;

        if previous_case_id.as_deref() != record.case_id.as_deref() {
            if let Some(case_id) = previous_case_id.as_deref() {
                sqlx::query(
                    "UPDATE investigation_cases
                     SET final_outcome = NULL,
                         reviewer_notes = NULL,
                         investigation_result_id = NULL,
                         updated_at = now()
                     WHERE case_id = $1
                       AND investigation_result_id = $2",
                )
                .bind(case_id)
                .bind(&record.investigation_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        if let Some(case_id) = record.case_id.as_deref() {
            let update = sqlx::query(
                "UPDATE investigation_cases
                 SET final_outcome = $1,
                     reviewer_notes = $2,
                     investigation_result_id = $3,
                     updated_at = now()
                 WHERE case_id = $4
                   AND claim_id = $5",
            )
            .bind(&record.outcome)
            .bind(&record.notes)
            .bind(&record.investigation_id)
            .bind(case_id)
            .bind(&record.claim_id)
            .execute(&mut *tx)
            .await?;
            if update.rows_affected() == 0 {
                anyhow::bail!("case not found for investigation result: {case_id}");
            }
        }

        sqlx::query("DELETE FROM saving_attributions WHERE investigation_id = $1")
            .bind(&record.investigation_id)
            .execute(&mut *tx)
            .await?;
        for attribution in saving_attributions {
            sqlx::query(
                "INSERT INTO saving_attributions
                 (attribution_id, claim_id, investigation_id, source_type, source_id, financial_impact_type, action, saving_amount, currency, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(&attribution.attribution_id)
            .bind(&attribution.claim_id)
            .bind(&attribution.investigation_id)
            .bind(&attribution.source_type)
            .bind(&attribution.source_id)
            .bind(&attribution.financial_impact_type)
            .bind(&attribution.action)
            .bind(attribution.saving_amount)
            .bind(&attribution.currency)
            .bind(serde_json::json!(attribution.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }

        let event = AuditHistoryEventRecord {
            audit_id: format!("audit_investigation_{}", record.investigation_id),
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
        insert_pilot_audit_event(&mut tx, &record.claim_id, &event).await?;
        tx.commit().await?;
        Ok(event)
    }

    async fn save_qa_review(
        &self,
        mut record: QaReviewRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        record.feedback_target = canonical_feedback_target(&record.feedback_target).into();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO qa_reviews
             (qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, feedback_status, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, 'open', $6, $7)
             ON CONFLICT (qa_case_id) DO UPDATE
             SET qa_conclusion = EXCLUDED.qa_conclusion,
                 issue_type = EXCLUDED.issue_type,
                 feedback_target = EXCLUDED.feedback_target,
                 feedback_status = EXCLUDED.feedback_status,
                 notes = EXCLUDED.notes,
                 evidence_refs = EXCLUDED.evidence_refs",
        )
        .bind(&record.qa_case_id)
        .bind(&record.claim_id)
        .bind(&record.qa_conclusion)
        .bind(&record.issue_type)
        .bind(&record.feedback_target)
        .bind(&record.notes)
        .bind(serde_json::json!(record.evidence_refs))
        .execute(&mut *tx)
        .await?;

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
        insert_pilot_audit_event(&mut tx, &record.claim_id, &event).await?;
        tx.commit().await?;
        Ok(event)
    }

    async fn list_qa_feedback_items(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaFeedbackItemRecord>> {
        let allowed_qa_case_ids = if let Some(scope) = customer_scope_id {
            Some(
                self.list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("qa.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| event.payload["qa_case_id"].as_str().map(str::to_string))
                .collect::<BTreeSet<_>>(),
            )
        } else {
            None
        };
        let mut status_events = self
            .list_audit_events(AuditEventListFilter {
                limit: 10_000,
                event_type: Some("qa.feedback.status.updated".into()),
                customer_scope_id: customer_scope_id.map(str::to_string),
                ..Default::default()
            })
            .await?;
        status_events.reverse();
        let feedback_statuses = latest_qa_feedback_statuses(
            &status_events
                .into_iter()
                .map(|event| {
                    (
                        event.payload["claim_id"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        event,
                    )
                })
                .collect::<Vec<_>>(),
        );
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, feedback_status, notes, evidence_refs, created_at
             FROM qa_reviews
             WHERE qa_conclusion <> 'pass'
             ORDER BY created_at, qa_case_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut items = rows
            .into_iter()
            .filter(|(qa_case_id, _, _, _, _, _, _, _, _)| {
                allowed_qa_case_ids
                    .as_ref()
                    .is_none_or(|ids| ids.contains(qa_case_id))
            })
            .map(
                |(
                    qa_case_id,
                    claim_id,
                    qa_conclusion,
                    issue_type,
                    feedback_target,
                    feedback_status,
                    notes,
                    evidence_refs,
                    created_at,
                )| {
                    let feedback_id = qa_feedback_id(&qa_case_id);
                    let status_update = feedback_statuses.get(&feedback_id);
                    qa_review_to_feedback_item(
                        QaReviewRecord {
                            qa_case_id,
                            claim_id,
                            qa_conclusion,
                            issue_type,
                            feedback_target,
                            notes,
                            evidence_refs: json_array_to_strings(evidence_refs),
                            customer_scope_id: None,
                            actor_id: None,
                            actor_role: None,
                        },
                        Some(created_at.to_rfc3339()),
                        &feedback_status,
                        status_update,
                    )
                },
            )
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
        let Some(qa_case_id) = qa_case_id_from_feedback_id(feedback_id) else {
            return Ok(None);
        };
        if let Some(scope) = customer_scope_id {
            let is_in_scope = self
                .list_audit_events(AuditEventListFilter {
                    limit: 1,
                    event_type: Some("qa.result.received".into()),
                    qa_case_id: Some(qa_case_id.into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .next()
                .is_some();
            if !is_in_scope {
                return Ok(None);
            }
        }
        let mut tx = self.pool.begin().await?;
        let row: Option<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "WITH existing AS (
                 SELECT qa_case_id, feedback_status AS from_status
                 FROM qa_reviews
                 WHERE qa_case_id = $1 AND qa_conclusion <> 'pass'
             ),
             updated AS (
                 UPDATE qa_reviews
                 SET feedback_status = $2
                 FROM existing
                 WHERE qa_reviews.qa_case_id = existing.qa_case_id
                 RETURNING existing.from_status,
                           qa_reviews.qa_case_id,
                           qa_reviews.claim_id,
                           qa_reviews.qa_conclusion,
                           qa_reviews.issue_type,
                           qa_reviews.feedback_target,
                           qa_reviews.feedback_status,
                           qa_reviews.notes,
                           qa_reviews.evidence_refs,
                           qa_reviews.created_at
             )
             SELECT * FROM updated",
        )
        .bind(qa_case_id)
        .bind(&input.status)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((
            from_status,
            qa_case_id,
            claim_id,
            qa_conclusion,
            issue_type,
            feedback_target,
            feedback_status,
            notes,
            evidence_refs,
            created_at,
        )) = row
        else {
            return Ok(None);
        };
        let audit_id = AuditEventId::new().to_string();
        let item = qa_review_to_feedback_item(
            QaReviewRecord {
                qa_case_id,
                claim_id: claim_id.clone(),
                qa_conclusion,
                issue_type,
                feedback_target,
                notes,
                evidence_refs: json_array_to_strings(evidence_refs),
                customer_scope_id: None,
                actor_id: None,
                actor_role: None,
            },
            Some(created_at.to_rfc3339()),
            &feedback_status,
            Some(&QaFeedbackStatusUpdate {
                status: feedback_status.clone(),
                actor_id: Some(input.actor_id.clone()),
                audit_id: audit_id.clone(),
                updated_at: None,
                evidence_refs: input.evidence_refs.clone(),
            }),
        );
        insert_pilot_audit_event(
            &mut tx,
            &claim_id,
            &AuditHistoryEventRecord {
                audit_id: audit_id.clone(),
                run_id: format!("qa_feedback_status_{}", item.feedback_id),
                actor_role: "fwa_operator".into(),
                event_type: "qa.feedback.status.updated".into(),
                event_status: "succeeded".into(),
                summary: format!("QA feedback status updated: {}", item.status),
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
            },
        )
        .await?;
        tx.commit().await?;
        Ok(Some(UpdateQaFeedbackStatusRecord { item, audit_id }))
    }

    async fn list_qa_reviews(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaReviewRecord>> {
        let allowed_qa_case_ids = if let Some(scope) = customer_scope_id {
            Some(
                self.list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("qa.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| event.payload["qa_case_id"].as_str().map(str::to_string))
                .collect::<BTreeSet<_>>(),
            )
        } else {
            None
        };
        let rows: Vec<(String, String, String, String, String, String, Value)> = sqlx::query_as(
            "SELECT qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, notes, evidence_refs
             FROM qa_reviews
             ORDER BY qa_case_id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter(|(qa_case_id, _, _, _, _, _, _)| {
                allowed_qa_case_ids
                    .as_ref()
                    .is_none_or(|ids| ids.contains(qa_case_id))
            })
            .map(
                |(
                    qa_case_id,
                    claim_id,
                    qa_conclusion,
                    issue_type,
                    feedback_target,
                    notes,
                    evidence_refs,
                )| {
                    let feedback_target = canonical_feedback_target(&feedback_target).into();
                    QaReviewRecord {
                        qa_case_id,
                        claim_id,
                        qa_conclusion,
                        issue_type,
                        feedback_target,
                        notes,
                        evidence_refs: json_array_to_strings(evidence_refs),
                        customer_scope_id: None,
                        actor_id: None,
                        actor_role: None,
                    }
                },
            )
            .collect())
    }

    async fn list_outcome_labels(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<OutcomeLabelRecord>> {
        let allowed_investigation_ids = if let Some(scope) = customer_scope_id {
            Some(
                self.list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("investigation.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| {
                    event.payload["investigation_id"]
                        .as_str()
                        .map(str::to_string)
                })
                .collect::<BTreeSet<_>>(),
            )
        } else {
            None
        };
        let allowed_qa_case_ids = if let Some(scope) = customer_scope_id {
            Some(
                self.list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("qa.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| event.payload["qa_case_id"].as_str().map(str::to_string))
                .collect::<BTreeSet<_>>(),
            )
        } else {
            None
        };
        let investigation_rows: Vec<(
            String,
            String,
            String,
            bool,
            Option<String>,
            Option<Decimal>,
            Option<String>,
            String,
            Value,
        )> = sqlx::query_as(
            "SELECT investigation_id, claim_id, outcome, confirmed_fwa, financial_impact_type, saving_amount, currency, notes, evidence_refs
             FROM investigation_results
             ORDER BY created_at, investigation_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let qa_rows: Vec<(String, String, String, String, String, String, String, Value)> =
            sqlx::query_as(
                "SELECT qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, feedback_status, notes, evidence_refs
                 FROM qa_reviews
                 ORDER BY created_at, qa_case_id",
            )
            .fetch_all(&self.pool)
            .await?;
        let medical_review_rows: Vec<(String, String, Value, Value)> = sqlx::query_as(
            "SELECT audit_id, actor_role, payload, evidence_refs
             FROM audit_events
             WHERE event_type = 'medical.review.recorded'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)
             ORDER BY created_at, audit_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        let lead_triage_rows: Vec<(String, String, String, Value, Value)> = sqlx::query_as(
            "SELECT audit_id, run_id, actor_role, payload, evidence_refs
             FROM audit_events
             WHERE event_type = 'lead.triaged'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)
             ORDER BY created_at, audit_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        let label_bootstrap_rows: Vec<(String, String, String, Value, Value)> = sqlx::query_as(
            "SELECT audit_id, run_id, actor_role, payload, evidence_refs
             FROM audit_events
             WHERE event_type = 'label.bootstrap.reviewed'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)
             ORDER BY created_at, audit_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;

        let mut labels = investigation_rows
            .into_iter()
            .filter(|(investigation_id, _, _, _, _, _, _, _, _)| {
                allowed_investigation_ids
                    .as_ref()
                    .is_none_or(|ids| ids.contains(investigation_id))
            })
            .flat_map(
                |(
                    investigation_id,
                    claim_id,
                    outcome,
                    confirmed_fwa,
                    financial_impact_type,
                    saving_amount,
                    currency,
                    notes,
                    evidence_refs,
                )| {
                    labels_from_investigation_result(InvestigationResultRecord {
                        investigation_id,
                        case_id: None,
                        claim_id,
                        outcome,
                        confirmed_fwa,
                        financial_impact_type,
                        saving_amount,
                        currency,
                        notes,
                        evidence_refs: json_array_to_strings(evidence_refs),
                        customer_scope_id: None,
                        actor_id: None,
                        actor_role: None,
                    })
                },
            )
            .chain(
                qa_rows
                    .into_iter()
                    .filter(|(qa_case_id, _, _, _, _, _, _, _)| {
                        allowed_qa_case_ids
                            .as_ref()
                            .is_none_or(|ids| ids.contains(qa_case_id))
                    })
                    .map(
                        |(
                            qa_case_id,
                            claim_id,
                            qa_conclusion,
                            issue_type,
                            feedback_target,
                            feedback_status,
                            notes,
                            evidence_refs,
                        )| {
                            label_from_qa_review(
                                QaReviewRecord {
                                    qa_case_id,
                                    claim_id,
                                    qa_conclusion,
                                    issue_type,
                                    feedback_target,
                                    notes,
                                    evidence_refs: json_array_to_strings(evidence_refs),
                                    customer_scope_id: None,
                                    actor_id: None,
                                    actor_role: None,
                                },
                                &feedback_status,
                            )
                        },
                    ),
            )
            .chain(medical_review_rows.into_iter().flat_map(
                |(audit_id, actor_role, payload, evidence_refs)| {
                    labels_from_medical_review_event(&AuditHistoryEventRecord {
                        audit_id,
                        run_id: String::new(),
                        actor_role,
                        event_type: "medical.review.recorded".into(),
                        event_status: "succeeded".into(),
                        summary: String::new(),
                        payload,
                        evidence_refs: json_array_to_strings(evidence_refs),
                        created_at: None,
                    })
                },
            ))
            .chain(label_bootstrap_rows.into_iter().filter_map(
                |(audit_id, run_id, actor_role, payload, evidence_refs)| {
                    label_from_bootstrap_review_event(&AuditHistoryEventRecord {
                        audit_id,
                        run_id,
                        actor_role,
                        event_type: "label.bootstrap.reviewed".into(),
                        event_status: "succeeded".into(),
                        summary: String::new(),
                        payload,
                        evidence_refs: json_array_to_strings(evidence_refs),
                        created_at: None,
                    })
                },
            ))
            .collect::<Vec<_>>();
        labels.extend(labels_from_lead_triage_events(
            lead_triage_rows.into_iter().map(
                |(audit_id, run_id, actor_role, payload, evidence_refs)| AuditHistoryEventRecord {
                    audit_id,
                    run_id,
                    actor_role,
                    event_type: "lead.triaged".into(),
                    event_status: "succeeded".into(),
                    summary: String::new(),
                    payload,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: None,
                },
            ),
        ));
        labels.extend(
            self.list_cases(None)
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
        let rows: Vec<(String, String, String, String, String, String, Value, Value, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as(
                "SELECT ae.audit_id, ae.run_id, ae.actor_role, ae.event_type, ae.event_status, ae.summary, ae.payload, ae.evidence_refs, ae.created_at
                 FROM audit_events ae
                 LEFT JOIN claims c ON c.id = ae.claim_id
                 WHERE (payload ->> 'claim_id' = $1 OR c.external_claim_id = $1)
                   AND ($2::text IS NULL OR ae.payload ->> 'customer_scope_id' = $2)
                 ORDER BY ae.created_at, ae.audit_id",
            )
            .bind(claim_id)
            .bind(customer_scope_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs,
                    created_at,
                )| AuditHistoryEventRecord {
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: Some(created_at.to_rfc3339()),
                },
            )
            .collect())
    }

    async fn list_audit_events(
        &self,
        filter: AuditEventListFilter,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT ae.audit_id, ae.run_id, ae.actor_role, ae.event_type, ae.event_status, ae.summary, ae.payload, ae.evidence_refs, ae.created_at
             FROM audit_events ae
             LEFT JOIN claims c ON c.id = ae.claim_id
             WHERE ($2::text IS NULL OR ae.event_type = $2)
               AND ($3::text IS NULL OR ae.actor_id = $3)
               AND ($4::text IS NULL OR ae.run_id = $4)
               AND (
                 $5::text IS NULL
                 OR ae.payload ->> 'claim_id' = $5
                 OR c.external_claim_id = $5
                 OR ae.claim_id::text = $5
               )
               AND ($6::text IS NULL OR ae.payload ->> 'policy_id' = $6)
               AND ($7::text IS NULL OR ae.payload ->> 'version' = $7)
               AND ($8::text IS NULL OR ae.payload ->> 'review_mode' = $8)
               AND ($9::text IS NULL OR ae.payload ->> 'rule_id' = $9)
               AND ($10::text IS NULL OR ae.payload ->> 'rule_version' = $10)
               AND ($11::text IS NULL OR ae.payload ->> 'model_key' = $11)
               AND ($12::text IS NULL OR ae.payload ->> 'model_version' = $12)
               AND (
                 $13::text IS NULL
                 OR (
                   $13 = 'governance'
                   AND ae.event_type = ANY($14::text[])
                 )
               )
               AND ($15::text IS NULL OR ae.payload ->> 'sample_id' = $15)
               AND ($16::text IS NULL OR ae.payload ->> 'agent_run_id' = $16)
               AND ($17::text IS NULL OR ae.payload ->> 'dataset_id' = $17)
               AND ($18::text IS NULL OR ae.payload ->> 'feature_set_id' = $18)
               AND ($19::text IS NULL OR ae.payload ->> 'model_dataset_id' = $19)
               AND ($20::text IS NULL OR ae.payload ->> 'evaluation_run_id' = $20)
               AND ($21::bool IS NULL OR $21 = false OR ae.payload ? 'canonical_claim_context_trace')
               AND ($22::text IS NULL OR ae.payload ->> 'customer_scope_id' = $22)
             ORDER BY ae.created_at DESC, ae.audit_id DESC
             LIMIT $1",
        )
        .bind(filter.limit as i64)
        .bind(filter.event_type.as_deref())
        .bind(filter.actor_id.as_deref())
        .bind(filter.run_id.as_deref())
        .bind(filter.claim_id.as_deref())
        .bind(filter.routing_policy_id.as_deref())
        .bind(filter.routing_policy_version.as_deref())
        .bind(filter.review_mode.as_deref())
        .bind(filter.rule_id.as_deref())
        .bind(filter.rule_version.as_deref())
        .bind(filter.model_key.as_deref())
        .bind(filter.model_version.as_deref())
        .bind(filter.event_group.as_deref())
        .bind(GOVERNANCE_AUDIT_EVENT_TYPES)
        .bind(filter.sample_id.as_deref())
        .bind(filter.agent_run_id.as_deref())
        .bind(filter.dataset_id.as_deref())
        .bind(filter.feature_set_id.as_deref())
        .bind(filter.model_dataset_id.as_deref())
        .bind(filter.evaluation_run_id.as_deref())
        .bind(filter.has_canonical_trace)
        .bind(filter.customer_scope_id.as_deref())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs,
                    created_at,
                )| AuditHistoryEventRecord {
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: Some(created_at.to_rfc3339()),
                },
            )
            .collect())
    }

    async fn list_webhook_events(&self) -> anyhow::Result<Vec<WebhookEventRecord>> {
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT audit_id, run_id, actor_role, event_type, event_status, summary, payload, evidence_refs, created_at
             FROM audit_events
             ORDER BY created_at, audit_id",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut events = rows
            .into_iter()
            .filter_map(
                |(
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs,
                    created_at,
                )| {
                    webhook_event_from_audit(
                        None,
                        &AuditHistoryEventRecord {
                            audit_id,
                            run_id,
                            actor_role,
                            event_type,
                            event_status,
                            summary,
                            payload,
                            evidence_refs: json_array_to_strings(evidence_refs),
                            created_at: Some(created_at.to_rfc3339()),
                        },
                    )
                },
            )
            .collect::<Vec<_>>();
        let attempt_rows: Vec<(
            String,
            i32,
            String,
            Option<i32>,
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT event_id, attempt_number, delivery_status, response_status_code, error_message, next_attempt_at, attempted_at
             FROM webhook_delivery_attempts
             ORDER BY event_id, attempt_number",
        )
        .fetch_all(&self.pool)
        .await?;
        let attempts = attempt_rows
            .into_iter()
            .map(
                |(
                    event_id,
                    attempt_number,
                    delivery_status,
                    response_status_code,
                    error_message,
                    next_attempt_at,
                    attempted_at,
                )| WebhookDeliveryAttemptRecord {
                    event_id,
                    attempt_number: attempt_number.max(0) as u32,
                    delivery_status,
                    response_status_code: response_status_code.map(|value| value.max(0) as u16),
                    error_message,
                    next_attempt_at: next_attempt_at.map(|timestamp| timestamp.to_rfc3339()),
                    attempted_at: Some(attempted_at.to_rfc3339()),
                },
            )
            .collect::<Vec<_>>();
        apply_webhook_delivery_state(&mut events, &attempts);
        sort_webhook_events(&mut events);
        Ok(events)
    }

    async fn save_webhook_delivery_attempt(
        &self,
        input: WebhookDeliveryAttemptInput,
    ) -> anyhow::Result<WebhookDeliveryAttemptRecord> {
        let row: (Option<i32>,) = sqlx::query_as(
            "SELECT MAX(attempt_number)
             FROM webhook_delivery_attempts
             WHERE event_id = $1",
        )
        .bind(&input.event_id)
        .fetch_one(&self.pool)
        .await?;
        let attempt_number = row.0.unwrap_or(0) + 1;
        let attempted_at = chrono::Utc::now();
        let next_attempt_at =
            next_webhook_attempt_at(&input.delivery_status, attempt_number as u32, attempted_at);
        let inserted: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
            "INSERT INTO webhook_delivery_attempts
             (event_id, attempt_number, delivery_status, response_status_code, error_message, next_attempt_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING attempted_at",
        )
        .bind(&input.event_id)
        .bind(attempt_number)
        .bind(&input.delivery_status)
        .bind(input.response_status_code.map(i32::from))
        .bind(&input.error_message)
        .bind(next_attempt_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(WebhookDeliveryAttemptRecord {
            event_id: input.event_id,
            attempt_number: attempt_number as u32,
            delivery_status: input.delivery_status,
            response_status_code: input.response_status_code,
            error_message: input.error_message,
            next_attempt_at: next_attempt_at.map(|timestamp| timestamp.to_rfc3339()),
            attempted_at: Some(inserted.0.to_rfc3339()),
        })
    }

    async fn register_feature_set(
        &self,
        input: RegisterFeatureSetInput,
    ) -> anyhow::Result<Option<FeatureSetRecord>> {
        if load_dataset_record(&self.pool, &input.dataset_id)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let row: (String,) = sqlx::query_as(
            "INSERT INTO feature_set_versions
             (feature_set_key, business_domain, version, dataset_id, features_uri, feature_list_json, row_count, label_column, status)
             VALUES ($1, $2, $3, $4::uuid, $5, $6, $7, $8, $9)
             ON CONFLICT (feature_set_key, version) DO UPDATE
             SET features_uri = EXCLUDED.features_uri,
                 feature_list_json = EXCLUDED.feature_list_json,
                 row_count = EXCLUDED.row_count,
                 status = EXCLUDED.status
             RETURNING id::text",
        )
        .bind(&input.feature_set_key)
        .bind(&input.business_domain)
        .bind(&input.version)
        .bind(&input.dataset_id)
        .bind(&input.features_uri)
        .bind(&input.feature_list_json)
        .bind(input.row_count as i64)
        .bind(&input.label_column)
        .bind(&input.status)
        .fetch_one(&self.pool)
        .await?;
        Ok(Some(FeatureSetRecord {
            feature_set_id: row.0,
            business_domain: input.business_domain,
            feature_set_key: input.feature_set_key,
            version: input.version,
            dataset_id: input.dataset_id,
            features_uri: input.features_uri,
            feature_list_json: input.feature_list_json,
            row_count: input.row_count,
            label_column: input.label_column,
            status: input.status,
        }))
    }

    async fn register_model_dataset(
        &self,
        input: RegisterModelDatasetInput,
    ) -> anyhow::Result<Option<ModelDatasetRecord>> {
        let feature_set_known: Option<(String,)> =
            sqlx::query_as("SELECT id::text FROM feature_set_versions WHERE id = $1::uuid")
                .bind(&input.feature_set_id)
                .fetch_optional(&self.pool)
                .await?;
        if feature_set_known.is_none() {
            return Ok(None);
        }

        let row: (String,) = sqlx::query_as(
            "INSERT INTO model_dataset_versions
             (business_domain, task_type, label_name, feature_set_id, train_uri, validation_uri, test_uri, row_counts_json, label_distribution_json, status)
             VALUES ($1, $2, $3, $4::uuid, $5, $6, $7, $8, $9, $10)
             RETURNING id::text",
        )
        .bind(&input.business_domain)
        .bind(&input.task_type)
        .bind(&input.label_name)
        .bind(&input.feature_set_id)
        .bind(&input.train_uri)
        .bind(&input.validation_uri)
        .bind(&input.test_uri)
        .bind(&input.row_counts_json)
        .bind(&input.label_distribution_json)
        .bind(&input.status)
        .fetch_one(&self.pool)
        .await?;

        Ok(Some(ModelDatasetRecord {
            model_dataset_id: row.0,
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
        }))
    }

    async fn get_model_dataset_source_dataset(
        &self,
        model_dataset_id: &str,
    ) -> anyhow::Result<Option<DatasetRecord>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT fs.dataset_id::text
             FROM model_dataset_versions md
             JOIN feature_set_versions fs ON fs.id = md.feature_set_id
             WHERE md.id = $1::uuid",
        )
        .bind(model_dataset_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some((dataset_id,)) = row else {
            return Ok(None);
        };
        load_dataset_record(&self.pool, &dataset_id).await
    }

    async fn register_model_evaluation(
        &self,
        input: RegisterModelEvaluationInput,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        let model_dataset_known: Option<(String,)> =
            sqlx::query_as("SELECT id::text FROM model_dataset_versions WHERE id = $1::uuid")
                .bind(&input.model_dataset_id)
                .fetch_optional(&self.pool)
                .await?;
        if model_dataset_known.is_none() {
            return Ok(None);
        }

        sqlx::query(
            "INSERT INTO model_evaluation_runs
             (evaluation_run_id, model_key, model_version, model_dataset_id, scheme_family, auc, ks, precision_value, recall_value, f1, accuracy, threshold, confusion_matrix_json, feature_importance_uri, permutation_importance_uri, metrics_json)
             VALUES ($1, $2, $3, $4::uuid, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
             ON CONFLICT (evaluation_run_id) DO UPDATE
             SET model_key = EXCLUDED.model_key,
                 model_version = EXCLUDED.model_version,
                 model_dataset_id = EXCLUDED.model_dataset_id,
                 scheme_family = EXCLUDED.scheme_family,
                 auc = EXCLUDED.auc,
                 ks = EXCLUDED.ks,
                 precision_value = EXCLUDED.precision_value,
                 recall_value = EXCLUDED.recall_value,
                 f1 = EXCLUDED.f1,
                 accuracy = EXCLUDED.accuracy,
                 threshold = EXCLUDED.threshold,
                 confusion_matrix_json = EXCLUDED.confusion_matrix_json,
                 feature_importance_uri = EXCLUDED.feature_importance_uri,
                 permutation_importance_uri = EXCLUDED.permutation_importance_uri,
                 metrics_json = EXCLUDED.metrics_json",
        )
        .bind(&input.evaluation_run_id)
        .bind(&input.model_key)
        .bind(&input.model_version)
        .bind(&input.model_dataset_id)
        .bind(&input.scheme_family)
        .bind(input.auc)
        .bind(input.ks)
        .bind(input.precision)
        .bind(input.recall)
        .bind(input.f1)
        .bind(input.accuracy)
        .bind(input.threshold)
        .bind(&input.confusion_matrix_json)
        .bind(&input.feature_importance_uri)
        .bind(&input.permutation_importance_uri)
        .bind(&input.metrics_json)
        .execute(&self.pool)
        .await?;

        self.get_model_evaluation(&input.evaluation_run_id).await
    }

    async fn get_model_evaluation(
        &self,
        evaluation_run_id: &str,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        let row: Option<(
            String,
            String,
            String,
            String,
            String,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Value,
            Option<String>,
            Option<String>,
            Value,
        )> = sqlx::query_as(
            "SELECT evaluation_run_id, model_key, model_version, model_dataset_id::text, scheme_family, auc, ks, precision_value, recall_value, f1, accuracy, threshold, confusion_matrix_json, feature_importance_uri, permutation_importance_uri, metrics_json
             FROM model_evaluation_runs
             WHERE evaluation_run_id = $1",
        )
        .bind(evaluation_run_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                evaluation_run_id,
                model_key,
                model_version,
                model_dataset_id,
                scheme_family,
                auc,
                ks,
                precision,
                recall,
                f1,
                accuracy,
                threshold,
                confusion_matrix_json,
                feature_importance_uri,
                permutation_importance_uri,
                metrics_json,
            )| ModelEvaluationRecord {
                evaluation_run_id,
                model_key,
                model_version,
                model_dataset_id,
                scheme_family,
                auc,
                ks,
                precision,
                recall,
                f1,
                accuracy,
                threshold,
                confusion_matrix_json,
                feature_importance_uri,
                permutation_importance_uri,
                metrics_json,
            },
        ))
    }

    async fn list_model_evaluations(&self) -> anyhow::Result<Vec<ModelEvaluationRecord>> {
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Option<Decimal>,
            Value,
            Option<String>,
            Option<String>,
            Value,
        )> = sqlx::query_as(
            "SELECT evaluation_run_id, model_key, model_version, model_dataset_id::text, scheme_family, auc, ks, precision_value, recall_value, f1, accuracy, threshold, confusion_matrix_json, feature_importance_uri, permutation_importance_uri, metrics_json
             FROM model_evaluation_runs
             ORDER BY evaluation_run_id",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    evaluation_run_id,
                    model_key,
                    model_version,
                    model_dataset_id,
                    scheme_family,
                    auc,
                    ks,
                    precision,
                    recall,
                    f1,
                    accuracy,
                    threshold,
                    confusion_matrix_json,
                    feature_importance_uri,
                    permutation_importance_uri,
                    metrics_json,
                )| ModelEvaluationRecord {
                    evaluation_run_id,
                    model_key,
                    model_version,
                    model_dataset_id,
                    scheme_family,
                    auc,
                    ks,
                    precision,
                    recall,
                    f1,
                    accuracy,
                    threshold,
                    confusion_matrix_json,
                    feature_importance_uri,
                    permutation_importance_uri,
                    metrics_json,
                },
            )
            .collect())
    }

    async fn save_evidence_document(
        &self,
        input: CreateEvidenceDocumentInput,
    ) -> anyhow::Result<EvidenceDocumentRecord> {
        let row = sqlx::query(
            "WITH input_claim AS (
               SELECT id FROM claims WHERE external_claim_id = $5 LIMIT 1
             )
             INSERT INTO evidence_documents
             (document_id, customer_scope_id, source_system, source_record_ref, claim_id, external_document_id, document_type, storage_uri, content_checksum, ingestion_status, redaction_status, retention_policy_id, evidence_refs, metadata_json)
             VALUES ($1, $2, $3, $4, (SELECT id FROM input_claim), $6, $7, $8, $9, $10, $11, $12, $13, $14)
             ON CONFLICT (document_id) DO UPDATE SET
               customer_scope_id = EXCLUDED.customer_scope_id,
               source_system = EXCLUDED.source_system,
               source_record_ref = EXCLUDED.source_record_ref,
               claim_id = EXCLUDED.claim_id,
               external_document_id = EXCLUDED.external_document_id,
               document_type = EXCLUDED.document_type,
               storage_uri = EXCLUDED.storage_uri,
               content_checksum = EXCLUDED.content_checksum,
               ingestion_status = EXCLUDED.ingestion_status,
               redaction_status = EXCLUDED.redaction_status,
               retention_policy_id = EXCLUDED.retention_policy_id,
               evidence_refs = EXCLUDED.evidence_refs,
               metadata_json = EXCLUDED.metadata_json,
               updated_at = now()
             RETURNING document_id, customer_scope_id, source_system, source_record_ref,
               (SELECT external_claim_id FROM claims WHERE id = evidence_documents.claim_id) AS claim_id,
               external_document_id, document_type, storage_uri, content_checksum, ingestion_status,
               redaction_status, retention_policy_id, evidence_refs, metadata_json, created_at, updated_at",
        )
        .bind(&input.document_id)
        .bind(&input.customer_scope_id)
        .bind(&input.source_system)
        .bind(&input.source_record_ref)
        .bind(&input.claim_id)
        .bind(&input.external_document_id)
        .bind(&input.document_type)
        .bind(&input.storage_uri)
        .bind(&input.content_checksum)
        .bind(&input.ingestion_status)
        .bind(&input.redaction_status)
        .bind(&input.retention_policy_id)
        .bind(string_values(&input.evidence_refs))
        .bind(&input.metadata_json)
        .fetch_one(&self.pool)
        .await?;
        evidence_document_from_row(row)
    }

    async fn list_evidence_documents(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentRecord>> {
        let rows = sqlx::query(
            "SELECT d.document_id, d.customer_scope_id, d.source_system, d.source_record_ref,
                    c.external_claim_id AS claim_id, d.external_document_id, d.document_type,
                    d.storage_uri, d.content_checksum, d.ingestion_status, d.redaction_status,
                    d.retention_policy_id, d.evidence_refs, d.metadata_json, d.created_at, d.updated_at
             FROM evidence_documents d
             LEFT JOIN claims c ON c.id = d.claim_id
             WHERE ($1::text IS NULL OR d.customer_scope_id = $1)
             ORDER BY d.document_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(evidence_document_from_row).collect()
    }

    async fn get_evidence_document(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentRecord>> {
        let row = sqlx::query(
            "SELECT d.document_id, d.customer_scope_id, d.source_system, d.source_record_ref,
                    c.external_claim_id AS claim_id, d.external_document_id, d.document_type,
                    d.storage_uri, d.content_checksum, d.ingestion_status, d.redaction_status,
                    d.retention_policy_id, d.evidence_refs, d.metadata_json, d.created_at, d.updated_at
             FROM evidence_documents d
             LEFT JOIN claims c ON c.id = d.claim_id
             WHERE d.document_id = $1
               AND ($2::text IS NULL OR d.customer_scope_id = $2)",
        )
        .bind(document_id)
        .bind(customer_scope_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(evidence_document_from_row).transpose()
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
        let row = sqlx::query(
            "INSERT INTO evidence_document_chunks
             (chunk_id, document_id, chunk_index, chunking_version, redaction_status, text_checksum, token_count, storage_uri, source_offsets_json, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (document_id, chunk_index, chunking_version) DO UPDATE SET
               redaction_status = EXCLUDED.redaction_status,
               text_checksum = EXCLUDED.text_checksum,
               token_count = EXCLUDED.token_count,
               storage_uri = EXCLUDED.storage_uri,
               source_offsets_json = EXCLUDED.source_offsets_json,
               evidence_refs = EXCLUDED.evidence_refs
             RETURNING chunk_id, document_id, chunk_index, chunking_version, redaction_status, text_checksum, token_count, storage_uri, source_offsets_json, evidence_refs, created_at",
        )
        .bind(&input.chunk_id)
        .bind(&input.document_id)
        .bind(input.chunk_index)
        .bind(&input.chunking_version)
        .bind(&input.redaction_status)
        .bind(&input.text_checksum)
        .bind(input.token_count)
        .bind(&input.storage_uri)
        .bind(&input.source_offsets_json)
        .bind(string_values(&input.evidence_refs))
        .fetch_one(&self.pool)
        .await?;
        Ok(Some(evidence_document_chunk_from_row(row)?))
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
        let rows = sqlx::query(
            "SELECT chunk_id, document_id, chunk_index, chunking_version, redaction_status, text_checksum, token_count, storage_uri, source_offsets_json, evidence_refs, created_at
             FROM evidence_document_chunks
             WHERE document_id = $1
             ORDER BY chunk_index, chunk_id",
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(evidence_document_chunk_from_row)
            .collect()
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
        let row = sqlx::query(
            "INSERT INTO evidence_ocr_outputs
             (ocr_output_id, document_id, ocr_engine, ocr_engine_version, output_uri, output_checksum, confidence_score, quality_status, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (ocr_output_id) DO UPDATE SET
               ocr_engine = EXCLUDED.ocr_engine,
               ocr_engine_version = EXCLUDED.ocr_engine_version,
               output_uri = EXCLUDED.output_uri,
               output_checksum = EXCLUDED.output_checksum,
               confidence_score = EXCLUDED.confidence_score,
               quality_status = EXCLUDED.quality_status,
               evidence_refs = EXCLUDED.evidence_refs
             RETURNING ocr_output_id, document_id, ocr_engine, ocr_engine_version, output_uri, output_checksum, confidence_score, quality_status, evidence_refs, created_at",
        )
        .bind(&input.ocr_output_id)
        .bind(&input.document_id)
        .bind(&input.ocr_engine)
        .bind(&input.ocr_engine_version)
        .bind(&input.output_uri)
        .bind(&input.output_checksum)
        .bind(input.confidence_score)
        .bind(&input.quality_status)
        .bind(string_values(&input.evidence_refs))
        .fetch_one(&self.pool)
        .await?;
        Ok(Some(evidence_ocr_output_from_row(row)?))
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
        let rows = sqlx::query(
            "SELECT ocr_output_id, document_id, ocr_engine, ocr_engine_version, output_uri, output_checksum, confidence_score, quality_status, evidence_refs, created_at
             FROM evidence_ocr_outputs
             WHERE document_id = $1
             ORDER BY ocr_output_id",
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(evidence_ocr_output_from_row).collect()
    }

    async fn save_evidence_embedding_job(
        &self,
        input: CreateEvidenceEmbeddingJobInput,
    ) -> anyhow::Result<EvidenceEmbeddingJobRecord> {
        let row = sqlx::query(
            "INSERT INTO evidence_embedding_jobs
             (embedding_job_id, customer_scope_id, target_kind, target_ref, embedding_model, embedding_model_version, chunking_version, redaction_status, vector_store_kind, vector_store_ref, embedding_checksum, status, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (embedding_job_id) DO UPDATE SET
               customer_scope_id = EXCLUDED.customer_scope_id,
               target_kind = EXCLUDED.target_kind,
               target_ref = EXCLUDED.target_ref,
               embedding_model = EXCLUDED.embedding_model,
               embedding_model_version = EXCLUDED.embedding_model_version,
               chunking_version = EXCLUDED.chunking_version,
               redaction_status = EXCLUDED.redaction_status,
               vector_store_kind = EXCLUDED.vector_store_kind,
               vector_store_ref = EXCLUDED.vector_store_ref,
               embedding_checksum = EXCLUDED.embedding_checksum,
               status = EXCLUDED.status,
               evidence_refs = EXCLUDED.evidence_refs
             RETURNING embedding_job_id, customer_scope_id, target_kind, target_ref, embedding_model, embedding_model_version, chunking_version, redaction_status, vector_store_kind, vector_store_ref, embedding_checksum, status, evidence_refs, created_at, completed_at",
        )
        .bind(&input.embedding_job_id)
        .bind(&input.customer_scope_id)
        .bind(&input.target_kind)
        .bind(&input.target_ref)
        .bind(&input.embedding_model)
        .bind(&input.embedding_model_version)
        .bind(&input.chunking_version)
        .bind(&input.redaction_status)
        .bind(&input.vector_store_kind)
        .bind(&input.vector_store_ref)
        .bind(&input.embedding_checksum)
        .bind(&input.status)
        .bind(string_values(&input.evidence_refs))
        .fetch_one(&self.pool)
        .await?;
        evidence_embedding_job_from_row(row)
    }

    async fn list_evidence_embedding_jobs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceEmbeddingJobRecord>> {
        let rows = sqlx::query(
            "SELECT embedding_job_id, customer_scope_id, target_kind, target_ref, embedding_model, embedding_model_version, chunking_version, redaction_status, vector_store_kind, vector_store_ref, embedding_checksum, status, evidence_refs, created_at, completed_at
             FROM evidence_embedding_jobs
             WHERE ($1::text IS NULL OR customer_scope_id = $1)
             ORDER BY embedding_job_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(evidence_embedding_job_from_row)
            .collect()
    }

    async fn save_evidence_retrieval_audit_event(
        &self,
        input: CreateEvidenceRetrievalAuditEventInput,
    ) -> anyhow::Result<EvidenceRetrievalAuditEventRecord> {
        let row = sqlx::query(
            "INSERT INTO evidence_retrieval_audit_events
             (retrieval_id, customer_scope_id, actor_id, actor_role, query_kind, query_checksum, retrieval_method, embedding_model_version, top_k, source_refs, result_refs, redaction_status, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (retrieval_id) DO UPDATE SET
               customer_scope_id = EXCLUDED.customer_scope_id,
               actor_id = EXCLUDED.actor_id,
               actor_role = EXCLUDED.actor_role,
               query_kind = EXCLUDED.query_kind,
               query_checksum = EXCLUDED.query_checksum,
               retrieval_method = EXCLUDED.retrieval_method,
               embedding_model_version = EXCLUDED.embedding_model_version,
               top_k = EXCLUDED.top_k,
               source_refs = EXCLUDED.source_refs,
               result_refs = EXCLUDED.result_refs,
               redaction_status = EXCLUDED.redaction_status,
               evidence_refs = EXCLUDED.evidence_refs
             RETURNING retrieval_id, customer_scope_id, actor_id, actor_role, query_kind, query_checksum, retrieval_method, embedding_model_version, top_k, source_refs, result_refs, redaction_status, evidence_refs, created_at",
        )
        .bind(&input.retrieval_id)
        .bind(&input.customer_scope_id)
        .bind(&input.actor_id)
        .bind(&input.actor_role)
        .bind(&input.query_kind)
        .bind(&input.query_checksum)
        .bind(&input.retrieval_method)
        .bind(&input.embedding_model_version)
        .bind(input.top_k)
        .bind(string_values(&input.source_refs))
        .bind(string_values(&input.result_refs))
        .bind(&input.redaction_status)
        .bind(string_values(&input.evidence_refs))
        .fetch_one(&self.pool)
        .await?;
        evidence_retrieval_audit_event_from_row(row)
    }

    async fn list_evidence_retrieval_audit_events(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceRetrievalAuditEventRecord>> {
        let rows = sqlx::query(
            "SELECT retrieval_id, customer_scope_id, actor_id, actor_role, query_kind, query_checksum, retrieval_method, embedding_model_version, top_k, source_refs, result_refs, redaction_status, evidence_refs, created_at
             FROM evidence_retrieval_audit_events
             WHERE ($1::text IS NULL OR customer_scope_id = $1)
             ORDER BY created_at DESC, retrieval_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(evidence_retrieval_audit_event_from_row)
            .collect()
    }
}

pub(crate) fn normalize_scheme_family(value: &str) -> String {
    canonical_scheme_family(value).unwrap_or_else(|| "high_risk_claim".into())
}

pub(crate) fn scheme_family_from_knowledge_signals(fwa_type: &str, tags: &[String]) -> String {
    if let Some(scheme_family) = tags
        .iter()
        .find_map(|tag| tag.strip_prefix("scheme:").map(normalize_scheme_family))
    {
        return scheme_family;
    }

    if tags
        .iter()
        .any(|tag| tag.contains("medical_mismatch") || tag.contains("diagnosis"))
    {
        "diagnosis_procedure_mismatch".into()
    } else if tags
        .iter()
        .any(|tag| tag.contains("lab") || tag.contains("testing"))
    {
        "laboratory_testing_abuse".into()
    } else if tags.iter().any(|tag| tag.contains("provider")) {
        "provider_peer_outlier".into()
    } else if tags
        .iter()
        .any(|tag| tag.contains("early") || tag.contains("high_amount"))
    {
        "early_high_value_claim".into()
    } else {
        match fwa_type {
            "Waste" => "excessive_utilization".into(),
            "Abuse" => "high_risk_claim".into(),
            "Fraud" => "relationship_concentration".into(),
            _ => "high_risk_claim".into(),
        }
    }
}

pub(super) fn json_array_to_strings(value: Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn string_values(values: &[String]) -> Value {
    Value::Array(values.iter().cloned().map(Value::String).collect())
}

async fn ensure_default_routing_policies_seeded(pool: &PgPool) -> anyhow::Result<()> {
    for policy in default_routing_policies() {
        sqlx::query(
            "INSERT INTO routing_policies
             (policy_key, version, review_mode, status, owner, policy_json, activated_at)
             VALUES ($1, $2, $3, 'active', 'system', $4, now())
             ON CONFLICT (policy_key, version, review_mode) DO UPDATE SET
               policy_json = EXCLUDED.policy_json",
        )
        .bind(&policy.policy_id)
        .bind(policy.version as i32)
        .bind(&policy.review_mode)
        .bind(serde_json::to_value(&policy)?)
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn insert_audit_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event: &PersistedAuditEvent,
    claim_uuid: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO audit_events
         (audit_id, run_id, claim_id, actor_id, actor_role, source_system, event_type, event_status, summary, payload, evidence_refs)
         VALUES ($1, $2, $3::uuid, $4, $5, $6, $7, $8, $9, $10, $11)
         ON CONFLICT (audit_id) DO UPDATE
         SET run_id = EXCLUDED.run_id,
             claim_id = EXCLUDED.claim_id,
             actor_id = EXCLUDED.actor_id,
             actor_role = EXCLUDED.actor_role,
             source_system = EXCLUDED.source_system,
             event_type = EXCLUDED.event_type,
             event_status = EXCLUDED.event_status,
             summary = EXCLUDED.summary,
             payload = EXCLUDED.payload,
             evidence_refs = EXCLUDED.evidence_refs",
    )
    .bind(&event.audit_id)
    .bind(&event.run_id)
    .bind(claim_uuid)
    .bind(&event.actor_id)
    .bind(&event.actor_role)
    .bind(&event.source_system)
    .bind(&event.event_type)
    .bind(&event.event_status)
    .bind(&event.summary)
    .bind(mask_audit_payload(event.payload.clone()))
    .bind(serde_json::Value::Array(event.evidence_refs.clone()))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_pilot_audit_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    claim_id: &str,
    event: &AuditHistoryEventRecord,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO scoring_runs
         (run_id, source_system, actor_id, status, completed_at)
         VALUES ($1, 'pilot-loop', $2, 'succeeded', now())
         ON CONFLICT (run_id) DO NOTHING",
    )
    .bind(&event.run_id)
    .bind(&event.actor_role)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "INSERT INTO audit_events
         (audit_id, run_id, claim_id, actor_id, actor_role, source_system, event_type, event_status, summary, payload, evidence_refs)
         VALUES ($1, $2, NULL, $3, $4, 'pilot-loop', $5, $6, $7, $8, $9)
         ON CONFLICT (audit_id) DO UPDATE
         SET event_status = EXCLUDED.event_status,
             summary = EXCLUDED.summary,
             payload = EXCLUDED.payload,
             evidence_refs = EXCLUDED.evidence_refs",
    )
    .bind(&event.audit_id)
    .bind(&event.run_id)
    .bind(claim_id)
    .bind(&event.actor_role)
    .bind(&event.event_type)
    .bind(&event.event_status)
    .bind(&event.summary)
    .bind(mask_audit_payload(event.payload.clone()))
    .bind(serde_json::json!(event.evidence_refs))
    .execute(&mut **tx)
    .await?;
    Ok(())
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
