use serde_json::{json, Map, Value};

#[path = "openapi_schemas_data_models_datasets.rs"]
mod openapi_schemas_data_models_datasets;
#[path = "openapi_schemas_data_models_evidence.rs"]
mod openapi_schemas_data_models_evidence;
#[path = "openapi_schemas_data_models_factors.rs"]
mod openapi_schemas_data_models_factors;

pub(super) fn data_model_schemas() -> Value {
    let mut schemas = Map::new();
    append_schemas(
        &mut schemas,
        openapi_schemas_data_models_datasets::dataset_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_data_models_evidence::evidence_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_data_models_factors::factor_schemas(),
    );
    append_schemas(
        &mut schemas,
        json!({
                "ModelVersion": {
                    "type": "object",
                    "required": ["model_key", "version", "model_type", "runtime_kind", "execution_provider", "status", "review_mode"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "version": { "type": "string" },
                        "model_type": { "type": "string" },
                        "runtime_kind": { "type": "string" },
                        "execution_provider": { "type": "string" },
                        "status": { "type": "string" },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "artifact_uri": { "type": ["string", "null"] },
                        "endpoint_url": { "type": ["string", "null"] }
                    }
                },
                "ModelEvaluation": {
                    "type": "object",
                    "required": ["evaluation_run_id", "model_key", "model_version", "model_dataset_id", "scheme_family", "confusion_matrix_json", "metrics_json"],
                    "properties": {
                        "evaluation_run_id": { "type": "string" },
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "model_dataset_id": { "type": "string" },
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "auc": { "type": ["string", "null"] },
                        "ks": { "type": ["string", "null"] },
                        "precision": { "type": ["string", "null"] },
                        "recall": { "type": ["string", "null"] },
                        "f1": { "type": ["string", "null"] },
                        "accuracy": { "type": ["string", "null"] },
                        "threshold": { "type": ["string", "null"] },
                        "confusion_matrix_json": { "type": "object" },
                        "feature_importance_uri": {
                            "type": ["string", "null"],
                            "description": "Feature importance artifact must be a Parquet file or Parquet partition directory."
                        },
                        "permutation_importance_uri": {
                            "type": ["string", "null"],
                            "description": "Permutation importance artifact must be a Parquet file or Parquet partition directory."
                        },
                        "metrics_json": {
                            "type": "object",
                            "description": "Model governance metrics. Promotion-ready evaluations should include time_group_split_status, time_split_field, group_split_fields, leakage_check_status, shadow_comparison_status, serving_version_lock_status, artifact_integrity_status, feature_store_materialization_status, segment_fairness_status, label_provenance_status, and pilot_validation_status or customer_validation_status. Public or Kaggle-inspired offline research data must not be used as production promotion evidence."
                        }
                    }
                },
                "ModelEvaluationRegistrationRequest": {
                    "type": "object",
                    "required": ["evaluation_run_id", "model_key", "model_version", "model_dataset_id", "scheme_family", "confusion_matrix_json", "metrics_json"],
                    "properties": {
                        "evaluation_run_id": { "type": "string", "minLength": 1 },
                        "model_key": { "type": "string", "minLength": 1 },
                        "model_version": { "type": "string", "minLength": 1 },
                        "model_dataset_id": { "type": "string", "minLength": 1 },
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "auc": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "ks": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "precision": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "recall": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "f1": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "accuracy": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "threshold": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "confusion_matrix_json": { "type": "object", "minProperties": 1 },
                        "feature_importance_uri": {
                            "type": ["string", "null"],
                            "minLength": 1,
                            "description": "Feature importance artifact must be a Parquet file or Parquet partition directory."
                        },
                        "permutation_importance_uri": {
                            "type": ["string", "null"],
                            "minLength": 1,
                            "description": "Permutation importance artifact must be a Parquet file or Parquet partition directory."
                        },
                        "metrics_json": {
                            "type": "object",
                            "minProperties": 1,
                            "description": "Model governance metrics. Promotion-ready evaluations should include time_group_split_status, time_split_field, group_split_fields, leakage_check_status, shadow_comparison_status, serving_version_lock_status, artifact_integrity_status, feature_store_materialization_status, segment_fairness_status, label_provenance_status, and pilot_validation_status or customer_validation_status. Public or Kaggle-inspired offline research data must not be used as production promotion evidence."
                        }
                    }
                },
                "ModelEvaluationLineage": {
                    "type": "object",
                    "required": ["evaluation_run_id", "model_key", "model_version", "model_dataset_id", "source_dataset_id", "source_dataset_key", "source_dataset_version", "source_data_quality_score", "source_data_quality_status"],
                    "properties": {
                        "evaluation_run_id": { "type": "string" },
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "model_dataset_id": { "type": "string" },
                        "source_dataset_id": { "type": ["string", "null"] },
                        "source_dataset_key": { "type": ["string", "null"] },
                        "source_dataset_version": { "type": ["string", "null"] },
                        "source_data_quality_score": { "type": ["number", "null"] },
                        "source_data_quality_status": { "type": ["string", "null"], "enum": ["empty", "ready", "watch", "blocked", null] }
                    }
                },
                "ModelEvaluationListResponse": {
                    "type": "object",
                    "required": ["evaluations", "lineage"],
                    "properties": {
                        "evaluations": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ModelEvaluation" }
                        },
                        "lineage": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ModelEvaluationLineage" }
                        }
                    }
                },
                "ModelListResponse": {
                    "type": "object",
                    "required": ["models"],
                    "properties": {
                        "models": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ModelVersion" }
                        }
                    }
                },
                "RoutingPolicyRecord": {
                    "type": "object",
                    "required": ["policy_id", "version", "review_mode", "status", "owner", "risk_thresholds", "confidence_thresholds", "provider_review_threshold", "activated_at", "created_at"],
                    "properties": {
                        "policy_id": { "type": "string" },
                        "version": { "type": "integer", "minimum": 1 },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "status": { "type": "string", "enum": ["draft", "submitted", "approved", "active", "retired"] },
                        "owner": { "type": "string" },
                        "risk_thresholds": { "$ref": "#/components/schemas/RiskThresholds" },
                        "confidence_thresholds": { "$ref": "#/components/schemas/ConfidenceThresholds" },
                        "provider_review_threshold": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "activated_at": { "type": ["string", "null"], "format": "date-time" },
                        "created_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "RoutingPolicyListResponse": {
                    "type": "object",
                    "required": ["policies"],
                    "properties": {
                        "policies": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/RoutingPolicyRecord" }
                        }
                    }
                },
                "SaveRoutingPolicyCandidateRequest": {
                    "type": "object",
                    "required": ["policy"],
                    "properties": {
                        "policy": { "$ref": "#/components/schemas/RoutingPolicy" },
                        "owner": { "type": ["string", "null"], "minLength": 1 }
                    }
                },
                "RoutingPolicyLifecycleRequest": {
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
                "RoutingPolicyPromotionGate": {
                    "type": "object",
                    "required": ["label", "passed", "blocker", "evidence_source"],
                    "properties": {
                        "label": { "type": "string" },
                        "passed": { "type": "boolean" },
                        "blocker": { "type": "string" },
                        "evidence_source": { "type": "string", "enum": ["metadata", "approval", "policy_json"] }
                    }
                },
                "RoutingPolicyPromotionGatesResponse": {
                    "type": "object",
                    "required": ["policy_id", "version", "review_mode", "status", "decision", "passed_count", "total_count", "gates", "blockers"],
                    "properties": {
                        "policy_id": { "type": "string" },
                        "version": { "type": "integer" },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "status": { "type": "string" },
                        "decision": { "type": "string", "enum": ["activation_allowed", "activation_blocked"] },
                        "passed_count": { "type": "integer" },
                        "total_count": { "type": "integer" },
                        "gates": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/RoutingPolicyPromotionGate" }
                        },
                        "blockers": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ModelLifecycleResponse": {
                    "type": "object",
                    "required": ["model_key", "version", "status"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "version": { "type": "string" },
                        "status": { "type": "string" }
                    }
                },
                "ModelLifecycleRequest": {
                    "type": "object",
                    "required": ["evidence_refs"],
                    "properties": {
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "description": "Structured evidence references must not contain PII and must include model_versions:{model_key}:{model_version} for the activation target or rollback active version.",
                            "items": { "type": "string", "minLength": 1 },
                            "contains": { "type": "string", "pattern": "^model_versions:[^:]+:[^:]+$" }
                        }
                    }
                },
                "ModelPerformanceResponse": {
                    "type": "object",
                    "required": ["model_key", "data_status", "scored_runs", "average_score", "high_risk_count", "score_psi", "drift_status"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "data_status": { "type": "string", "enum": ["empty", "ready"] },
                        "scored_runs": { "type": "integer" },
                        "average_score": { "type": "number" },
                        "high_risk_count": { "type": "integer" },
                        "score_psi": { "type": ["number", "null"] },
                        "drift_status": { "type": "string", "enum": ["not_available", "stable", "watch", "drift"] },
                        "latest_scored_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "ModelPromotionGate": {
                    "type": "object",
                    "required": ["label", "passed", "blocker", "evidence_source"],
                    "properties": {
                        "label": { "type": "string" },
                        "passed": { "type": "boolean" },
                        "blocker": { "type": "string" },
                        "evidence_source": {
                            "type": "string",
                            "enum": ["runtime", "approval", "dataset", "evaluation", "labels", "qa_feedback", "metadata", "missing"]
                        }
                    }
                },
                "ModelArtifactEvidenceSummary": {
                    "type": "object",
                    "required": ["serving_manifest_uri", "model_artifact_evaluation_report_uri", "permutation_importance_uri", "rust_serving_status", "rust_serving_latency_status", "rust_serving_p95_latency_ms", "rust_serving_latency_measurement_kind", "rust_serving_latency_sample_count"],
                    "properties": {
                        "serving_manifest_uri": { "type": ["string", "null"] },
                        "model_artifact_evaluation_report_uri": { "type": ["string", "null"] },
                        "permutation_importance_uri": { "type": ["string", "null"] },
                        "rust_serving_status": { "type": ["string", "null"] },
                        "rust_serving_latency_status": { "type": ["string", "null"] },
                        "rust_serving_p95_latency_ms": { "type": ["integer", "null"] },
                        "rust_serving_latency_measurement_kind": {
                            "type": ["string", "null"],
                            "description": "Describes whether the latency number is measured runtime evidence or a simulated fixture."
                        },
                        "rust_serving_latency_sample_count": {
                            "type": ["integer", "null"],
                            "minimum": 0
                        }
                    }
                },
                "ModelPromotionGatesResponse": {
                    "type": "object",
                    "required": ["model_key", "model_version", "review_mode", "decision", "passed_count", "total_count", "latest_evaluation_id", "source_dataset_id", "source_data_quality_score", "source_data_quality_status", "data_status", "scored_runs", "open_model_feedback_count", "unresolved_model_feedback_count", "approved_label_count", "needs_review_label_count", "artifact_evidence", "gates", "blockers"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "decision": { "type": "string", "enum": ["routing_allowed", "routing_blocked"] },
                        "passed_count": { "type": "integer" },
                        "total_count": { "type": "integer" },
                        "latest_evaluation_id": { "type": "string" },
                        "source_dataset_id": { "type": "string" },
                        "source_data_quality_score": { "type": ["number", "null"] },
                        "source_data_quality_status": { "type": "string", "enum": ["missing", "ready", "watch", "blocked"] },
                        "data_status": { "type": "string" },
                        "scored_runs": { "type": "integer" },
                        "open_model_feedback_count": { "type": "integer" },
                        "unresolved_model_feedback_count": { "type": "integer" },
                        "approved_label_count": { "type": "integer" },
                        "needs_review_label_count": { "type": "integer" },
                        "artifact_evidence": { "$ref": "#/components/schemas/ModelArtifactEvidenceSummary" },
                        "gates": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ModelPromotionGate" }
                        },
                        "blockers": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ModelRetrainingReadinessResponse": {
                    "type": "object",
                    "required": ["model_key", "model_version", "recommendation", "latest_evaluation_id", "drift_status", "source_dataset_id", "source_data_quality_score", "source_data_quality_status", "open_model_feedback_count", "approved_label_count", "needs_review_label_count", "retraining_triggers", "blockers"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "recommendation": { "type": "string", "enum": ["monitor", "prepare_retraining", "blocked"] },
                        "latest_evaluation_id": { "type": "string" },
                        "drift_status": { "type": "string", "enum": ["not_available", "stable", "watch", "drift"] },
                        "source_dataset_id": { "type": "string" },
                        "source_data_quality_score": { "type": ["number", "null"] },
                        "source_data_quality_status": { "type": "string", "enum": ["missing", "ready", "watch", "blocked"] },
                        "open_model_feedback_count": { "type": "integer" },
                        "approved_label_count": { "type": "integer" },
                        "needs_review_label_count": { "type": "integer" },
                        "retraining_triggers": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "blockers": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ModelRetrainingJob": {
                    "type": "object",
                    "required": ["job_id", "model_key", "model_version", "status", "requested_by", "request_notes", "status_note", "updated_by", "readiness_recommendation", "latest_evaluation_id", "source_dataset_id", "source_data_quality_score", "source_data_quality_status", "trigger_summary", "blocker_summary", "created_at", "updated_at"],
                    "properties": {
                        "job_id": { "type": "string" },
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "status": {
                            "type": "string",
                            "enum": ["queued", "running", "validation", "completed", "failed", "cancelled"],
                            "description": "Job records reach completed only after external training output is registered through /api/v1/ops/model-retraining-jobs/{job_id}/output."
                        },
                        "requested_by": { "type": "string" },
                        "request_notes": { "type": "string" },
                        "status_note": { "type": "string" },
                        "updated_by": { "type": "string" },
                        "readiness_recommendation": { "type": "string" },
                        "latest_evaluation_id": { "type": "string" },
                        "source_dataset_id": { "type": "string" },
                        "source_data_quality_score": { "type": ["number", "null"] },
                        "source_data_quality_status": { "type": "string" },
                        "trigger_summary": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "blocker_summary": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "candidate_model_version": { "type": ["string", "null"] },
                        "candidate_artifact_uri": { "type": ["string", "null"] },
                        "candidate_endpoint_url": { "type": ["string", "null"] },
                        "validation_report_uri": { "type": ["string", "null"] },
                        "output_evaluation_id": { "type": ["string", "null"] },
                        "created_at": { "type": ["string", "null"], "format": "date-time" },
                        "updated_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "ModelRetrainingJobListResponse": {
                    "type": "object",
                    "required": ["jobs"],
                    "properties": {
                        "jobs": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ModelRetrainingJob" }
                        }
                    }
                },
                "ModelMonitoringReviewTask": {
                    "type": "object",
                    "required": ["task_id", "audit_id", "model_key", "model_version", "report_uri", "monitoring_status", "retraining_recommendation", "task_kind", "trigger", "review_status", "reviewer", "review_audit_id", "task", "evidence_refs", "created_at"],
                    "properties": {
                        "task_id": { "type": "string" },
                        "audit_id": { "type": "string" },
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "report_uri": { "type": "string" },
                        "monitoring_status": { "type": "string", "enum": ["passed", "watch", "blocked"] },
                        "retraining_recommendation": { "type": "string", "enum": ["monitor", "prepare_retraining", "blocked"] },
                        "task_kind": { "type": "string" },
                        "trigger": { "type": "string" },
                        "review_status": { "type": "string" },
                        "reviewer": { "type": ["string", "null"] },
                        "review_audit_id": { "type": ["string", "null"] },
                        "task": { "type": "object", "additionalProperties": true },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "created_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "ModelMonitoringReviewQueueResponse": {
                    "type": "object",
                    "required": ["tasks"],
                    "properties": {
                        "tasks": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ModelMonitoringReviewTask" }
                        }
                    }
                },
                "SubmitModelMonitoringReviewTaskReviewRequest": {
                    "type": "object",
                    "required": ["decision", "reviewer", "notes", "evidence_refs"],
                    "properties": {
                        "decision": { "type": "string", "enum": ["acknowledged", "rejected", "prepare_retraining", "open_shadow_review", "open_rollback_review", "closed"] },
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
                            "description": "Must include model_versions:{model_key}:{model_version}, model_monitoring_reports:{report_uri}, and model_monitoring_review_tasks:{task_id}."
                        }
                    }
                },
                "ModelMonitoringReviewTaskReviewResponse": {
                    "type": "object",
                    "required": ["task_id", "model_key", "model_version", "decision", "reviewer", "governance_boundary"],
                    "properties": {
                        "task_id": { "type": "string" },
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "decision": { "type": "string" },
                        "reviewer": { "type": "string" },
                        "governance_boundary": { "type": "string" }
                    }
                },
                "MlopsAlertDeliveryTask": {
                    "type": "object",
                    "required": ["task_id", "audit_id", "model_key", "model_version", "scheduler_execution_report_uri", "alert_delivery_status", "task_kind", "trigger", "route_key", "delivery_status", "review_status", "reviewer", "review_audit_id", "task", "evidence_refs", "created_at"],
                    "properties": {
                        "task_id": { "type": "string" },
                        "audit_id": { "type": "string" },
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "scheduler_execution_report_uri": { "type": "string" },
                        "alert_delivery_status": { "type": "string" },
                        "task_kind": { "type": "string" },
                        "trigger": { "type": "string" },
                        "route_key": { "type": "string" },
                        "delivery_status": { "type": "string" },
                        "review_status": { "type": "string" },
                        "reviewer": { "type": ["string", "null"] },
                        "review_audit_id": { "type": ["string", "null"] },
                        "task": { "type": "object", "additionalProperties": true },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "created_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "MlopsAlertDeliveryQueueResponse": {
                    "type": "object",
                    "required": ["tasks"],
                    "properties": {
                        "tasks": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/MlopsAlertDeliveryTask" }
                        }
                    }
                },
                "SubmitMlopsAlertDeliveryTaskReviewRequest": {
                    "type": "object",
                    "required": ["decision", "reviewer", "notes", "evidence_refs"],
                    "properties": {
                        "decision": { "type": "string", "enum": ["receipt_confirmed", "delivery_failed", "closed_no_action", "escalated_for_governance_review"] },
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
                            "description": "Must include model_versions:{model_key}:{model_version}, mlops_scheduler_execution_reports:{scheduler_execution_report_uri}, and mlops_alert_delivery_tasks:{task_id}."
                        }
                    }
                },
                "MlopsAlertDeliveryTaskReviewResponse": {
                    "type": "object",
                    "required": ["task_id", "model_key", "model_version", "decision", "reviewer", "governance_boundary"],
                    "properties": {
                        "task_id": { "type": "string" },
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "decision": { "type": "string" },
                        "reviewer": { "type": "string" },
                        "governance_boundary": { "type": "string" }
                    }
                },
                "CreateModelRetrainingJobRequest": {
                    "type": "object",
                    "required": ["requested_by", "notes"],
                    "properties": {
                        "requested_by": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Model retraining notes must not contain PII."
                        }
                    }
                },
                "SubmitMlopsMonitoringReportRequest": {
                    "type": "object",
                    "required": ["actor", "notes", "report_uri", "report_kind", "model_version", "overall_status", "retraining_recommendation", "triggers", "review_tasks", "evidence_refs"],
                    "properties": {
                        "actor": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Monitoring notes must not contain PII."
                        },
                        "report_uri": {
                            "type": "string",
                            "minLength": 1,
                            "description": "URI of mlops_monitoring_report.json."
                        },
                        "report_kind": { "type": "string", "enum": ["mlops_monitoring_report"] },
                        "model_version": { "type": "string", "minLength": 1 },
                        "overall_status": { "type": "string", "enum": ["passed", "watch", "blocked"] },
                        "retraining_recommendation": { "type": "string", "enum": ["monitor", "prepare_retraining", "blocked"] },
                        "triggers": {
                            "type": "array",
                            "items": { "type": "string", "minLength": 1 }
                        },
                        "review_tasks": {
                            "type": "array",
                            "description": "Human review tasks opened by monitoring; task content must not contain PII.",
                            "items": { "type": "object", "minProperties": 1 }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 },
                            "description": "Must include model_versions:{model_key}:{model_version} and model_monitoring_reports:{report_uri}."
                        }
                    }
                },
                "SubmitMlopsMonitoringReportResponse": {
                    "type": "object",
                    "required": ["model_key", "model_version", "report_uri", "monitoring_status", "retraining_recommendation", "trigger_count", "review_task_count", "next_actions", "governance_boundary"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "report_uri": { "type": "string" },
                        "monitoring_status": { "type": "string", "enum": ["passed", "watch", "blocked"] },
                        "retraining_recommendation": { "type": "string", "enum": ["monitor", "prepare_retraining", "blocked"] },
                        "trigger_count": { "type": "integer" },
                        "review_task_count": { "type": "integer" },
                        "next_actions": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "governance_boundary": { "type": "string" }
                    }
                },
                "SubmitMlopsAlertDeliveryRequest": {
                    "type": "object",
                    "required": ["actor", "notes", "scheduler_execution_report_uri", "report_kind", "model_version", "alert_delivery_status", "alert_delivery_tasks", "evidence_refs"],
                    "properties": {
                        "actor": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Alert delivery notes must not contain PII."
                        },
                        "scheduler_execution_report_uri": {
                            "type": "string",
                            "minLength": 1,
                            "description": "URI of mlops_scheduler_execution_report.json."
                        },
                        "report_kind": { "type": "string", "enum": ["mlops_scheduler_execution_report"] },
                        "model_version": { "type": "string", "minLength": 1 },
                        "alert_delivery_status": {
                            "type": "string",
                            "enum": ["no_alerts_required", "queued_for_external_alert_router"]
                        },
                        "alert_delivery_tasks": {
                            "type": "array",
                            "items": { "type": "object", "minProperties": 1 }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 },
                            "description": "Must include model_versions:{model_key}:{model_version} and mlops_scheduler_execution_reports:{scheduler_execution_report_uri}."
                        }
                    }
                },
                "SubmitMlopsAlertDeliveryResponse": {
                    "type": "object",
                    "required": ["model_key", "model_version", "scheduler_execution_report_uri", "alert_delivery_status", "alert_delivery_task_count", "alert_routing_policy_configured", "next_actions", "governance_boundary"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "scheduler_execution_report_uri": { "type": "string" },
                        "alert_delivery_status": {
                            "type": "string",
                            "enum": ["no_alerts_required", "queued_for_external_alert_router"]
                        },
                        "alert_delivery_task_count": { "type": "integer" },
                        "alert_routing_policy_configured": { "type": "boolean" },
                        "next_actions": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "governance_boundary": { "type": "string" }
                    }
                },
                "UpdateModelRetrainingJobStatusRequest": {
                    "type": "object",
                    "required": ["status", "actor", "notes"],
                    "properties": {
                        "status": {
                            "type": "string",
                            "enum": ["queued", "running", "validation", "failed", "cancelled"],
                            "description": "Manual worker status updates cannot set completed; completion requires registering external training output through /api/v1/ops/model-retraining-jobs/{job_id}/output."
                        },
                        "actor": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Model retraining notes must not contain PII."
                        }
                    }
                },
                "ClaimModelRetrainingJobRequest": {
                    "type": "object",
                    "required": ["actor", "notes"],
                    "properties": {
                        "actor": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Model retraining notes must not contain PII."
                        },
                        "model_key": { "type": ["string", "null"] }
                    }
                },
                "CompleteModelRetrainingJobRequest": {
                    "type": "object",
                    "required": ["actor", "notes", "candidate_model_version", "artifact_uri", "validation_report_uri", "evaluation_run_id", "evidence_refs", "confusion_matrix_json", "feature_importance_uri", "permutation_importance_uri", "metrics_json"],
                    "properties": {
                        "actor": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Model retraining notes must not contain PII."
                        },
                        "candidate_model_version": { "type": "string", "minLength": 1 },
                        "artifact_uri": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Supported serving model artifact formats: .onnx, .pkl, .joblib, or .json. Rust serving exports should use rust_serving_artifact.json."
                        },
                        "artifact_sha256": {
                            "type": ["string", "null"],
                            "minLength": 1,
                            "description": "Optional sha256 digest for the serving artifact."
                        },
                        "training_artifact_uri": {
                            "type": ["string", "null"],
                            "minLength": 1,
                            "description": "Optional Python training artifact URI for audit and fallback reproducibility. Supported formats: .pkl or .joblib."
                        },
                        "training_artifact_sha256": {
                            "type": ["string", "null"],
                            "minLength": 1,
                            "description": "Optional sha256 digest for training_artifact_uri."
                        },
                        "serving_manifest_uri": {
                            "type": ["string", "null"],
                            "minLength": 1,
                            "description": "Optional Rust serving manifest URI. Must point to serving_manifest.json when provided."
                        },
                        "endpoint_url": { "type": ["string", "null"], "minLength": 1 },
                        "validation_report_uri": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Validation report URI must point to a JSON report."
                        },
                        "evaluation_run_id": { "type": "string", "minLength": 1 },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 },
                            "description": "Model retraining output evidence_refs must not contain PII and must include model_artifacts, model_validation_reports, model_evaluations, model_feature_importance, model_permutation_importance, model_training_artifacts when training_artifact_uri is present, and model_serving_manifests or serving_manifests when serving_manifest_uri is present."
                        },
                        "auc": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "ks": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "precision": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "recall": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "f1": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "accuracy": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "threshold": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "confusion_matrix_json": { "type": "object", "minProperties": 1 },
                        "feature_importance_uri": {
                            "type": ["string", "null"],
                            "minLength": 1,
                            "description": "Feature importance artifact must be a Parquet file or Parquet partition directory."
                        },
                        "permutation_importance_uri": {
                            "type": ["string", "null"],
                            "minLength": 1,
                            "description": "Permutation importance artifact must be a Parquet file or Parquet partition directory."
                        },
                        "metrics_json": {
                            "type": "object",
                            "minProperties": 1,
                            "description": "Model governance metrics. Retraining outputs must include automatic factor and overfitting evidence: time_group_split_status=passed, time_split_field, group_split_fields, leakage_check_status=passed, out_of_time_validation_status=passed, score_stability_status=passed, feature_stability_status=passed, overfitting_diagnostics_status=passed, overfitting_diagnostics_report_uri with a model_overfitting_diagnostics evidence ref, out_of_time_auc, out_of_time_precision, out_of_time_recall, score_psi or psi, max_feature_psi, and a sha256 feature_reproducibility_hash. Promotion-ready retraining outputs should also include shadow_comparison_status, label_provenance_status, and pilot_validation_status or customer_validation_status. Public or Kaggle-inspired offline research data must not be used as production promotion evidence."
                        },
                        "mined_rule_owner": {
                            "type": ["string", "null"],
                            "minLength": 1,
                            "description": "Optional owner for mined rule candidates. Defaults to external-training-platform."
                        },
                        "mined_rule_candidates": {
                            "type": ["array", "null"],
                            "items": { "$ref": "#/components/schemas/RuleDefinition" },
                            "description": "Explainable rules mined by the external training platform. FWA stores them as draft candidates only; human review is required before rule library writeback."
                        }
                    }
                },
                "CompleteModelRetrainingJobResponse": {
                    "type": "object",
                    "required": ["job", "candidate_model", "evaluation", "mined_rule_candidates"],
                    "properties": {
                        "job": { "$ref": "#/components/schemas/ModelRetrainingJob" },
                        "candidate_model": { "$ref": "#/components/schemas/ModelVersion" },
                        "evaluation": { "$ref": "#/components/schemas/ModelEvaluation" },
                        "mined_rule_candidates": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/RuleDetailResponse" },
                            "description": "Rule candidates saved from the external training package. These are drafts pending human review."
                        }
                    }
                },
                "SubmitModelPromotionReviewRequest": {
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
                            "description": "Structured evidence references must not contain PII and must include model_versions:{model_key}:{model_version} for the exact model under review.",
                            "items": { "type": "string", "minLength": 1 },
                            "contains": { "type": "string", "pattern": "^model_versions:[^:]+:[^:]+$" }
                        }
                    }
                },
                "ModelPromotionReview": {
                    "type": "object",
                    "required": ["model_key", "model_version", "decision", "reviewer", "notes", "evidence_refs"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "decision": { "type": "string", "enum": ["approved", "rejected"] },
                        "reviewer": { "type": "string" },
                        "notes": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "created_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
        }),
    );
    Value::Object(schemas)
}

fn append_schemas(target: &mut Map<String, Value>, schemas: Value) {
    let Value::Object(schemas) = schemas else {
        unreachable!("OpenAPI data model schema group must be a JSON object");
    };
    target.extend(schemas);
}
