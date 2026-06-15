use serde_json::{json, Value};

pub(super) fn candidate_review_schemas() -> Value {
    json!({
        "ReviewRuleCandidateRequest": {
            "type": "object",
            "required": ["rule", "decision", "reviewer", "notes", "evidence_refs"],
            "properties": {
                "rule": { "$ref": "#/components/schemas/RuleDefinition" },
                "decision": { "type": "string", "enum": ["accepted", "rejected"] },
                "reviewer": { "type": "string", "minLength": 1 },
                "notes": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Candidate review notes must not contain PII."
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "description": "Structured evidence references must not contain PII and must be production refs, not local/template refs. Accepted candidates must include backtest evidence.",
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "ReviewRuleCandidateResponse": {
            "type": "object",
            "required": ["rule_id", "decision", "entered_rule_library", "accepted_for_governance_review", "active_rule_writeback", "evidence_refs"],
            "properties": {
                "rule_id": { "type": "string" },
                "decision": { "type": "string", "enum": ["accepted", "rejected"] },
                "entered_rule_library": {
                    "type": "boolean",
                    "description": "Always false for candidate review; activation requires later rule lifecycle gates."
                },
                "accepted_for_governance_review": { "type": "boolean" },
                "saved_draft_rule_id": { "type": ["string", "null"] },
                "active_rule_writeback": {
                    "type": "boolean",
                    "description": "Always false for candidate review."
                },
                "evidence_refs": { "type": "array", "items": { "type": "string" } }
            }
        },
        "RuleDetailResponse": {
            "type": "object",
            "required": ["summary", "versions", "audit_events"],
            "properties": {
                "summary": { "$ref": "#/components/schemas/RuleSummary" },
                "versions": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/RuleVersion" }
                },
                "audit_events": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/AuditHistoryEvent" }
                }
            }
        },
        "RuleBacktestRequest": {
            "type": "object",
            "required": ["rule"],
            "properties": {
                "rule": { "$ref": "#/components/schemas/RuleDefinition" },
                "expected_review_capacity": { "type": "integer", "minimum": 0 },
                "dataset_uri": {
                    "type": "string",
                    "description": "Optional local Parquet dataset used for backtesting dataset-mined rules."
                },
                "label_column": {
                    "type": "string",
                    "description": "Boolean or 0/1 label column, defaults to confirmed_fwa."
                },
                "claim_id_column": {
                    "type": "string",
                    "description": "Claim identifier column, defaults to claim_id."
                },
                "samples": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "confirmed_fwa": { "type": "boolean" }
                        }
                    }
                }
            }
        },
        "RuleBacktestResponse": {
            "type": "object",
            "required": ["sample_count", "matched_count", "reviewed_count", "confirmed_fwa_count", "false_positive_count", "match_rate", "precision", "recall", "lift", "false_positive_rate", "average_score_contribution", "estimated_saving", "promotion_recommendation", "blockers", "matched_claim_ids", "evidence_refs"],
            "properties": {
                "sample_count": { "type": "integer" },
                "matched_count": { "type": "integer" },
                "reviewed_count": { "type": "integer" },
                "confirmed_fwa_count": { "type": "integer" },
                "false_positive_count": { "type": "integer" },
                "match_rate": { "type": "number" },
                "precision": { "type": "number" },
                "recall": { "type": "number" },
                "lift": { "type": "number" },
                "false_positive_rate": { "type": "number" },
                "average_score_contribution": { "type": "number" },
                "estimated_saving": { "type": "string", "format": "decimal" },
                "promotion_recommendation": { "type": "string" },
                "blockers": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "matched_claim_ids": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            }
        },
    })
}
