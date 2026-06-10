use serde_json::{json, Value};

pub(super) fn scoring_response_schemas() -> Value {
    json!({
        "ScoreClaimResponse": {
            "type": "object",
            "required": [
                "run_id",
                "audit_id",
                "claim_id",
                "review_mode",
                "risk_score",
                "rag",
                "risk_level",
                "recommended_action",
                "decision_outcome",
                "decision_authority",
                "decision_confidence",
                "appeal_or_review_required",
                "reason_code",
                "confidence_score",
                "confidence",
                "routing_reason",
                "routing_policy",
                "scores",
                "model_score",
                "alerts",
                "top_reasons",
                "layers",
                "clinical_evidence",
                "provider_profile",
                "provider_relationships",
                "similar_cases",
                "feature_values",
                "evidence_refs",
                "agent_investigation_prefill"
            ],
            "properties": {
                "run_id": {
                    "type": "string"
                },
                "audit_id": {
                    "type": "string"
                },
                "claim_id": {
                    "type": "string"
                },
                "review_mode": {
                    "type": "string",
                    "enum": ["pre_payment", "post_payment"]
                },
                "risk_score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100
                },
                "rag": {
                    "type": "string",
                    "enum": ["Green", "Amber", "Red"]
                },
                "risk_level": {
                    "type": "string",
                    "enum": ["Low", "Medium", "High", "Critical"]
                },
                "recommended_action": {
                    "type": "string",
                    "enum": [
                        "StandardProcessing",
                        "QaSample",
                        "ManualReview",
                        "RequestEvidence",
                        "EscalateInvestigation",
                        "PostPaymentAudit",
                        "ProviderReview",
                        "RecoveryReview"
                    ]
                },
                "decision_outcome": {
                    "type": "string",
                    "enum": [
                        "straight_through",
                        "auto_deny",
                        "pending_evidence",
                        "manual_review",
                        "qa_sample",
                        "post_payment_audit"
                    ]
                },
                "decision_authority": {
                    "type": "string",
                    "enum": [
                        "customer_policy_rule",
                        "clinical_policy_rule",
                        "risk_routing_policy",
                        "human_reviewer",
                        "qa_policy"
                    ]
                },
                "decision_confidence": {
                    "type": "string",
                    "enum": ["deterministic", "high", "medium", "low"]
                },
                "appeal_or_review_required": {
                    "type": "boolean"
                },
                "reason_code": {
                    "type": "string",
                    "minLength": 1
                },
                "confidence_score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100
                },
                "confidence": {
                    "type": "string",
                    "enum": ["Low", "Medium", "High"]
                },
                "routing_reason": {
                    "type": "string"
                },
                "routing_policy": {
                    "$ref": "#/components/schemas/RoutingPolicy"
                },
                "scores": {
                    "$ref": "#/components/schemas/ScoreBreakdown"
                },
                "model_score": {
                    "$ref": "#/components/schemas/ModelScore"
                },
                "alerts": {
                    "type": "array",
                    "items": {
                        "$ref": "#/components/schemas/AlertResponse"
                    }
                },
                "top_reasons": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "minLength": 1
                    }
                },
                "layers": {
                    "type": "array",
                    "minItems": 7,
                    "maxItems": 7,
                    "items": {
                        "$ref": "#/components/schemas/DetectionLayerScore"
                    }
                },
                "clinical_evidence": {
                    "$ref": "#/components/schemas/ClinicalEvidenceAssessment"
                },
                "provider_profile": {
                    "$ref": "#/components/schemas/ProviderProfileAssessment"
                },
                "provider_relationships": {
                    "$ref": "#/components/schemas/ProviderRelationshipGraphAssessment"
                },
                "similar_cases": {
                    "type": "array",
                    "items": {
                        "$ref": "#/components/schemas/SimilarCase"
                    }
                },
                "feature_values": {
                    "type": "array",
                    "items": {
                        "$ref": "#/components/schemas/FeatureValue"
                    }
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": {
                        "oneOf": [
                            { "type": "object" },
                            { "type": "string" }
                        ]
                    }
                },
                "agent_investigation_prefill": {
                    "$ref": "#/components/schemas/AgentInvestigationPrefill"
                }
            }
        },
        "ModelScore": {
            "type": "object",
            "required": [
                "model_key",
                "model_version",
                "runtime_kind",
                "execution_provider",
                "score",
                "label",
                "explanations",
                "metadata",
                "latency_ms"
            ],
            "properties": {
                "model_key": { "type": "string" },
                "model_version": { "type": "string" },
                "runtime_kind": { "type": "string" },
                "execution_provider": { "type": "string" },
                "score": { "type": "integer", "minimum": 0, "maximum": 100 },
                "label": { "type": "string" },
                "explanations": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/ModelExplanation" }
                },
                "metadata": {
                    "type": "object",
                    "properties": {
                        "fraud_probability": { "type": "number", "minimum": 0, "maximum": 1 },
                        "abuse_probability": { "type": "number", "minimum": 0, "maximum": 1 },
                        "waste_probability": { "type": "number", "minimum": 0, "maximum": 1 }
                    },
                    "additionalProperties": true
                },
                "latency_ms": { "type": "integer", "minimum": 0 }
            }
        },
        "ModelExplanation": {
            "type": "object",
            "required": ["feature", "direction", "contribution", "reason"],
            "properties": {
                "feature": { "type": "string" },
                "direction": { "type": "string" },
                "contribution": { "type": "number" },
                "reason": { "type": "string" }
            }
        },
        "FeatureValue": {
            "type": "object",
            "required": ["name", "version", "value", "evidence_refs"],
            "properties": {
                "name": { "type": "string" },
                "version": { "type": "integer", "minimum": 0 },
                "value": {},
                "evidence_refs": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/EvidenceRef" }
                }
            }
        },
        "EvidenceRef": {
            "type": "object",
            "required": ["entity_type", "entity_id", "field"],
            "properties": {
                "entity_type": { "type": "string" },
                "entity_id": { "type": "string" },
                "field": { "type": "string" }
            }
        },
        "DetectionLayerScore": {
            "type": "object",
            "required": ["layer_id", "name", "score", "status", "reason", "evidence_refs"],
            "properties": {
                "layer_id": {
                    "type": "string",
                    "enum": [
                        "L1_PEER_BENCHMARK",
                        "L2_RULE_DETECTION",
                        "L3_UNSUPERVISED_ANOMALY",
                        "L4_SUPERVISED_ML",
                        "L5_MEDICAL_REASONABLENESS",
                        "L6_PROVIDER_GRAPH_RISK",
                        "L7_RISK_FUSION_ROUTING"
                    ]
                },
                "name": { "type": "string" },
                "score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100
                },
                "status": {
                    "type": "string",
                    "enum": ["active", "baseline", "no_data"]
                },
                "reason": { "type": "string" },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": {
                        "oneOf": [
                            { "type": "string" },
                            { "$ref": "#/components/schemas/EvidenceRef" }
                        ]
                    }
                }
            }
        },
        "RoutingPolicy": {
            "type": "object",
            "required": ["policy_id", "version", "review_mode", "risk_thresholds", "confidence_thresholds", "provider_review_threshold"],
            "properties": {
                "policy_id": { "type": "string", "minLength": 1 },
                "version": { "type": "integer", "minimum": 1 },
                "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                "risk_thresholds": { "$ref": "#/components/schemas/RiskThresholds" },
                "confidence_thresholds": { "$ref": "#/components/schemas/ConfidenceThresholds" },
                "provider_review_threshold": { "type": "integer", "minimum": 0, "maximum": 100 }
            }
        },
        "RiskThresholds": {
            "type": "object",
            "required": ["low_max", "medium_min", "high_min", "critical_min"],
            "properties": {
                "low_max": { "type": "integer", "minimum": 0, "maximum": 100 },
                "medium_min": { "type": "integer", "minimum": 0, "maximum": 100 },
                "high_min": { "type": "integer", "minimum": 0, "maximum": 100 },
                "critical_min": { "type": "integer", "minimum": 0, "maximum": 100 }
            }
        },
        "ConfidenceThresholds": {
            "type": "object",
            "required": ["low_confidence_below", "high_confidence_min"],
            "properties": {
                "low_confidence_below": { "type": "integer", "minimum": 0, "maximum": 100 },
                "high_confidence_min": { "type": "integer", "minimum": 0, "maximum": 100 }
            }
        },
    })
}
