use serde_json::{json, Value};

pub(super) fn case_schemas() -> Value {
    json!({
        "Lead": {
            "type": "object",
            "required": ["lead_id", "run_id", "claim_id", "member_id", "provider_id", "source_system", "review_mode", "scheme_family", "lead_source", "status", "disposition", "risk_score", "rag", "reason", "evidence_refs"],
            "properties": {
                "lead_id": { "type": "string" },
                "run_id": { "type": "string" },
                "claim_id": { "type": "string" },
                "member_id": { "type": "string" },
                "provider_id": { "type": "string" },
                "source_system": { "type": "string" },
                "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                "scheme_family": { "type": "string" },
                "lead_source": { "type": "string" },
                "status": { "type": "string" },
                "disposition": { "type": "string" },
                "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "rag": { "type": "string" },
                "reason": { "type": "string" },
                "evidence_refs": { "type": "array", "items": { "type": "string" } }
            }
        },
        "LeadListResponse": {
            "type": "object",
            "required": ["leads"],
            "properties": {
                "leads": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/Lead" }
                }
            }
        },
        "Case": {
            "type": "object",
            "required": ["case_id", "lead_id", "claim_id", "member_id", "provider_id", "source_system", "review_mode", "scheme_family", "lead_source", "status", "assignee", "reviewer", "priority", "routing_reason", "evidence_package", "sla_target_hours", "sla_status", "time_to_triage_hours", "time_to_closure_hours", "final_outcome", "reviewer_notes", "investigation_result_id"],
            "properties": {
                "case_id": { "type": "string" },
                "lead_id": { "type": "string" },
                "claim_id": { "type": "string" },
                "member_id": { "type": "string" },
                "provider_id": { "type": "string" },
                "source_system": { "type": "string" },
                "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                "scheme_family": { "type": "string" },
                "lead_source": { "type": "string" },
                "status": { "type": "string" },
                "assignee": { "type": "string" },
                "reviewer": { "type": "string" },
                "priority": { "type": "string" },
                "routing_reason": { "type": "string" },
                "evidence_package": { "$ref": "#/components/schemas/CaseEvidencePackage" },
                "sla_target_hours": { "type": "integer" },
                "sla_status": { "type": "string" },
                "time_to_triage_hours": { "type": "number" },
                "time_to_closure_hours": { "type": ["number", "null"] },
                "final_outcome": { "type": ["string", "null"] },
                "reviewer_notes": { "type": ["string", "null"] },
                "investigation_result_id": { "type": ["string", "null"] }
            }
        },
        "CaseEvidencePackage": {
            "type": "object",
            "required": ["lead_id", "claim_id", "risk_score", "rag", "reason", "triage_notes", "evidence_sufficiency", "evidence_refs", "evidence_refs_by_type"],
            "properties": {
                "lead_id": { "type": "string" },
                "claim_id": { "type": "string" },
                "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "rag": { "type": "string" },
                "reason": { "type": "string" },
                "triage_notes": { "type": "string" },
                "evidence_sufficiency": { "$ref": "#/components/schemas/EvidenceSufficiency" },
                "evidence_refs": { "type": "array", "items": { "type": "string" } },
                "evidence_refs_by_type": { "$ref": "#/components/schemas/EvidenceReferenceBuckets" }
            }
        },
        "CaseListResponse": {
            "type": "object",
            "required": ["cases"],
            "properties": {
                "cases": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/Case" }
                }
            }
        },
        "TriageLeadRequest": {
            "type": "object",
            "required": ["decision", "assignee", "reviewer", "priority", "notes", "evidence_refs"],
            "properties": {
                "decision": {
                    "type": "string",
                    "enum": ["open_case", "reject_lead", "request_evidence", "merge_lead"]
                },
                "merge_target_lead_id": { "type": ["string", "null"] },
                "assignee": { "type": "string", "minLength": 1 },
                "reviewer": { "type": "string", "minLength": 1 },
                "priority": { "type": "string", "minLength": 1 },
                "notes": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Triage notes must not contain PII."
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "description": "Structured triage decision evidence references must be production refs not local/template refs and must not contain PII.",
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "TriageLeadResponse": {
            "type": "object",
            "required": ["lead", "audit_id"],
            "properties": {
                "lead": { "$ref": "#/components/schemas/Lead" },
                "case": {
                    "oneOf": [
                        { "$ref": "#/components/schemas/Case" },
                        { "type": "null" }
                    ]
                },
                "audit_id": { "type": "string" }
            }
        },
        "UpdateCaseStatusRequest": {
            "type": "object",
            "required": ["status", "actor_id", "notes", "evidence_refs"],
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["triage", "investigating", "pending_evidence", "confirmed", "rejected", "closed"]
                },
                "actor_id": { "type": "string", "minLength": 1 },
                "notes": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Case status notes must not contain PII."
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "description": "Structured evidence references must be production refs not local/template refs and must not contain PII.",
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "UpdateCaseStatusResponse": {
            "type": "object",
            "required": ["case", "audit_id"],
            "properties": {
                "case": { "$ref": "#/components/schemas/Case" },
                "audit_id": { "type": "string" }
            }
        }
    })
}
