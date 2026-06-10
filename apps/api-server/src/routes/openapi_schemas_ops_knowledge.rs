use serde_json::{json, Value};

pub(super) fn knowledge_schemas() -> Value {
    json!({
        "KnowledgeCase": {
            "type": "object",
            "required": ["case_id", "title", "fwa_type", "scheme_family", "diagnosis_code", "provider_region", "summary", "outcome", "tags", "evidence_refs"],
            "properties": {
                "case_id": { "type": "string" },
                "title": { "type": "string" },
                "fwa_type": { "type": "string" },
                "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                "diagnosis_code": { "type": "string" },
                "provider_region": { "type": "string" },
                "provider_type": { "type": "string" },
                "summary": { "type": "string" },
                "outcome": { "type": "string" },
                "tags": { "type": "array", "items": { "type": "string" } },
                "evidence_refs": { "type": "array", "items": { "type": "string" } }
            }
        },
        "KnowledgeCaseListResponse": {
            "type": "object",
            "required": ["cases"],
            "properties": {
                "cases": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/KnowledgeCase" }
                }
            }
        },
        "PublishKnowledgeCaseRequest": {
            "type": "object",
            "required": ["case_id", "title", "fwa_type", "diagnosis_code", "provider_region", "provider_type", "summary", "outcome", "tags", "evidence_refs"],
            "properties": {
                "case_id": { "type": "string", "minLength": 1 },
                "title": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Knowledge case title must not contain PII."
                },
                "fwa_type": { "type": "string", "minLength": 1 },
                "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                "diagnosis_code": { "type": "string", "minLength": 1 },
                "provider_region": { "type": "string", "minLength": 1 },
                "provider_type": { "type": "string", "minLength": 1 },
                "summary": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Knowledge case summary must not contain PII."
                },
                "outcome": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Knowledge case outcome must not contain PII."
                },
                "tags": {
                    "type": "array",
                    "minItems": 1,
                    "description": "Knowledge case tags must not contain PII.",
                    "items": { "type": "string", "minLength": 1 }
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "description": "Must include at least one confirmed review source: investigation_results:* or qa_reviews:* and must not contain PII. When source_claim_id has a prior canonical_claim_context_trace, publish automatically preserves canonical evidence_refs from the scoring audit.",
                    "items": { "type": "string", "minLength": 1 },
                    "contains": {
                        "type": "string",
                        "pattern": "^(investigation_results|qa_reviews):"
                    }
                },
                "source_claim_id": { "type": ["string", "null"] }
            }
        },
        "PublishKnowledgeCaseResponse": {
            "type": "object",
            "required": ["case", "audit_id"],
            "properties": {
                "case": { "$ref": "#/components/schemas/KnowledgeCase" },
                "audit_id": { "type": "string" }
            }
        },
        "SimilarCaseSearchRequest": {
            "type": "object",
            "required": ["diagnosis_code", "provider_region", "tags"],
            "properties": {
                "claim_id": { "type": ["string", "null"] },
                "diagnosis_code": { "type": "string", "minLength": 1 },
                "provider_region": { "type": "string", "minLength": 1 },
                "tags": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } }
            }
        },
    })
}
