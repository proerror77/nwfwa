use serde_json::{json, Map, Value};

#[path = "openapi_schemas_rules_observability.rs"]
mod openapi_schemas_rules_observability;
#[path = "openapi_schemas_rules_promotion.rs"]
mod openapi_schemas_rules_promotion;

pub(super) fn rule_schemas() -> Value {
    let mut schemas = Map::new();
    append_schemas(
        &mut schemas,
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
                "RuleLifecycleResponse": {
                    "type": "object",
                    "required": ["rule_id", "status", "active_version", "latest_version"],
                    "properties": {
                        "rule_id": { "type": "string" },
                        "status": { "type": "string", "enum": ["draft", "active", "submitted", "approved"] },
                        "active_version": { "type": ["integer", "null"] },
                        "latest_version": { "type": "integer" }
                    }
                },
                "RuleLifecycleRequest": {
                    "type": "object",
                    "required": ["evidence_refs"],
                    "properties": {
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "description": "Structured evidence references must not contain PII.",
                            "items": { "type": "string", "minLength": 1 }
                        }
                    }
                },
                "RulePerformanceRecord": {
                    "type": "object",
                    "required": [
                        "rule_id",
                        "alert_code",
                        "trigger_count",
                        "reviewed_count",
                        "confirmed_fwa_count",
                        "false_positive_count",
                        "mark_rate",
                        "precision",
                        "false_positive_rate",
                        "saving_amount",
                        "roi"
                    ],
                    "properties": {
                        "rule_id": { "type": "string" },
                        "alert_code": { "type": "string" },
                        "trigger_count": { "type": "integer", "minimum": 0 },
                        "reviewed_count": { "type": "integer", "minimum": 0 },
                        "confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                        "false_positive_count": { "type": "integer", "minimum": 0 },
                        "mark_rate": { "type": "number", "minimum": 0 },
                        "precision": { "type": "number", "minimum": 0 },
                        "false_positive_rate": { "type": "number", "minimum": 0 },
                        "saving_amount": { "type": "string", "format": "decimal" },
                        "roi": { "type": "number" }
                    }
                },
                "RulePerformanceResponse": {
                    "type": "object",
                    "required": ["rules"],
                    "properties": {
                        "rules": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/RulePerformanceRecord" }
                        }
                    }
                },
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
                            "description": "Structured evidence references must not contain PII. Accepted candidates must include backtest evidence.",
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
        }),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_rules_promotion::promotion_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_rules_observability::observability_schemas(),
    );
    Value::Object(schemas)
}

fn append_schemas(target: &mut Map<String, Value>, schemas: Value) {
    let Value::Object(schemas) = schemas else {
        unreachable!("OpenAPI rule schema group must be a JSON object");
    };
    target.extend(schemas);
}
