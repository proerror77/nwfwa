use serde_json::{json, Value};

pub(super) fn mlops_paths() -> Value {
    json!({
        "/api/v1/ops/models/{model_key}/mlops-monitoring-review-queue": {
            "get": {
                "summary": "List human review tasks opened by submitted MLOps monitoring reports",
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
                        "description": "MLOps monitoring review queue",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelMonitoringReviewQueueResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/models/{model_key}/mlops-monitoring-review-tasks/{task_id}/reviews": {
            "post": {
                "summary": "Record a human decision for an MLOps monitoring review task",
                "security": [{ "ApiKeyAuth": [] }],
                "parameters": [
                    {
                        "name": "model_key",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "task_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/SubmitModelMonitoringReviewTaskReviewRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Recorded monitoring review task decision",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelMonitoringReviewTaskReviewResponse" }
                            }
                        }
                    },
                    "400": {
                        "description": "Invalid decision or missing evidence refs",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                            }
                        }
                    },
                    "404": {
                        "description": "Monitoring review task not found",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/models/{model_key}/mlops-monitoring-reports": {
            "post": {
                "summary": "Submit a Rust MLOps monitoring report into governance audit",
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
                            "schema": { "$ref": "#/components/schemas/SubmitMlopsMonitoringReportRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Recorded MLOps monitoring report governance event",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SubmitMlopsMonitoringReportResponse" }
                            }
                        }
                    },
                    "400": {
                        "description": "Invalid monitoring report submission",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/models/{model_key}/mlops-alert-deliveries": {
            "post": {
                "summary": "Submit Rust MLOps alert-router delivery evidence into governance audit",
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
                            "schema": { "$ref": "#/components/schemas/SubmitMlopsAlertDeliveryRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Recorded MLOps alert delivery governance event",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SubmitMlopsAlertDeliveryResponse" }
                            }
                        }
                    },
                    "400": {
                        "description": "Invalid alert delivery submission",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/models/{model_key}/mlops-alert-delivery-queue": {
            "get": {
                "summary": "List alert delivery tasks opened by submitted MLOps scheduler reports",
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
                        "description": "MLOps alert delivery queue",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/MlopsAlertDeliveryQueueResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/api/v1/ops/models/{model_key}/mlops-alert-delivery-tasks/{task_id}/reviews": {
            "post": {
                "summary": "Record a human receipt or escalation decision for an MLOps alert delivery task",
                "security": [{ "ApiKeyAuth": [] }],
                "parameters": [
                    {
                        "name": "model_key",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "task_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/SubmitMlopsAlertDeliveryTaskReviewRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Recorded alert delivery task review",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/MlopsAlertDeliveryTaskReviewResponse" }
                            }
                        }
                    },
                    "400": {
                        "description": "Invalid decision or missing evidence refs",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                            }
                        }
                    },
                    "404": {
                        "description": "Alert delivery task not found",
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
