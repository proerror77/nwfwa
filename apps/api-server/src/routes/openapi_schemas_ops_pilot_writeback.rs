use serde_json::{json, Value};

pub(super) fn pilot_writeback_schemas() -> Value {
    json!({
        "InvestigationResultRequest": {
            "type": "object",
            "required": ["investigation_id", "claim_id", "outcome", "confirmed_fwa", "notes", "evidence_refs"],
            "properties": {
                "investigation_id": { "type": "string", "minLength": 1 },
                "case_id": { "type": ["string", "null"], "minLength": 1 },
                "claim_id": { "type": "string", "minLength": 1 },
                "outcome": { "type": "string", "minLength": 1 },
                "confirmed_fwa": { "type": "boolean" },
                "financial_impact_type": {
                    "type": ["string", "null"],
                    "enum": ["prevented_payment", "recovered_amount", "avoided_future_exposure", "deterrence_estimate", "estimated_impact", null]
                },
                "saving_amount": {
                    "type": ["string", "null"],
                    "format": "decimal",
                    "description": "Non-negative decimal string."
                },
                "currency": { "type": ["string", "null"] },
                "notes": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Investigation writeback notes must not contain PII."
                },
                "evidence_refs": {
                    "type": "array",
                    "description": "Structured evidence references must be production refs not local/template refs and must not contain PII. For claims with a prior normalized scoring trace, canonical evidence refs from that trace are merged into the persisted investigation result and response.",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "QaResultRequest": {
            "type": "object",
            "required": ["qa_case_id", "claim_id", "qa_conclusion", "issue_type", "feedback_target", "notes", "evidence_refs"],
            "properties": {
                "qa_case_id": { "type": "string", "minLength": 1 },
                "claim_id": { "type": "string", "minLength": 1 },
                "qa_conclusion": {
                    "type": "string",
                    "enum": ["pass", "issue_found_return", "issue_found_escalate"]
                },
                "issue_type": {
                    "type": "string",
                    "enum": ["none", "confirmed_fwa", "false_positive", "improper_payment", "insufficient_evidence", "abuse_not_fraud", "documentation_issue", "medical_necessity_issue", "policy_exclusion", "qa_review_completed", "alert_handling_incomplete", "medical_reasonableness", "provider_pattern", "model_under_scored_confirmed_issue", "workflow_missing_evidence"]
                },
                "feedback_target": {
                    "type": "string",
                    "enum": ["rules", "model", "models", "features", "provider_profile", "workflow", "tpa"]
                },
                "notes": {
                    "type": "string",
                    "minLength": 1,
                    "description": "QA writeback notes must not contain PII."
                },
                "evidence_refs": {
                    "type": "array",
                    "description": "Structured evidence references must be production refs not local/template refs and must not contain PII.",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "PilotWritebackResponse": {
            "type": "object",
            "required": ["claim_id", "event_type", "event_status", "audit_id", "run_id", "idempotency_key", "evidence_refs"],
            "properties": {
                "claim_id": { "type": "string" },
                "event_type": { "type": "string" },
                "event_status": { "type": "string" },
                "audit_id": { "type": "string" },
                "run_id": { "type": "string" },
                "idempotency_key": {
                    "type": "string",
                    "description": "Stable key for retry-safe TPA writeback processing."
                },
                "evidence_refs": { "type": "array", "items": { "type": "string" } }
            }
        },
        "MemberProfileSummaryResponse": {
            "type": "object",
            "required": ["member_id", "claim_count", "policy_count", "total_claim_amount", "currency", "high_risk_claim_count", "risk_level_summary", "profile_summary", "evidence_refs"],
            "properties": {
                "member_id": { "type": "string" },
                "claim_count": { "type": "integer" },
                "policy_count": { "type": "integer" },
                "total_claim_amount": { "type": "string", "format": "decimal" },
                "currency": { "type": "string" },
                "high_risk_claim_count": { "type": "integer" },
                "latest_claim_id": { "type": ["string", "null"] },
                "risk_level_summary": { "type": "string", "enum": ["no_high_risk_history", "has_high_risk_history"] },
                "profile_summary": { "type": "string" },
                "evidence_refs": { "type": "array", "items": { "type": "string" } }
            }
        },
    })
}
