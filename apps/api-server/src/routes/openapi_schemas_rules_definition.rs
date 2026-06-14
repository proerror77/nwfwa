use serde_json::{json, Value};

pub(super) fn definition_schemas() -> Value {
    json!({
        "SaveRuleCandidateRequest": {
            "type": "object",
            "required": ["rule"],
            "properties": {
                "owner": { "type": "string" },
                "rule": { "$ref": "#/components/schemas/RuleDefinition" }
            }
        },
        "RuleDefinition": {
            "type": "object",
            "required": ["rule_id", "version", "name", "review_mode", "scheme_family", "conditions", "action"],
            "properties": {
                "rule_id": { "type": "string" },
                "version": { "type": "integer", "minimum": 1 },
                "name": { "type": "string" },
                "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                "conditions": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/RuleCondition" }
                },
                "action": { "$ref": "#/components/schemas/RuleAction" }
            }
        },
        "RuleCondition": {
            "type": "object",
            "required": ["field", "operator", "value"],
            "properties": {
                "field": { "type": "string" },
                "operator": { "type": "string", "enum": ["<=", "<", ">=", ">", "==", "in"] },
                "value": {}
            }
        },
        "RuleAction": {
            "type": "object",
            "required": ["score", "alert_code", "recommended_action", "reason"],
            "properties": {
                "score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "alert_code": { "type": "string" },
                "recommended_action": {
                    "type": "string",
                    "enum": ["StandardProcessing", "QaSample", "ManualReview", "RequestEvidence", "EscalateInvestigation", "PostPaymentAudit", "ProviderReview", "RecoveryReview"]
                },
                "action_class": {
                    "type": "string",
                    "enum": ["hard_deny", "straight_through", "pending_evidence", "manual_review", "score_only"],
                    "default": "manual_review"
                },
                "required_evidence": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/RequiredEvidence" },
                    "default": []
                },
                "adjudication_policy": {
                    "anyOf": [
                        { "$ref": "#/components/schemas/AdjudicationPolicy" },
                        { "type": "null" }
                    ],
                    "description": "Required only for customer-approved hard-deny or straight-through adjudication rules."
                },
                "reason": { "type": "string" }
            }
        },
        "RuleDiscoveryRequest": {
            "type": "object",
            "properties": {
                "min_support": { "type": "integer", "minimum": 1 },
                "max_candidates": { "type": "integer", "minimum": 1 },
                "max_tree_depth": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 3,
                    "description": "Maximum depth for shallow decision-tree rule mining, defaults to 2."
                },
                "dataset_uri": {
                    "type": "string",
                    "description": "Local Parquet file used for dataset-backed rule mining."
                },
                "label_column": {
                    "type": "string",
                    "description": "Boolean or 0/1 label column, defaults to confirmed_fwa."
                },
                "claim_id_column": {
                    "type": "string",
                    "description": "Claim identifier column, defaults to claim_id."
                },
                "candidate_feature_fields": {
                    "type": "array",
                    "description": "Optional numeric feature allowlist for mining.",
                    "items": { "type": "string" }
                },
                "source_model_key": { "type": "string" },
                "source_model_version": { "type": "string" },
                "feature_importance_uri": {
                    "type": "string",
                    "description": "Feature importance or SHAP-style artifact URI used as candidate-rule evidence."
                },
                "min_abs_contribution": {
                    "type": "number",
                    "description": "Minimum absolute model explanation contribution required before proposing a rule candidate."
                },
                "model_explanations": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/RuleDiscoveryModelExplanation" }
                },
                "samples": {
                    "type": "array",
                    "items": { "type": "object" }
                }
            }
        },
        "RuleDiscoveryModelExplanation": {
            "type": "object",
            "required": ["feature", "direction", "contribution", "reason"],
            "properties": {
                "feature": { "type": "string" },
                "direction": { "type": "string", "enum": ["increases_risk", "decreases_risk"] },
                "contribution": { "type": "number" },
                "reason": { "type": "string" }
            }
        },
        "RuleDiscoveryCandidate": {
            "type": "object",
            "required": ["rule", "support", "precision", "recall", "lift", "estimated_saving", "false_positive_rate", "matched_claim_ids", "explanation", "condition_refs", "evidence_refs"],
            "properties": {
                "rule": { "$ref": "#/components/schemas/RuleDefinition" },
                "support": { "type": "integer" },
                "precision": { "type": "number" },
                "recall": { "type": "number" },
                "lift": { "type": "number" },
                "estimated_saving": { "type": "string", "format": "decimal" },
                "false_positive_rate": { "type": "number" },
                "matched_claim_ids": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "explanation": { "type": "string" },
                "condition_refs": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            }
        },
        "RuleDiscoveryResponse": {
            "type": "object",
            "required": ["sample_count", "positive_count", "candidates"],
            "properties": {
                "sample_count": { "type": "integer" },
                "positive_count": { "type": "integer" },
                "candidates": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/RuleDiscoveryCandidate" }
                }
            }
        },
    })
}
