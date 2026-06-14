use serde_json::{json, Value};

pub(super) fn provider_profile_schemas() -> Value {
    json!({
        "ProviderProfileAssessment": {
            "type": "object",
            "required": [
                "provider_id",
                "risk_score",
                "risk_tier",
                "review_required",
                "review_route",
                "review_failure_count",
                "confirmed_fwa_count",
                "false_positive_count",
                "oig_excluded",
                "sam_debarred",
                "outlier_flags",
                "window_findings",
                "evidence_refs"
            ],
            "properties": {
                "provider_id": { "type": "string" },
                "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "risk_tier": { "type": "string", "enum": ["low", "medium", "high"] },
                "review_required": { "type": "boolean" },
                "review_route": { "type": "string", "enum": ["none", "provider_review", "provider_sanctions_review"] },
                "specialty": { "type": ["string", "null"] },
                "network_status": { "type": ["string", "null"] },
                "review_failure_count": { "type": "integer", "minimum": 0 },
                "confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                "false_positive_count": { "type": "integer", "minimum": 0 },
                "oig_excluded": { "type": "boolean" },
                "sam_debarred": { "type": "boolean" },
                "outlier_flags": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "window_findings": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/ProviderProfileWindowFinding" }
                },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            }
        },
        "ProviderProfileWindowFinding": {
            "type": "object",
            "required": [
                "window_days",
                "risk_score",
                "outlier_flags",
                "reason",
                "evidence_ref"
            ],
            "properties": {
                "window_days": { "type": "integer" },
                "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "outlier_flags": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "reason": { "type": "string" },
                "evidence_ref": { "type": "string" }
            }
        },
        "ProviderRelationshipGraphAssessment": {
            "type": "object",
            "required": [
                "provider_id",
                "risk_score",
                "risk_tier",
                "review_required",
                "review_route",
                "graph_reasons",
                "findings",
                "evidence_refs"
            ],
            "properties": {
                "provider_id": { "type": "string" },
                "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "risk_tier": { "type": "string", "enum": ["no_data", "low", "medium", "high"] },
                "review_required": { "type": "boolean" },
                "review_route": { "type": "string", "enum": ["none", "provider_graph_review"] },
                "graph_reasons": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "findings": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/ProviderRelationshipGraphFinding" }
                },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            }
        },
        "ProviderRelationshipGraphFinding": {
            "type": "object",
            "required": ["signal", "risk_score", "reason", "evidence_ref"],
            "properties": {
                "signal": { "type": "string" },
                "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "reason": { "type": "string" },
                "evidence_ref": { "type": "string" }
            }
        },
        "ProviderRiskSummaryItem": {
            "type": "object",
            "required": ["provider_id", "risk_score", "risk_tier", "review_required", "review_route", "claim_count", "specialty", "network_status", "review_failure_count", "confirmed_fwa_count", "false_positive_count", "network_risk_score", "outlier_flags", "graph_reasons", "evidence_refs"],
            "properties": {
                "provider_id": { "type": "string" },
                "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "risk_tier": { "type": "string" },
                "review_required": { "type": "boolean" },
                "review_route": { "type": "string" },
                "claim_count": { "type": "integer" },
                "specialty": { "type": ["string", "null"] },
                "network_status": { "type": ["string", "null"] },
                "review_failure_count": { "type": "integer", "minimum": 0 },
                "confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                "false_positive_count": { "type": "integer", "minimum": 0 },
                "network_risk_score": { "type": ["integer", "null"], "minimum": 0, "maximum": 100 },
                "latest_claim_id": { "type": ["string", "null"] },
                "outlier_flags": { "type": "array", "items": { "type": "string" } },
                "graph_reasons": { "type": "array", "items": { "type": "string" } },
                "evidence_refs": { "type": "array", "items": { "type": "string" } }
            }
        },
        "ProviderRiskSummaryResponse": {
            "type": "object",
            "required": ["provider_count", "review_required_count", "high_risk_count", "providers"],
            "properties": {
                "provider_count": { "type": "integer" },
                "review_required_count": { "type": "integer" },
                "high_risk_count": { "type": "integer" },
                "providers": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/ProviderRiskSummaryItem" }
                }
            }
        },
    })
}
