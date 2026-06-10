use serde_json::{json, Map, Value};

#[path = "openapi_schemas_ops_cases.rs"]
mod openapi_schemas_ops_cases;
#[path = "openapi_schemas_ops_dashboard.rs"]
mod openapi_schemas_ops_dashboard;
#[path = "openapi_schemas_ops_sampling.rs"]
mod openapi_schemas_ops_sampling;

pub(super) fn ops_schemas() -> Value {
    let mut schemas = Map::new();
    append_schemas(
        &mut schemas,
        openapi_schemas_ops_dashboard::dashboard_schemas(),
    );
    append_schemas(&mut schemas, openapi_schemas_ops_cases::case_schemas());
    append_schemas(
        &mut schemas,
        openapi_schemas_ops_sampling::sampling_schemas(),
    );
    append_schemas(
        &mut schemas,
        json!({
                "AgentRunLogRecord": {
                    "type": "object",
                    "required": ["agent_run_id", "claim_id", "status", "decision_boundary", "output_json", "evidence_refs", "steps", "context_snapshots", "policy_checks", "tool_calls", "tool_results", "approvals"],
                    "properties": {
                        "agent_run_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "status": { "type": "string" },
                        "decision_boundary": { "type": "string" },
                        "output_json": { "type": "object" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "steps": { "type": "array", "items": { "type": "object" } },
                        "context_snapshots": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AgentContextSnapshotRecord" }
                        },
                        "policy_checks": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AgentPolicyCheckRecord" }
                        },
                        "tool_calls": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AgentToolCallRecord" }
                        },
                        "tool_results": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AgentToolResultRecord" }
                        },
                        "approvals": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AgentApprovalRecord" }
                        },
                        "created_at": { "type": ["string", "null"] },
                        "completed_at": { "type": ["string", "null"] }
                    }
                },
                "AgentContextSnapshotRecord": {
                    "type": "object",
                    "required": ["snapshot_id", "redaction_status", "context_json", "source_refs", "checksum"],
                    "properties": {
                        "snapshot_id": { "type": "string" },
                        "redaction_status": { "type": "string" },
                        "context_json": { "type": "object" },
                        "source_refs": { "type": "array", "items": { "type": "string" } },
                        "checksum": { "type": "string" }
                    }
                },
                "AgentToolCallRecord": {
                    "type": "object",
                    "required": ["tool_call_id", "tool_name", "status", "input_json", "evidence_refs"],
                    "properties": {
                        "tool_call_id": { "type": "string" },
                        "tool_name": { "type": "string" },
                        "status": { "type": "string" },
                        "input_json": { "type": "object" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "AgentPolicyCheckRecord": {
                    "type": "object",
                    "required": ["policy_check_id", "agent_run_id", "tool_call_id", "tool_name", "policy_name", "decision", "reason", "evidence_refs"],
                    "properties": {
                        "policy_check_id": { "type": "string" },
                        "agent_run_id": { "type": "string" },
                        "tool_call_id": { "type": "string" },
                        "tool_name": { "type": "string" },
                        "policy_name": { "type": "string" },
                        "decision": { "type": "string", "enum": ["allowed", "denied"] },
                        "reason": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "created_at": { "type": ["string", "null"] }
                    }
                },
                "AgentToolResultRecord": {
                    "type": "object",
                    "required": ["tool_result_id", "tool_call_id", "tool_name", "status", "output_json", "evidence_refs"],
                    "properties": {
                        "tool_result_id": { "type": "string" },
                        "tool_call_id": { "type": "string" },
                        "tool_name": { "type": "string" },
                        "status": { "type": "string" },
                        "output_json": { "type": "object" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "AgentApprovalRecord": {
                    "type": "object",
                    "required": ["approval_id", "agent_run_id", "proposed_action", "decision", "approver", "reason", "evidence_refs"],
                    "properties": {
                        "approval_id": { "type": "string" },
                        "agent_run_id": { "type": "string" },
                        "proposed_action": { "type": "string" },
                        "decision": { "type": "string", "enum": ["pending", "approved", "rejected"] },
                        "approver": { "type": "string" },
                        "reason": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "created_at": { "type": ["string", "null"] }
                    }
                },
                "SubmitAgentApprovalRequest": {
                    "type": "object",
                    "required": ["decision", "approver", "reason", "evidence_refs"],
                    "properties": {
                        "decision": { "type": "string", "enum": ["approved", "rejected"] },
                        "approver": { "type": "string", "minLength": 1 },
                        "reason": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Agent approval reason must not contain PII."
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "description": "Must include agent_run:{agent_run_id} for the approved or rejected run and must not contain PII. The platform appends policy:{FWA_AGENT_POLICY_ID} to the persisted approval and audit event.",
                            "items": { "type": "string", "minLength": 1 },
                            "contains": {
                                "type": "string",
                                "pattern": "^agent_run:"
                            }
                        }
                    }
                },
                "SubmitAgentApprovalResponse": {
                    "type": "object",
                    "required": ["approval", "audit_id"],
                    "properties": {
                        "approval": { "$ref": "#/components/schemas/AgentApprovalRecord" },
                        "audit_id": { "type": "string" }
                    }
                },
                "AgentRunLogListResponse": {
                    "type": "object",
                    "required": ["runs"],
                    "properties": {
                        "runs": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AgentRunLogRecord" }
                        }
                    }
                },
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
                    "required": ["agent_run_id", "decision_boundary", "risk_summary", "findings", "investigation_checklist", "similar_cases", "qa_opinion_draft", "evidence_sufficiency", "evidence_refs", "evidence_refs_by_type"],
                    "properties": {
                        "agent_run_id": { "type": "string" },
                        "decision_boundary": { "type": "string", "const": "assistive_only" },
                        "risk_summary": { "type": "string" },
                        "findings": { "type": "array", "items": { "type": "object" } },
                        "investigation_checklist": { "type": "array", "items": { "type": "string" } },
                        "similar_cases": { "type": "array", "items": { "$ref": "#/components/schemas/AgentSimilarCase" } },
                        "qa_opinion_draft": { "type": "string" },
                        "evidence_sufficiency": { "$ref": "#/components/schemas/EvidenceSufficiency" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "evidence_refs_by_type": { "$ref": "#/components/schemas/EvidenceReferenceBuckets" }
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
                            "description": "Structured evidence references must not contain PII. For claims with a prior normalized scoring trace, canonical evidence refs from that trace are merged into the persisted investigation result and response.",
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
                            "description": "Structured evidence references must not contain PII.",
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
        }),
    );
    Value::Object(schemas)
}

fn append_schemas(target: &mut Map<String, Value>, schemas: Value) {
    let Value::Object(schemas) = schemas else {
        unreachable!("OpenAPI ops schema group must be a JSON object");
    };
    target.extend(schemas);
}
