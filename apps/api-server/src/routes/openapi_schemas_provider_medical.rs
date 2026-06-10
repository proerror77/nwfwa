use serde_json::{json, Value};

pub(super) fn provider_medical_schemas() -> Value {
    json!({
        "SubmitMedicalReviewResultRequest": {
            "type": "object",
            "required": ["claim_id", "scoring_audit_id", "reviewer", "decision", "notes", "evidence_refs"],
            "properties": {
                "claim_id": { "type": "string", "minLength": 1 },
                "scoring_audit_id": { "type": "string", "minLength": 1 },
                "reviewer": { "type": "string", "minLength": 1 },
                "decision": {
                    "type": "string",
                    "enum": ["evidence_sufficient", "request_more_evidence", "medical_necessity_issue", "no_medical_issue"]
                },
                "clinical_outcomes": {
                    "type": "array",
                    "description": "Optional controlled clinical outcome fields for model training and rule tuning. When omitted, the platform derives one compatible outcome from decision.",
                    "items": {
                        "type": "string",
                        "enum": ["documentation_issue", "medical_necessity_review_required", "insufficient_evidence", "medical_necessity_issue", "clinical_evidence_sufficient", "false_positive"]
                    }
                },
                "notes": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Medical review notes must not contain PII."
                },
                "evidence_refs": {
                    "type": "array",
                    "description": "Structured evidence references must not contain PII. For claims with the referenced normalized scoring trace, canonical evidence refs from that trace are merged into the persisted medical review and response.",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "MedicalReviewResultResponse": {
            "type": "object",
            "required": ["claim_id", "event_type", "event_status", "audit_id", "run_id", "review_status", "clinical_outcomes", "evidence_refs"],
            "properties": {
                "claim_id": { "type": "string" },
                "event_type": { "type": "string" },
                "event_status": { "type": "string" },
                "audit_id": { "type": "string" },
                "run_id": { "type": "string" },
                "review_status": { "type": "string" },
                "clinical_outcomes": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "evidence_refs": { "type": "array", "items": { "type": "string" } }
            }
        },
        "MedicalReviewQueueItem": {
            "type": "object",
            "required": ["claim_id", "run_id", "audit_id", "medical_reasonableness_score", "review_route", "evidence_status", "missing_evidence", "item_finding_count", "evidence_refs", "canonical_source_refs", "canonical_evidence_refs", "review_status"],
            "properties": {
                "claim_id": { "type": "string" },
                "run_id": { "type": "string" },
                "audit_id": { "type": "string" },
                "medical_reasonableness_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "review_route": { "type": "string" },
                "evidence_status": { "type": "string" },
                "missing_evidence": { "type": "array", "items": { "type": "string" } },
                "item_finding_count": { "type": "integer" },
                "first_item_code": { "type": ["string", "null"] },
                "first_issue_type": { "type": ["string", "null"] },
                "evidence_refs": { "type": "array", "items": { "type": "string" } },
                "canonical_source_refs": { "type": "array", "items": { "type": "string" } },
                "canonical_evidence_refs": { "type": "array", "items": { "type": "string" } },
                "created_at": { "type": ["string", "null"], "format": "date-time" },
                "review_status": { "type": "string" },
                "review_audit_id": { "type": ["string", "null"] },
                "review_decision": { "type": ["string", "null"] },
                "reviewer": { "type": ["string", "null"] },
                "reviewed_at": { "type": ["string", "null"], "format": "date-time" }
            }
        },
        "MedicalReviewQueueResponse": {
            "type": "object",
            "required": ["items"],
            "properties": {
                "items": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/MedicalReviewQueueItem" }
                }
            }
        },
        "ClinicalEvidenceAssessment": {
            "type": "object",
            "required": [
                "review_required",
                "review_route",
                "evidence_status",
                "minimum_evidence",
                "missing_evidence",
                "item_findings",
                "evidence_refs"
            ],
            "properties": {
                "review_required": { "type": "boolean" },
                "review_route": { "type": "string", "enum": ["none", "medical_review"] },
                "evidence_status": {
                    "type": "string",
                    "enum": [
                        "no_clinical_evidence_required",
                        "sufficient_for_basic_review",
                        "missing_required_evidence"
                    ]
                },
                "minimum_evidence": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "missing_evidence": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "item_findings": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/ClinicalEvidenceFinding" }
                },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            }
        },
        "ClinicalEvidenceFinding": {
            "type": "object",
            "required": [
                "item_code",
                "issue_type",
                "required_evidence",
                "missing_evidence",
                "reason",
                "review_route",
                "evidence_refs"
            ],
            "properties": {
                "item_code": { "type": "string" },
                "issue_type": { "type": "string" },
                "required_evidence": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "missing_evidence": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "reason": { "type": "string" },
                "review_route": { "type": "string" },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            }
        },
    })
}
