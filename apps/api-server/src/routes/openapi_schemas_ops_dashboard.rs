use serde_json::{json, Value};

pub(super) fn dashboard_schemas() -> Value {
    json!({
        "DashboardModelScore": {
            "type": "object",
            "required": ["scored_runs", "average_score", "high_risk_count"],
            "properties": {
                "scored_runs": { "type": "integer" },
                "average_score": { "type": "number" },
                "high_risk_count": { "type": "integer" }
            }
        },
        "DashboardLayerScore": {
            "type": "object",
            "required": ["name", "scored_runs", "average_score", "high_risk_count"],
            "properties": {
                "name": { "type": "string" },
                "scored_runs": { "type": "integer" },
                "average_score": { "type": "number" },
                "high_risk_count": { "type": "integer" }
            }
        },
        "DashboardAuditCoverage": {
            "type": "object",
            "required": ["scoring_runs", "canonical_trace_runs", "canonical_trace_coverage"],
            "properties": {
                "scoring_runs": { "type": "integer" },
                "canonical_trace_runs": { "type": "integer" },
                "canonical_trace_coverage": { "type": "number" }
            }
        },
        "SavingAttributionSummary": {
            "type": "object",
            "required": ["source_type", "source_id", "financial_impact_type", "action", "saving_amount", "currency", "claim_count", "evidence_refs"],
            "properties": {
                "source_type": { "type": "string" },
                "source_id": { "type": "string" },
                "financial_impact_type": {
                    "type": "string",
                    "enum": ["prevented_payment", "recovered_amount", "avoided_future_exposure", "deterrence_estimate", "estimated_impact"]
                },
                "action": { "type": "string" },
                "saving_amount": { "type": "string", "format": "decimal" },
                "currency": { "type": "string" },
                "claim_count": { "type": "integer" },
                "evidence_refs": { "type": "array", "items": { "type": "string", "minLength": 1 } }
            }
        },
        "SavingSegmentSummary": {
            "type": "object",
            "required": ["segment_type", "segment_id", "saving_amount", "currency", "claim_count", "attribution_count", "roi"],
            "properties": {
                "segment_type": { "type": "string", "enum": ["provider", "scheme", "campaign"] },
                "segment_id": { "type": "string" },
                "saving_amount": { "type": "string", "format": "decimal" },
                "currency": { "type": "string" },
                "claim_count": { "type": "integer" },
                "attribution_count": { "type": "integer" },
                "roi": { "type": "number" }
            }
        },
        "DashboardValueMeasurement": {
            "type": "object",
            "required": ["prevented_payment", "recovered_amount", "avoided_future_exposure", "deterrence_estimate", "estimated_impact", "review_cost", "false_positive_operational_cost", "reviewer_capacity_hours", "net_value", "currency", "evidence_caveat"],
            "properties": {
                "prevented_payment": { "type": "string", "format": "decimal" },
                "recovered_amount": { "type": "string", "format": "decimal" },
                "avoided_future_exposure": { "type": "string", "format": "decimal" },
                "deterrence_estimate": { "type": "string", "format": "decimal" },
                "estimated_impact": { "type": "string", "format": "decimal" },
                "review_cost": { "type": "string", "format": "decimal" },
                "false_positive_operational_cost": { "type": "string", "format": "decimal" },
                "reviewer_capacity_hours": { "type": "string", "format": "decimal" },
                "net_value": { "type": "string", "format": "decimal" },
                "currency": { "type": "string" },
                "evidence_caveat": { "type": "string" }
            }
        },
        "DashboardLabelPool": {
            "type": "object",
            "required": ["total_labels", "approved_for_training", "needs_review", "rule_feedback", "model_feedback", "features_feedback", "provider_profile_feedback", "workflow_feedback", "case_status_labels", "medical_review_labels", "false_positive_labels", "evidence_backed_labels"],
            "properties": {
                "total_labels": { "type": "integer" },
                "approved_for_training": { "type": "integer" },
                "needs_review": { "type": "integer" },
                "rule_feedback": { "type": "integer" },
                "model_feedback": { "type": "integer" },
                "features_feedback": { "type": "integer" },
                "provider_profile_feedback": { "type": "integer" },
                "workflow_feedback": { "type": "integer" },
                "case_status_labels": { "type": "integer" },
                "medical_review_labels": { "type": "integer" },
                "false_positive_labels": { "type": "integer" },
                "evidence_backed_labels": { "type": "integer" }
            }
        },
        "DashboardQaQueue": {
            "type": "object",
            "required": ["sampled_cases", "open_cases", "reviewed_cases", "disagreement_cases", "disagreement_rate", "feedback_open_count", "feedback_in_progress_count", "feedback_resolved_count", "feedback_dismissed_count", "unresolved_feedback_count", "rules_unresolved_feedback_count", "models_unresolved_feedback_count", "features_unresolved_feedback_count", "provider_profile_unresolved_feedback_count", "workflow_unresolved_feedback_count", "tpa_unresolved_feedback_count"],
            "properties": {
                "sampled_cases": { "type": "integer" },
                "open_cases": { "type": "integer" },
                "reviewed_cases": { "type": "integer" },
                "disagreement_cases": { "type": "integer" },
                "disagreement_rate": { "type": "number" },
                "feedback_open_count": { "type": "integer" },
                "feedback_in_progress_count": { "type": "integer" },
                "feedback_resolved_count": { "type": "integer" },
                "feedback_dismissed_count": { "type": "integer" },
                "unresolved_feedback_count": { "type": "integer" },
                "rules_unresolved_feedback_count": { "type": "integer" },
                "models_unresolved_feedback_count": { "type": "integer" },
                "features_unresolved_feedback_count": { "type": "integer" },
                "provider_profile_unresolved_feedback_count": { "type": "integer" },
                "workflow_unresolved_feedback_count": { "type": "integer" },
                "tpa_unresolved_feedback_count": { "type": "integer" }
            }
        },
        "DashboardCaseSla": {
            "type": "object",
            "required": ["total_cases", "open_cases", "closed_cases", "breached_cases", "sla_breach_rate", "average_time_to_triage_hours", "average_time_to_closure_hours"],
            "properties": {
                "total_cases": { "type": "integer" },
                "open_cases": { "type": "integer" },
                "closed_cases": { "type": "integer" },
                "breached_cases": { "type": "integer" },
                "sla_breach_rate": { "type": "number" },
                "average_time_to_triage_hours": { "type": "number" },
                "average_time_to_closure_hours": { "type": "number" }
            }
        },
        "DashboardAgentGovernance": {
            "type": "object",
            "required": ["total_runs", "successful_runs", "evidence_backed_runs", "tool_call_count", "policy_check_count", "denied_policy_check_count", "failed_tool_call_count", "pending_approvals", "approved_approvals", "rejected_approvals"],
            "properties": {
                "total_runs": { "type": "integer" },
                "successful_runs": { "type": "integer" },
                "evidence_backed_runs": { "type": "integer" },
                "tool_call_count": { "type": "integer" },
                "policy_check_count": { "type": "integer" },
                "denied_policy_check_count": { "type": "integer" },
                "failed_tool_call_count": { "type": "integer" },
                "pending_approvals": { "type": "integer" },
                "approved_approvals": { "type": "integer" },
                "rejected_approvals": { "type": "integer" }
            }
        },
        "DashboardModelGovernance": {
            "type": "object",
            "required": ["total_models", "evaluated_models", "drift_watch_count", "drift_detected_count", "average_precision", "average_recall"],
            "properties": {
                "total_models": { "type": "integer" },
                "evaluated_models": { "type": "integer" },
                "drift_watch_count": { "type": "integer" },
                "drift_detected_count": { "type": "integer" },
                "average_precision": { "type": ["number", "null"] },
                "average_recall": { "type": ["number", "null"] }
            }
        },
        "DashboardRuleGovernance": {
            "type": "object",
            "required": ["total_rules", "active_rules", "triggered_rules", "total_trigger_count", "reviewed_count", "confirmed_fwa_count", "false_positive_count", "precision", "false_positive_rate", "saving_amount", "roi"],
            "properties": {
                "total_rules": { "type": "integer" },
                "active_rules": { "type": "integer" },
                "triggered_rules": { "type": "integer" },
                "total_trigger_count": { "type": "integer" },
                "reviewed_count": { "type": "integer" },
                "confirmed_fwa_count": { "type": "integer" },
                "false_positive_count": { "type": "integer" },
                "precision": { "type": "number" },
                "false_positive_rate": { "type": "number" },
                "saving_amount": { "type": "string", "format": "decimal" },
                "roi": { "type": "number" }
            }
        },
        "DashboardSummaryResponse": {
            "type": "object",
            "required": ["suspected_claims", "confirmed_fwa", "risk_amount", "saving_amount", "rag_distribution", "scheme_distribution", "rule_hits", "model_scores", "layer_scores", "saving_attributions", "saving_segments", "value_measurement", "audit_coverage", "label_pool", "qa_queue", "case_sla", "agent_governance", "model_governance", "rule_governance", "investigation_results", "qa_reviews"],
            "properties": {
                "suspected_claims": { "type": "integer" },
                "confirmed_fwa": { "type": "integer" },
                "risk_amount": { "type": "string", "format": "decimal" },
                "saving_amount": { "type": "string", "format": "decimal" },
                "rag_distribution": {
                    "type": "object",
                    "additionalProperties": { "type": "integer" }
                },
                "scheme_distribution": {
                    "type": "object",
                    "additionalProperties": { "type": "integer" }
                },
                "rule_hits": { "type": "integer" },
                "model_scores": {
                    "type": "object",
                    "additionalProperties": { "$ref": "#/components/schemas/DashboardModelScore" }
                },
                "layer_scores": {
                    "type": "object",
                    "additionalProperties": { "$ref": "#/components/schemas/DashboardLayerScore" }
                },
                "saving_attributions": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/SavingAttributionSummary" }
                },
                "saving_segments": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/SavingSegmentSummary" }
                },
                "value_measurement": { "$ref": "#/components/schemas/DashboardValueMeasurement" },
                "audit_coverage": { "$ref": "#/components/schemas/DashboardAuditCoverage" },
                "label_pool": { "$ref": "#/components/schemas/DashboardLabelPool" },
                "qa_queue": { "$ref": "#/components/schemas/DashboardQaQueue" },
                "case_sla": { "$ref": "#/components/schemas/DashboardCaseSla" },
                "agent_governance": { "$ref": "#/components/schemas/DashboardAgentGovernance" },
                "model_governance": { "$ref": "#/components/schemas/DashboardModelGovernance" },
                "rule_governance": { "$ref": "#/components/schemas/DashboardRuleGovernance" },
                "investigation_results": { "type": "integer" },
                "qa_reviews": { "type": "integer" }
            }
        },
    })
}
