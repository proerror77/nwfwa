use serde_json::{json, Value};

pub(super) fn routing_policy_schemas() -> Value {
    json!({
        "RoutingPolicyRecord": {
            "type": "object",
            "required": ["policy_id", "version", "review_mode", "status", "owner", "risk_thresholds", "confidence_thresholds", "provider_review_threshold", "activated_at", "created_at"],
            "properties": {
                "policy_id": { "type": "string" },
                "version": { "type": "integer", "minimum": 1 },
                "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                "status": { "type": "string", "enum": ["draft", "submitted", "approved", "active", "retired"] },
                "owner": { "type": "string" },
                "risk_thresholds": { "$ref": "#/components/schemas/RiskThresholds" },
                "confidence_thresholds": { "$ref": "#/components/schemas/ConfidenceThresholds" },
                "provider_review_threshold": { "type": "integer", "minimum": 0, "maximum": 100 },
                "activated_at": { "type": ["string", "null"], "format": "date-time" },
                "created_at": { "type": ["string", "null"], "format": "date-time" }
            }
        },
        "RoutingPolicyListResponse": {
            "type": "object",
            "required": ["policies"],
            "properties": {
                "policies": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/RoutingPolicyRecord" }
                }
            }
        },
        "SaveRoutingPolicyCandidateRequest": {
            "type": "object",
            "required": ["policy"],
            "properties": {
                "policy": { "$ref": "#/components/schemas/RoutingPolicy" },
                "owner": { "type": ["string", "null"], "minLength": 1 }
            }
        },
        "RoutingPolicyLifecycleRequest": {
            "type": "object",
            "required": ["evidence_refs"],
            "properties": {
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "description": "Structured evidence references must not contain PII.",
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "RoutingPolicyPromotionGate": {
            "type": "object",
            "required": ["label", "passed", "blocker", "evidence_source"],
            "properties": {
                "label": { "type": "string" },
                "passed": { "type": "boolean" },
                "blocker": { "type": "string" },
                "evidence_source": { "type": "string", "enum": ["metadata", "approval", "policy_json"] }
            }
        },
        "RoutingPolicyPromotionGatesResponse": {
            "type": "object",
            "required": ["policy_id", "version", "review_mode", "status", "decision", "passed_count", "total_count", "gates", "blockers"],
            "properties": {
                "policy_id": { "type": "string" },
                "version": { "type": "integer" },
                "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                "status": { "type": "string" },
                "decision": { "type": "string", "enum": ["activation_allowed", "activation_blocked"] },
                "passed_count": { "type": "integer" },
                "total_count": { "type": "integer" },
                "gates": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/RoutingPolicyPromotionGate" }
                },
                "blockers": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            }
        },
    })
}
