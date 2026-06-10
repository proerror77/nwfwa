use serde_json::{json, Value};

pub(super) fn retraining_paths() -> Value {
    json!({
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
    })
}
