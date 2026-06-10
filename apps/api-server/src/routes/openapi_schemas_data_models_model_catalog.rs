use serde_json::{json, Value};

pub(super) fn model_catalog_schemas() -> Value {
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
    })
}
