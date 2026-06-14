use serde_json::{json, Value};

pub(super) fn model_core_paths() -> Value {
    json!({
        "/api/v1/ops/models/{model_key}/performance": {
            "get": {
                "summary": "Get model performance metrics",
                "security": [{ "ApiKeyAuth": [] }],
                "parameters": [
                    {
                        "name": "model_key",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Model performance metrics",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelPerformanceResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/models/{model_key}/promotion-gates": {
            "get": {
                "summary": "Get model promotion gates before routing impact",
                "security": [{ "ApiKeyAuth": [] }],
                "parameters": [
                    {
                        "name": "model_key",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Model promotion gate summary",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelPromotionGatesResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/models/{model_key}/versions/{model_version}/promotion-gates": {
            "get": {
                "summary": "Get promotion gates for an explicit model version before routing impact",
                "security": [{ "ApiKeyAuth": [] }],
                "parameters": [
                    {
                        "name": "model_key",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "model_version",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Version-scoped model promotion gate summary",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelPromotionGatesResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/models/{model_key}/retraining-readiness": {
            "get": {
                "summary": "Get model retraining readiness from drift, labels, feedback, and source data quality",
                "security": [{ "ApiKeyAuth": [] }],
                "parameters": [
                    {
                        "name": "model_key",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Model retraining readiness summary",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelRetrainingReadinessResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/models/{model_key}/retraining-jobs": {
            "get": {
                "summary": "List model retraining jobs",
                "security": [{ "ApiKeyAuth": [] }],
                "parameters": [
                    {
                        "name": "model_key",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Model retraining jobs",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelRetrainingJobListResponse" }
                            }
                        }
                    }
                }
            },
            "post": {
                "summary": "Queue a model retraining job from readiness",
                "security": [{ "ApiKeyAuth": [] }],
                "parameters": [
                    {
                        "name": "model_key",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/CreateModelRetrainingJobRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Queued model retraining job",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelRetrainingJob" }
                            }
                        }
                    }
                }
            }
        },
    })
}
