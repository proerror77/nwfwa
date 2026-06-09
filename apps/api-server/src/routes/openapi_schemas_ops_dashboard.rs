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
