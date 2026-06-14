use serde_json::{json, Value};

pub(super) fn sampling_schemas() -> Value {
    json!({
        "CreateAuditSampleRequest": {
            "type": "object",
            "required": ["sample_mode", "population_definition", "inclusion_criteria", "sample_size", "reviewer", "assignment_queue"],
            "properties": {
                "sample_mode": {
                    "type": "string",
                    "enum": ["risk_ranked", "random_control", "stratified", "post_payment_audit", "qa_calibration"]
                },
                "population_definition": { "type": "string", "minLength": 1 },
                "inclusion_criteria": {
                    "type": "object",
                    "properties": {
                        "min_risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "scheme_family": { "type": "string" },
                        "rag": { "type": "string", "enum": ["GREEN", "AMBER", "RED"] },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "provider_type": { "type": "string" },
                        "provider_region": { "type": "string" },
                        "policy_type": { "type": "string" },
                        "risk_band": { "type": "string", "enum": ["low", "medium", "high", "critical"] }
                    }
                },
                "deterministic_seed": { "type": ["string", "null"] },
                "sample_size": { "type": "integer", "minimum": 1 },
                "reviewer": { "type": "string", "minLength": 1 },
                "assignment_queue": { "type": "string", "minLength": 1 }
            }
        },
        "AuditSampleLeadRecord": {
            "type": "object",
            "required": ["lead_id", "claim_id", "scheme_family", "review_mode", "provider_id", "provider_type", "provider_region", "policy_type", "risk_band", "strata_key", "prior_reviewer_sample_count", "risk_score", "rag", "evidence_refs"],
            "properties": {
                "lead_id": { "type": "string" },
                "claim_id": { "type": "string" },
                "scheme_family": { "type": "string" },
                "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                "provider_id": { "type": "string" },
                "provider_type": { "type": "string" },
                "provider_region": { "type": "string" },
                "policy_type": { "type": "string" },
                "risk_band": { "type": "string", "enum": ["low", "medium", "high", "critical"] },
                "strata_key": { "type": "string" },
                "prior_reviewer_sample_count": { "type": "integer", "minimum": 0 },
                "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "rag": { "type": "string", "enum": ["GREEN", "AMBER", "RED"] },
                "evidence_refs": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } }
            }
        },
        "AuditSampleRecord": {
            "type": "object",
            "required": ["sample_id", "customer_scope_id", "sample_mode", "population_definition", "inclusion_criteria", "selection_method", "sample_size", "reviewer", "assignment_queue", "selected_leads", "outcome_distribution"],
            "properties": {
                "sample_id": { "type": "string" },
                "customer_scope_id": {
                    "type": "string",
                    "description": "Derived from the authenticated API key and used to scope sample population and list visibility."
                },
                "sample_mode": { "type": "string" },
                "population_definition": { "type": "string" },
                "inclusion_criteria": { "type": "object" },
                "deterministic_seed": { "type": ["string", "null"] },
                "selection_method": { "type": "string" },
                "sample_size": { "type": "integer" },
                "reviewer": { "type": "string" },
                "assignment_queue": { "type": "string" },
                "selected_leads": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/AuditSampleLeadRecord" }
                },
                "outcome_distribution": {
                    "type": "object",
                    "properties": {
                        "selected_count": { "type": "integer", "minimum": 0 },
                        "reviewed_count": { "type": "integer", "minimum": 0 },
                        "open_count": { "type": "integer", "minimum": 0 },
                        "qa_conclusions": { "type": "object" },
                        "issue_types": { "type": "object" },
                        "feedback_targets": { "type": "object" },
                        "strata_distribution": { "type": "object" },
                        "review_mode_distribution": { "type": "object" },
                        "reviewer_history_distribution": { "type": "object" },
                        "baseline_measurement": {
                            "type": "object",
                            "properties": {
                                "control_cohort": { "type": "boolean" },
                                "measurement_goal": { "type": "string", "enum": ["false_positive_and_missed_risk_baseline"] },
                                "missed_risk_review_targets": { "type": "integer", "minimum": 0 },
                                "false_positive_review_targets": { "type": "integer", "minimum": 0 }
                            }
                        }
                    }
                },
                "created_at": { "type": ["string", "null"] }
            }
        },
        "AuditSampleListResponse": {
            "type": "object",
            "required": ["samples"],
            "properties": {
                "samples": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/AuditSampleRecord" }
                }
            }
        },
    })
}
