use serde_json::{json, Value};

pub(super) fn mlops_queue_schemas() -> Value {
    json!({
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
    })
}
