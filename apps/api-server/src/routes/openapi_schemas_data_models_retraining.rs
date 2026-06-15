use serde_json::{json, Value};

pub(super) fn retraining_schemas() -> Value {
    json!({
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
                    "description": "Must include model_versions:{model_key}:{model_version} and model_monitoring_reports:{report_uri}; values must not contain PII or local dry-run/template refs."
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
        "SubmitProbabilityCalibrationReportRequest": {
            "type": "object",
            "required": ["actor", "notes", "report_uri", "report_kind", "model_version", "as_of_date", "row_count", "minimum_calibration_rows", "bin_count", "expected_calibration_error", "max_expected_calibration_error", "brier_score", "max_brier_score", "calibration_status", "bins", "review_tasks", "evidence_refs", "governance_boundary"],
            "properties": {
                "actor": { "type": "string", "minLength": 1 },
                "notes": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Calibration notes must not contain PII."
                },
                "report_uri": {
                    "type": "string",
                    "minLength": 1,
                    "description": "URI of probability_calibration_report.json."
                },
                "report_kind": { "type": "string", "const": "probability_calibration_report" },
                "model_version": { "type": "string", "minLength": 1 },
                "as_of_date": { "type": "string", "minLength": 1 },
                "row_count": { "type": "integer", "minimum": 1 },
                "minimum_calibration_rows": { "type": "integer", "minimum": 1 },
                "bin_count": { "type": "integer", "minimum": 1 },
                "expected_calibration_error": { "type": "number", "minimum": 0, "maximum": 1 },
                "max_expected_calibration_error": { "type": "number", "minimum": 0, "maximum": 1 },
                "brier_score": { "type": "number", "minimum": 0, "maximum": 1 },
                "max_brier_score": { "type": "number", "minimum": 0, "maximum": 1 },
                "calibration_status": {
                    "type": "string",
                    "enum": ["passed", "needs_calibration_review", "insufficient_sample"],
                    "description": "Must match row_count, minimum_calibration_rows, expected_calibration_error, max_expected_calibration_error, brier_score, and max_brier_score."
                },
                "bins": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "object", "minProperties": 1 }
                },
                "review_tasks": {
                    "type": "array",
                    "description": "Human review tasks opened by calibration evidence; task content must not contain PII.",
                    "items": { "type": "object", "minProperties": 1 }
                },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 },
                    "description": "Must include model_versions:{model_key}:{model_version}, probability_calibration_reports:{report_uri}, probability_calibration_input:{source_uri}, and calibration_labels:{label_source_uri}."
                },
                "governance_boundary": { "type": "string", "minLength": 1 }
            }
        },
        "SubmitProbabilityCalibrationReportResponse": {
            "type": "object",
            "required": ["model_key", "model_version", "report_uri", "calibration_status", "row_count", "expected_calibration_error", "brier_score", "review_task_count", "active_calibration_change", "calibrated_probability_serving_activation", "threshold_change", "label_assignment", "persisted_report", "governance_boundary"],
            "properties": {
                "model_key": { "type": "string" },
                "model_version": { "type": "string" },
                "report_uri": { "type": "string" },
                "calibration_status": {
                    "type": "string",
                    "enum": ["passed", "needs_calibration_review", "insufficient_sample"]
                },
                "row_count": { "type": "integer" },
                "expected_calibration_error": { "type": "number" },
                "brier_score": { "type": "number" },
                "review_task_count": { "type": "integer" },
                "active_calibration_change": { "type": "boolean", "const": false },
                "calibrated_probability_serving_activation": { "type": "boolean", "const": false },
                "threshold_change": { "type": "boolean", "const": false },
                "label_assignment": { "type": "boolean", "const": false },
                "persisted_report": {
                    "type": "object",
                    "description": "Persisted model-governance calibration evidence. It does not activate calibrated serving."
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
                    "description": "Must include model_versions:{model_key}:{model_version} and mlops_scheduler_execution_reports:{scheduler_execution_report_uri}; values must not contain PII or local dry-run/template refs."
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
                    "description": "Production artifact URI for the serving model artifact. Supported formats: .onnx, .pkl, .joblib, or .json. Rust serving exports should use rust_serving_artifact.json."
                },
                "artifact_sha256": {
                    "type": ["string", "null"],
                    "minLength": 1,
                    "description": "Optional sha256 digest for the serving artifact."
                },
                "training_artifact_uri": {
                    "type": ["string", "null"],
                    "minLength": 1,
                    "description": "Optional production artifact URI for Python training artifact audit and fallback reproducibility. Supported formats: .pkl or .joblib."
                },
                "training_artifact_sha256": {
                    "type": ["string", "null"],
                    "minLength": 1,
                    "description": "Optional sha256 digest for training_artifact_uri."
                },
                "serving_manifest_uri": {
                    "type": ["string", "null"],
                    "minLength": 1,
                    "description": "Optional production artifact URI for the Rust serving manifest. Must point to serving_manifest.json when provided."
                },
                "endpoint_url": { "type": ["string", "null"], "minLength": 1 },
                "validation_report_uri": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Production artifact URI for the validation report. Must point to a JSON report."
                },
                "evaluation_run_id": { "type": "string", "minLength": 1 },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 },
                    "description": "Model retraining output evidence_refs must not contain PII or local dry-run/template refs, and must include model_artifacts, model_validation_reports, model_evaluations, model_feature_importance, model_permutation_importance, model_training_artifacts when training_artifact_uri is present, and model_serving_manifests or serving_manifests when serving_manifest_uri is present."
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
    })
}
