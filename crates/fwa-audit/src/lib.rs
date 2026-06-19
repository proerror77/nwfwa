use chrono::{DateTime, Utc};
use fwa_core::{AuditEventId, ClaimId, ScoringRunId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use fwa_core::ActorContext;

/// Discrete set of known audit event types.
///
/// The `AuditEvent.event_type` field remains a `String` for forward
/// compatibility (the repository may store events produced by older code or
/// external systems).  Use `AuditEventType` to produce and pattern-match
/// well-known types in platform code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditEventType {
    // Scoring pipeline
    ScoringCompleted,
    ScoringFailed,
    // Claim ingestion
    InboxNormalized,
    InboxRejected,
    // Rule lifecycle
    RuleEvaluationCompleted,
    RulePromotionApproved,
    RulePublished,
    RuleRolledBack,
    // Model / ML lifecycle
    ModelActivated,
    ModelRolledBack,
    // Agent / investigation
    AgentInvestigationCompleted,
    AgentInvestigationFailed,
    AgentApprovalDecided,
    AgentRunCancelled,
    // Case workflow
    LeadTriaged,
    CaseStatusChanged,
    // Medical & QA review
    MedicalReviewRecorded,
    QaReviewRecorded,
    // Knowledge base
    KnowledgeCasePublished,
    // Catch-all for types not yet enumerated
    Other(String),
}

impl AuditEventType {
    /// Returns the canonical string representation used in the database and API.
    pub fn as_str(&self) -> &str {
        match self {
            Self::ScoringCompleted => "scoring.completed",
            Self::ScoringFailed => "scoring.failed",
            Self::InboxNormalized => "inbox.normalized",
            Self::InboxRejected => "inbox.rejected",
            Self::RuleEvaluationCompleted => "rule.evaluation.completed",
            Self::RulePromotionApproved => "rule.promotion.approved",
            Self::RulePublished => "rule.published",
            Self::RuleRolledBack => "rule.rolled_back",
            Self::ModelActivated => "model.activated",
            Self::ModelRolledBack => "model.rolled_back",
            Self::AgentInvestigationCompleted => "agent.investigation.completed",
            Self::AgentInvestigationFailed => "agent.investigation.failed",
            Self::AgentApprovalDecided => "agent.approval.decided",
            Self::AgentRunCancelled => "agent.run.cancelled",
            Self::LeadTriaged => "lead.triaged",
            Self::CaseStatusChanged => "case.status.changed",
            Self::MedicalReviewRecorded => "medical.review.recorded",
            Self::QaReviewRecorded => "qa.review.recorded",
            Self::KnowledgeCasePublished => "knowledge.case.published",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Parse a string into the matching variant (returns `Other` for unknowns).
    pub fn from_event_type(s: &str) -> Self {
        match s {
            "scoring.completed" => Self::ScoringCompleted,
            "scoring.failed" => Self::ScoringFailed,
            "inbox.normalized" => Self::InboxNormalized,
            "inbox.rejected" => Self::InboxRejected,
            "rule.evaluation.completed" => Self::RuleEvaluationCompleted,
            "rule.promotion.approved" => Self::RulePromotionApproved,
            "rule.published" => Self::RulePublished,
            "rule.rolled_back" => Self::RuleRolledBack,
            "model.activated" => Self::ModelActivated,
            "model.rolled_back" => Self::ModelRolledBack,
            "agent.investigation.completed" => Self::AgentInvestigationCompleted,
            "agent.investigation.failed" => Self::AgentInvestigationFailed,
            "agent.approval.decided" => Self::AgentApprovalDecided,
            "agent.run.cancelled" => Self::AgentRunCancelled,
            "lead.triaged" => Self::LeadTriaged,
            "case.status.changed" => Self::CaseStatusChanged,
            "medical.review.recorded" => Self::MedicalReviewRecorded,
            "qa.review.recorded" => Self::QaReviewRecorded,
            "knowledge.case.published" => Self::KnowledgeCasePublished,
            other => Self::Other(other.to_string()),
        }
    }
}

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
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
    /// Canonical event type string — use `AuditEventType::from_event_type` to match
    /// against known variants.
    pub event_type: String,
    pub event_status: AuditEventStatus,
    pub summary: String,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

// ── Scoring ───────────────────────────────────────────────────────────────────

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
        event_type: AuditEventType::ScoringCompleted.to_string(),
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
        event_type: AuditEventType::ScoringFailed.to_string(),
        event_status: AuditEventStatus::Failed,
        summary: "FWA scoring failed".into(),
        payload,
        created_at: Utc::now(),
    }
}

// ── Inbox ─────────────────────────────────────────────────────────────────────

pub fn inbox_normalized(
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
        event_type: AuditEventType::InboxNormalized.to_string(),
        event_status: AuditEventStatus::Succeeded,
        summary: "Claim inbox normalised".into(),
        payload,
        created_at: Utc::now(),
    }
}

pub fn inbox_rejected(
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
        event_type: AuditEventType::InboxRejected.to_string(),
        event_status: AuditEventStatus::Failed,
        summary: "Claim inbox rejected".into(),
        payload,
        created_at: Utc::now(),
    }
}

// ── Agent investigation ───────────────────────────────────────────────────────

pub fn agent_investigation_completed(
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
        event_type: AuditEventType::AgentInvestigationCompleted.to_string(),
        event_status: AuditEventStatus::Succeeded,
        summary: "Agent investigation completed".into(),
        payload,
        created_at: Utc::now(),
    }
}

pub fn agent_investigation_failed(
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
        event_type: AuditEventType::AgentInvestigationFailed.to_string(),
        event_status: AuditEventStatus::Failed,
        summary: "Agent investigation failed".into(),
        payload,
        created_at: Utc::now(),
    }
}

// ── Case workflow ─────────────────────────────────────────────────────────────

pub fn lead_triaged(
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
        event_type: AuditEventType::LeadTriaged.to_string(),
        event_status: AuditEventStatus::Succeeded,
        summary: "Lead triaged".into(),
        payload,
        created_at: Utc::now(),
    }
}

pub fn case_status_changed(
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
        event_type: AuditEventType::CaseStatusChanged.to_string(),
        event_status: AuditEventStatus::Succeeded,
        summary: "Case status changed".into(),
        payload,
        created_at: Utc::now(),
    }
}

// ── Medical & QA review ───────────────────────────────────────────────────────

pub fn medical_review_recorded(
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
        event_type: AuditEventType::MedicalReviewRecorded.to_string(),
        event_status: AuditEventStatus::Succeeded,
        summary: "Medical review recorded".into(),
        payload,
        created_at: Utc::now(),
    }
}

pub fn qa_review_recorded(
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
        event_type: AuditEventType::QaReviewRecorded.to_string(),
        event_status: AuditEventStatus::Succeeded,
        summary: "QA review recorded".into(),
        payload,
        created_at: Utc::now(),
    }
}

// ── Knowledge base ────────────────────────────────────────────────────────────

pub fn knowledge_case_published(
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
        event_type: AuditEventType::KnowledgeCasePublished.to_string(),
        event_status: AuditEventStatus::Succeeded,
        summary: "Knowledge case published".into(),
        payload,
        created_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_actor() -> ActorContext {
        ActorContext {
            actor_id: "tpa-demo".into(),
            actor_role: "system".into(),
            source_system: "tpa-demo".into(),
            customer_scope_id: "demo-customer".into(),
        }
    }

    #[test]
    fn completed_event_contains_run_and_claim_ids() {
        let event = scoring_completed(
            ScoringRunId::from_external("run_1"),
            ClaimId::from_external("CLM-1"),
            demo_actor(),
            serde_json::json!({"risk_score": 80}),
        );

        assert_eq!(event.run_id.as_str(), "run_1");
        assert_eq!(event.claim_id.as_str(), "CLM-1");
        assert_eq!(event.event_status, AuditEventStatus::Succeeded);
        assert_eq!(event.event_type, "scoring.completed");
    }

    #[test]
    fn failed_event_has_failed_status() {
        let event = scoring_failed(
            ScoringRunId::from_external("run_2"),
            ClaimId::from_external("CLM-2"),
            demo_actor(),
            serde_json::json!({"error": "model unavailable"}),
        );

        assert_eq!(event.event_status, AuditEventStatus::Failed);
        assert_eq!(event.event_type, "scoring.failed");
    }

    #[test]
    fn audit_event_type_round_trips() {
        let known = [
            AuditEventType::ScoringCompleted,
            AuditEventType::ScoringFailed,
            AuditEventType::InboxNormalized,
            AuditEventType::AgentInvestigationCompleted,
            AuditEventType::LeadTriaged,
            AuditEventType::CaseStatusChanged,
            AuditEventType::MedicalReviewRecorded,
            AuditEventType::QaReviewRecorded,
            AuditEventType::KnowledgeCasePublished,
        ];
        for variant in known {
            let s = variant.as_str().to_string();
            assert_eq!(AuditEventType::from_event_type(&s).as_str(), s);
        }
    }

    #[test]
    fn unknown_event_type_becomes_other() {
        let t = AuditEventType::from_event_type("custom.event.type");
        assert!(matches!(t, AuditEventType::Other(_)));
        assert_eq!(t.as_str(), "custom.event.type");
    }

    #[test]
    fn all_constructors_produce_correct_event_type() {
        let pairs: Vec<(&str, AuditEvent)> = vec![
            (
                "inbox.normalized",
                inbox_normalized(
                    ScoringRunId::from_external("r"),
                    ClaimId::from_external("c"),
                    demo_actor(),
                    Value::Null,
                ),
            ),
            (
                "agent.investigation.completed",
                agent_investigation_completed(
                    ScoringRunId::from_external("r"),
                    ClaimId::from_external("c"),
                    demo_actor(),
                    Value::Null,
                ),
            ),
            (
                "medical.review.recorded",
                medical_review_recorded(
                    ScoringRunId::from_external("r"),
                    ClaimId::from_external("c"),
                    demo_actor(),
                    Value::Null,
                ),
            ),
            (
                "knowledge.case.published",
                knowledge_case_published(
                    ScoringRunId::from_external("r"),
                    ClaimId::from_external("c"),
                    demo_actor(),
                    Value::Null,
                ),
            ),
        ];
        for (expected_type, event) in pairs {
            assert_eq!(event.event_type, expected_type);
            assert_eq!(event.event_status, AuditEventStatus::Succeeded);
        }
    }
}
