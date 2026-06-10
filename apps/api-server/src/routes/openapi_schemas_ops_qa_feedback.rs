use serde_json::{json, Value};

pub(super) fn qa_feedback_schemas() -> Value {
    json!({
        "QaFeedbackItem": {
            "type": "object",
            "required": ["feedback_id", "qa_case_id", "claim_id", "feedback_target", "issue_type", "qa_conclusion", "source", "status", "priority", "summary", "note_present", "evidence_refs", "status_evidence_refs"],
            "properties": {
                "feedback_id": { "type": "string" },
                "qa_case_id": { "type": "string" },
                "claim_id": { "type": "string" },
                "feedback_target": {
                    "type": "string",
                    "enum": ["rules", "model", "features", "provider_profile", "workflow", "tpa"]
                },
                "issue_type": {
                    "type": "string",
                    "enum": ["none", "confirmed_fwa", "false_positive", "improper_payment", "insufficient_evidence", "abuse_not_fraud", "documentation_issue", "medical_necessity_issue", "policy_exclusion", "qa_review_completed", "alert_handling_incomplete", "medical_reasonableness", "provider_pattern", "model_under_scored_confirmed_issue", "workflow_missing_evidence"]
                },
                "qa_conclusion": {
                    "type": "string",
                    "enum": ["pass", "issue_found_return", "issue_found_escalate"]
                },
                "source": { "type": "string", "const": "qa_review" },
                "status": { "type": "string" },
                "priority": { "type": "string" },
                "summary": { "type": "string" },
                "note_present": { "type": "boolean" },
                "evidence_refs": { "type": "array", "items": { "type": "string" } },
                "created_at": { "type": ["string", "null"], "format": "date-time" },
                "status_updated_by": { "type": ["string", "null"] },
                "status_audit_id": { "type": ["string", "null"] },
                "status_updated_at": { "type": ["string", "null"], "format": "date-time" },
                "status_evidence_refs": { "type": "array", "items": { "type": "string" } }
            }
        },
        "QaFeedbackItemListResponse": {
            "type": "object",
            "required": ["items"],
            "properties": {
                "items": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/QaFeedbackItem" }
                }
            }
        },
        "UpdateQaFeedbackStatusRequest": {
            "type": "object",
            "required": ["status", "actor_id", "notes", "evidence_refs"],
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["open", "in_progress", "resolved", "dismissed"]
                },
                "actor_id": { "type": "string", "minLength": 1 },
                "notes": {
                    "type": "string",
                    "minLength": 1,
                    "description": "QA feedback status notes must not contain PII."
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "description": "Structured evidence references must include qa_feedback:{feedback_id} for the updated feedback item and must not contain PII.",
                    "items": { "type": "string", "minLength": 1 },
                    "contains": { "type": "string", "pattern": "^qa_feedback:" }
                }
            }
        },
        "UpdateQaFeedbackStatusResponse": {
            "type": "object",
            "required": ["item", "audit_id"],
            "properties": {
                "item": { "$ref": "#/components/schemas/QaFeedbackItem" },
                "audit_id": { "type": "string" }
            }
        },
        "QaQueueItem": {
            "type": "object",
            "required": ["qa_case_id", "sample_id", "lead_id", "claim_id", "scheme_family", "rag", "risk_score", "reviewer", "assignment_queue", "status", "evidence_refs", "canonical_source_refs", "canonical_evidence_refs"],
            "properties": {
                "qa_case_id": { "type": "string" },
                "sample_id": { "type": "string" },
                "lead_id": { "type": "string" },
                "claim_id": { "type": "string" },
                "scheme_family": { "type": "string" },
                "rag": { "type": "string" },
                "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "reviewer": { "type": "string" },
                "assignment_queue": { "type": "string" },
                "status": { "type": "string", "enum": ["open", "reviewed"] },
                "qa_conclusion": {
                    "type": ["string", "null"],
                    "enum": ["pass", "issue_found_return", "issue_found_escalate", null]
                },
                "issue_type": {
                    "type": ["string", "null"],
                    "enum": ["none", "confirmed_fwa", "false_positive", "improper_payment", "insufficient_evidence", "abuse_not_fraud", "documentation_issue", "medical_necessity_issue", "policy_exclusion", "qa_review_completed", "alert_handling_incomplete", "medical_reasonableness", "provider_pattern", "model_under_scored_confirmed_issue", "workflow_missing_evidence", null]
                },
                "feedback_target": {
                    "type": ["string", "null"],
                    "enum": ["rules", "model", "features", "provider_profile", "workflow", "tpa", null]
                },
                "evidence_refs": { "type": "array", "items": { "type": "string" } },
                "canonical_source_refs": { "type": "array", "items": { "type": "string" } },
                "canonical_evidence_refs": { "type": "array", "items": { "type": "string" } }
            }
        },
        "QaQueueListResponse": {
            "type": "object",
            "required": ["items"],
            "properties": {
                "items": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/QaQueueItem" }
                }
            }
        },
        "QaQueueSummaryResponse": {
            "type": "object",
            "required": ["open_count", "in_progress_count", "resolved_count", "dismissed_count", "unresolved_count", "rules_feedback_count", "models_feedback_count", "features_feedback_count", "provider_profile_feedback_count", "workflow_feedback_count", "tpa_feedback_count", "high_priority_count", "evidence_backed_count", "highest_priority"],
            "properties": {
                "open_count": { "type": "integer" },
                "in_progress_count": { "type": "integer" },
                "resolved_count": { "type": "integer" },
                "dismissed_count": { "type": "integer" },
                "unresolved_count": { "type": "integer" },
                "rules_feedback_count": { "type": "integer" },
                "models_feedback_count": { "type": "integer" },
                "features_feedback_count": { "type": "integer" },
                "provider_profile_feedback_count": { "type": "integer" },
                "workflow_feedback_count": { "type": "integer" },
                "tpa_feedback_count": { "type": "integer" },
                "high_priority_count": { "type": "integer" },
                "evidence_backed_count": { "type": "integer" },
                "highest_priority": { "type": "string", "enum": ["none", "low", "medium", "high"] }
            }
        },
    })
}
