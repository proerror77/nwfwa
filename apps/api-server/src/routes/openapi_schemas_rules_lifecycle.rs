use serde_json::{json, Value};

pub(super) fn lifecycle_schemas() -> Value {
    json!({
        "RuleLifecycleResponse": {
            "type": "object",
            "required": ["rule_id", "status", "active_version", "latest_version"],
            "properties": {
                "rule_id": { "type": "string" },
                "status": { "type": "string", "enum": ["draft", "active", "submitted", "approved"] },
                "active_version": { "type": ["integer", "null"] },
                "latest_version": { "type": "integer" }
            }
        },
        "RuleLifecycleRequest": {
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
        "RulePerformanceRecord": {
            "type": "object",
            "required": [
                "rule_id",
                "alert_code",
                "trigger_count",
                "reviewed_count",
                "confirmed_fwa_count",
                "false_positive_count",
                "mark_rate",
                "precision",
                "false_positive_rate",
                "saving_amount",
                "roi"
            ],
            "properties": {
                "rule_id": { "type": "string" },
                "alert_code": { "type": "string" },
                "trigger_count": { "type": "integer", "minimum": 0 },
                "reviewed_count": { "type": "integer", "minimum": 0 },
                "confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                "false_positive_count": { "type": "integer", "minimum": 0 },
                "mark_rate": { "type": "number", "minimum": 0 },
                "precision": { "type": "number", "minimum": 0 },
                "false_positive_rate": { "type": "number", "minimum": 0 },
                "saving_amount": { "type": "string", "format": "decimal" },
                "roi": { "type": "number" }
            }
        },
        "RulePerformanceResponse": {
            "type": "object",
            "required": ["rules"],
            "properties": {
                "rules": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/RulePerformanceRecord" }
                }
            }
        },
    })
}
