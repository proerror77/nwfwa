use serde_json::{json, Value};

pub(super) fn rule_schemas() -> Value {
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
                "AuditHistoryEvent": {
                    "type": "object",
                    "required": ["audit_id", "run_id", "actor_role", "event_type", "event_status", "summary", "payload", "evidence_refs"],
                    "properties": {
                        "audit_id": { "type": "string" },
                        "run_id": { "type": "string" },
                        "actor_role": { "type": "string" },
                        "event_type": { "type": "string" },
                        "event_status": { "type": "string" },
                        "summary": { "type": "string" },
                        "payload": { "type": "object" },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "created_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "AuditEventListResponse": {
                    "type": "object",
                    "required": ["events"],
                    "properties": {
                        "events": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AuditHistoryEvent" }
                        }
                    }
                },
                "ApiCallRecord": {
                    "type": "object",
                    "required": ["call_id", "endpoint", "method", "status_code", "result", "source_system", "actor_role", "customer_scope_id", "claim_id", "run_id", "audit_id", "event_type", "idempotency_key", "evidence_refs", "observed_at"],
                    "properties": {
                        "call_id": { "type": "string" },
                        "endpoint": { "type": "string" },
                        "method": { "type": "string" },
                        "status_code": { "type": "integer" },
                        "result": { "type": "string" },
                        "source_system": { "type": "string" },
                        "actor_role": { "type": "string" },
                        "customer_scope_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "run_id": { "type": "string" },
                        "audit_id": { "type": "string" },
                        "event_type": { "type": "string" },
                        "idempotency_key": { "type": ["string", "null"] },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "observed_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "ApiCallListResponse": {
                    "type": "object",
                    "required": ["calls"],
                    "properties": {
                        "calls": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ApiCallRecord" }
                        }
                    }
                },
                "WebhookEvent": {
                    "type": "object",
                    "required": ["event_id", "event_type", "source_event_type", "source_audit_id", "customer_scope_id", "claim_id", "run_id", "delivery_status", "retry_count", "max_attempts", "next_attempt_at", "last_attempt_at", "last_response_status_code", "last_error_message", "idempotency_key", "signature_key_id", "signature_algorithm", "signature_base_string", "payload", "evidence_refs", "occurred_at"],
                    "properties": {
                        "event_id": { "type": "string" },
                        "event_type": {
                            "type": "string",
                            "enum": ["fwa.score.completed", "fwa.case.routed", "fwa.investigation.closed", "fwa.qa.reviewed", "fwa.medical.reviewed"]
                        },
                        "source_event_type": { "type": "string" },
                        "source_audit_id": { "type": "string" },
                        "customer_scope_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "run_id": { "type": "string" },
                        "delivery_status": { "type": "string", "enum": ["pending", "retry_wait", "delivered", "failed"] },
                        "retry_count": { "type": "integer" },
                        "max_attempts": { "type": "integer" },
                        "next_attempt_at": { "type": ["string", "null"], "format": "date-time" },
                        "last_attempt_at": { "type": ["string", "null"], "format": "date-time" },
                        "last_response_status_code": { "type": ["integer", "null"] },
                        "last_error_message": { "type": ["string", "null"] },
                        "idempotency_key": { "type": "string" },
                        "signature_key_id": { "type": "string" },
                        "signature_algorithm": { "type": "string", "enum": ["hmac-sha256"] },
                        "signature_base_string": { "type": "string" },
                        "payload": { "type": "object" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "occurred_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "SubmitWebhookDeliveryAttemptRequest": {
                    "type": "object",
                    "required": ["delivery_status"],
                    "properties": {
                        "delivery_status": { "type": "string", "enum": ["delivered", "failed"] },
                        "response_status_code": { "type": ["integer", "null"] },
                        "error_message": {
                            "type": ["string", "null"],
                            "description": "Webhook delivery error message; must not contain PII."
                        }
                    }
                },
                "WebhookDeliveryAttempt": {
                    "type": "object",
                    "required": ["event_id", "attempt_number", "delivery_status", "response_status_code", "error_message", "next_attempt_at", "attempted_at"],
                    "properties": {
                        "event_id": { "type": "string" },
                        "attempt_number": { "type": "integer" },
                        "delivery_status": { "type": "string", "enum": ["delivered", "failed"] },
                        "response_status_code": { "type": ["integer", "null"] },
                        "error_message": { "type": ["string", "null"] },
                        "next_attempt_at": { "type": ["string", "null"], "format": "date-time" },
                        "attempted_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "WebhookEventListResponse": {
                    "type": "object",
                    "required": ["events"],
                    "properties": {
                        "events": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/WebhookEvent" }
                        }
                    }
                },
                "OpsAlert": {
                    "type": "object",
                    "required": ["alert_id", "alert_type", "severity", "status", "claim_id", "lead_id", "case_id", "scheme_family", "message", "recommended_action", "evidence_refs"],
                    "properties": {
                        "alert_id": { "type": "string" },
                        "alert_type": {
                            "type": "string",
                            "enum": ["high_risk_routing", "sla_breach", "medical_review_required", "agent_approval_pending"]
                        },
                        "severity": {
                            "type": "string",
                            "enum": ["critical", "high", "medium", "low"]
                        },
                        "status": {
                            "type": "string",
                            "enum": ["open", "closed"]
                        },
                        "claim_id": { "type": "string" },
                        "lead_id": { "type": ["string", "null"] },
                        "case_id": { "type": ["string", "null"] },
                        "scheme_family": { "type": "string" },
                        "message": { "type": "string" },
                        "recommended_action": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "OpsAlertListResponse": {
                    "type": "object",
                    "required": ["alerts"],
                    "properties": {
                        "alerts": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/OpsAlert" }
                        }
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
    })
}
