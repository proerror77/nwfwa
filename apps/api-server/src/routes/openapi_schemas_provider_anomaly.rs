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
                "source_record_count": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Optional source artifact count; when all count fields are supplied, must equal valid_record_count + invalid_record_count."
                },
                "valid_record_count": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Optional source artifact count; when supplied, must match provider_upserts length."
                },
                "invalid_record_count": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Optional source artifact count; when supplied, must match review_tasks length."
                },
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
        "ProviderProfileWindowUpsert": {
            "type": "object",
            "required": ["provider_id", "windows", "evidence_refs"],
            "properties": {
                "provider_id": { "type": "string", "minLength": 1 },
                "specialty": { "type": ["string", "null"] },
                "network_status": { "type": ["string", "null"] },
                "windows": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "$ref": "#/components/schemas/ProviderProfileWindowPayload" }
                },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "ProviderProfileWindowRecord": {
            "allOf": [
                { "$ref": "#/components/schemas/ProviderProfileWindowUpsert" },
                {
                    "type": "object",
                    "required": ["customer_scope_id", "as_of_date", "source_report_uri", "submitted_by", "notes"],
                    "properties": {
                        "customer_scope_id": { "type": "string" },
                        "as_of_date": { "type": "string" },
                        "source_report_uri": { "type": "string" },
                        "submitted_by": { "type": "string" },
                        "notes": { "type": "string" }
                    }
                }
            ]
        },
        "SubmitProviderProfileWindowRollupRequest": {
            "type": "object",
            "required": ["actor", "notes", "source_report_uri", "report_kind", "as_of_date", "source_uri", "provider_count", "claim_count", "provider_profiles", "evidence_refs", "governance_boundary"],
            "properties": {
                "actor": { "type": "string", "minLength": 1 },
                "notes": { "type": "string", "minLength": 1 },
                "source_report_uri": { "type": "string", "minLength": 1 },
                "report_kind": { "type": "string", "const": "provider_profile_window_rollup" },
                "as_of_date": { "type": "string", "minLength": 1 },
                "source_uri": { "type": "string", "minLength": 1 },
                "provider_count": { "type": "integer", "minimum": 1 },
                "claim_count": { "type": "integer", "minimum": 0 },
                "provider_profiles": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "$ref": "#/components/schemas/ProviderProfileWindowUpsert" }
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 },
                    "description": "Must include provider_profile_window_rollups:{source_report_uri}."
                },
                "governance_boundary": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Source worker report boundary. Submission writes provider profile windows only."
                }
            }
        },
        "SubmitProviderProfileWindowRollupResponse": {
            "type": "object",
            "required": ["report_kind", "source_report_uri", "provider_profile_count", "claim_count", "persisted_provider_profiles", "active_scoring_policy_change", "label_assignment", "governance_boundary", "audit_event_type"],
            "properties": {
                "report_kind": { "type": "string", "const": "provider_profile_window_rollup" },
                "source_report_uri": { "type": "string" },
                "provider_profile_count": { "type": "integer" },
                "claim_count": { "type": "integer" },
                "persisted_provider_profiles": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/ProviderProfileWindowRecord" }
                },
                "active_scoring_policy_change": { "type": "boolean", "const": false },
                "label_assignment": { "type": "boolean", "const": false },
                "governance_boundary": { "type": "string" },
                "audit_event_type": { "type": "string", "enum": ["provider.profile_windows.submitted"] }
            }
        },
        "ProviderGraphSignalUpsert": {
            "type": "object",
            "required": ["provider_id", "billing_ring_membership", "temporal_co_billing_frequency_7d", "shared_member_provider_count", "evidence_refs"],
            "properties": {
                "provider_id": { "type": "string", "minLength": 1 },
                "high_risk_neighbor_ratio": {
                    "type": ["number", "null"],
                    "minimum": 0,
                    "maximum": 1
                },
                "provider_patient_overlap_score": {
                    "type": ["number", "null"],
                    "minimum": 0,
                    "maximum": 1
                },
                "referral_concentration_score": {
                    "type": ["number", "null"],
                    "minimum": 0,
                    "maximum": 1
                },
                "billing_ring_membership": { "type": "boolean" },
                "temporal_co_billing_frequency_7d": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 1
                },
                "referral_concentration_entropy": {
                    "type": ["number", "null"],
                    "minimum": 0,
                    "maximum": 1
                },
                "shared_member_provider_count": { "type": "integer", "minimum": 0 },
                "connected_confirmed_fwa_count": {
                    "type": ["integer", "null"],
                    "minimum": 0
                },
                "network_component_risk_score": {
                    "type": ["integer", "null"],
                    "minimum": 0,
                    "maximum": 100
                },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "ProviderGraphSignalRecord": {
            "allOf": [
                { "$ref": "#/components/schemas/ProviderGraphSignalUpsert" },
                {
                    "type": "object",
                    "required": ["customer_scope_id", "as_of_date", "source_report_uri", "submitted_by", "notes"],
                    "properties": {
                        "customer_scope_id": { "type": "string" },
                        "as_of_date": { "type": "string" },
                        "source_report_uri": { "type": "string" },
                        "submitted_by": { "type": "string" },
                        "notes": { "type": "string" }
                    }
                }
            ]
        },
        "SubmitProviderGraphSignalRollupRequest": {
            "type": "object",
            "required": ["actor", "notes", "source_report_uri", "report_kind", "as_of_date", "source_uri", "provider_count", "claim_count", "provider_relationships", "evidence_refs", "governance_boundary"],
            "properties": {
                "actor": { "type": "string", "minLength": 1 },
                "notes": { "type": "string", "minLength": 1 },
                "source_report_uri": { "type": "string", "minLength": 1 },
                "report_kind": { "type": "string", "const": "provider_graph_signal_rollup" },
                "as_of_date": { "type": "string", "minLength": 1 },
                "source_uri": { "type": "string", "minLength": 1 },
                "provider_count": { "type": "integer", "minimum": 1 },
                "claim_count": { "type": "integer", "minimum": 0 },
                "provider_relationships": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "$ref": "#/components/schemas/ProviderGraphSignalUpsert" }
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 },
                    "description": "Must include provider_graph_signal_rollups:{source_report_uri}."
                },
                "governance_boundary": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Source worker report boundary. Submission writes provider graph signals only."
                }
            }
        },
        "SubmitProviderGraphSignalRollupResponse": {
            "type": "object",
            "required": ["report_kind", "source_report_uri", "provider_relationship_count", "claim_count", "persisted_provider_relationships", "active_scoring_policy_change", "label_assignment", "case_creation", "governance_boundary", "audit_event_type"],
            "properties": {
                "report_kind": { "type": "string", "const": "provider_graph_signal_rollup" },
                "source_report_uri": { "type": "string" },
                "provider_relationship_count": { "type": "integer" },
                "claim_count": { "type": "integer" },
                "persisted_provider_relationships": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/ProviderGraphSignalRecord" }
                },
                "active_scoring_policy_change": { "type": "boolean", "const": false },
                "label_assignment": { "type": "boolean", "const": false },
                "case_creation": { "type": "boolean", "const": false },
                "governance_boundary": { "type": "string" },
                "audit_event_type": { "type": "string", "enum": ["provider.graph_signals.submitted"] }
            }
        },
        "PeerBenchmarkGroupUpsert": {
            "type": "object",
            "required": ["peer_group_key", "specialty", "region", "service_segment", "claim_count", "p25", "p50", "p75", "p90", "p99", "evidence_refs"],
            "properties": {
                "peer_group_key": { "type": "string", "minLength": 1 },
                "specialty": { "type": "string", "minLength": 1 },
                "region": { "type": "string", "minLength": 1 },
                "service_segment": { "type": "string", "minLength": 1 },
                "claim_count": { "type": "integer", "minimum": 1 },
                "p25": { "type": "number", "minimum": 0 },
                "p50": { "type": "number", "minimum": 0 },
                "p75": { "type": "number", "minimum": 0 },
                "p90": { "type": "number", "minimum": 0 },
                "p99": { "type": "number", "minimum": 0 },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "PeerBenchmarkGroupRecord": {
            "allOf": [
                { "$ref": "#/components/schemas/PeerBenchmarkGroupUpsert" },
                {
                    "type": "object",
                    "required": ["customer_scope_id", "benchmark_month", "source_report_uri", "submitted_by", "notes"],
                    "properties": {
                        "customer_scope_id": { "type": "string" },
                        "benchmark_month": { "type": "string" },
                        "source_report_uri": { "type": "string" },
                        "submitted_by": { "type": "string" },
                        "notes": { "type": "string" }
                    }
                }
            ]
        },
        "SubmitPeerBenchmarkRequest": {
            "type": "object",
            "required": ["actor", "notes", "source_report_uri", "report_kind", "benchmark_month", "source_uri", "claim_count", "peer_group_count", "peer_groups", "evidence_refs", "governance_boundary"],
            "properties": {
                "actor": { "type": "string", "minLength": 1 },
                "notes": { "type": "string", "minLength": 1 },
                "source_report_uri": { "type": "string", "minLength": 1 },
                "report_kind": { "type": "string", "const": "peer_percentile_benchmark" },
                "benchmark_month": { "type": "string", "minLength": 1 },
                "source_uri": { "type": "string", "minLength": 1 },
                "claim_count": { "type": "integer", "minimum": 0 },
                "peer_group_count": { "type": "integer", "minimum": 1 },
                "peer_groups": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "$ref": "#/components/schemas/PeerBenchmarkGroupUpsert" }
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 },
                    "description": "Must include peer_benchmarks:{source_report_uri}."
                },
                "governance_boundary": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Source worker report boundary. Submission writes peer benchmark reference data only."
                }
            }
        },
        "SubmitPeerBenchmarkResponse": {
            "type": "object",
            "required": ["report_kind", "source_report_uri", "benchmark_month", "peer_group_count", "claim_count", "persisted_peer_groups", "active_scoring_policy_change", "label_assignment", "claim_scoring", "governance_boundary", "audit_event_type"],
            "properties": {
                "report_kind": { "type": "string", "const": "peer_percentile_benchmark" },
                "source_report_uri": { "type": "string" },
                "benchmark_month": { "type": "string" },
                "peer_group_count": { "type": "integer" },
                "claim_count": { "type": "integer" },
                "persisted_peer_groups": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/PeerBenchmarkGroupRecord" }
                },
                "active_scoring_policy_change": { "type": "boolean", "const": false },
                "label_assignment": { "type": "boolean", "const": false },
                "claim_scoring": { "type": "boolean", "const": false },
                "governance_boundary": { "type": "string" },
                "audit_event_type": { "type": "string", "enum": ["provider.peer_benchmarks.submitted"] }
            }
        },
        "EpisodeWindowRollupPayload": {
            "type": "object",
            "required": ["window_days", "claim_count", "total_claim_amount", "unique_procedure_code_count", "max_procedure_code_frequency", "duplicate_amount_day_count"],
            "properties": {
                "window_days": {
                    "type": "integer",
                    "enum": [30, 90, 365]
                },
                "claim_count": { "type": "integer", "minimum": 0 },
                "total_claim_amount": { "type": "number", "minimum": 0 },
                "unique_procedure_code_count": { "type": "integer", "minimum": 0 },
                "max_procedure_code_frequency": { "type": "integer", "minimum": 0 },
                "duplicate_amount_day_count": { "type": "integer", "minimum": 0 }
            }
        },
        "EpisodeRollupUpsert": {
            "type": "object",
            "required": ["episode_key", "member_id", "provider_id", "windows", "evidence_refs"],
            "properties": {
                "episode_key": { "type": "string", "minLength": 1 },
                "member_id": { "type": "string", "minLength": 1 },
                "provider_id": { "type": "string", "minLength": 1 },
                "windows": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "$ref": "#/components/schemas/EpisodeWindowRollupPayload" }
                },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "EpisodeRollupRecord": {
            "allOf": [
                { "$ref": "#/components/schemas/EpisodeRollupUpsert" },
                {
                    "type": "object",
                    "required": ["customer_scope_id", "as_of_date", "source_report_uri", "submitted_by", "notes"],
                    "properties": {
                        "customer_scope_id": { "type": "string" },
                        "as_of_date": { "type": "string" },
                        "source_report_uri": { "type": "string" },
                        "submitted_by": { "type": "string" },
                        "notes": { "type": "string" }
                    }
                }
            ]
        },
        "SubmitEpisodeRollupRequest": {
            "type": "object",
            "required": ["actor", "notes", "source_report_uri", "report_kind", "as_of_date", "source_uri", "episode_count", "claim_count", "episodes", "evidence_refs", "governance_boundary"],
            "properties": {
                "actor": { "type": "string", "minLength": 1 },
                "notes": { "type": "string", "minLength": 1 },
                "source_report_uri": { "type": "string", "minLength": 1 },
                "report_kind": { "type": "string", "const": "member_provider_episode_aggregation" },
                "as_of_date": { "type": "string", "minLength": 1 },
                "source_uri": { "type": "string", "minLength": 1 },
                "episode_count": { "type": "integer", "minimum": 1 },
                "claim_count": { "type": "integer", "minimum": 0 },
                "episodes": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "$ref": "#/components/schemas/EpisodeRollupUpsert" }
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 },
                    "description": "Must include episode_rollups:{source_report_uri}."
                },
                "governance_boundary": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Source worker report boundary. Submission writes episode rollups only."
                }
            }
        },
        "SubmitEpisodeRollupResponse": {
            "type": "object",
            "required": ["report_kind", "source_report_uri", "episode_count", "claim_count", "persisted_episode_rollups", "active_scoring_policy_change", "label_assignment", "case_creation", "claim_denial", "governance_boundary", "audit_event_type"],
            "properties": {
                "report_kind": { "type": "string", "const": "member_provider_episode_aggregation" },
                "source_report_uri": { "type": "string" },
                "episode_count": { "type": "integer" },
                "claim_count": { "type": "integer" },
                "persisted_episode_rollups": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/EpisodeRollupRecord" }
                },
                "active_scoring_policy_change": { "type": "boolean", "const": false },
                "label_assignment": { "type": "boolean", "const": false },
                "case_creation": { "type": "boolean", "const": false },
                "claim_denial": { "type": "boolean", "const": false },
                "governance_boundary": { "type": "string" },
                "audit_event_type": { "type": "string", "enum": ["provider.episode_rollups.submitted"] }
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
