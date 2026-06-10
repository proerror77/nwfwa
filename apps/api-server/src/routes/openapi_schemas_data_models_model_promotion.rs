use serde_json::{json, Value};

pub(super) fn model_promotion_schemas() -> Value {
    json!({
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
    })
}
