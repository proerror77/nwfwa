use serde_json::{json, Value};

pub(super) fn dataset_paths() -> Value {
    json!({
        "/api/v1/ops/datasets": {
            "get": {
                "summary": "List registered external datasets",
                "security": [{ "ApiKeyAuth": [] }],
                "responses": {
                    "200": {
                        "description": "Dataset catalog entries",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/DatasetListResponse" }
                            }
                        }
                    }
                }
            },
            "post": {
                "summary": "Register a governed Parquet dataset",
                "security": [{ "ApiKeyAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/DatasetRegistrationRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Registered dataset",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/DatasetRecord" }
                            }
                        }
                    },
                    "400": {
                        "description": "Only parquet datasets can be registered",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/datasets/{dataset_id}": {
            "get": {
                "summary": "Get external dataset catalog detail",
                "security": [{ "ApiKeyAuth": [] }],
                "parameters": [
                    {
                        "name": "dataset_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Dataset catalog detail",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/DatasetRecord" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/datasets/{dataset_id}/mappings": {
            "post": {
                "summary": "Add an external field mapping for a dataset",
                "security": [{ "ApiKeyAuth": [] }],
                "parameters": [
                    {
                        "name": "dataset_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/FieldMappingRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Created field mapping",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/FieldMappingResponse" }
                            }
                        }
                    }
                }
            }
        },
    })
}
