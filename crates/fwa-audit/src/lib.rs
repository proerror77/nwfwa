use chrono::{DateTime, Utc};
use fwa_core::{AuditEventId, ClaimId, ScoringRunId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActorContext {
    pub actor_id: String,
    pub actor_role: String,
    pub source_system: String,
    pub customer_scope_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditEventStatus {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditEvent {
    pub audit_id: AuditEventId,
    pub run_id: ScoringRunId,
    pub claim_id: ClaimId,
    pub actor: ActorContext,
    pub event_type: String,
    pub event_status: AuditEventStatus,
    pub summary: String,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

pub fn scoring_completed(
    run_id: ScoringRunId,
    claim_id: ClaimId,
    actor: ActorContext,
    payload: Value,
) -> AuditEvent {
    AuditEvent {
        audit_id: AuditEventId::new(),
        run_id,
        claim_id,
        actor,
        event_type: "scoring.completed".into(),
        event_status: AuditEventStatus::Succeeded,
        summary: "FWA scoring completed".into(),
        payload,
        created_at: Utc::now(),
    }
}

pub fn scoring_failed(
    run_id: ScoringRunId,
    claim_id: ClaimId,
    actor: ActorContext,
    payload: Value,
) -> AuditEvent {
    AuditEvent {
        audit_id: AuditEventId::new(),
        run_id,
        claim_id,
        actor,
        event_type: "scoring.failed".into(),
        event_status: AuditEventStatus::Failed,
        summary: "FWA scoring failed".into(),
        payload,
        created_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_event_contains_run_and_claim_ids() {
        let event = scoring_completed(
            ScoringRunId::from_external("run_1"),
            ClaimId::from_external("CLM-1"),
            ActorContext {
                actor_id: "tpa-demo".into(),
                actor_role: "system".into(),
                source_system: "tpa-demo".into(),
                customer_scope_id: "demo-customer".into(),
            },
            serde_json::json!({"risk_score": 80}),
        );

        assert_eq!(event.run_id.as_str(), "run_1");
        assert_eq!(event.claim_id.as_str(), "CLM-1");
        assert_eq!(event.event_status, AuditEventStatus::Succeeded);
    }
}
