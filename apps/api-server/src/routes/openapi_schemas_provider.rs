use serde_json::{json, Value};

pub(super) fn provider_schemas() -> Value {
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
                        "outlier_flags",
                        "window_findings",
                        "evidence_refs"
                    ],
                    "properties": {
                        "provider_id": { "type": "string" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "risk_tier": { "type": "string", "enum": ["low", "medium", "high"] },
                        "review_required": { "type": "boolean" },
                        "review_route": { "type": "string", "enum": ["none", "provider_review"] },
                        "specialty": { "type": ["string", "null"] },
                        "network_status": { "type": ["string", "null"] },
                        "review_failure_count": { "type": "integer", "minimum": 0 },
                        "confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                        "false_positive_count": { "type": "integer", "minimum": 0 },
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
                "AnomalyClusteringReviewTaskInput": {
                    "type": "object",
                    "required": ["candidate_kind", "candidate_id", "task_kind", "review_queue", "required_review", "evidence_refs"],
                    "properties": {
                        "candidate_kind": {
                            "type": "string",
                            "enum": ["provider_peer_anomaly", "provider_graph_anomaly", "claim_entity_anomaly"]
                        },
                        "candidate_id": { "type": "string", "minLength": 1 },
                        "task_kind": { "type": "string", "minLength": 1 },
                        "review_queue": { "type": "string", "minLength": 1 },
                        "required_review": { "type": "string", "minLength": 1 },
                        "decision_options": {
                            "type": "array",
                            "items": { "type": "string", "minLength": 1 }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 },
                            "description": "Must include anomaly_clustering_reports:{source_report_uri}; values must not contain PII."
                        },
                        "candidate_payload": {
                            "type": "object",
                            "additionalProperties": true,
                            "description": "Explainable candidate fields copied from the clustering report."
                        }
                    }
                },
                "SubmitAnomalyClusteringReportRequest": {
                    "type": "object",
                    "required": ["actor", "notes", "source_report_uri", "report_kind", "dataset_key", "dataset_version", "label_policy", "governance_boundary", "review_tasks", "evidence_refs"],
                    "properties": {
                        "actor": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Submission notes must not contain PII."
                        },
                        "source_report_uri": {
                            "type": "string",
                            "minLength": 1,
                            "description": "URI of provider_peer_clustering_report.json, provider_graph_community_report.json, or claim_entity_clustering_report.json."
                        },
                        "report_kind": {
                            "type": "string",
                            "enum": ["provider_peer_clustering", "provider_graph_community_clustering", "claim_entity_clustering"]
                        },
                        "dataset_key": { "type": "string", "minLength": 1 },
                        "dataset_version": { "type": "string", "minLength": 1 },
                        "label_policy": { "type": "string", "minLength": 1 },
                        "governance_boundary": {
                            "type": "string",
                            "minLength": 1,
                            "description": "The source report boundary. Report submission opens review tasks only."
                        },
                        "review_tasks": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "$ref": "#/components/schemas/AnomalyClusteringReviewTaskInput" }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 },
                            "description": "Must include anomaly_clustering_reports:{source_report_uri}; values must not contain PII."
                        }
                    }
                },
                "SubmitAnomalyClusteringReportResponse": {
                    "type": "object",
                    "required": ["report_kind", "source_report_uri", "review_task_count", "accepted_for_review_queue", "active_rule_writeback", "model_activation", "label_assignment", "case_creation", "governance_boundary", "audit_event_type"],
                    "properties": {
                        "report_kind": { "type": "string" },
                        "source_report_uri": { "type": "string" },
                        "review_task_count": { "type": "integer" },
                        "accepted_for_review_queue": { "type": "boolean" },
                        "active_rule_writeback": { "type": "boolean", "const": false },
                        "model_activation": { "type": "boolean", "const": false },
                        "label_assignment": { "type": "boolean", "const": false },
                        "case_creation": { "type": "boolean", "const": false },
                        "governance_boundary": { "type": "string" },
                        "audit_event_type": { "type": "string", "enum": ["provider.anomaly_clustering.report_submitted"] }
                    }
                },
                "AnomalyReviewQueueTask": {
                    "type": "object",
                    "required": ["candidate_kind", "candidate_id", "task_kind", "review_queue", "required_review", "decision_options", "source_report_uri", "report_kind", "dataset_key", "dataset_version", "label_policy", "governance_boundary", "review_status", "candidate_payload", "evidence_refs"],
                    "properties": {
                        "candidate_kind": {
                            "type": "string",
                            "enum": ["provider_peer_anomaly", "provider_graph_anomaly", "claim_entity_anomaly"]
                        },
                        "candidate_id": { "type": "string" },
                        "task_kind": { "type": "string" },
                        "review_queue": { "type": "string" },
                        "required_review": { "type": "string" },
                        "decision_options": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "source_report_uri": { "type": "string" },
                        "report_kind": { "type": "string" },
                        "dataset_key": { "type": "string" },
                        "dataset_version": { "type": "string" },
                        "label_policy": { "type": "string" },
                        "governance_boundary": { "type": "string" },
                        "review_status": {
                            "type": "string",
                            "enum": ["pending_human_review", "reviewed"]
                        },
                        "reviewer": { "type": ["string", "null"] },
                        "decision": { "type": ["string", "null"] },
                        "candidate_payload": { "type": "object", "additionalProperties": true },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "AnomalyReviewQueueResponse": {
                    "type": "object",
                    "required": ["tasks"],
                    "properties": {
                        "tasks": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AnomalyReviewQueueTask" }
                        }
                    }
                },
                "ReviewAnomalyCandidateRequest": {
                    "type": "object",
                    "required": ["candidate_kind", "candidate_id", "source_report_uri", "decision", "reviewer", "notes", "evidence_refs"],
                    "properties": {
                        "candidate_kind": {
                            "type": "string",
                            "enum": ["provider_peer_anomaly", "provider_graph_anomaly", "claim_entity_anomaly"]
                        },
                        "candidate_id": { "type": "string", "minLength": 1 },
                        "source_report_uri": {
                            "type": "string",
                            "minLength": 1,
                            "description": "URI of provider_peer_clustering_report.json, provider_graph_community_report.json, or claim_entity_clustering_report.json."
                        },
                        "decision": {
                            "type": "string",
                            "enum": ["accepted_for_review", "rejected", "open_investigation_review", "request_more_evidence"]
                        },
                        "reviewer": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Review notes must not contain PII."
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 },
                            "description": "Must include anomaly_clustering_reports:{source_report_uri}; values must not contain PII."
                        },
                        "candidate_payload": {
                            "type": "object",
                            "additionalProperties": true,
                            "description": "Optional non-decisional candidate context from the clustering report."
                        }
                    }
                },
                "ReviewAnomalyCandidateResponse": {
                    "type": "object",
                    "required": ["candidate_kind", "candidate_id", "decision", "reviewer", "accepted_for_review", "active_rule_writeback", "model_activation", "label_assignment", "governance_boundary", "audit_event_type"],
                    "properties": {
                        "candidate_kind": { "type": "string" },
                        "candidate_id": { "type": "string" },
                        "decision": { "type": "string" },
                        "reviewer": { "type": "string" },
                        "accepted_for_review": { "type": "boolean" },
                        "active_rule_writeback": { "type": "boolean", "const": false },
                        "model_activation": { "type": "boolean", "const": false },
                        "label_assignment": { "type": "boolean", "const": false },
                        "governance_boundary": { "type": "string" },
                        "audit_event_type": { "type": "string", "enum": ["anomaly.candidate.reviewed"] }
                    }
                },
                "SubmitMedicalReviewResultRequest": {
                    "type": "object",
                    "required": ["claim_id", "scoring_audit_id", "reviewer", "decision", "notes", "evidence_refs"],
                    "properties": {
                        "claim_id": { "type": "string", "minLength": 1 },
                        "scoring_audit_id": { "type": "string", "minLength": 1 },
                        "reviewer": { "type": "string", "minLength": 1 },
                        "decision": {
                            "type": "string",
                            "enum": ["evidence_sufficient", "request_more_evidence", "medical_necessity_issue", "no_medical_issue"]
                        },
                        "clinical_outcomes": {
                            "type": "array",
                            "description": "Optional controlled clinical outcome fields for model training and rule tuning. When omitted, the platform derives one compatible outcome from decision.",
                            "items": {
                                "type": "string",
                                "enum": ["documentation_issue", "medical_necessity_review_required", "insufficient_evidence", "medical_necessity_issue", "clinical_evidence_sufficient", "false_positive"]
                            }
                        },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Medical review notes must not contain PII."
                        },
                        "evidence_refs": {
                            "type": "array",
                            "description": "Structured evidence references must not contain PII. For claims with the referenced normalized scoring trace, canonical evidence refs from that trace are merged into the persisted medical review and response.",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 }
                        }
                    }
                },
                "MedicalReviewResultResponse": {
                    "type": "object",
                    "required": ["claim_id", "event_type", "event_status", "audit_id", "run_id", "review_status", "clinical_outcomes", "evidence_refs"],
                    "properties": {
                        "claim_id": { "type": "string" },
                        "event_type": { "type": "string" },
                        "event_status": { "type": "string" },
                        "audit_id": { "type": "string" },
                        "run_id": { "type": "string" },
                        "review_status": { "type": "string" },
                        "clinical_outcomes": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "MedicalReviewQueueItem": {
                    "type": "object",
                    "required": ["claim_id", "run_id", "audit_id", "medical_reasonableness_score", "review_route", "evidence_status", "missing_evidence", "item_finding_count", "evidence_refs", "canonical_source_refs", "canonical_evidence_refs", "review_status"],
                    "properties": {
                        "claim_id": { "type": "string" },
                        "run_id": { "type": "string" },
                        "audit_id": { "type": "string" },
                        "medical_reasonableness_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "review_route": { "type": "string" },
                        "evidence_status": { "type": "string" },
                        "missing_evidence": { "type": "array", "items": { "type": "string" } },
                        "item_finding_count": { "type": "integer" },
                        "first_item_code": { "type": ["string", "null"] },
                        "first_issue_type": { "type": ["string", "null"] },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "canonical_source_refs": { "type": "array", "items": { "type": "string" } },
                        "canonical_evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "created_at": { "type": ["string", "null"], "format": "date-time" },
                        "review_status": { "type": "string" },
                        "review_audit_id": { "type": ["string", "null"] },
                        "review_decision": { "type": ["string", "null"] },
                        "reviewer": { "type": ["string", "null"] },
                        "reviewed_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "MedicalReviewQueueResponse": {
                    "type": "object",
                    "required": ["items"],
                    "properties": {
                        "items": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/MedicalReviewQueueItem" }
                        }
                    }
                },
                "ClinicalEvidenceAssessment": {
                    "type": "object",
                    "required": [
                        "review_required",
                        "review_route",
                        "evidence_status",
                        "minimum_evidence",
                        "missing_evidence",
                        "item_findings",
                        "evidence_refs"
                    ],
                    "properties": {
                        "review_required": { "type": "boolean" },
                        "review_route": { "type": "string", "enum": ["none", "medical_review"] },
                        "evidence_status": {
                            "type": "string",
                            "enum": [
                                "no_clinical_evidence_required",
                                "sufficient_for_basic_review",
                                "missing_required_evidence"
                            ]
                        },
                        "minimum_evidence": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "missing_evidence": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "item_findings": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ClinicalEvidenceFinding" }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ClinicalEvidenceFinding": {
                    "type": "object",
                    "required": [
                        "item_code",
                        "issue_type",
                        "required_evidence",
                        "missing_evidence",
                        "reason",
                        "review_route",
                        "evidence_refs"
                    ],
                    "properties": {
                        "item_code": { "type": "string" },
                        "issue_type": { "type": "string" },
                        "required_evidence": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "missing_evidence": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "reason": { "type": "string" },
                        "review_route": { "type": "string" },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ScoreBreakdown": {
                    "type": "object",
                    "required": [
                        "peer_deviation_score",
                        "rule_score",
                        "anomaly_score",
                        "ml_score",
                        "medical_reasonableness_score",
                        "provider_network_score",
                        "similar_case_score",
                        "final_score"
                    ],
                    "properties": {
                        "peer_deviation_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "rule_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "anomaly_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "ml_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "medical_reasonableness_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "provider_network_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "similar_case_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "final_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        }
                    }
                },
                "AlertResponse": {
                    "type": "object",
                    "required": ["alert_code", "severity", "reason", "rule_id", "rule_version", "required_evidence"],
                    "properties": {
                        "alert_code": {
                            "type": "string"
                        },
                        "severity": {
                            "type": "string"
                        },
                        "reason": {
                            "type": "string"
                        },
                        "rule_id": {
                            "type": "string"
                        },
                        "rule_version": {
                            "type": "integer"
                        },
                        "required_evidence": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/RequiredEvidence" }
                        }
                    }
                },
                "RequiredEvidence": {
                    "type": "object",
                    "required": ["evidence_type", "blocking"],
                    "properties": {
                        "evidence_type": { "type": "string", "minLength": 1 },
                        "evidence_request_type": { "type": ["string", "null"] },
                        "blocking": { "type": "boolean", "default": true },
                        "policy_authority_ref": { "type": ["string", "null"] },
                        "exception_check": { "type": ["string", "null"] }
                    }
                },
                "AdjudicationPolicy": {
                    "type": "object",
                    "required": ["customer_approval_ref", "appeal_or_override_route", "effective_date", "rollback_plan_ref", "production_threshold_ref", "routing_impact_ref"],
                    "properties": {
                        "customer_approval_ref": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Customer-approved deterministic rule-list or policy approval reference."
                        },
                        "appeal_or_override_route": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Customer-approved appeal, exception, or reviewer override route."
                        },
                        "effective_date": { "type": "string", "minLength": 1 },
                        "rollback_plan_ref": { "type": "string", "minLength": 1 },
                        "production_threshold_ref": { "type": "string", "minLength": 1 },
                        "routing_impact_ref": { "type": "string", "minLength": 1 }
                    }
                },
    })
}
