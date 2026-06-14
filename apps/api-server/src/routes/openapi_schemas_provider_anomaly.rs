use serde_json::{json, Value};

pub(super) fn provider_anomaly_schemas() -> Value {
    json!({
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
        "ProviderSanctionUpsert": {
            "type": "object",
            "required": ["sanction_key", "list", "provider_name", "risk_feature", "risk_score"],
            "properties": {
                "sanction_key": { "type": "string", "minLength": 1 },
                "list": { "type": "string", "enum": ["OIG", "SAM"] },
                "provider_id": { "type": ["string", "null"] },
                "npi": { "type": ["string", "null"] },
                "provider_name": { "type": "string", "minLength": 1 },
                "sanction_type": { "type": ["string", "null"] },
                "effective_date": { "type": ["string", "null"] },
                "source_ref": { "type": ["string", "null"] },
                "risk_feature": { "type": "string", "const": "provider_sanctions_excluded" },
                "risk_score": { "type": "integer", "const": 100 }
            }
        },
        "ProviderSanctionRecord": {
            "allOf": [
                { "$ref": "#/components/schemas/ProviderSanctionUpsert" },
                {
                    "type": "object",
                    "required": ["customer_scope_id", "source_report_uri", "submitted_by", "notes"],
                    "properties": {
                        "customer_scope_id": { "type": "string" },
                        "source_report_uri": { "type": "string" },
                        "submitted_by": { "type": "string" },
                        "notes": { "type": "string" }
                    }
                }
            ]
        },
        "SubmitSanctionsSyncReportRequest": {
            "type": "object",
            "required": ["actor", "notes", "source_report_uri", "report_kind", "run_date", "source_uri", "sync_status", "provider_upserts", "evidence_refs", "governance_boundary"],
            "properties": {
                "actor": { "type": "string", "minLength": 1 },
                "notes": { "type": "string", "minLength": 1 },
                "source_report_uri": { "type": "string", "minLength": 1 },
                "report_kind": { "type": "string", "const": "oig_sam_sanctions_sync_report" },
                "run_date": { "type": "string", "minLength": 1 },
                "source_uri": { "type": "string", "minLength": 1 },
                "source_date": { "type": ["string", "null"] },
                "sync_status": { "type": "string", "minLength": 1 },
                "provider_upserts": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "$ref": "#/components/schemas/ProviderSanctionUpsert" }
                },
                "review_tasks": {
                    "type": "array",
                    "items": { "type": "object" }
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 },
                    "description": "Must include sanctions_sync_reports:{source_report_uri}."
                },
                "governance_boundary": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Source worker report boundary. Submission writes provider sanctions only."
                }
            }
        },
        "SubmitSanctionsSyncReportResponse": {
            "type": "object",
            "required": ["report_kind", "source_report_uri", "provider_upsert_count", "review_task_count", "persisted_provider_sanctions", "active_scoring_policy_change", "label_assignment", "governance_boundary", "audit_event_type"],
            "properties": {
                "report_kind": { "type": "string", "const": "oig_sam_sanctions_sync_report" },
                "source_report_uri": { "type": "string" },
                "provider_upsert_count": { "type": "integer" },
                "review_task_count": { "type": "integer" },
                "persisted_provider_sanctions": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/ProviderSanctionRecord" }
                },
                "active_scoring_policy_change": { "type": "boolean", "const": false },
                "label_assignment": { "type": "boolean", "const": false },
                "governance_boundary": { "type": "string" },
                "audit_event_type": { "type": "string", "enum": ["provider.sanctions_sync.submitted"] }
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
    })
}
