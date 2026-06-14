use serde_json::{json, Value};

pub(super) fn provider_scoring_schemas() -> Value {
    json!({
        "ScoreBreakdown": {
            "type": "object",
            "required": [
                "peer_deviation_score",
                "rule_score",
                "anomaly_score",
                "ml_score",
                "medical_reasonableness_score",
                "provider_network_score",
                "similar_case_score",
                "final_score"
            ],
            "properties": {
                "peer_deviation_score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100
                },
                "rule_score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100
                },
                "anomaly_score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100
                },
                "ml_score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100
                },
                "medical_reasonableness_score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100
                },
                "provider_network_score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100
                },
                "similar_case_score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100
                },
                "final_score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100
                }
            }
        },
        "AlertResponse": {
            "type": "object",
            "required": ["alert_code", "severity", "reason", "rule_id", "rule_version", "required_evidence"],
            "properties": {
                "alert_code": {
                    "type": "string"
                },
                "severity": {
                    "type": "string"
                },
                "reason": {
                    "type": "string"
                },
                "rule_id": {
                    "type": "string"
                },
                "rule_version": {
                    "type": "integer"
                },
                "required_evidence": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/RequiredEvidence" }
                }
            }
        },
        "RequiredEvidence": {
            "type": "object",
            "required": ["evidence_type", "blocking"],
            "properties": {
                "evidence_type": { "type": "string", "minLength": 1 },
                "evidence_request_type": { "type": ["string", "null"] },
                "blocking": { "type": "boolean", "default": true },
                "policy_authority_ref": { "type": ["string", "null"] },
                "exception_check": { "type": ["string", "null"] }
            }
        },
        "AdjudicationPolicy": {
            "type": "object",
            "required": ["customer_approval_ref", "appeal_or_override_route", "effective_date", "rollback_plan_ref", "production_threshold_ref", "routing_impact_ref"],
            "properties": {
                "customer_approval_ref": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Customer-approved deterministic rule-list or policy approval reference."
                },
                "appeal_or_override_route": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Customer-approved appeal, exception, or reviewer override route."
                },
                "effective_date": { "type": "string", "minLength": 1 },
                "rollback_plan_ref": { "type": "string", "minLength": 1 },
                "production_threshold_ref": { "type": "string", "minLength": 1 },
                "routing_impact_ref": { "type": "string", "minLength": 1 }
            }
        },
    })
}
