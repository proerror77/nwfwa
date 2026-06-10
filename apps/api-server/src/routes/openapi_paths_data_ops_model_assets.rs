use serde_json::{json, Value};

pub(super) fn model_asset_paths() -> Value {
    json!({
        "/api/v1/ops/feature-sets": {
            "post": {
                "summary": "Register a Parquet feature set version",
                "security": [{ "ApiKeyAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/FeatureSetRegistrationRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Registered feature set",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/FeatureSet" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/factors/readiness": {
            "get": {
                "summary": "Summarize factor factory readiness across registered datasets",
                "security": [{ "ApiKeyAuth": [] }],
                "responses": {
                    "200": {
                        "description": "Factor readiness summary",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/FactorReadinessResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/model-datasets": {
            "post": {
                "summary": "Register a model dataset version",
                "security": [{ "ApiKeyAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/ModelDatasetRegistrationRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Registered model dataset",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelDataset" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/model-evaluations": {
            "get": {
                "summary": "List model evaluation metrics",
                "security": [{ "ApiKeyAuth": [] }],
                "responses": {
                    "200": {
                        "description": "Model evaluation metric list",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelEvaluationListResponse" }
                            }
                        }
                    }
                }
            },
            "post": {
                "summary": "Register model evaluation metrics for a model dataset",
                "security": [{ "ApiKeyAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/ModelEvaluationRegistrationRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Registered model evaluation",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelEvaluationResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/model-evaluations/{evaluation_run_id}": {
            "get": {
                "summary": "Get model evaluation metrics",
                "security": [{ "ApiKeyAuth": [] }],
                "parameters": [
                    {
                        "name": "evaluation_run_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Model evaluation metrics"
                    }
                }
            }
        },
    })
}
