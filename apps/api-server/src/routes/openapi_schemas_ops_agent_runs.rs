use serde_json::{json, Value};

pub(super) fn agent_run_schemas() -> Value {
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
    })
}
