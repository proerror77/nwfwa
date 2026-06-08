use super::{
    AuditEventListFilter, AuditHistoryEventRecord, PersistedAuditEvent,
    GOVERNANCE_AUDIT_EVENT_TYPES,
};
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn persisted_audit_event_matches_filter(
    event: &PersistedAuditEvent,
    filter: &AuditEventListFilter,
) -> bool {
    if !audit_event_matches_group(&event.event_type, filter) {
        return false;
    }
    if filter
        .event_type
        .as_deref()
        .is_some_and(|event_type| event.event_type != event_type)
    {
        return false;
    }
    if filter.actor_id.as_deref().is_some_and(|actor_id| {
        event.actor_id != actor_id && !audit_event_payload_matches_actor(&event.payload, actor_id)
    }) {
        return false;
    }
    if filter
        .customer_scope_id
        .as_deref()
        .is_some_and(|scope| !audit_event_payload_matches_customer_scope(&event.payload, scope))
    {
        return false;
    }
    if filter
        .run_id
        .as_deref()
        .is_some_and(|run_id| event.run_id != run_id)
    {
        return false;
    }
    if let Some(claim_id) = filter.claim_id.as_deref() {
        let payload_claim_id = event.payload["claim_id"].as_str();
        if event.claim_id != claim_id && payload_claim_id != Some(claim_id) {
            return false;
        }
    }
    if !audit_event_payload_matches_routing_policy_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_rule_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_model_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_qa_feedback_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_audit_sample_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_agent_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_data_lineage_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_canonical_trace_filter(&event.payload, filter) {
        return false;
    }
    true
}

pub(super) fn pilot_audit_event_matches_filter(
    claim_id: &str,
    event: &AuditHistoryEventRecord,
    filter: &AuditEventListFilter,
) -> bool {
    if !audit_event_matches_group(&event.event_type, filter) {
        return false;
    }
    if filter
        .event_type
        .as_deref()
        .is_some_and(|event_type| event.event_type != event_type)
    {
        return false;
    }
    if let Some(actor_id) = filter.actor_id.as_deref() {
        if !audit_event_payload_matches_actor(&event.payload, actor_id) {
            return false;
        }
    }
    if filter
        .customer_scope_id
        .as_deref()
        .is_some_and(|scope| !audit_event_payload_matches_customer_scope(&event.payload, scope))
    {
        return false;
    }
    if filter
        .run_id
        .as_deref()
        .is_some_and(|run_id| event.run_id != run_id)
    {
        return false;
    }
    if let Some(filter_claim_id) = filter.claim_id.as_deref() {
        let payload_claim_id = event.payload["claim_id"].as_str();
        if claim_id != filter_claim_id && payload_claim_id != Some(filter_claim_id) {
            return false;
        }
    }
    if !audit_event_payload_matches_routing_policy_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_rule_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_model_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_qa_feedback_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_audit_sample_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_agent_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_data_lineage_filter(&event.payload, filter) {
        return false;
    }
    if !audit_event_payload_matches_canonical_trace_filter(&event.payload, filter) {
        return false;
    }
    true
}

pub(super) fn audit_event_payload_matches_actor(payload: &Value, actor_id: &str) -> bool {
    payload["actor_id"].as_str() == Some(actor_id)
        || payload["reviewer"].as_str() == Some(actor_id)
        || payload["owner"].as_str() == Some(actor_id)
        || payload["approver"].as_str() == Some(actor_id)
        || payload["requested_by"].as_str() == Some(actor_id)
}

pub(super) fn audit_event_payload_matches_customer_scope(
    payload: &Value,
    customer_scope_id: &str,
) -> bool {
    payload["customer_scope_id"].as_str() == Some(customer_scope_id)
}

pub(super) fn scoped_claim_ids_from_audit_events<'a>(
    events: impl Iterator<Item = &'a PersistedAuditEvent>,
    customer_scope_id: &str,
) -> BTreeSet<String> {
    events
        .filter(|event| {
            audit_event_payload_matches_customer_scope(&event.payload, customer_scope_id)
        })
        .map(|event| event.claim_id.clone())
        .collect()
}

fn audit_event_matches_group(event_type: &str, filter: &AuditEventListFilter) -> bool {
    match filter.event_group.as_deref() {
        None => true,
        Some("governance") => GOVERNANCE_AUDIT_EVENT_TYPES.contains(&event_type),
        Some(_) => false,
    }
}

fn audit_event_payload_matches_canonical_trace_filter(
    payload: &Value,
    filter: &AuditEventListFilter,
) -> bool {
    if filter.has_canonical_trace != Some(true) {
        return true;
    }
    payload
        .get("canonical_claim_context_trace")
        .and_then(Value::as_object)
        .is_some()
}

fn audit_event_payload_matches_rule_filter(payload: &Value, filter: &AuditEventListFilter) -> bool {
    if filter
        .rule_id
        .as_deref()
        .is_some_and(|rule_id| payload["rule_id"].as_str() != Some(rule_id))
    {
        return false;
    }
    if filter
        .rule_version
        .as_deref()
        .is_some_and(|version| !payload_field_matches_text(payload, "rule_version", version))
    {
        return false;
    }
    true
}

fn audit_event_payload_matches_model_filter(
    payload: &Value,
    filter: &AuditEventListFilter,
) -> bool {
    if filter
        .model_key
        .as_deref()
        .is_some_and(|model_key| payload["model_key"].as_str() != Some(model_key))
    {
        return false;
    }
    if filter
        .model_version
        .as_deref()
        .is_some_and(|version| payload["model_version"].as_str() != Some(version))
    {
        return false;
    }
    true
}

fn audit_event_payload_matches_routing_policy_filter(
    payload: &Value,
    filter: &AuditEventListFilter,
) -> bool {
    if filter
        .routing_policy_id
        .as_deref()
        .is_some_and(|policy_id| payload["policy_id"].as_str() != Some(policy_id))
    {
        return false;
    }
    if filter
        .routing_policy_version
        .as_deref()
        .is_some_and(|version| !payload_field_matches_text(payload, "version", version))
    {
        return false;
    }
    if filter
        .review_mode
        .as_deref()
        .is_some_and(|review_mode| payload["review_mode"].as_str() != Some(review_mode))
    {
        return false;
    }
    true
}

fn audit_event_payload_matches_qa_feedback_filter(
    payload: &Value,
    filter: &AuditEventListFilter,
) -> bool {
    if filter
        .feedback_id
        .as_deref()
        .is_some_and(|feedback_id| payload["feedback_id"].as_str() != Some(feedback_id))
    {
        return false;
    }
    if filter
        .qa_case_id
        .as_deref()
        .is_some_and(|qa_case_id| payload["qa_case_id"].as_str() != Some(qa_case_id))
    {
        return false;
    }
    true
}

fn audit_event_payload_matches_audit_sample_filter(
    payload: &Value,
    filter: &AuditEventListFilter,
) -> bool {
    if filter
        .sample_id
        .as_deref()
        .is_some_and(|sample_id| payload["sample_id"].as_str() != Some(sample_id))
    {
        return false;
    }
    true
}

fn audit_event_payload_matches_agent_filter(
    payload: &Value,
    filter: &AuditEventListFilter,
) -> bool {
    if filter
        .agent_run_id
        .as_deref()
        .is_some_and(|agent_run_id| payload["agent_run_id"].as_str() != Some(agent_run_id))
    {
        return false;
    }
    true
}

fn audit_event_payload_matches_data_lineage_filter(
    payload: &Value,
    filter: &AuditEventListFilter,
) -> bool {
    if filter
        .dataset_id
        .as_deref()
        .is_some_and(|dataset_id| payload["dataset_id"].as_str() != Some(dataset_id))
    {
        return false;
    }
    if filter
        .feature_set_id
        .as_deref()
        .is_some_and(|feature_set_id| payload["feature_set_id"].as_str() != Some(feature_set_id))
    {
        return false;
    }
    if filter
        .model_dataset_id
        .as_deref()
        .is_some_and(|model_dataset_id| {
            payload["model_dataset_id"].as_str() != Some(model_dataset_id)
        })
    {
        return false;
    }
    if filter
        .evaluation_run_id
        .as_deref()
        .is_some_and(|evaluation_run_id| {
            payload["evaluation_run_id"].as_str() != Some(evaluation_run_id)
        })
    {
        return false;
    }
    true
}

fn payload_field_matches_text(payload: &Value, field: &str, expected: &str) -> bool {
    payload[field].as_str() == Some(expected)
        || payload[field]
            .as_u64()
            .map(|value| value.to_string())
            .as_deref()
            == Some(expected)
}

pub(super) fn audit_history_from_persisted(event: &PersistedAuditEvent) -> AuditHistoryEventRecord {
    AuditHistoryEventRecord {
        audit_id: event.audit_id.clone(),
        run_id: event.run_id.clone(),
        actor_role: event.actor_role.clone(),
        event_type: event.event_type.clone(),
        event_status: event.event_status.clone(),
        summary: event.summary.clone(),
        payload: event.payload.clone(),
        evidence_refs: evidence_values_to_strings(&event.evidence_refs),
        created_at: None,
    }
}

pub(super) fn evidence_values_to_strings(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .map(|value| match value {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .collect()
}
