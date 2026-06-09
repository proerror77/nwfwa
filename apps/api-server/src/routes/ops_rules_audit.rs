use crate::{
    app::AppState,
    repository::{
        PersistedAuditEvent, RuleBacktestRecord, RulePromotionReviewRecord, RuleShadowRunRecord,
        RuleSummaryRecord,
    },
};
use fwa_audit::ActorContext;
use fwa_core::{AuditEventId, ScoringRunId};

pub(super) struct RuleAuditInput<'a> {
    pub(super) rule: &'a RuleSummaryRecord,
    pub(super) event_type: &'static str,
    pub(super) from_status: Option<&'a str>,
    pub(super) to_status: &'a str,
    pub(super) summary: &'static str,
    pub(super) evidence_refs: Vec<String>,
}

pub(super) async fn record_rule_audit(
    state: &AppState,
    actor: &ActorContext,
    input: RuleAuditInput<'_>,
) -> anyhow::Result<()> {
    let payload = serde_json::json!({
        "customer_scope_id": actor.customer_scope_id,
        "rule_id": input.rule.rule_id,
        "rule_version": input.rule.latest_version,
        "from_status": input.from_status,
        "to_status": input.to_status,
        "owner": input.rule.owner,
        "alert_code": input.rule.alert_code,
        "recommended_action": input.rule.recommended_action,
    });
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: input.event_type.to_string(),
            event_status: "succeeded".into(),
            summary: input.summary.into(),
            payload,
            evidence_refs: input
                .evidence_refs
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

pub(super) fn default_rule_evidence_refs(rule: &RuleSummaryRecord) -> Vec<String> {
    vec![format!("rules:{}:v{}", rule.rule_id, rule.latest_version)]
}

pub(super) async fn record_rule_backtest_audit(
    state: &AppState,
    actor: &ActorContext,
    record: &RuleBacktestRecord,
) -> anyhow::Result<()> {
    let mut payload = serde_json::to_value(record)?;
    if let Some(payload) = payload.as_object_mut() {
        payload.insert(
            "customer_scope_id".into(),
            serde_json::json!(actor.customer_scope_id),
        );
    }
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "rule.backtest.completed".into(),
            event_status: "succeeded".into(),
            summary: "Rule backtest completed".into(),
            payload,
            evidence_refs: record
                .evidence_refs
                .iter()
                .map(|reference| serde_json::json!(reference))
                .collect(),
        })
        .await
}

pub(super) async fn record_rule_promotion_audit(
    state: &AppState,
    actor: &ActorContext,
    review: &RulePromotionReviewRecord,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "rule.promotion.reviewed".into(),
            event_status: "succeeded".into(),
            summary: format!("Rule promotion review: {}", review.decision),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "rule_id": review.rule_id,
                "rule_version": review.rule_version,
                "decision": review.decision,
                "reviewer": review.reviewer,
                "note_present": !review.notes.trim().is_empty(),
            }),
            evidence_refs: review
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

pub(super) async fn record_rule_shadow_run_audit(
    state: &AppState,
    actor: &ActorContext,
    record: &RuleShadowRunRecord,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "rule.shadow_run.reviewed".into(),
            event_status: "succeeded".into(),
            summary: format!("Rule shadow run reviewed: {}", record.decision),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "rule_id": record.rule_id,
                "rule_version": record.rule_version,
                "decision": record.decision,
                "reviewer": record.reviewer,
                "report_uri": record.report_uri,
                "reviewed_count": record.reviewed_count,
                "matched_count": record.matched_count,
                "false_positive_count": record.false_positive_count,
                "false_positive_rate": record.false_positive_rate,
                "blocker_count": record.blockers.len(),
                "active_rule_writeback": false,
            }),
            evidence_refs: record
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}
