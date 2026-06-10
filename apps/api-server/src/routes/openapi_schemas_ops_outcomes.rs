use serde_json::{json, Value};

pub(super) fn outcome_schemas() -> Value {
    json!({
        "OutcomeLabel": {
            "type": "object",
            "required": ["label_id", "claim_id", "label_name", "label_value", "source_type", "source_id", "governance_status", "feedback_target", "evidence_refs"],
            "properties": {
                "label_id": { "type": "string" },
                "claim_id": { "type": "string" },
                "label_name": { "type": "string" },
                "label_value": { "type": "string" },
                "source_type": { "type": "string", "enum": ["investigation_result", "qa_review", "case_status", "medical_review", "lead_triage"] },
                "source_id": { "type": "string" },
                "governance_status": { "type": "string", "enum": ["approved_for_training", "needs_review"] },
                "feedback_target": {
                    "type": "string",
                    "enum": ["rules", "model", "features", "provider_profile", "workflow", "tpa"]
                },
                "currency": { "type": ["string", "null"] },
                "evidence_refs": { "type": "array", "items": { "type": "string" } }
            }
        },
        "OutcomeLabelListResponse": {
            "type": "object",
            "required": ["labels"],
            "properties": {
                "labels": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/OutcomeLabel" }
                }
            }
        },
        "ClaimAuditHistoryResponse": {
            "type": "object",
            "required": ["claim_id", "events"],
            "properties": {
                "claim_id": { "type": "string" },
                "events": { "type": "array", "items": { "type": "object" } }
            }
        }
    })
}
