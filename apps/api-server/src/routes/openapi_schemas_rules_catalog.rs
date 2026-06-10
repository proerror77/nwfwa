use serde_json::{json, Value};

pub(super) fn catalog_schemas() -> Value {
    json!({
        "RuleSummary": {
            "type": "object",
            "required": ["rule_id", "name", "status", "owner", "latest_version", "review_mode", "scheme_family", "score", "alert_code", "recommended_action", "applicability_scope", "backtest_result", "estimated_saving", "false_positive_history", "evidence_refs"],
            "properties": {
                "rule_id": { "type": "string" },
                "name": { "type": "string" },
                "status": { "type": "string", "enum": ["draft", "active", "submitted", "approved"] },
                "owner": { "type": "string" },
                "active_version": { "type": ["integer", "null"] },
                "latest_version": { "type": "integer" },
                "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                "score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "alert_code": { "type": "string" },
                "recommended_action": { "type": "string" },
                "applicability_scope": { "$ref": "#/components/schemas/RuleApplicabilityScope" },
                "backtest_result": { "$ref": "#/components/schemas/RuleBacktestSummary" },
                "estimated_saving": { "type": "string", "format": "decimal" },
                "false_positive_history": { "$ref": "#/components/schemas/RuleFalsePositiveHistory" },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "RuleApplicabilityScope": {
            "type": "object",
            "required": ["review_mode", "scheme_family", "source"],
            "properties": {
                "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                "source": { "type": "string" }
            }
        },
        "RuleBacktestSummary": {
            "type": "object",
            "required": ["status", "sample_count", "matched_count", "precision", "recall", "lift", "false_positive_rate", "estimated_saving", "evidence_refs", "created_at"],
            "properties": {
                "status": { "type": "string", "enum": ["not_run", "completed"] },
                "sample_count": { "type": "integer", "minimum": 0 },
                "matched_count": { "type": "integer", "minimum": 0 },
                "precision": { "type": "number", "minimum": 0 },
                "recall": { "type": "number", "minimum": 0 },
                "lift": { "type": "number", "minimum": 0 },
                "false_positive_rate": { "type": "number", "minimum": 0 },
                "estimated_saving": { "type": "string", "format": "decimal" },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string", "minLength": 1 }
                },
                "created_at": { "type": ["string", "null"], "format": "date-time" }
            }
        },
        "RuleFalsePositiveHistory": {
            "type": "object",
            "required": ["status", "false_positive_count", "false_positive_rate", "evidence_refs"],
            "properties": {
                "status": { "type": "string", "enum": ["not_observed", "observed"] },
                "false_positive_count": { "type": "integer", "minimum": 0 },
                "false_positive_rate": { "type": "number", "minimum": 0 },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "FwaSchemeFamily": {
            "type": "string",
            "enum": [
                "duplicate_billing",
                "upcoding",
                "unbundling",
                "medically_unnecessary_service",
                "excessive_utilization",
                "diagnosis_procedure_mismatch",
                "laboratory_testing_abuse",
                "telehealth_abuse",
                "genetic_testing_abuse",
                "pharmacy_controlled_substance_abuse",
                "dme_home_health_hospice_rehab_risk",
                "provider_peer_outlier",
                "relationship_concentration",
                "early_high_value_claim",
                "high_risk_claim"
            ]
        },
        "FwaSchemeDefinition": {
            "type": "object",
            "required": ["scheme_family", "display_name", "risk_domain", "description", "minimum_evidence", "default_review_route", "primary_layers"],
            "properties": {
                "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                "display_name": { "type": "string" },
                "risk_domain": { "type": "string" },
                "description": { "type": "string" },
                "minimum_evidence": { "type": "array", "items": { "type": "string" } },
                "default_review_route": { "type": "string" },
                "primary_layers": { "type": "array", "items": { "type": "string" } }
            }
        },
        "FwaSchemeListResponse": {
            "type": "object",
            "required": ["schemes", "scheme_count"],
            "properties": {
                "schemes": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/FwaSchemeDefinition" }
                },
                "scheme_count": { "type": "integer", "minimum": 0 }
            }
        },
        "RuleVersion": {
            "type": "object",
            "required": ["version", "status", "dsl", "review_mode", "scheme_family", "score", "alert_code", "recommended_action", "reason"],
            "properties": {
                "version": { "type": "integer" },
                "status": { "type": "string" },
                "dsl": { "type": "object" },
                "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                "score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "alert_code": { "type": "string" },
                "recommended_action": { "type": "string" },
                "reason": { "type": "string" }
            }
        },
        "RuleListResponse": {
            "type": "object",
            "required": ["rules"],
            "properties": {
                "rules": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/RuleSummary" }
                }
            }
        },
        "RuleConditionLibraryRecord": {
            "type": "object",
            "required": ["condition_key", "source_rule_key", "source_rule_version", "condition_index", "field", "operator", "value", "review_mode", "scheme_family", "status", "owner", "evidence_refs"],
            "properties": {
                "condition_key": { "type": "string" },
                "source_rule_key": { "type": "string" },
                "source_rule_version": { "type": "integer", "minimum": 0 },
                "condition_index": { "type": "integer", "minimum": 0 },
                "field": { "type": "string" },
                "operator": { "type": "string", "enum": ["<=", "<", ">=", ">", "==", "in"] },
                "value": {},
                "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                "status": { "type": "string", "enum": ["candidate", "governance_review", "active", "retired"] },
                "owner": { "type": "string" },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "created_at": { "type": ["string", "null"], "format": "date-time" },
                "updated_at": { "type": ["string", "null"], "format": "date-time" }
            }
        },
        "RuleConditionLibraryResponse": {
            "type": "object",
            "required": ["conditions"],
            "properties": {
                "conditions": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/RuleConditionLibraryRecord" }
                }
            }
        },
    })
}
