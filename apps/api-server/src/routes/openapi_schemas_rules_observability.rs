use serde_json::{json, Value};

pub(super) fn observability_schemas() -> Value {
    json!({
        "AuditHistoryEvent": {
            "type": "object",
            "required": ["audit_id", "run_id", "actor_role", "event_type", "event_status", "summary", "payload", "evidence_refs"],
            "properties": {
                "audit_id": { "type": "string" },
                "run_id": { "type": "string" },
                "actor_role": { "type": "string" },
                "event_type": { "type": "string" },
                "event_status": { "type": "string" },
                "summary": { "type": "string" },
                "payload": { "type": "object" },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "created_at": { "type": ["string", "null"], "format": "date-time" }
            }
        },
        "AuditEventListResponse": {
            "type": "object",
            "required": ["events"],
            "properties": {
                "events": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/AuditHistoryEvent" }
                }
            }
        },
        "ApiCallRecord": {
            "type": "object",
            "required": ["call_id", "endpoint", "method", "status_code", "result", "source_system", "actor_role", "customer_scope_id", "claim_id", "run_id", "audit_id", "event_type", "idempotency_key", "evidence_refs", "observed_at"],
            "properties": {
                "call_id": { "type": "string" },
                "endpoint": { "type": "string" },
                "method": { "type": "string" },
                "status_code": { "type": "integer" },
                "result": { "type": "string" },
                "source_system": { "type": "string" },
                "actor_role": { "type": "string" },
                "customer_scope_id": { "type": "string" },
                "claim_id": { "type": "string" },
                "run_id": { "type": "string" },
                "audit_id": { "type": "string" },
                "event_type": { "type": "string" },
                "idempotency_key": { "type": ["string", "null"] },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "observed_at": { "type": ["string", "null"], "format": "date-time" }
            }
        },
        "ApiCallListResponse": {
            "type": "object",
            "required": ["calls"],
            "properties": {
                "calls": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/ApiCallRecord" }
                }
            }
        },
        "WebhookEvent": {
            "type": "object",
            "required": ["event_id", "event_type", "source_event_type", "source_audit_id", "customer_scope_id", "claim_id", "run_id", "delivery_status", "retry_count", "max_attempts", "next_attempt_at", "last_attempt_at", "last_response_status_code", "last_error_message", "idempotency_key", "signature_key_id", "signature_algorithm", "signature_base_string", "payload", "evidence_refs", "occurred_at"],
            "properties": {
                "event_id": { "type": "string" },
                "event_type": {
                    "type": "string",
                    "enum": ["fwa.score.completed", "fwa.case.routed", "fwa.investigation.closed", "fwa.qa.reviewed", "fwa.medical.reviewed"]
                },
                "source_event_type": { "type": "string" },
                "source_audit_id": { "type": "string" },
                "customer_scope_id": { "type": "string" },
                "claim_id": { "type": "string" },
                "run_id": { "type": "string" },
                "delivery_status": { "type": "string", "enum": ["pending", "retry_wait", "delivered", "failed"] },
                "retry_count": { "type": "integer" },
                "max_attempts": { "type": "integer" },
                "next_attempt_at": { "type": ["string", "null"], "format": "date-time" },
                "last_attempt_at": { "type": ["string", "null"], "format": "date-time" },
                "last_response_status_code": { "type": ["integer", "null"] },
                "last_error_message": { "type": ["string", "null"] },
                "idempotency_key": { "type": "string" },
                "signature_key_id": { "type": "string" },
                "signature_algorithm": { "type": "string", "enum": ["hmac-sha256"] },
                "signature_base_string": { "type": "string" },
                "payload": { "type": "object" },
                "evidence_refs": { "type": "array", "items": { "type": "string" } },
                "occurred_at": { "type": ["string", "null"], "format": "date-time" }
            }
        },
        "SubmitWebhookDeliveryAttemptRequest": {
            "type": "object",
            "required": ["delivery_status"],
            "properties": {
                "delivery_status": { "type": "string", "enum": ["delivered", "failed"] },
                "response_status_code": { "type": ["integer", "null"] },
                "error_message": {
                    "type": ["string", "null"],
                    "description": "Webhook delivery error message; must not contain PII."
                }
            }
        },
        "WebhookDeliveryAttempt": {
            "type": "object",
            "required": ["event_id", "attempt_number", "delivery_status", "response_status_code", "error_message", "next_attempt_at", "attempted_at"],
            "properties": {
                "event_id": { "type": "string" },
                "attempt_number": { "type": "integer" },
                "delivery_status": { "type": "string", "enum": ["delivered", "failed"] },
                "response_status_code": { "type": ["integer", "null"] },
                "error_message": { "type": ["string", "null"] },
                "next_attempt_at": { "type": ["string", "null"], "format": "date-time" },
                "attempted_at": { "type": ["string", "null"], "format": "date-time" }
            }
        },
        "WebhookEventListResponse": {
            "type": "object",
            "required": ["events"],
            "properties": {
                "events": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/WebhookEvent" }
                }
            }
        },
        "OpsAlert": {
            "type": "object",
            "required": ["alert_id", "alert_type", "severity", "status", "claim_id", "lead_id", "case_id", "scheme_family", "message", "recommended_action", "evidence_refs"],
            "properties": {
                "alert_id": { "type": "string" },
                "alert_type": {
                    "type": "string",
                    "enum": ["high_risk_routing", "sla_breach", "medical_review_required", "agent_approval_pending"]
                },
                "severity": {
                    "type": "string",
                    "enum": ["critical", "high", "medium", "low"]
                },
                "status": {
                    "type": "string",
                    "enum": ["open", "closed"]
                },
                "claim_id": { "type": "string" },
                "lead_id": { "type": ["string", "null"] },
                "case_id": { "type": ["string", "null"] },
                "scheme_family": { "type": "string" },
                "message": { "type": "string" },
                "recommended_action": { "type": "string" },
                "evidence_refs": { "type": "array", "items": { "type": "string" } }
            }
        },
        "OpsAlertListResponse": {
            "type": "object",
            "required": ["alerts"],
            "properties": {
                "alerts": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/OpsAlert" }
                }
            }
        },
    })
}
