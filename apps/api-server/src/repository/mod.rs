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
mod in_memory;
mod knowledge_helpers;
mod member_profile_helpers;
mod model_helpers;
mod outcome_helpers;
mod postgres;
mod postgres_agent_helpers;
mod postgres_agents;
mod postgres_audit;
mod postgres_audit_samples;
mod postgres_cases;
mod postgres_dashboard;
mod postgres_datasets;
mod postgres_evidence;
mod postgres_inbox;
mod postgres_knowledge;
mod postgres_models;
mod postgres_outcomes;
mod postgres_providers;
mod postgres_routing_policies;
mod postgres_rules;
mod postgres_webhooks;
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
pub use self::in_memory::InMemoryScoringRepository;
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
pub use self::postgres::PostgresScoringRepository;
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
