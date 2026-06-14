use serde_json::{json, Value};

pub(super) fn promotion_schemas() -> Value {
    json!({
        "RulePromotionGate": {
            "type": "object",
            "required": ["label", "passed", "blocker", "evidence_source"],
            "properties": {
                "label": { "type": "string" },
                "passed": { "type": "boolean" },
                "blocker": { "type": "string" },
                "evidence_source": {
                    "type": "string",
                    "enum": ["runtime", "backtest", "approval", "labels", "qa_feedback", "metadata", "missing", "shadow"]
                }
            }
        },
        "RulePromotionGatesResponse": {
            "type": "object",
            "required": [
                "rule_id",
                "rule_version",
                "review_mode",
                "decision",
                "status",
                "passed_count",
                "total_count",
                "trigger_count",
                "reviewed_count",
                "false_positive_rate",
                "saving_amount",
                "open_rule_feedback_count",
                "unresolved_rule_feedback_count",
                "approved_label_count",
                "needs_review_label_count",
                "gates",
                "blockers"
            ],
            "properties": {
                "rule_id": { "type": "string" },
                "rule_version": { "type": "integer" },
                "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                "decision": { "type": "string", "enum": ["routing_allowed", "routing_blocked"] },
                "status": { "type": "string" },
                "passed_count": { "type": "integer" },
                "total_count": { "type": "integer" },
                "trigger_count": { "type": "integer", "minimum": 0 },
                "reviewed_count": { "type": "integer", "minimum": 0 },
                "false_positive_rate": { "type": "number", "minimum": 0 },
                "saving_amount": { "type": "string", "format": "decimal" },
                "open_rule_feedback_count": { "type": "integer" },
                "unresolved_rule_feedback_count": { "type": "integer" },
                "approved_label_count": { "type": "integer" },
                "needs_review_label_count": { "type": "integer" },
                "gates": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/RulePromotionGate" }
                },
                "blockers": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            }
        },
        "SubmitRulePromotionReviewRequest": {
            "type": "object",
            "required": ["decision", "reviewer", "notes", "evidence_refs"],
            "properties": {
                "decision": { "type": "string", "enum": ["approved", "rejected"] },
                "reviewer": { "type": "string", "minLength": 1 },
                "notes": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Promotion review notes must not contain PII."
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "description": "Structured evidence references must not contain PII.",
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "RulePromotionReview": {
            "type": "object",
            "required": ["rule_id", "rule_version", "decision", "reviewer", "notes", "evidence_refs"],
            "properties": {
                "rule_id": { "type": "string" },
                "rule_version": { "type": "integer" },
                "decision": { "type": "string", "enum": ["approved", "rejected"] },
                "reviewer": { "type": "string" },
                "notes": { "type": "string" },
                "evidence_refs": { "type": "array", "items": { "type": "string" } },
                "created_at": { "type": ["string", "null"], "format": "date-time" }
            }
        },
        "SubmitRuleShadowRunRequest": {
            "type": "object",
            "required": [
                "rule_version",
                "reviewed_count",
                "matched_count",
                "false_positive_count",
                "false_positive_rate",
                "report_uri",
                "decision",
                "reviewer",
                "notes",
                "evidence_refs"
            ],
            "properties": {
                "rule_version": { "type": "integer", "minimum": 1 },
                "reviewed_count": { "type": "integer", "minimum": 1 },
                "matched_count": { "type": "integer", "minimum": 0 },
                "false_positive_count": { "type": "integer", "minimum": 0 },
                "false_positive_rate": { "type": "number", "minimum": 0, "maximum": 1 },
                "report_uri": { "type": "string", "minLength": 1 },
                "decision": { "type": "string", "enum": ["shadow_passed", "shadow_blocked"] },
                "reviewer": { "type": "string", "minLength": 1 },
                "notes": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Shadow review notes must not contain PII."
                },
                "blockers": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "description": "Must include rules:{rule_id}:v{rule_version} and a rule_shadow_runs reference. Values must not contain PII.",
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "RuleShadowRun": {
            "type": "object",
            "required": [
                "rule_id",
                "rule_version",
                "report_uri",
                "decision",
                "reviewer",
                "notes",
                "reviewed_count",
                "matched_count",
                "false_positive_count",
                "false_positive_rate",
                "blockers",
                "evidence_refs"
            ],
            "properties": {
                "rule_id": { "type": "string" },
                "rule_version": { "type": "integer" },
                "report_uri": { "type": "string" },
                "decision": { "type": "string", "enum": ["shadow_passed", "shadow_blocked"] },
                "reviewer": { "type": "string" },
                "notes": { "type": "string" },
                "reviewed_count": { "type": "integer", "minimum": 0 },
                "matched_count": { "type": "integer", "minimum": 0 },
                "false_positive_count": { "type": "integer", "minimum": 0 },
                "false_positive_rate": { "type": "number", "minimum": 0, "maximum": 1 },
                "blockers": { "type": "array", "items": { "type": "string" } },
                "evidence_refs": { "type": "array", "items": { "type": "string" } },
                "created_at": { "type": ["string", "null"], "format": "date-time" }
            }
        },
    })
}
