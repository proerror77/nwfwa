use serde_json::{json, Value};

pub(super) fn scoring_schemas() -> Value {
    json!({
                "ScoreClaimRequest": {
                    "oneOf": [
                        {
                            "$ref": "#/components/schemas/ClaimIdScoreClaimRequest"
                        },
                        {
                            "$ref": "#/components/schemas/FullPayloadScoreClaimRequest"
                        },
                        {
                            "$ref": "#/components/schemas/CanonicalContextScoreClaimRequest"
                        },
                        {
                            "$ref": "#/components/schemas/InboxHandoffScoreClaimRequest"
                        }
                    ]
                },
                "ClaimIdScoreClaimRequest": {
                    "type": "object",
                    "required": ["source_system", "claim_id"],
                    "properties": {
                        "source_system": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Must match the source system bound to the authenticated API key.",
                            "examples": ["tpa-demo"]
                        },
                        "claim_id": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Existing claim id to load from FWA storage."
                        },
                        "review_mode": {
                            "type": "string",
                            "enum": ["pre_payment", "post_payment"],
                            "default": "pre_payment",
                            "description": "Runtime scoring context for pre-payment or post-payment review."
                        }
                    },
                    "not": {
                        "anyOf": [
                            { "required": ["claim"] },
                            { "required": ["items"] },
                            { "required": ["member"] },
                            { "required": ["policy"] },
                            { "required": ["provider"] },
                            { "required": ["documents"] },
                            { "required": ["provider_profile"] },
                            { "required": ["provider_relationships"] },
                            { "required": ["canonical_claim_context"] },
                            { "required": ["inbox_run_id"] },
                            { "required": ["inbox_idempotency_key"] }
                        ]
                    }
                },
                "CanonicalContextScoreClaimRequest": {
                    "type": "object",
                    "required": ["source_system", "canonical_claim_context"],
                    "properties": {
                        "source_system": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Must match the source system bound to the authenticated API key.",
                            "examples": ["tpa-demo"]
                        },
                        "review_mode": {
                            "type": "string",
                            "enum": ["pre_payment", "post_payment"],
                            "default": "pre_payment",
                            "description": "Runtime scoring context for pre-payment or post-payment review."
                        },
                        "canonical_claim_context": {
                            "$ref": "#/components/schemas/InboxCanonicalClaimContext"
                        }
                    },
                    "not": {
                        "anyOf": [
                            { "required": ["claim_id"] },
                            { "required": ["claim"] },
                            { "required": ["items"] },
                            { "required": ["member"] },
                            { "required": ["policy"] },
                            { "required": ["provider"] },
                            { "required": ["documents"] },
                            { "required": ["provider_profile"] },
                            { "required": ["provider_relationships"] },
                            { "required": ["inbox_run_id"] },
                            { "required": ["inbox_idempotency_key"] }
                        ]
                    }
                },
                "InboxHandoffScoreClaimRequest": {
                    "type": "object",
                    "required": ["source_system"],
                    "properties": {
                        "source_system": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Must match the source system bound to the authenticated API key.",
                            "examples": ["tpa-demo"]
                        },
                        "review_mode": {
                            "type": "string",
                            "enum": ["pre_payment", "post_payment"],
                            "default": "pre_payment",
                            "description": "Runtime scoring context for pre-payment or post-payment review."
                        },
                        "inbox_run_id": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Scoring-ready inbox normalization run id returned by /api/v1/inbox/claims/normalize."
                        },
                        "inbox_idempotency_key": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Stable scoring-ready inbox normalization idempotency key returned by /api/v1/inbox/claims/normalize."
                        }
                    },
                    "oneOf": [
                        { "required": ["inbox_run_id"] },
                        { "required": ["inbox_idempotency_key"] }
                    ],
                    "not": {
                        "anyOf": [
                            { "required": ["claim_id"] },
                            { "required": ["claim"] },
                            { "required": ["items"] },
                            { "required": ["member"] },
                            { "required": ["policy"] },
                            { "required": ["provider"] },
                            { "required": ["documents"] },
                            { "required": ["provider_profile"] },
                            { "required": ["provider_relationships"] },
                            { "required": ["canonical_claim_context"] }
                        ]
                    }
                },
                "FullPayloadScoreClaimRequest": {
                    "type": "object",
                    "required": ["source_system", "claim"],
                    "properties": {
                        "source_system": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Must match the source system bound to the authenticated API key.",
                            "examples": ["tpa-demo"]
                        },
                        "claim": {
                            "$ref": "#/components/schemas/FullClaimPayload"
                        },
                        "review_mode": {
                            "type": "string",
                            "enum": ["pre_payment", "post_payment"],
                            "default": "pre_payment",
                            "description": "Runtime scoring context for pre-payment or post-payment review."
                        },
                        "items": {
                            "type": "array",
                            "description": "Top-level claim items for spec-style full payload requests. Do not send the same entity both nested under claim and at the top level.",
                            "items": {
                                "$ref": "#/components/schemas/ClaimItemPayload"
                            }
                        },
                        "member": {
                            "$ref": "#/components/schemas/MemberPayload"
                        },
                        "policy": {
                            "$ref": "#/components/schemas/PolicyPayload"
                        },
                        "provider": {
                            "$ref": "#/components/schemas/ProviderPayload"
                        },
                        "documents": {
                            "type": "array",
                            "description": "Clinical documents linked to claim items for evidence sufficiency review.",
                            "items": {
                                "$ref": "#/components/schemas/DocumentPayload"
                            }
                        },
                        "provider_profile": {
                            "$ref": "#/components/schemas/ProviderProfilePayload"
                        },
                        "provider_relationships": {
                            "$ref": "#/components/schemas/ProviderRelationshipGraphPayload"
                        }
                    },
                    "not": {
                        "anyOf": [
                            { "required": ["claim_id"] },
                            { "required": ["canonical_claim_context"] },
                            { "required": ["inbox_run_id"] },
                            { "required": ["inbox_idempotency_key"] }
                        ]
                    }
                },
                "FullClaimPayload": {
                    "type": "object",
                    "required": ["external_claim_id", "claim_amount", "currency"],
                    "properties": {
                        "external_claim_id": {
                            "type": "string",
                            "minLength": 1
                        },
                        "claim_amount": {
                            "type": "string",
                            "format": "decimal",
                            "description": "Positive decimal string."
                        },
                        "currency": {
                            "type": "string",
                            "minLength": 1
                        },
                        "service_date": {
                            "type": "string",
                            "format": "date"
                        },
                        "diagnosis_code": {
                            "type": "string",
                            "minLength": 1
                        },
                        "items": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/ClaimItemPayload"
                            }
                        },
                        "member": {
                            "$ref": "#/components/schemas/MemberPayload"
                        },
                        "policy": {
                            "$ref": "#/components/schemas/PolicyPayload"
                        },
                        "provider": {
                            "$ref": "#/components/schemas/ProviderPayload"
                        },
                        "documents": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/DocumentPayload"
                            }
                        },
                        "provider_profile": {
                            "$ref": "#/components/schemas/ProviderProfilePayload"
                        },
                        "provider_relationships": {
                            "$ref": "#/components/schemas/ProviderRelationshipGraphPayload"
                        }
                    }
                },
                "ClaimItemPayload": {
                    "type": "object",
                    "required": ["item_code", "item_type", "description", "quantity", "unit_amount", "total_amount"],
                    "properties": {
                        "item_code": {
                            "type": "string",
                            "minLength": 1
                        },
                        "item_type": {
                            "type": "string",
                            "minLength": 1
                        },
                        "description": {
                            "type": "string",
                            "minLength": 1
                        },
                        "quantity": {
                            "type": "integer",
                            "minimum": 1
                        },
                        "unit_amount": {
                            "type": "string",
                            "format": "decimal",
                            "description": "Non-negative decimal string."
                        },
                        "total_amount": {
                            "type": "string",
                            "format": "decimal",
                            "description": "Non-negative decimal string."
                        },
                        "currency": {
                            "type": "string",
                            "minLength": 1
                        }
                    }
                },
                "MemberPayload": {
                    "type": "object",
                    "required": ["external_member_id"],
                    "properties": {
                        "external_member_id": {
                            "type": "string",
                            "minLength": 1
                        },
                        "dob": {
                            "type": "string",
                            "format": "date"
                        },
                        "gender": {
                            "type": "string",
                            "minLength": 1
                        }
                    }
                },
                "PolicyPayload": {
                    "type": "object",
                    "required": ["external_policy_id", "coverage_start_date", "coverage_end_date", "coverage_limit"],
                    "properties": {
                        "external_policy_id": {
                            "type": "string",
                            "minLength": 1
                        },
                        "product_code": {
                            "type": "string",
                            "minLength": 1
                        },
                        "coverage_start_date": {
                            "type": "string",
                            "format": "date"
                        },
                        "coverage_end_date": {
                            "type": "string",
                            "format": "date"
                        },
                        "coverage_limit": {
                            "type": "string",
                            "format": "decimal",
                            "description": "Positive decimal string."
                        },
                        "currency": {
                            "type": "string",
                            "minLength": 1
                        }
                    }
                },
                "ProviderPayload": {
                    "type": "object",
                    "required": ["external_provider_id", "name", "provider_type", "region"],
                    "properties": {
                        "external_provider_id": {
                            "type": "string",
                            "minLength": 1
                        },
                        "name": {
                            "type": "string",
                            "minLength": 1
                        },
                        "provider_type": {
                            "type": "string",
                            "minLength": 1
                        },
                        "region": {
                            "type": "string",
                            "minLength": 1
                        },
                        "risk_tier": {
                            "type": "string",
                            "enum": ["Low", "Medium", "High"]
                        }
                    }
                },
                "DocumentPayload": {
                    "type": "object",
                    "required": ["external_document_id", "document_type"],
                    "properties": {
                        "external_document_id": {
                            "type": "string",
                            "minLength": 1
                        },
                        "document_type": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Examples: medical_record, clinical_order, radiology_report, dental_xray, prescription_detail, operation_record, lab_result"
                        },
                        "linked_item_codes": {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "minLength": 1
                            }
                        }
                    }
                },
                "ProviderProfilePayload": {
                    "type": "object",
                    "required": ["windows"],
                    "properties": {
                        "specialty": { "type": "string" },
                        "network_status": { "type": "string" },
                        "windows": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "$ref": "#/components/schemas/ProviderProfileWindowPayload" }
                        }
                    }
                },
                "ProviderProfileWindowPayload": {
                    "type": "object",
                    "required": [
                        "window_days",
                        "claim_count",
                        "total_claim_amount",
                        "high_cost_item_ratio",
                        "diagnosis_procedure_mismatch_rate",
                        "peer_amount_percentile",
                        "peer_frequency_percentile",
                        "review_failure_count",
                        "confirmed_fwa_count",
                        "false_positive_count"
                    ],
                    "properties": {
                        "window_days": { "type": "integer", "enum": [30, 90, 180] },
                        "claim_count": { "type": "integer", "minimum": 0 },
                        "total_claim_amount": { "type": "string", "format": "decimal", "description": "Non-negative decimal string." },
                        "high_cost_item_ratio": { "type": "number", "minimum": 0, "maximum": 1 },
                        "diagnosis_procedure_mismatch_rate": { "type": "number", "minimum": 0, "maximum": 1 },
                        "peer_amount_percentile": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "peer_frequency_percentile": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "review_failure_count": { "type": "integer", "minimum": 0 },
                        "confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                        "false_positive_count": { "type": "integer", "minimum": 0 }
                    }
                },
                "ProviderRelationshipGraphPayload": {
                    "type": "object",
                    "required": [
                        "high_risk_neighbor_ratio",
                        "provider_patient_overlap_score",
                        "connected_confirmed_fwa_count"
                    ],
                    "properties": {
                        "high_risk_neighbor_ratio": { "type": "number", "minimum": 0, "maximum": 1 },
                        "provider_patient_overlap_score": { "type": "number", "minimum": 0, "maximum": 1 },
                        "referral_concentration_score": { "type": ["number", "null"], "minimum": 0, "maximum": 1 },
                        "connected_confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                        "network_component_risk_score": { "type": ["integer", "null"], "minimum": 0, "maximum": 100 },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
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
