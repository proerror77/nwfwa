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
    })
}
