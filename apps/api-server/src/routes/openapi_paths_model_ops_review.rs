use serde_json::{json, Value};

pub(super) fn review_paths() -> Value {
    json!({
        "/api/v1/ops/medical-review/queue": {
            "get": {
                "summary": "List claims that require medical review from clinical evidence audit events",
                "security": [{ "ApiKeyAuth": [] }],
                "parameters": [
                    {
                        "name": "limit",
                        "in": "query",
                        "required": false,
                        "schema": { "type": "integer", "minimum": 1, "maximum": 200 }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Medical review queue",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/MedicalReviewQueueResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/medical-review/results": {
            "post": {
                "summary": "Record a medical review result with evidence references",
                "security": [{ "ApiKeyAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/SubmitMedicalReviewResultRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Medical review result recorded",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/MedicalReviewResultResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/fwa-schemes": {
            "get": {
                "summary": "List governed FWA scheme taxonomy and evidence requirements",
                "security": [{ "ApiKeyAuth": [] }],
                "responses": {
                    "200": {
                        "description": "FWA scheme taxonomy",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/FwaSchemeListResponse" }
                            }
                        }
                    },
                    "401": {
                        "description": "Missing or invalid API key",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                            }
                        }
                    }
                }
            }
        },
    })
}
