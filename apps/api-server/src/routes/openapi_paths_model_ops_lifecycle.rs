use super::super::model_lifecycle_request_body;
use serde_json::{json, Value};

pub(super) fn lifecycle_paths() -> Value {
    json!({
        "/api/v1/ops/models/{model_key}/promotion-reviews": {
            "post": {
                "summary": "Record a model promotion review decision",
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
                            "schema": { "$ref": "#/components/schemas/SubmitModelPromotionReviewRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Recorded model promotion review",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelPromotionReview" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/models/{model_key}/versions/{model_version}/promotion-reviews": {
            "post": {
                "summary": "Record a model promotion review decision for an explicit model version",
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
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/SubmitModelPromotionReviewRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Recorded version-scoped model promotion review",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelPromotionReview" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/models/{model_key}/activate": {
            "post": {
                "summary": "Activate the latest governed model version for production routing",
                "security": [{ "ApiKeyAuth": [] }],
                "parameters": [
                    {
                        "name": "model_key",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "requestBody": model_lifecycle_request_body(),
                "responses": {
                    "200": {
                        "description": "Model lifecycle status after activation",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelLifecycleResponse" }
                            }
                        }
                    },
                    "409": {
                        "description": "Model activation is blocked by governance gates",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/models/{model_key}/versions/{model_version}/activate": {
            "post": {
                "summary": "Activate an explicit governed model version for production routing",
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
                "requestBody": model_lifecycle_request_body(),
                "responses": {
                    "200": {
                        "description": "Model lifecycle status after version-scoped activation",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelLifecycleResponse" }
                            }
                        }
                    },
                    "409": {
                        "description": "Model activation is blocked by governance gates",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/models/{model_key}/rollback": {
            "post": {
                "summary": "Roll back an active model to the previous active version",
                "security": [{ "ApiKeyAuth": [] }],
                "parameters": [
                    {
                        "name": "model_key",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "requestBody": model_lifecycle_request_body(),
                "responses": {
                    "200": {
                        "description": "Model lifecycle status after rollback",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelLifecycleResponse" }
                            }
                        }
                    }
                }
            }
        },
    })
}
