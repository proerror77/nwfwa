use super::model_lifecycle_request_body;
use serde_json::{json, Map, Value};

#[path = "openapi_paths_model_ops_providers.rs"]
mod openapi_paths_model_ops_providers;

pub(super) fn model_ops_paths() -> Value {
    let mut paths = Map::new();
    append_paths(
        &mut paths,
        openapi_paths_model_ops_providers::provider_paths(),
    );
    append_paths(
        &mut paths,
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
            "/api/v1/ops/model-retraining-jobs/{job_id}/status": {
                "post": {
                    "summary": "Update model retraining job status",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "job_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/UpdateModelRetrainingJobStatusRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Updated model retraining job",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelRetrainingJob" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/model-retraining-jobs/claim-next": {
                "post": {
                    "summary": "Claim the next queued model retraining job for a worker",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ClaimModelRetrainingJobRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Claimed model retraining job",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelRetrainingJob" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/model-retraining-jobs/{job_id}/output": {
                "post": {
                    "summary": "Register external training output, candidate model, validation evaluation, and mined rule candidates",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "job_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/CompleteModelRetrainingJobRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Completed model retraining job output and saved mined rule candidates",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/CompleteModelRetrainingJobResponse" }
                                }
                            }
                        }
                    }
                }
            },
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
        }),
    );
    Value::Object(paths)
}

fn append_paths(target: &mut Map<String, Value>, paths: Value) {
    let Value::Object(paths) = paths else {
        unreachable!("OpenAPI model ops path group must be a JSON object");
    };
    target.extend(paths);
}
