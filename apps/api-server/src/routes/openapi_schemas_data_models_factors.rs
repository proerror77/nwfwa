use serde_json::{json, Value};

pub(super) fn factor_schemas() -> Value {
    json!({
        "FeatureSet": {
            "type": "object",
            "required": ["feature_set_id", "business_domain", "feature_set_key", "version", "dataset_id", "features_uri", "feature_list_json", "row_count", "label_column", "status"],
            "properties": {
                "feature_set_id": { "type": "string" },
                "business_domain": { "type": "string" },
                "feature_set_key": { "type": "string" },
                "version": { "type": "string" },
                "dataset_id": { "type": "string" },
                "features_uri": { "type": "string" },
                "feature_list_json": { "type": "array", "items": { "type": "string" } },
                "row_count": { "type": "integer", "minimum": 1 },
                "label_column": { "type": "string" },
                "status": { "type": "string", "enum": ["draft", "active", "deprecated"] }
            }
        },
        "FeatureSetRegistrationRequest": {
            "type": "object",
            "required": ["business_domain", "feature_set_key", "version", "dataset_id", "features_uri", "feature_list_json", "row_count", "label_column", "status"],
            "properties": {
                "business_domain": { "type": "string", "minLength": 1 },
                "feature_set_key": { "type": "string", "minLength": 1 },
                "version": { "type": "string", "minLength": 1 },
                "dataset_id": { "type": "string", "minLength": 1 },
                "features_uri": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Feature matrix artifact URI. Active feature sets require production artifact URI evidence."
                },
                "feature_list_json": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } },
                "row_count": { "type": "integer", "minimum": 1 },
                "label_column": { "type": "string", "minLength": 1 },
                "status": { "type": "string", "enum": ["draft", "active", "deprecated"] }
            }
        },
        "ModelDataset": {
            "type": "object",
            "required": ["model_dataset_id", "business_domain", "task_type", "label_name", "feature_set_id", "train_uri", "validation_uri", "row_counts_json", "label_distribution_json", "status"],
            "properties": {
                "model_dataset_id": { "type": "string" },
                "business_domain": { "type": "string" },
                "task_type": { "type": "string" },
                "label_name": { "type": "string" },
                "feature_set_id": { "type": "string" },
                "train_uri": { "type": "string" },
                "validation_uri": { "type": "string" },
                "test_uri": { "type": ["string", "null"] },
                "row_counts_json": { "type": "object", "additionalProperties": true },
                "label_distribution_json": { "type": "object", "additionalProperties": true },
                "status": { "type": "string", "enum": ["draft", "active", "deprecated"] }
            }
        },
        "ModelDatasetRegistrationRequest": {
            "type": "object",
            "required": ["business_domain", "task_type", "label_name", "feature_set_id", "train_uri", "validation_uri", "row_counts_json", "label_distribution_json", "status"],
            "properties": {
                "business_domain": { "type": "string", "minLength": 1 },
                "task_type": { "type": "string", "minLength": 1 },
                "label_name": { "type": "string", "minLength": 1 },
                "feature_set_id": { "type": "string", "minLength": 1 },
                "train_uri": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Training split artifact URI. Active model datasets require production artifact URI evidence."
                },
                "validation_uri": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Validation split artifact URI. Active model datasets require production artifact URI evidence."
                },
                "test_uri": {
                    "type": ["string", "null"],
                    "minLength": 1,
                    "description": "Optional test split artifact URI. Active model datasets require production artifact URI evidence when provided."
                },
                "row_counts_json": { "type": "object", "minProperties": 1, "additionalProperties": true },
                "label_distribution_json": { "type": "object", "minProperties": 1, "additionalProperties": true },
                "status": { "type": "string", "enum": ["draft", "active", "deprecated"] }
            }
        },
        "FactorReadinessResponse": {
            "type": "object",
            "required": ["dataset_count", "factor_count", "label_count", "entity_key_count", "data_quality_score", "data_quality_status", "online_ready_count", "rule_convertible_count", "mapped_factor_count", "high_missing_count", "unstable_factor_count", "unowned_factor_count", "ready_factor_count", "review_factor_count", "readiness_issue_counts", "scheme_readiness", "factor_cards"],
            "properties": {
                "dataset_count": { "type": "integer" },
                "factor_count": { "type": "integer" },
                "label_count": { "type": "integer" },
                "entity_key_count": { "type": "integer" },
                "data_quality_score": { "type": "number" },
                "data_quality_status": { "type": "string", "enum": ["empty", "ready", "watch", "blocked"] },
                "online_ready_count": { "type": "integer" },
                "rule_convertible_count": { "type": "integer" },
                "mapped_factor_count": { "type": "integer" },
                "high_missing_count": { "type": "integer" },
                "unstable_factor_count": { "type": "integer" },
                "unowned_factor_count": { "type": "integer" },
                "ready_factor_count": { "type": "integer" },
                "review_factor_count": { "type": "integer" },
                "readiness_issue_counts": {
                    "type": "object",
                    "additionalProperties": { "type": "integer" }
                },
                "scheme_readiness": { "type": "array", "items": { "$ref": "#/components/schemas/FactorSchemeReadiness" } },
                "factor_cards": { "type": "array", "items": { "$ref": "#/components/schemas/FactorCard" } }
            }
        },
        "FactorSchemeReadiness": {
            "type": "object",
            "required": ["scheme_family", "factor_count", "ready_factor_count", "review_factor_count", "online_ready_count", "rule_convertible_count", "readiness_issue_counts"],
            "properties": {
                "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                "factor_count": { "type": "integer" },
                "ready_factor_count": { "type": "integer" },
                "review_factor_count": { "type": "integer" },
                "online_ready_count": { "type": "integer" },
                "rule_convertible_count": { "type": "integer" },
                "readiness_issue_counts": {
                    "type": "object",
                    "additionalProperties": { "type": "integer" }
                }
            }
        },
        "FactorCard": {
            "type": "object",
            "required": ["dataset_id", "dataset_key", "dataset_version", "factor_name", "scheme_family", "chinese_name", "entity_type", "semantic_role", "logical_type", "calculation_window", "calculation_logic", "source_table", "source_fields", "business_meaning", "risk_direction", "missing_rate", "iv", "auc_gain", "lift", "psi", "stability", "model_contribution", "rule_convertible", "online_available", "readiness_status", "readiness_issues", "version", "owner", "is_label", "is_entity_key", "evidence_refs"],
            "properties": {
                "dataset_id": { "type": "string" },
                "dataset_key": { "type": "string" },
                "dataset_version": { "type": "string" },
                "factor_name": { "type": "string" },
                "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                "chinese_name": { "type": "string" },
                "entity_type": { "type": "string" },
                "semantic_role": { "type": "string" },
                "logical_type": { "type": "string" },
                "calculation_window": { "type": "string" },
                "calculation_logic": { "type": "string" },
                "source_table": { "type": "string" },
                "source_fields": { "type": "array", "items": { "type": "string" } },
                "business_meaning": { "type": "string" },
                "risk_direction": { "type": "string" },
                "missing_rate": { "type": ["number", "null"] },
                "iv": { "type": ["number", "null"] },
                "auc_gain": { "type": ["number", "null"] },
                "lift": { "type": ["number", "null"] },
                "psi": { "type": ["number", "null"] },
                "stability": { "type": "string", "enum": ["unmeasured", "stable", "watch", "drift"] },
                "model_contribution": { "type": ["number", "null"] },
                "rule_convertible": { "type": "boolean" },
                "online_available": { "type": "boolean" },
                "readiness_status": { "type": "string", "enum": ["ready", "needs_review"] },
                "readiness_issues": { "type": "array", "items": { "type": "string" } },
                "version": { "type": "string" },
                "owner": { "type": "string" },
                "is_label": { "type": "boolean" },
                "is_entity_key": { "type": "boolean" },
                "evidence_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string", "minLength": 1 }
                }
            }
        }
    })
}
