use serde_json::{json, Value};

pub(super) fn dataset_schemas() -> Value {
    json!({
        "DatasetSplit": {
            "type": "object",
            "required": ["split_name", "data_uri", "row_count", "label_distribution_json"],
            "properties": {
                "split_name": { "type": "string" },
                "data_uri": { "type": "string" },
                "row_count": { "type": "integer" },
                "positive_count": { "type": ["integer", "null"] },
                "negative_count": { "type": ["integer", "null"] },
                "label_distribution_json": { "type": "object" }
            }
        },
        "SchemaField": {
            "type": "object",
            "required": ["field_name", "logical_type", "nullable", "semantic_role", "description", "profile_json"],
            "properties": {
                "field_name": { "type": "string" },
                "logical_type": { "type": "string" },
                "nullable": { "type": "boolean" },
                "semantic_role": { "type": "string" },
                "description": {
                    "type": "string",
                    "description": "Business description for the factor; must not contain PII."
                },
                "profile_json": { "type": "object" }
            }
        },
        "FieldMapping": {
            "type": "object",
            "required": ["mapping_id", "dataset_id", "external_field", "canonical_target", "transform_kind", "transform_json", "status"],
            "properties": {
                "mapping_id": { "type": "string" },
                "dataset_id": { "type": "string" },
                "external_field": { "type": "string" },
                "canonical_target": { "type": "string" },
                "feature_name": { "type": ["string", "null"] },
                "transform_kind": { "type": "string" },
                "transform_json": { "type": "object" },
                "status": { "type": "string" }
            }
        },
        "DatasetRecord": {
            "type": "object",
            "required": ["dataset_id", "source_key", "display_name", "business_domain", "dataset_key", "dataset_version", "sample_grain", "label_column", "entity_keys", "manifest_uri", "schema_uri", "profile_uri", "storage_format", "schema_hash", "row_count", "status", "splits", "fields", "mappings"],
            "properties": {
                "dataset_id": { "type": "string" },
                "source_key": { "type": "string" },
                "display_name": { "type": "string" },
                "business_domain": { "type": "string" },
                "dataset_key": { "type": "string" },
                "dataset_version": { "type": "string" },
                "sample_grain": { "type": "string" },
                "label_column": { "type": "string" },
                "entity_keys": { "type": "array", "items": { "type": "string" } },
                "manifest_uri": { "type": "string" },
                "schema_uri": { "type": "string" },
                "profile_uri": { "type": "string" },
                "storage_format": { "type": "string", "const": "parquet" },
                "schema_hash": { "type": "string" },
                "row_count": { "type": "integer" },
                "status": { "type": "string" },
                "splits": { "type": "array", "items": { "$ref": "#/components/schemas/DatasetSplit" } },
                "fields": { "type": "array", "items": { "$ref": "#/components/schemas/SchemaField" } },
                "mappings": { "type": "array", "items": { "$ref": "#/components/schemas/FieldMapping" } }
            }
        },
        "DatasetRegistrationRequest": {
            "type": "object",
            "required": ["source_key", "display_name", "business_domain", "owner", "description", "dataset_key", "dataset_version", "sample_grain", "label_column", "entity_keys", "manifest_uri", "schema_uri", "profile_uri", "storage_format", "schema_hash", "row_count", "status", "splits", "fields"],
            "properties": {
                "source_key": { "type": "string" },
                "display_name": { "type": "string" },
                "business_domain": { "type": "string" },
                "owner": { "type": "string" },
                "description": {
                    "type": "string",
                    "description": "Dataset business description; must not contain PII."
                },
                "dataset_key": { "type": "string" },
                "dataset_version": { "type": "string" },
                "sample_grain": { "type": "string" },
                "label_column": { "type": "string" },
                "entity_keys": { "type": "array", "items": { "type": "string" } },
                "manifest_uri": { "type": "string" },
                "schema_uri": { "type": "string" },
                "profile_uri": { "type": "string" },
                "storage_format": { "type": "string", "const": "parquet" },
                "schema_hash": { "type": "string" },
                "row_count": { "type": "integer" },
                "status": { "type": "string" },
                "splits": { "type": "array", "items": { "$ref": "#/components/schemas/DatasetSplit" } },
                "fields": { "type": "array", "items": { "$ref": "#/components/schemas/SchemaField" } }
            }
        },
        "DatasetListResponse": {
            "type": "object",
            "required": ["datasets", "health"],
            "properties": {
                "datasets": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/DatasetRecord" }
                },
                "health": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/DatasetHealth" }
                }
            }
        },
        "DatasetHealth": {
            "type": "object",
            "required": ["dataset_id", "dataset_key", "dataset_version", "data_quality_score", "data_quality_status", "field_count", "label_count", "entity_key_count", "high_missing_count", "unstable_field_count", "unowned_field_count", "online_ready_count", "issue_count"],
            "properties": {
                "dataset_id": { "type": "string" },
                "dataset_key": { "type": "string" },
                "dataset_version": { "type": "string" },
                "data_quality_score": { "type": "number" },
                "data_quality_status": { "type": "string", "enum": ["empty", "ready", "watch", "blocked"] },
                "field_count": { "type": "integer" },
                "label_count": { "type": "integer" },
                "entity_key_count": { "type": "integer" },
                "high_missing_count": { "type": "integer" },
                "unstable_field_count": { "type": "integer" },
                "unowned_field_count": { "type": "integer" },
                "online_ready_count": { "type": "integer" },
                "issue_count": { "type": "integer" }
            }
        },
        "FieldMappingRequest": {
            "type": "object",
            "required": ["external_field", "canonical_target", "transform_kind", "transform_json", "status"],
            "properties": {
                "external_field": { "type": "string", "minLength": 1 },
                "canonical_target": { "type": "string", "minLength": 1 },
                "feature_name": { "type": ["string", "null"], "minLength": 1 },
                "transform_kind": { "type": "string", "enum": ["direct", "cast", "enum_map", "derived", "aggregate"] },
                "transform_json": { "type": "object" },
                "status": { "type": "string", "enum": ["draft", "active", "deprecated"] }
            }
        },
        "FieldMappingResponse": {
            "type": "object",
            "required": ["mapping"],
            "properties": {
                "mapping": { "$ref": "#/components/schemas/FieldMapping" }
            }
        },
        "ScoringFeatureContextMaterialization": {
            "type": "object",
            "required": ["materialization_id", "customer_scope_id", "as_of_date", "report_uri", "report_kind", "source_uris", "claim_count", "context_count", "contexts_json", "evidence_refs", "governance_boundary", "submitted_by", "notes"],
            "properties": {
                "materialization_id": { "type": "string" },
                "customer_scope_id": { "type": "string" },
                "as_of_date": { "type": "string" },
                "report_uri": { "type": "string" },
                "report_kind": { "type": "string", "const": "scoring_feature_context_materialization" },
                "source_uris": { "type": "object" },
                "claim_count": { "type": "integer" },
                "context_count": { "type": "integer" },
                "contexts_json": {
                    "type": "array",
                    "description": "Claim-level PeerFeatureContext, ClinicalCompatibilityFeatureContext, and EpisodeUtilizationFeatureContext payloads generated by the worker.",
                    "items": { "type": "object" }
                },
                "evidence_refs": { "type": "array", "items": { "type": "string" } },
                "governance_boundary": { "type": "string" },
                "submitted_by": { "type": "string" },
                "notes": { "type": "string" }
            }
        },
        "ScoringFeatureContextMaterializationRequest": {
            "type": "object",
            "required": ["materialization_id", "actor", "notes", "report_uri", "report_kind", "as_of_date", "source_uris", "claim_count", "context_count", "contexts", "evidence_refs", "governance_boundary"],
            "properties": {
                "materialization_id": { "type": "string" },
                "actor": { "type": "string" },
                "notes": { "type": "string" },
                "report_uri": { "type": "string" },
                "report_kind": { "type": "string", "const": "scoring_feature_context_materialization" },
                "as_of_date": { "type": "string" },
                "source_uris": { "type": "object" },
                "claim_count": { "type": "integer" },
                "context_count": { "type": "integer" },
                "contexts": {
                    "type": "array",
                    "items": { "type": "object" }
                },
                "evidence_refs": { "type": "array", "items": { "type": "string" } },
                "governance_boundary": { "type": "string" }
            }
        },
        "ScoringFeatureContextMaterializationResponse": {
            "type": "object",
            "required": ["materialization"],
            "properties": {
                "materialization": { "$ref": "#/components/schemas/ScoringFeatureContextMaterialization" }
            }
        },
        "ClinicalCompatibilityReferenceUpsert": {
            "type": "object",
            "required": ["compatibility_key", "diagnosis_code_prefix", "procedure_code", "diagnosis_procedure_match_score", "data_source", "policy_authority_ref", "rationale", "evidence_refs"],
            "properties": {
                "compatibility_key": { "type": "string", "minLength": 1 },
                "diagnosis_code_prefix": { "type": "string", "minLength": 1 },
                "procedure_code": { "type": "string", "minLength": 1 },
                "diagnosis_procedure_match_score": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 1
                },
                "data_source": { "type": "string", "minLength": 1 },
                "policy_authority_ref": { "type": "string", "minLength": 1 },
                "rationale": { "type": "string", "minLength": 1 },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        },
        "ClinicalCompatibilityReferenceRecord": {
            "allOf": [
                { "$ref": "#/components/schemas/ClinicalCompatibilityReferenceUpsert" },
                {
                    "type": "object",
                    "required": ["customer_scope_id", "reference_version", "effective_date", "source_authority", "source_report_uri", "submitted_by", "notes"],
                    "properties": {
                        "customer_scope_id": { "type": "string" },
                        "reference_version": { "type": "string" },
                        "effective_date": { "type": "string" },
                        "source_authority": { "type": "string" },
                        "source_report_uri": { "type": "string" },
                        "submitted_by": { "type": "string" },
                        "notes": { "type": "string" }
                    }
                }
            ]
        },
        "ClinicalCompatibilityReferenceSubmissionRequest": {
            "type": "object",
            "required": ["actor", "notes", "source_report_uri", "report_kind", "reference_version", "effective_date", "source_authority", "source_uri", "record_count", "records", "evidence_refs", "governance_boundary"],
            "properties": {
                "actor": { "type": "string", "minLength": 1 },
                "notes": { "type": "string", "minLength": 1 },
                "source_report_uri": { "type": "string", "minLength": 1 },
                "report_kind": { "type": "string", "const": "clinical_compatibility_reference" },
                "reference_version": { "type": "string", "minLength": 1 },
                "effective_date": { "type": "string", "minLength": 1 },
                "source_authority": { "type": "string", "minLength": 1 },
                "source_uri": { "type": "string", "minLength": 1 },
                "record_count": { "type": "integer", "minimum": 1 },
                "records": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "$ref": "#/components/schemas/ClinicalCompatibilityReferenceUpsert" }
                },
                "review_tasks": {
                    "type": "array",
                    "items": { "type": "object" }
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 },
                    "description": "Must include clinical_compatibility_references:{source_report_uri}."
                },
                "governance_boundary": { "type": "string", "minLength": 1 }
            }
        },
        "ClinicalCompatibilityReferenceSubmissionResponse": {
            "type": "object",
            "required": ["report_kind", "source_report_uri", "reference_version", "record_count", "review_task_count", "persisted_records", "active_scoring_policy_change", "claim_scoring", "label_assignment", "claim_denial", "medical_review_replacement", "governance_boundary", "audit_event_type"],
            "properties": {
                "report_kind": { "type": "string", "const": "clinical_compatibility_reference" },
                "source_report_uri": { "type": "string" },
                "reference_version": { "type": "string" },
                "record_count": { "type": "integer" },
                "review_task_count": { "type": "integer" },
                "persisted_records": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/ClinicalCompatibilityReferenceRecord" }
                },
                "active_scoring_policy_change": { "type": "boolean", "const": false },
                "claim_scoring": { "type": "boolean", "const": false },
                "label_assignment": { "type": "boolean", "const": false },
                "claim_denial": { "type": "boolean", "const": false },
                "medical_review_replacement": { "type": "boolean", "const": false },
                "governance_boundary": { "type": "string" },
                "audit_event_type": { "type": "string", "enum": ["clinical_compatibility.reference.submitted"] }
            }
        },
        "UnbundlingComparatorCandidateUpsert": {
            "type": "object",
            "required": ["candidate_id", "rule_id", "episode_key", "member_id", "provider_id", "window_days", "bundled_code", "matched_component_codes", "claim_ids", "policy_authority_ref", "evidence_refs", "recommended_review"],
            "properties": {
                "candidate_id": { "type": "string", "minLength": 1 },
                "rule_id": { "type": "string", "minLength": 1 },
                "episode_key": { "type": "string", "minLength": 1 },
                "member_id": { "type": "string", "minLength": 1 },
                "provider_id": { "type": "string", "minLength": 1 },
                "window_days": { "type": "integer", "enum": [30, 90, 365] },
                "bundled_code": { "type": "string", "minLength": 1 },
                "matched_component_codes": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 }
                },
                "claim_ids": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 }
                },
                "policy_authority_ref": { "type": "string", "minLength": 1 },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 }
                },
                "recommended_review": { "type": "string", "const": "medical_review_candidate" }
            }
        },
        "UnbundlingComparatorCandidateRecord": {
            "allOf": [
                { "$ref": "#/components/schemas/UnbundlingComparatorCandidateUpsert" },
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
        "UnbundlingComparatorCandidatesSubmissionRequest": {
            "type": "object",
            "required": ["actor", "notes", "source_report_uri", "report_kind", "as_of_date", "source_uri", "rule_count", "episode_count", "candidate_count", "candidates", "evidence_refs", "governance_boundary"],
            "properties": {
                "actor": { "type": "string", "minLength": 1 },
                "notes": { "type": "string", "minLength": 1 },
                "source_report_uri": { "type": "string", "minLength": 1 },
                "report_kind": { "type": "string", "const": "unbundling_comparator" },
                "as_of_date": { "type": "string", "minLength": 1 },
                "source_uri": { "type": "string", "minLength": 1 },
                "rule_count": { "type": "integer" },
                "episode_count": { "type": "integer" },
                "candidate_count": { "type": "integer", "minimum": 1 },
                "candidates": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "$ref": "#/components/schemas/UnbundlingComparatorCandidateUpsert" }
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 },
                    "description": "Must include unbundling_comparator_candidates:{source_report_uri}."
                },
                "governance_boundary": { "type": "string", "minLength": 1 }
            }
        },
        "UnbundlingComparatorCandidatesSubmissionResponse": {
            "type": "object",
            "required": ["report_kind", "source_report_uri", "as_of_date", "rule_count", "episode_count", "candidate_count", "persisted_candidates", "active_scoring_policy_change", "claim_scoring", "label_assignment", "claim_denial", "case_creation", "medical_review_replacement", "governance_boundary", "audit_event_type"],
            "properties": {
                "report_kind": { "type": "string", "const": "unbundling_comparator" },
                "source_report_uri": { "type": "string" },
                "as_of_date": { "type": "string" },
                "rule_count": { "type": "integer" },
                "episode_count": { "type": "integer" },
                "candidate_count": { "type": "integer" },
                "persisted_candidates": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/UnbundlingComparatorCandidateRecord" }
                },
                "active_scoring_policy_change": { "type": "boolean", "const": false },
                "claim_scoring": { "type": "boolean", "const": false },
                "label_assignment": { "type": "boolean", "const": false },
                "claim_denial": { "type": "boolean", "const": false },
                "case_creation": { "type": "boolean", "const": false },
                "medical_review_replacement": { "type": "boolean", "const": false },
                "governance_boundary": { "type": "string" },
                "audit_event_type": { "type": "string", "enum": ["unbundling_comparator.candidates.submitted"] }
            }
        },
        "WorkerDataPipelineExecutionReportSubmissionRequest": {
            "type": "object",
            "required": ["actor", "notes", "source_report_uri", "report_kind", "plan_uri", "run_status_uri", "run_id", "execution_date", "job_count", "pending_or_failed_job_count", "review_task_count", "job_executions", "review_tasks", "evidence_refs", "governance_boundary"],
            "properties": {
                "actor": { "type": "string", "minLength": 1 },
                "notes": { "type": "string", "minLength": 1 },
                "source_report_uri": { "type": "string", "minLength": 1 },
                "report_kind": { "type": "string", "const": "worker_data_pipeline_execution_report" },
                "plan_uri": { "type": "string", "minLength": 1 },
                "run_status_uri": { "type": "string", "minLength": 1 },
                "readiness_report_uri": {
                    "type": "string",
                    "minLength": 1,
                    "nullable": true,
                    "description": "Optional URI for the worker_data_pipeline_readiness_report used as this scheduler run's readiness gate."
                },
                "readiness_gate_status": { "type": "string", "enum": ["ready", "blocked", "missing"], "nullable": true },
                "run_id": { "type": "string", "minLength": 1 },
                "execution_date": { "type": "string", "minLength": 1 },
                "job_count": { "type": "integer", "minimum": 1 },
                "pending_or_failed_job_count": { "type": "integer", "minimum": 0 },
                "review_task_count": { "type": "integer", "minimum": 0 },
                "job_executions": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "object" }
                },
                "review_tasks": {
                    "type": "array",
                    "items": { "type": "object" }
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 },
                    "description": "Must include worker_data_pipeline_execution_reports:{source_report_uri}; when readiness_report_uri is supplied, must also include worker_data_pipeline_readiness_reports:{readiness_report_uri}."
                },
                "governance_boundary": { "type": "string", "minLength": 1 }
            }
        },
        "WorkerDataPipelineExecutionReportSubmissionResponse": {
            "type": "object",
            "required": ["report_kind", "source_report_uri", "run_id", "execution_date", "job_count", "pending_or_failed_job_count", "review_task_count", "active_scoring_policy_change", "claim_scoring", "label_assignment", "claim_denial", "model_activation", "routing_policy_change", "governance_boundary", "audit_event_type"],
            "properties": {
                "report_kind": { "type": "string", "const": "worker_data_pipeline_execution_report" },
                "source_report_uri": { "type": "string" },
                "readiness_report_uri": { "type": "string", "nullable": true },
                "readiness_gate_status": { "type": "string", "enum": ["ready", "blocked", "missing"], "nullable": true },
                "run_id": { "type": "string" },
                "execution_date": { "type": "string" },
                "job_count": { "type": "integer" },
                "pending_or_failed_job_count": { "type": "integer" },
                "review_task_count": { "type": "integer" },
                "active_scoring_policy_change": { "type": "boolean", "const": false },
                "claim_scoring": { "type": "boolean", "const": false },
                "label_assignment": { "type": "boolean", "const": false },
                "claim_denial": { "type": "boolean", "const": false },
                "model_activation": { "type": "boolean", "const": false },
                "routing_policy_change": { "type": "boolean", "const": false },
                "governance_boundary": { "type": "string" },
                "audit_event_type": { "type": "string", "enum": ["worker_data_pipeline.execution_report.submitted"] }
            }
        },
        "WorkerDataPipelineReadinessReportSubmissionRequest": {
            "type": "object",
            "required": ["actor", "notes", "source_report_uri", "report_kind", "plan_uri", "readiness_input_uri", "readiness_status", "job_count", "ready_job_count", "blocked_job_count", "review_task_count", "job_readiness", "review_tasks", "evidence_refs", "governance_boundary"],
            "properties": {
                "actor": { "type": "string", "minLength": 1 },
                "notes": { "type": "string", "minLength": 1 },
                "source_report_uri": { "type": "string", "minLength": 1 },
                "report_kind": { "type": "string", "const": "worker_data_pipeline_readiness_report" },
                "plan_uri": { "type": "string", "minLength": 1 },
                "readiness_input_uri": { "type": "string", "minLength": 1 },
                "readiness_status": { "type": "string", "enum": ["ready", "blocked"] },
                "job_count": { "type": "integer", "minimum": 1 },
                "ready_job_count": { "type": "integer", "minimum": 0 },
                "blocked_job_count": { "type": "integer", "minimum": 0 },
                "review_task_count": { "type": "integer", "minimum": 0 },
                "job_readiness": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "object" }
                },
                "review_tasks": {
                    "type": "array",
                    "items": { "type": "object" }
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 },
                    "description": "Must include worker_data_pipeline_readiness_reports:{source_report_uri}."
                },
                "governance_boundary": { "type": "string", "minLength": 1 }
            }
        },
        "WorkerDataPipelineReadinessReportSubmissionResponse": {
            "type": "object",
            "required": ["report_kind", "source_report_uri", "readiness_status", "job_count", "ready_job_count", "blocked_job_count", "review_task_count", "active_scoring_policy_change", "claim_scoring", "label_assignment", "claim_denial", "model_activation", "routing_policy_change", "external_fetch_execution", "artifact_submission", "governance_boundary", "audit_event_type"],
            "properties": {
                "report_kind": { "type": "string", "const": "worker_data_pipeline_readiness_report" },
                "source_report_uri": { "type": "string" },
                "readiness_status": { "type": "string", "enum": ["ready", "blocked"] },
                "job_count": { "type": "integer" },
                "ready_job_count": { "type": "integer" },
                "blocked_job_count": { "type": "integer" },
                "review_task_count": { "type": "integer" },
                "active_scoring_policy_change": { "type": "boolean", "const": false },
                "claim_scoring": { "type": "boolean", "const": false },
                "label_assignment": { "type": "boolean", "const": false },
                "claim_denial": { "type": "boolean", "const": false },
                "model_activation": { "type": "boolean", "const": false },
                "routing_policy_change": { "type": "boolean", "const": false },
                "external_fetch_execution": { "type": "boolean", "const": false },
                "artifact_submission": { "type": "boolean", "const": false },
                "governance_boundary": { "type": "string" },
                "audit_event_type": { "type": "string", "enum": ["worker_data_pipeline.readiness_report.submitted"] }
            }
        },
    })
}
