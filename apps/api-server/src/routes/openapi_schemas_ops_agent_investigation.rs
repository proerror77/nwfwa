use serde_json::{json, Value};

pub(super) fn agent_investigation_schemas() -> Value {
    json!({
        "AgentInvestigationPrefill": {
            "type": "object",
            "required": ["claim_id", "risk_score", "rag", "scheme_family", "top_reasons", "similar_case_query", "evidence_refs"],
            "properties": {
                "claim_id": { "type": "string", "minLength": 1 },
                "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "rag": { "type": "string", "enum": ["GREEN", "AMBER", "RED"] },
                "scheme_family": {
                    "oneOf": [
                        { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        { "type": "null" }
                    ]
                },
                "top_reasons": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } },
                "similar_case_query": { "$ref": "#/components/schemas/SimilarCaseSearchRequest" },
                "evidence_refs": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } }
            }
        },
        "SimilarCase": {
            "type": "object",
            "required": ["case_id", "title", "scheme_family", "similarity_score", "matched_signals", "retrieval_method", "provenance_refs", "summary", "outcome", "evidence_refs"],
            "properties": {
                "case_id": { "type": "string" },
                "title": { "type": "string" },
                "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                "similarity_score": { "type": "number" },
                "matched_signals": { "type": "array", "items": { "type": "string" } },
                "retrieval_method": { "type": "string" },
                "provenance_refs": { "type": "array", "items": { "type": "string" } },
                "summary": { "type": "string" },
                "outcome": { "type": "string" },
                "evidence_refs": { "type": "array", "items": { "type": "string" } }
            }
        },
        "SimilarCaseSearchResponse": {
            "type": "object",
            "required": ["results"],
            "properties": {
                "results": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/SimilarCase" }
                }
            }
        },
        "AgentSimilarCase": {
            "type": "object",
            "required": ["case_id", "similarity_score", "matched_signals", "provenance_refs", "evidence_refs"],
            "properties": {
                "case_id": { "type": "string" },
                "similarity_score": { "type": "number" },
                "matched_signals": { "type": "array", "items": { "type": "string" } },
                "provenance_refs": { "type": "array", "items": { "type": "string" } },
                "evidence_refs": { "type": "array", "items": { "type": "string" } }
            }
        },
        "AgentInvestigationRequest": {
            "type": "object",
            "required": ["claim_id", "risk_score", "rag", "top_reasons", "similar_case_query"],
            "properties": {
                "investigation_id": {
                    "type": ["string", "null"],
                    "minLength": 1,
                    "description": "Optional stable investigation id. Reuse it to group multiple agent runs under one investigation."
                },
                "claim_id": { "type": "string", "minLength": 1 },
                "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "rag": { "type": "string", "enum": ["GREEN", "AMBER", "RED"] },
                "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                "top_reasons": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } },
                "similar_case_query": { "$ref": "#/components/schemas/SimilarCaseSearchRequest" }
            }
        },
        "AgentInvestigationResponse": {
            "type": "object",
            "required": ["investigation_id", "agent_run_id", "decision_boundary", "risk_summary", "findings", "investigation_checklist", "similar_cases", "qa_opinion_draft", "evidence_sufficiency", "evidence_refs", "evidence_refs_by_type", "specialist_executions"],
            "properties": {
                "investigation_id": { "type": "string" },
                "agent_run_id": { "type": "string" },
                "decision_boundary": { "type": "string", "const": "assistive_only" },
                "risk_summary": { "type": "string" },
                "findings": { "type": "array", "items": { "type": "object" } },
                "investigation_checklist": { "type": "array", "items": { "type": "string" } },
                "similar_cases": { "type": "array", "items": { "$ref": "#/components/schemas/AgentSimilarCase" } },
                "qa_opinion_draft": { "type": "string" },
                "evidence_sufficiency": { "$ref": "#/components/schemas/EvidenceSufficiency" },
                "evidence_refs": { "type": "array", "items": { "type": "string" } },
                "evidence_refs_by_type": { "$ref": "#/components/schemas/EvidenceReferenceBuckets" },
                "specialist_executions": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/SpecialistAgentExecution" }
                }
            }
        },
        "SpecialistAgentExecution": {
            "type": "object",
            "required": ["agent_kind", "status", "responsibility", "decision_boundary", "phi_fields_allowed", "tool_calls", "evidence_refs", "summary"],
            "properties": {
                "agent_kind": { "type": "string" },
                "status": { "type": "string" },
                "responsibility": { "type": "string" },
                "decision_boundary": { "type": "string", "const": "assistive_only" },
                "phi_fields_allowed": { "type": "array", "items": { "type": "string" } },
                "tool_calls": { "type": "array", "items": { "$ref": "#/components/schemas/MediatedToolCall" } },
                "evidence_refs": { "type": "array", "items": { "type": "string" } },
                "summary": { "type": "string" }
            }
        },
        "MediatedToolCall": {
            "type": "object",
            "required": ["tool_name", "purpose", "input_scope", "policy_check", "cancellation_checkpoint", "execution_mode", "decision_boundary"],
            "properties": {
                "tool_name": { "type": "string" },
                "purpose": { "type": "string" },
                "input_scope": { "type": "array", "items": { "type": "string" } },
                "policy_check": { "type": "string" },
                "cancellation_checkpoint": {
                    "type": "string",
                    "description": "Checkpoint that must be evaluated before executing the mediated tool call."
                },
                "execution_mode": { "type": "string", "const": "contract_only_not_executed" },
                "decision_boundary": { "type": "string", "const": "assistive_only" }
            }
        },
        "EvidenceReferenceBuckets": {
            "type": "object",
            "required": ["claim", "rule", "model", "anomaly", "document", "similar_case"],
            "properties": {
                "claim": { "type": "array", "items": { "type": "string" } },
                "rule": { "type": "array", "items": { "type": "string" } },
                "model": { "type": "array", "items": { "type": "string" } },
                "anomaly": { "type": "array", "items": { "type": "string" } },
                "document": { "type": "array", "items": { "type": "string" } },
                "similar_case": { "type": "array", "items": { "type": "string" } }
            }
        },
        "EvidenceSufficiency": {
            "type": "object",
            "required": ["scheme_family", "status", "minimum_evidence", "present_evidence", "missing_evidence"],
            "properties": {
                "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                "status": { "type": "string", "enum": ["sufficient", "needs_more_evidence"] },
                "minimum_evidence": { "type": "array", "items": { "type": "string" } },
                "present_evidence": { "type": "array", "items": { "type": "string" } },
                "missing_evidence": { "type": "array", "items": { "type": "string" } }
            }
        },
    })
}
